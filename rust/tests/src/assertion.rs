use crate::{
    normalize_generated_ids, normalize_golden_output, normalize_sat_solver_names, readable_diff,
};
use bsc_test_plan::{
    Action as PlanAction, DiagnosticKind as PlanDiagnosticKind, GoldenNormalization,
};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlanAssertionFailure {
    Infrastructure(String),
    ContractMismatch(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComparisonMode {
    Exact,
    Golden,
    Verilog,
}

fn count_diagnostics(output: &str, kind: PlanDiagnosticKind, code: Option<&str>) -> usize {
    let marker = match kind {
        PlanDiagnosticKind::Error => "Error:",
        PlanDiagnosticKind::Warning => "Warning:",
    };
    output
        .lines()
        .filter(|raw_line| {
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
            let Some(code_start) = line.rfind('(') else {
                return false;
            };
            if !line.ends_with(')') {
                return false;
            }
            let found_code = &line[code_start + 1..line.len() - 1];
            code.map_or(found_code.len() == 5, |expected| found_code == expected)
                && line
                    .find(marker)
                    .is_some_and(|start| start + marker.len() < code_start)
        })
        .count()
}

pub(crate) fn check_plan_assertion_typed(
    action: &PlanAction,
    actual_root: &Path,
    expected_root: &Path,
    artifact_dir: &Path,
    context: &str,
) -> Result<(), PlanAssertionFailure> {
    let PlanAction::AssertDiagnosticCount {
        path,
        kind,
        code,
        count,
    } = action
    else {
        return check_plan_assertion(action, actual_root, expected_root, artifact_dir, context)
            .map_err(classify_assertion_failure);
    };
    let artifact = actual_root.join(path);
    let actual = fs::read_to_string(&artifact)
        .map(|text| text.replace("\r\n", "\n").replace('\r', "\n"))
        .map_err(|error| {
            PlanAssertionFailure::Infrastructure(format!(
                "read asserted artifact {} for {context}: {error}",
                artifact.display()
            ))
        })?;
    let found = count_diagnostics(&actual, *kind, code.as_deref());
    if found == *count {
        return Ok(());
    }
    let code = code.as_deref().unwrap_or("any code");
    Err(PlanAssertionFailure::ContractMismatch(format!(
        "expected {} diagnostics for {code} in {}, found {found}",
        count,
        artifact.display()
    )))
}

fn classify_assertion_failure(message: String) -> PlanAssertionFailure {
    if message.starts_with("read ")
        || message.starts_with("write ")
        || message.starts_with("create ")
    {
        PlanAssertionFailure::Infrastructure(message)
    } else {
        PlanAssertionFailure::ContractMismatch(message)
    }
}

pub(crate) fn check_plan_assertion(
    action: &PlanAction,
    actual_root: &Path,
    expected_root: &Path,
    artifact_dir: &Path,
    context: &str,
) -> Result<(), String> {
    let read_text = |path: &str| {
        let path = actual_root.join(path);
        fs::read_to_string(&path)
            .map(|text| text.replace("\r\n", "\n").replace('\r', "\n"))
            .map_err(|error| {
                format!(
                    "read asserted artifact {} for {context}: {error}",
                    path.display()
                )
            })
    };
    match action {
        PlanAction::AssertExists { path } => {
            let path = actual_root.join(path);
            path.is_file()
                .then_some(())
                .ok_or_else(|| format!("expected artifact {} for {context}", path.display()))
        }
        PlanAction::AssertTextContains { path, text } => {
            let actual = read_text(path)?;
            actual.contains(text).then_some(()).ok_or_else(|| {
                format!(
                    "expected {} to contain {text:?}",
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertTextAbsent { path, text } => {
            let actual = read_text(path)?;
            (!actual.contains(text)).then_some(()).ok_or_else(|| {
                format!(
                    "expected {} not to contain {text:?}",
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertRegex { path, pattern } => {
            let actual = read_text(path)?;
            let regex = compile_multiline_regex(pattern)?;
            regex.is_match(&actual).then_some(()).ok_or_else(|| {
                format!(
                    "expected {} to match regex {pattern:?}",
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertRegexAbsent { path, pattern } => {
            let actual = read_text(path)?;
            let matches = compile_multiline_regex(pattern)?.find_iter(&actual).count();
            (matches == 0).then_some(()).ok_or_else(|| {
                format!(
                    "expected {} not to match regex {pattern:?}, found {matches} matches",
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertTextCount { path, text, count } => {
            let actual = read_text(path)?;
            let found = actual.lines().filter(|line| line.contains(text)).count();
            (found == *count).then_some(()).ok_or_else(|| {
                format!(
                    "expected {} to contain {count} lines with {text:?}, found {found}",
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertRegexCount {
            path,
            pattern,
            count,
        } => {
            let actual = read_text(path)?;
            let found = compile_multiline_regex(pattern)?.find_iter(&actual).count();
            (found == *count).then_some(()).ok_or_else(|| {
                format!(
                    "expected {} to match {pattern:?} {count} times, found {found}",
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertDiagnosticCount {
            path,
            kind,
            code,
            count,
        } => {
            let actual = read_text(path)?;
            let kind = match kind {
                PlanDiagnosticKind::Error => PlanDiagnosticKind::Error,
                PlanDiagnosticKind::Warning => PlanDiagnosticKind::Warning,
            };
            let found = count_diagnostics(&actual, kind, code.as_deref());
            (found == *count).then_some(()).ok_or_else(|| {
                let code = code.as_deref().unwrap_or("any code");
                format!(
                    "expected {} diagnostics for {code} in {}, found {found}",
                    count,
                    actual_root.join(path).display()
                )
            })
        }
        PlanAction::AssertGolden { actual, expected } => compare_artifacts(
            &actual_root.join(actual),
            &expected_root.join(expected),
            ComparisonMode::Golden,
            &artifact_dir.join("plan-golden.diff"),
        ),
        PlanAction::AssertGoldenMissingXfail {
            actual,
            expected,
            reason,
        } => assert_missing_golden_xfail(
            &actual_root.join(actual),
            &expected_root.join(expected),
            reason,
        ),
        PlanAction::AssertGoldenAny { actual, expected } => compare_any_golden(
            &actual_root.join(actual),
            expected
                .iter()
                .map(|path| expected_root.join(path))
                .collect::<Vec<_>>(),
            artifact_dir,
        ),
        PlanAction::AssertGoldenNative { actual, expected } => compare_native_golden(
            &actual_root.join(actual),
            &expected_root.join(expected),
            &artifact_dir.join("plan-golden-native.diff"),
        ),
        PlanAction::AssertGoldenNormalized {
            actual,
            expected,
            normalizations,
        } => compare_normalized_golden(
            &actual_root.join(actual),
            &expected_root.join(expected),
            &artifact_dir.join("plan-golden-normalized.diff"),
            normalizations,
        ),
        PlanAction::AssertGoldenSortedLines { actual, expected } => compare_sorted_line_golden(
            &actual_root.join(actual),
            &expected_root.join(expected),
            &artifact_dir.join("plan-golden-sorted-lines.diff"),
        ),
        PlanAction::AssertGoldenXfail {
            actual,
            expected,
            reason,
        } => compare_xfail_golden(
            &actual_root.join(actual),
            &expected_root.join(expected),
            &artifact_dir.join("plan-golden-xfail.diff"),
            reason,
        ),
        PlanAction::AssertVerilog { actual, expected } => compare_artifacts(
            &actual_root.join(actual),
            &expected_root.join(expected),
            ComparisonMode::Verilog,
            &artifact_dir.join("plan-verilog.diff"),
        ),
        PlanAction::AssertVcd { actual, expected } => {
            let actual = actual_root.join(actual);
            let expected = expected_root.join(expected);
            crate::vcd::validate(&actual)?;
            crate::vcd::validate(&expected)?;
            compare_artifacts(
                &actual,
                &expected,
                ComparisonMode::Exact,
                &artifact_dir.join("plan-vcd.diff"),
            )
        }
        PlanAction::AssertVcdValid { path } => crate::vcd::validate(&actual_root.join(path)),
        PlanAction::AssertVcdValidIfPresent { path } => {
            let path = actual_root.join(path);
            if path.is_file() {
                crate::vcd::validate(&path)
            } else {
                Ok(())
            }
        }
        action => Err(format!(
            "{} is not an assertion operation",
            plan_action_name(action)
        )),
    }
}

fn plan_action_name(action: &PlanAction) -> &'static str {
    match action {
        PlanAction::BscCompile { .. } => "bsc.compile",
        PlanAction::BscOptions { .. } => "bsc.options",
        PlanAction::BscFlagPreflight { .. } => "bsc.flag_preflight",
        PlanAction::BluetclRun { .. } => "bluetcl.run",
        PlanAction::MakeTestData => "upstream.make_test_data",
        PlanAction::InterraOperatorVectors { .. } => "fixture.interra_operator_vectors",
        PlanAction::Bsc2Bsv { .. } => "internal.bsc2bsv",
        PlanAction::BscParsePretty { .. } => "bsc.parse_pretty_roundtrip",
        PlanAction::DumpIntermediate { .. } => "internal.dump",
        PlanAction::RenderGolden { .. } => "golden.render",
        PlanAction::M4CurdirRender { .. } => "template.m4_curdir",
        PlanAction::TextNormalize { .. } => "text.normalize",
        PlanAction::VerilogFilter { .. } => "verilog.filter",
        PlanAction::BscGenerate { .. } => "bsc.generate",
        PlanAction::BscSimirExport { .. } => "bsc.simir_export",
        PlanAction::SimirM0Step { .. } => "simir.m0_step",
        PlanAction::SimirM2Run { .. } => "simir.m2_run",
        PlanAction::CObjectBuild { .. } => "c.compile_object",
        PlanAction::BscLink { .. } => "bsc.link",
        PlanAction::BscSystemcLink { .. } => "bsc.systemc_link",
        PlanAction::SystemcCxxLink { .. } => "systemc.cxx_link",
        PlanAction::SystemcRun { .. } => "systemc.run",
        PlanAction::SimulationRun { .. } => "simulation.run",
        PlanAction::ShowRules { .. } => "vcd.showrules",
        PlanAction::VcdCheck { .. } => "vcd.check",
        PlanAction::FsCopy { .. } => "fs.copy",
        PlanAction::FsCopyReplace { .. } => "fs.copy_replace",
        PlanAction::FsMove { .. } => "fs.move",
        PlanAction::FsRemove { .. } => "fs.remove",
        PlanAction::FsEnsureAbsent { .. } => "fs.ensure_absent",
        PlanAction::FsEnsureDirectoryAbsent { .. } => "fs.ensure_dir_absent",
        PlanAction::FsMkdir { .. } => "fs.mkdir",
        PlanAction::FsCreateDirAll { .. } => "fs.create_dir_all",
        PlanAction::FsTouch { .. } => "fs.touch",
        PlanAction::FsTouchCreate { .. } => "fs.touch_create",
        PlanAction::FsRemoveUserRead { .. } => "fs.remove_user_read",
        PlanAction::FsRewriteDarwinCppIncludePath { .. } => "fs.rewrite_darwin_cpp_include_path",
        PlanAction::FsMoveReplace { .. } => "fs.move_replace",
        PlanAction::Delay { .. } => "time.delay",
        PlanAction::AssertExists { .. } => "assert.exists",
        PlanAction::AssertTextContains { .. } => "assert.text_contains",
        PlanAction::AssertTextAbsent { .. } => "assert.text_absent",
        PlanAction::AssertRegex { .. } => "assert.regex",
        PlanAction::AssertRegexAbsent { .. } => "assert.regex_absent",
        PlanAction::AssertTextCount { .. } => "assert.text_count",
        PlanAction::AssertRegexCount { .. } => "assert.regex_count",
        PlanAction::AssertDiagnosticCount { .. } => "assert.diagnostic_count",
        PlanAction::AssertGolden { .. } => "assert.golden",
        PlanAction::AssertGoldenMissingXfail { .. } => "assert.golden_missing_xfail",
        PlanAction::AssertGoldenAny { .. } => "assert.golden_any",
        PlanAction::AssertGoldenNative { .. } => "assert.golden_native",
        PlanAction::AssertGoldenNormalized { .. } => "assert.golden_normalized",
        PlanAction::AssertGoldenSortedLines { .. } => "assert.golden_sorted_lines",
        PlanAction::AssertGoldenXfail { .. } => "assert.golden_xfail",
        PlanAction::AssertVerilog { .. } => "assert.verilog",
        PlanAction::AssertVcd { .. } => "assert.vcd",
        PlanAction::AssertVcdValid { .. } => "assert.vcd_valid",
        PlanAction::AssertVcdValidIfPresent { .. } => "assert.vcd_valid_if_present",
    }
}

fn compare_any_golden(
    actual_path: &Path,
    expected_paths: Vec<PathBuf>,
    artifact_dir: &Path,
) -> Result<(), String> {
    let actual = fs::read(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    let actual = String::from_utf8(actual).map_err(|error| {
        format!(
            "actual artifact {} is not UTF-8: {error}",
            actual_path.display()
        )
    })?;
    let actual = normalize_golden_output(&actual);
    let mut mismatches = Vec::new();
    for (index, expected_path) in expected_paths.iter().enumerate() {
        let expected = fs::read_to_string(expected_path)
            .map_err(|error| format!("read golden {}: {error}", expected_path.display()))?;
        let expected = normalize_golden_output(&expected);
        if actual == expected {
            return Ok(());
        }
        let diff_path = artifact_dir.join(format!("plan-golden-any-{index}.diff"));
        let diff = readable_diff(
            &expected,
            &actual,
            &expected_path.display().to_string(),
            &actual_path.display().to_string(),
        );
        fs::write(&diff_path, diff).map_err(|error| {
            format!(
                "write golden alternative diff {}: {error}",
                diff_path.display()
            )
        })?;
        mismatches.push(format!(
            "{} ({})",
            expected_path.display(),
            diff_path.display()
        ));
    }
    Err(format!(
        "{} matches none of {} golden alternatives: {}",
        actual_path.display(),
        expected_paths.len(),
        mismatches.join(", ")
    ))
}

fn compare_normalized_golden(
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
    normalizations: &[GoldenNormalization],
) -> Result<(), String> {
    let actual = fs::read_to_string(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    compare_golden_output_with(&actual, expected_path, actual_path, diff_path, |text| {
        normalizations
            .iter()
            .try_fold(text.to_owned(), |normalized, normalization| {
                Ok(match normalization {
                    GoldenNormalization::GeneratedIds => normalize_generated_ids(&normalized),
                    GoldenNormalization::SatSolverNames => normalize_sat_solver_names(&normalized),
                    GoldenNormalization::VrWireIds => normalize_vr_wire_ids(&normalized),
                    GoldenNormalization::PreludePositions => {
                        normalize_prelude_positions(&normalized)
                    }
                    GoldenNormalization::PreludeBsvLineNumbers => {
                        normalize_prelude_bsv_line_numbers(&normalized)
                    }
                    GoldenNormalization::CompilerBannerLines => {
                        normalize_compiler_banner_lines(&normalized)
                    }
                    GoldenNormalization::WorkspaceRoot => normalize_workspace_root(&normalized),
                    GoldenNormalization::LineDirectivePositions => {
                        normalize_line_directive_positions(&normalized)
                    }
                    GoldenNormalization::BluetclOutput => normalize_bluetcl_output(&normalized),
                    GoldenNormalization::BluetclPositionDigits => {
                        normalize_bluetcl_position_digits(&normalized)
                    }
                    GoldenNormalization::BluetclCregPositions => {
                        normalize_bluetcl_creg_positions(&normalized)
                    }
                    GoldenNormalization::BluetclLibraries => {
                        normalize_bluetcl_library_span(&normalized, "Libraries")
                    }
                    GoldenNormalization::BluetclPreludeLibrary => {
                        normalize_bluetcl_library_span(&normalized, "Prelude")
                    }
                    GoldenNormalization::BracketedTimes => normalize_bracketed_times(&normalized),
                    GoldenNormalization::SplitIfRules => {
                        return normalize_split_if_rules(&normalized)
                    }
                    GoldenNormalization::SystemVerilogTaskDiagnostics => {
                        normalize_system_verilog_task_diagnostics(&normalized)
                    }
                })
            })
    })
}

fn normalize_system_verilog_task_diagnostics(text: &str) -> String {
    static PREFIX: OnceLock<Regex> = OnceLock::new();
    let prefix = PREFIX.get_or_init(|| {
        Regex::new(r"^(?:ERROR|FATAL):.*: ").expect("audited SystemVerilog task diagnostic regex")
    });
    let mut normalized = String::new();
    for line in text.lines() {
        let space_trimmed = line.trim_start_matches(' ');
        if space_trimmed.starts_with("Time:") || space_trimmed.starts_with("Scope:") {
            continue;
        }
        normalized.push_str(prefix.replace(line, "").as_ref());
        normalized.push('\n');
    }
    normalized
}

fn normalize_bluetcl_output(text: &str) -> String {
    const MACOS_WARNING: [&str; 3] = [
        "WARNING: This version of tcl is included in macOS for compatibility with legacy software.",
        "In future versions of macOS the tcl runtime will not be available by",
        "default, and may require you to install an additional package.",
    ];

    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut filtered = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("Welcome") || line.starts_with("Version") {
            index += 1;
            continue;
        }
        if index + MACOS_WARNING.len() <= lines.len()
            && lines[index..index + MACOS_WARNING.len()]
                .iter()
                .zip(MACOS_WARNING)
                .all(|(actual, expected)| actual.trim_end() == expected)
        {
            index += MACOS_WARNING.len();
            if lines.get(index).is_some_and(|line| line.is_empty()) {
                index += 1;
            }
            continue;
        }
        filtered.push(line);
        index += 1;
    }
    filtered.join("\n")
}

fn normalize_bluetcl_position_digits(text: &str) -> String {
    normalize_bluetcl_installed_library_positions(text)
        .lines()
        .map(|line| {
            if is_bluetcl_position_line(line) && (line.contains('%') || line.contains("{Library "))
            {
                line.chars()
                    .map(|character| {
                        if character.is_ascii_digit() {
                            'N'
                        } else {
                            character
                        }
                    })
                    .collect()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_bluetcl_installed_library_positions(text: &str) -> String {
    static INSTALLED_POSITION: OnceLock<Regex> = OnceLock::new();
    let installed_position = INSTALLED_POSITION.get_or_init(|| {
        Regex::new(
            r"(?P<lead>\{|= )(?P<file>[A-Za-z_][A-Za-z0-9_]*\.bsv?) (?P<line>[0-9]+) (?P<column>[0-9]+) \{Library (?P<library>[A-Za-z_][A-Za-z0-9_]*)\}",
        )
        .expect("valid closed Bluetcl installed-library position normalization")
    });
    text.lines()
        .map(|line| {
            if !is_bluetcl_position_line(line) || !line.contains("{Library ") {
                return line.to_owned();
            }
            installed_position
                .replace_all(line, |captures: &regex::Captures<'_>| {
                    let file = &captures["file"];
                    let stem = file
                        .strip_suffix(".bsv")
                        .or_else(|| file.strip_suffix(".bs"));
                    if stem != Some(&captures["library"]) {
                        return captures[0].to_owned();
                    }
                    format!(
                        "{}%/Libraries/{} {} {} {{Library {}}}",
                        &captures["lead"],
                        file,
                        &captures["line"],
                        &captures["column"],
                        &captures["library"]
                    )
                })
                .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_bluetcl_position_line(line: &str) -> bool {
    line.trim_start().starts_with("position ")
        || line.contains("(position)")
        || line.contains("{position ")
}

fn normalize_bluetcl_creg_positions(text: &str) -> String {
    Regex::new(r"CReg[0-9]+")
        .expect("valid closed CReg normalization")
        .replace_all(text, "CRegNNNN")
        .into_owned()
}

fn normalize_bluetcl_library_span(text: &str, prefix: &str) -> String {
    let canonical = normalize_bluetcl_installed_library_positions(text);
    Regex::new(&format!(r"{}.*Library", regex::escape(prefix)))
        .expect("valid closed Bluetcl library normalization")
        .replace_all(&canonical, "IGNORED")
        .into_owned()
}

fn normalize_bracketed_times(text: &str) -> String {
    // Mirrors the audited upstream sed shape `s/\[.*\]/\[TIME\]/g`: the first
    // greedy bracketed span on each line collapses to the `[TIME]` sentinel.
    text.lines()
        .map(|line| match (line.find('['), line.rfind(']')) {
            (Some(open), Some(close)) if open < close => {
                format!("{}[TIME]{}", &line[..open], &line[close + 1..])
            }
            _ => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_split_if_rules(text: &str) -> Result<String, String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if !normalized.lines().any(|line| line.contains("imod rules")) {
        let mut lines = normalized
            .lines()
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if lines.is_empty() || lines.iter().any(|line| !line.starts_with("when ")) {
            return Err("split-if canonical golden contains a non-rule line".to_owned());
        }
        lines.sort();
        return Ok(lines.into_iter().map(|line| format!("{line}\n")).collect());
    }

    static GENERATED_NAME: OnceLock<Regex> = OnceLock::new();
    static PRIM_NOT: OnceLock<Regex> = OnceLock::new();
    static DISPLAY_NAME: OnceLock<Regex> = OnceLock::new();
    static UNNAMED_RULE: OnceLock<Regex> = OnceLock::new();
    static RULE_LABEL: OnceLock<Regex> = OnceLock::new();

    let mut in_rules = false;
    let mut saw_interface = false;
    let mut tokens = Vec::new();
    for line in normalized.lines() {
        if !in_rules {
            if line.contains("imod rules") {
                in_rules = true;
            }
            continue;
        }
        if line.contains("imod interface") {
            saw_interface = true;
            break;
        }
        let line = GENERATED_NAME
            .get_or_init(|| Regex::new(r"v(\d+)__\w+").expect("valid split-if name regex"))
            .replace_all(line, "v$1");
        let line = PRIM_NOT
            .get_or_init(|| Regex::new(r"PrimBNot\s*").expect("valid split-if NOT regex"))
            .replace_all(&line, "NOT");
        let line = line.replace("PrimBAnd", " ").replace(['(', ')'], " ");
        let line = line.replace("·Prelude.PrimAction", " ");
        let line = DISPLAY_NAME
            .get_or_init(|| {
                Regex::new(r"Prelude.\$display#\d+").expect("valid split-if display regex")
            })
            .replace_all(&line, " ");
        let line = UNNAMED_RULE
            .get_or_init(|| Regex::new(r"^\s*RL_unnamed.*").expect("valid unnamed-rule regex"))
            .replace(&line, "");
        let line = RULE_LABEL
            .get_or_init(|| Regex::new(r#"^\s*"[_TF]+":$"#).expect("valid rule-label regex"))
            .replace(&line, "");
        tokens.extend(line.split_whitespace().map(str::to_owned));
    }
    if !saw_interface {
        return Err("split-if dump has no IModule interface marker".to_owned());
    }

    let mut first = true;
    let mut in_when = false;
    let mut clauses = Vec::<String>::new();
    let mut command = String::new();
    let mut rules = Vec::new();
    for token in tokens {
        if token == "when" {
            in_when = true;
            if !first {
                clauses.sort();
                let mut rule = "when".to_owned();
                for clause in &clauses {
                    rule.push(' ');
                    rule.push_str(clause);
                }
                rule.push_str(&command);
                rule.push('\n');
                rules.push(rule);
            }
            clauses.clear();
            continue;
        }
        if token == "==>" {
            in_when = false;
            command = " ==>".to_owned();
            continue;
        }
        if in_when {
            clauses.push(token);
        } else {
            command.push(' ');
            command.push_str(&token);
        }
        first = false;
    }
    if rules.is_empty() {
        return Err("split-if dump contains fewer than two canonicalizable rules".to_owned());
    }
    rules.sort();
    Ok(rules.concat())
}

fn normalize_vr_wire_ids(text: &str) -> String {
    static VR_WIRE_ID: OnceLock<Regex> = OnceLock::new();
    VR_WIRE_ID
        .get_or_init(|| Regex::new(r"VRWire[0-9]+").expect("valid VRWire identifier regex"))
        .replace_all(text, "VRWireNNNN")
        .into_owned()
}

fn normalize_prelude_positions(text: &str) -> String {
    static PRELUDE_POSITION: OnceLock<Regex> = OnceLock::new();
    PRELUDE_POSITION
        .get_or_init(|| {
            Regex::new(r#"(\"Prelude\.(?:bs|bsv)\", line )[0-9]+(, column )[0-9]+"#)
                .expect("valid Prelude position regex")
        })
        .replace_all(text, "${1}MMM${2}NNN")
        .into_owned()
}

fn normalize_prelude_bsv_line_numbers(text: &str) -> String {
    static PRELUDE_BSV_LINE: OnceLock<Regex> = OnceLock::new();
    PRELUDE_BSV_LINE
        .get_or_init(|| {
            Regex::new(r#"(\"PreludeBSV\.bsv\", line )[0-9]+,"#)
                .expect("valid PreludeBSV line regex")
        })
        .replace_all(text, "${1}NNNN,")
        .into_owned()
}

fn normalize_compiler_banner_lines(text: &str) -> String {
    text.lines()
        .filter(|line| !line.contains("Bluespec Compiler"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_line_directive_positions(text: &str) -> String {
    text.lines()
        .map(|line| {
            let Some(start) = line.find("`line(") else {
                return line.to_owned();
            };
            let Some(end) = line.rfind(')') else {
                return line.to_owned();
            };
            if end < start {
                return line.to_owned();
            }
            format!("{}{}{}", &line[..start], "`line(POS)", &line[end + 1..])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_workspace_root(text: &str) -> String {
    static WORKSPACE_ROOT: OnceLock<Regex> = OnceLock::new();
    WORKSPACE_ROOT
        .get_or_init(|| {
            Regex::new(r#"(?i)(?:[a-z]:)?[^\s\r\n]*?[/\\]rust-test-work[/\\]plans[/\\][^/\\\s]+"#)
                .expect("valid isolated workspace root regex")
        })
        .replace_all(text, "HERE")
        .into_owned()
}

fn assert_missing_golden_xfail(
    actual_path: &Path,
    expected_path: &Path,
    reason: &str,
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "actual artifact for missing-golden xfail is not a regular file: {}",
            actual_path.display()
        ));
    }
    match fs::symlink_metadata(expected_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(format!(
            "{reason} unexpectedly has a golden fixture at {}",
            expected_path.display()
        )),
        Err(error) => Err(format!(
            "inspect expected missing golden {}: {error}",
            expected_path.display()
        )),
    }
}

fn compare_xfail_golden(
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
    reason: &str,
) -> Result<(), String> {
    let actual = fs::read_to_string(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    let expected = fs::read_to_string(expected_path).map_err(|error| {
        format!(
            "read expected artifact {}: {error}",
            expected_path.display()
        )
    })?;
    let actual = normalize_golden_output(&actual);
    let expected = normalize_golden_output(&expected);
    if actual == expected {
        return Err(format!(
            "XPASS: {} unexpectedly matches {} ({reason})",
            actual_path.display(),
            expected_path.display()
        ));
    }
    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        &actual_path.display().to_string(),
    );
    fs::write(diff_path, diff)
        .map_err(|error| format!("write XFAIL diff {}: {error}", diff_path.display()))?;
    println!(
        "XFAIL: {} differs from {} ({reason}); see {}",
        actual_path.display(),
        expected_path.display(),
        diff_path.display()
    );
    Ok(())
}

fn compare_sorted_line_golden(
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    let actual = fs::read_to_string(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    let expected = fs::read_to_string(expected_path).map_err(|error| {
        format!(
            "read expected artifact {}: {error}",
            expected_path.display()
        )
    })?;
    let actual = sorted_golden_lines(&actual);
    let expected = sorted_golden_lines(&expected);
    if actual == expected {
        return Ok(());
    }
    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        &actual_path.display().to_string(),
    );
    write_mismatch(actual_path, expected_path, diff_path, &diff)
}

fn sorted_golden_lines(text: &str) -> String {
    let normalized = normalize_golden_output(text);
    let mut lines = normalized.lines().collect::<Vec<_>>();
    lines.sort_unstable();
    lines.join("\n")
}

fn compare_native_golden(
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    if !cfg!(windows) {
        return compare_artifacts(
            actual_path,
            expected_path,
            ComparisonMode::Golden,
            diff_path,
        );
    }

    let actual = fs::read_to_string(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    let expected = fs::read_to_string(expected_path).map_err(|error| {
        format!(
            "read expected artifact {}: {error}",
            expected_path.display()
        )
    })?;
    let actual = normalize_golden_output(&actual);
    let expected = normalize_golden_output(&expected);
    if native_golden_text_matches(&actual, &expected)? {
        return Ok(());
    }

    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        &actual_path.display().to_string(),
    );
    write_mismatch(actual_path, expected_path, diff_path, &diff)
}

pub(super) fn compare_golden_output_with(
    actual: &str,
    expected_path: &Path,
    actual_path: &Path,
    diff_path: &Path,
    normalize: impl Fn(&str) -> Result<String, String>,
) -> Result<(), String> {
    let expected = fs::read_to_string(expected_path)
        .map_err(|error| format!("read golden {}: {error}", expected_path.display()))?;
    let actual = normalize(actual)?;
    let expected = normalize(&expected)?;
    compare_normalized_text(
        &actual,
        &expected,
        ComparisonMode::Golden,
        actual_path,
        expected_path,
        diff_path,
    )
}

fn compare_artifacts(
    actual_path: &Path,
    expected_path: &Path,
    normalization: ComparisonMode,
    diff_path: &Path,
) -> Result<(), String> {
    let actual = fs::read(actual_path)
        .map_err(|error| format!("read actual artifact {}: {error}", actual_path.display()))?;
    let expected = fs::read(expected_path).map_err(|error| {
        format!(
            "read expected artifact {}: {error}",
            expected_path.display()
        )
    })?;

    if normalization == ComparisonMode::Exact {
        if actual == expected {
            return Ok(());
        }
        let diff = match (std::str::from_utf8(&expected), std::str::from_utf8(&actual)) {
            (Ok(expected), Ok(actual)) => readable_diff(
                expected,
                actual,
                &expected_path.display().to_string(),
                &actual_path.display().to_string(),
            ),
            _ => format!(
                "binary artifacts differ: expected {} bytes, actual {} bytes\n",
                expected.len(),
                actual.len()
            ),
        };
        return write_mismatch(actual_path, expected_path, diff_path, &diff);
    }

    let actual = String::from_utf8(actual).map_err(|error| {
        format!(
            "actual artifact {} is not UTF-8: {error}",
            actual_path.display()
        )
    })?;
    let expected = String::from_utf8(expected).map_err(|error| {
        format!(
            "expected artifact {} is not UTF-8: {error}",
            expected_path.display()
        )
    })?;
    compare_normalized_text(
        &actual,
        &expected,
        normalization,
        actual_path,
        expected_path,
        diff_path,
    )
}

fn compare_normalized_text(
    actual: &str,
    expected: &str,
    normalization: ComparisonMode,
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    let normalize = |text: &str| match normalization {
        ComparisonMode::Exact => text.to_owned(),
        ComparisonMode::Golden => normalize_golden_output(text),
        ComparisonMode::Verilog => {
            let without_banner = text
                .lines()
                .filter(|line| !line.contains("Bluespec Compiler"))
                .collect::<Vec<_>>()
                .join("\n");
            normalize_golden_output(&normalize_generated_ids(&without_banner))
        }
    };
    let actual = normalize(actual);
    let expected = normalize(expected);
    if actual == expected {
        return Ok(());
    }
    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        &actual_path.display().to_string(),
    );
    write_mismatch(actual_path, expected_path, diff_path, &diff)
}

fn native_golden_text_matches(actual: &str, expected: &str) -> Result<bool, String> {
    if actual == expected || decimal_text_within_last_place(actual, expected, 1)? {
        return Ok(true);
    }

    let mut actual_lines = actual.lines().collect::<Vec<_>>();
    let mut expected_lines = expected.lines().collect::<Vec<_>>();
    actual_lines.sort_unstable();
    expected_lines.sort_unstable();
    Ok(actual_lines == expected_lines)
}

fn decimal_text_within_last_place(
    actual: &str,
    expected: &str,
    max_units: u64,
) -> Result<bool, String> {
    static DECIMAL: OnceLock<Regex> = OnceLock::new();
    let decimal = DECIMAL.get_or_init(|| {
        Regex::new(r"(?P<sign>[+-]?)(?P<whole>[0-9]+)\.(?P<fraction>[0-9]+)")
            .expect("decimal token regex is valid")
    });
    let mut actual_tokens = decimal.captures_iter(actual);
    let mut expected_tokens = decimal.captures_iter(expected);
    let mut actual_end = 0;
    let mut expected_end = 0;

    loop {
        match (actual_tokens.next(), expected_tokens.next()) {
            (None, None) => return Ok(actual[actual_end..] == expected[expected_end..]),
            (Some(actual_token), Some(expected_token)) => {
                let actual_match = actual_token.get(0).expect("decimal capture has a match");
                let expected_match = expected_token.get(0).expect("decimal capture has a match");
                if actual[actual_end..actual_match.start()]
                    != expected[expected_end..expected_match.start()]
                {
                    return Ok(false);
                }
                let actual_fraction = actual_token
                    .name("fraction")
                    .expect("decimal capture has a fraction")
                    .as_str();
                let expected_fraction = expected_token
                    .name("fraction")
                    .expect("decimal capture has a fraction")
                    .as_str();
                if actual_fraction.len() != expected_fraction.len() {
                    return Ok(false);
                }
                let Ok(fractional_digits) = u8::try_from(actual_fraction.len()) else {
                    return Ok(false);
                };
                let Some(actual_value) = scaled_decimal(&actual_token, fractional_digits)? else {
                    return Ok(false);
                };
                let Some(expected_value) = scaled_decimal(&expected_token, fractional_digits)?
                else {
                    return Ok(false);
                };
                if actual_value.abs_diff(expected_value) > u128::from(max_units) {
                    return Ok(false);
                }
                actual_end = actual_match.end();
                expected_end = expected_match.end();
            }
            _ => return Ok(false),
        }
    }
}

fn scaled_decimal(
    token: &regex::Captures<'_>,
    fractional_digits: u8,
) -> Result<Option<i128>, String> {
    let fraction = token
        .name("fraction")
        .expect("decimal capture has a fraction")
        .as_str();
    if fraction.len() != usize::from(fractional_digits) {
        return Ok(None);
    }
    let whole = token
        .name("whole")
        .expect("decimal capture has a whole part")
        .as_str()
        .parse::<i128>()
        .map_err(|error| format!("parse decimal whole part: {error}"))?;
    let fraction = fraction
        .parse::<i128>()
        .map_err(|error| format!("parse decimal fractional part: {error}"))?;
    let scale = 10_i128
        .checked_pow(u32::from(fractional_digits))
        .ok_or_else(|| "decimal scale overflow".to_owned())?;
    let magnitude = whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| "decimal value overflow".to_owned())?;
    Ok(Some(
        if token.name("sign").is_some_and(|sign| sign.as_str() == "-") {
            -magnitude
        } else {
            magnitude
        },
    ))
}

fn write_mismatch(
    actual_path: &Path,
    expected_path: &Path,
    diff_path: &Path,
    diff: &str,
) -> Result<(), String> {
    fs::write(diff_path, diff)
        .map_err(|error| format!("write artifact diff {}: {error}", diff_path.display()))?;
    Err(format!(
        "{} differs from {}; see {}",
        actual_path.display(),
        expected_path.display(),
        diff_path.display()
    ))
}

fn compile_multiline_regex(pattern: &str) -> Result<Regex, String> {
    Regex::new(&format!("(?m:{pattern})"))
        .map_err(|error| format!("invalid multiline regex {pattern:?}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsc_test_plan::SimulationBackend;

    fn xfail_test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("bsc-rust-xfail-{name}-{}", crate::current_run_id()))
    }

    #[test]
    fn normalizes_only_the_closed_bluetcl_banner_and_macos_warning() {
        let input = concat!(
            "Welcome to Bluetcl\r\n",
            "Version 2026.01\r\n",
            "payload\r\n",
            "WARNING: This version of tcl is included in macOS for compatibility with legacy software. \r\n",
            "In future versions of macOS the tcl runtime will not be available by \r\n",
            "default, and may require you to install an additional package.\r\n",
            "\r\n",
            "payload Versioned\r\n",
        );
        assert_eq!(
            normalize_bluetcl_output(input),
            "payload\npayload Versioned"
        );
    }

    #[test]
    fn applies_only_the_closed_bluetcl_sed_shapes() {
        assert_eq!(
            normalize_bluetcl_position_digits(
                "position 42% x7\nposition 42 x7\nposition {Prelude.bs 583 21 {Library Prelude}}"
            ),
            "position NN% xN\nposition 42 x7\nposition {%/Libraries/Prelude.bs NNN NN {Library Prelude}}"
        );
        assert_eq!(
            normalize_bluetcl_creg_positions("CReg12 CRegx CReg3"),
            "CRegNNNN CRegx CRegNNNN"
        );
        assert_eq!(
            normalize_bluetcl_library_span("a Libraries/x Library y Library z", "Libraries"),
            "a IGNORED z"
        );
        assert_eq!(
            normalize_bluetcl_library_span(
                "X(position) = List.bs 748 4 {Library List}",
                "Libraries"
            ),
            "X(position) = %/IGNORED List}"
        );
        assert_eq!(
            normalize_bluetcl_library_span("Prelude/x Library rest", "Prelude"),
            "IGNORED rest"
        );
        assert_eq!(
            normalize_bluetcl_library_span(
                "position {Prelude.bs 1562 5 {Library Prelude}}",
                "Prelude"
            ),
            "position {%/Libraries/IGNORED Prelude}}"
        );
        assert_eq!(
            normalize_bracketed_times(
                "[Sat Aug 15 18:36:12 UTC 2026] elab progress: Elaborating module\n\
                 [TIME] elab progress: idempotent\n\
                 code generation for sysTest1 starts\n"
            ),
            "[TIME] elab progress: Elaborating module\n\
             [TIME] elab progress: idempotent\n\
             code generation for sysTest1 starts"
        );
        assert_eq!(
            normalize_bluetcl_installed_library_positions(
                "position {Local.bs 12 3 {Library Other}}\nnot_position = List.bs 748 4 {Library List}"
            ),
            "position {Local.bs 12 3 {Library Other}}\nnot_position = List.bs 748 4 {Library List}"
        );
    }

    #[test]
    fn split_if_normalization_preserves_the_upstream_missing_final_process() {
        let dump = concat!(
            "=== split-if dump:\n",
            "imod rules\n",
            "when b a ==> first\n",
            "when d c ==> second\n",
            "when f e ==> deliberately_ignored\n",
            "imod interface\n",
            "-----\n",
        );
        assert_eq!(
            normalize_split_if_rules(dump).unwrap(),
            "when a b ==> first\nwhen c d ==> second\n"
        );
        assert_eq!(
            normalize_split_if_rules("when c d ==> second\nwhen a b ==> first\n").unwrap(),
            "when a b ==> first\nwhen c d ==> second\n"
        );
        assert!(normalize_split_if_rules("imod rules\nwhen a ==> x\n").is_err());
        assert!(normalize_split_if_rules("not a canonical rule\n").is_err());
    }

    #[test]
    fn normalizes_only_prelude_bsv_line_numbers() {
        assert_eq!(
            normalize_prelude_bsv_line_numbers(
                "\"PreludeBSV.bsv\", line 42, column 7\n\"Prelude.bsv\", line 9, column 2\n",
            ),
            "\"PreludeBSV.bsv\", line NNNN, column 7\n\"Prelude.bsv\", line 9, column 2\n",
        );
    }

    #[test]
    fn system_verilog_task_diagnostics_apply_only_the_audited_filters() {
        let input = concat!(
            "Time: 10\n",
            "   Scope: main.top\n",
            "ERROR: simulator.v:12: actual error\n",
            "FATAL: prefix: with: colons: fatal text\n",
            "\tTime: tab-indented near match\n",
            " ERROR: leading-space near match\n",
            "WARNING: simulator.v:12: retained\n",
        );
        assert_eq!(
            normalize_system_verilog_task_diagnostics(input),
            concat!(
                "actual error\n",
                "fatal text\n",
                "\tTime: tab-indented near match\n",
                " ERROR: leading-space near match\n",
                "WARNING: simulator.v:12: retained\n",
            )
        );
    }

    #[test]
    fn diagnostic_assertions_distinguish_mismatch_from_infrastructure_failure() {
        let root = xfail_test_root("diagnostic");
        fs::create_dir_all(&root).unwrap();
        fs::write(&root.join("compile.out"), "Warning: message (G0010)\n").unwrap();
        let action = PlanAction::AssertDiagnosticCount {
            path: "compile.out".to_owned(),
            kind: PlanDiagnosticKind::Warning,
            code: None,
            count: 0,
        };

        assert!(matches!(
            check_plan_assertion_typed(&action, &root, &root, &root, "test"),
            Err(PlanAssertionFailure::ContractMismatch(message))
                if message.contains("expected 0 diagnostics")
        ));

        let missing_action = PlanAction::AssertDiagnosticCount {
            path: "missing.out".to_owned(),
            kind: PlanDiagnosticKind::Warning,
            code: None,
            count: 0,
        };
        assert!(matches!(
            check_plan_assertion_typed(&missing_action, &root, &root, &root, "test"),
            Err(PlanAssertionFailure::Infrastructure(message))
                if message.contains("read asserted artifact")
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_diagnostic_assertions_distinguish_mismatch_from_infrastructure_failure() {
        let root = xfail_test_root("assertion-classification");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("actual.v"), "actual\n").unwrap();
        fs::write(root.join("expected.v"), "expected\n").unwrap();
        let mismatch = PlanAction::AssertVerilog {
            actual: "actual.v".to_owned(),
            expected: "expected.v".to_owned(),
        };
        assert!(matches!(
            check_plan_assertion_typed(&mismatch, &root, &root, &root, "test"),
            Err(PlanAssertionFailure::ContractMismatch(message)) if message.contains("differs from")
        ));
        let missing = PlanAction::AssertTextContains {
            path: "missing.out".to_owned(),
            text: "expected".to_owned(),
        };
        assert!(matches!(
            check_plan_assertion_typed(&missing, &root, &root, &root, "test"),
            Err(PlanAssertionFailure::Infrastructure(message))
                if message.contains("read asserted artifact")
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn golden_any_accepts_a_later_matching_alternative() {
        let root = xfail_test_root("golden-any");
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.out");
        let first = root.join("first.expected");
        let second = root.join("second.expected");
        fs::write(&actual, "matches second\n").unwrap();
        fs::write(&first, "does not match\n").unwrap();
        fs::write(&second, "matches second\n").unwrap();

        compare_any_golden(&actual, vec![first.clone(), second], &root).unwrap();
        assert!(root.join("plan-golden-any-0.diff").is_file());
        assert!(!root.join("plan-golden-any-1.diff").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn golden_any_rejects_when_no_alternative_matches() {
        let root = xfail_test_root("golden-any-mismatch");
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.out");
        let first = root.join("first.expected");
        let second = root.join("second.expected");
        fs::write(&actual, "actual\n").unwrap();
        fs::write(&first, "first\n").unwrap();
        fs::write(&second, "second\n").unwrap();

        let error = compare_any_golden(&actual, vec![first, second], &root).unwrap_err();
        assert!(error.contains("matches none of 2 golden alternatives"));
        assert!(root.join("plan-golden-any-0.diff").is_file());
        assert!(root.join("plan-golden-any-1.diff").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn golden_any_requires_each_expected_fixture() {
        let root = xfail_test_root("golden-any-missing");
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.out");
        fs::write(&actual, "actual\n").unwrap();

        let error =
            compare_any_golden(&actual, vec![root.join("missing.expected")], &root).unwrap_err();
        assert!(error.contains("read golden"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn xfail_golden_accepts_mismatch_and_writes_diff() {
        let root = xfail_test_root("mismatch");
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.out");
        let expected = root.join("expected.out");
        let diff = root.join("xfail.diff");
        fs::write(&actual, "actual\n").unwrap();
        fs::write(&expected, "expected\n").unwrap();

        compare_xfail_golden(&actual, &expected, &diff, "upstream bug 138").unwrap();
        assert!(diff.is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn xfail_golden_rejects_xpass_after_normalization() {
        let root = xfail_test_root("xpass");
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.out");
        let expected = root.join("expected.out");
        let diff = root.join("xfail.diff");
        fs::write(&actual, "same\r\n").unwrap();
        fs::write(&expected, "same\n").unwrap();

        let error = compare_xfail_golden(&actual, &expected, &diff, "upstream bug 138")
            .expect_err("matching output must be XPASS");
        assert!(error.contains("XPASS"));
        assert!(!diff.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn xfail_golden_does_not_hide_missing_artifacts() {
        let root = xfail_test_root("missing");
        fs::create_dir_all(&root).unwrap();
        let expected = root.join("expected.out");
        fs::write(&expected, "expected\n").unwrap();

        let error = compare_xfail_golden(
            &root.join("missing.out"),
            &expected,
            &root.join("xfail.diff"),
            "upstream bug 138",
        )
        .expect_err("missing output must remain a real failure");
        assert!(error.contains("read actual artifact"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn line_directive_normalization_matches_the_audited_filter_scope() {
        assert_eq!(
            normalize_line_directive_positions(
                "prefix `line(one) middle `line(two) suffix\nunchanged\n",
            ),
            "prefix `line(POS) suffix\nunchanged"
        );
    }

    #[test]
    fn normalized_golden_applies_declared_profiles_to_both_artifacts() {
        let root = xfail_test_root("normalized-ids");
        fs::create_dir_all(&root).unwrap();
        let actual = root.join("actual.out");
        let expected = root.join("expected.out");
        let diff = root.join("generated-ids.diff");
        fs::write(
            &actual,
            "Bluespec Compiler, version actual\nrule__h12 value__d900 VRWire12;\n\"Prelude.bs\", line 12, column 34\n",
        )
        .unwrap();
        fs::write(
            &expected,
            "Bluespec Compiler, version expected\nrule__h77 value__d3 VRWire7;\n\"Prelude.bs\", line 987, column 6\n",
        )
        .unwrap();

        compare_normalized_golden(
            &actual,
            &expected,
            &diff,
            &[
                GoldenNormalization::GeneratedIds,
                GoldenNormalization::VrWireIds,
                GoldenNormalization::PreludePositions,
                GoldenNormalization::CompilerBannerLines,
            ],
        )
        .unwrap();
        assert!(!diff.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sorted_golden_lines_preserve_duplicates_and_ignore_order() {
        assert_eq!(
            sorted_golden_lines("second\nfirst\nsecond\n"),
            "first\nsecond\nsecond"
        );
    }

    #[test]
    fn native_golden_accepts_one_last_place_decimal_unit() {
        assert!(native_golden_text_matches("value = -0.007813", "value = -0.007812").unwrap());
    }

    #[test]
    fn native_golden_accepts_cross_stream_line_reordering() {
        assert!(native_golden_text_matches("first\nsecond", "second\nfirst").unwrap());
    }

    #[test]
    fn native_golden_rejects_larger_decimal_differences() {
        assert!(!native_golden_text_matches("value = -0.007814", "value = -0.007812").unwrap());
    }

    #[test]
    fn names_simulation_run_and_vcd_validity_assertion() {
        let run = PlanAction::SimulationRun {
            backend: SimulationBackend::Icarus,
            executable: "mkTestbench".to_owned(),
            args: Vec::new(),
            stdout: "simulation.out".to_owned(),
            expected_exits: bsc_test_plan::ExpectedExitSet::default(),
            vcd: None,
        };
        assert_eq!(plan_action_name(&run), "simulation.run");

        let assertion = PlanAction::AssertVcdValid {
            path: "trace.vcd".to_owned(),
        };
        assert_eq!(plan_action_name(&assertion), "assert.vcd_valid");
    }
}
