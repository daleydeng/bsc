use crate::locate_project_root;
use crate::upstream::{SimulationBackend, CASES, SIMULATION_CASES};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentSummary {
    pub scripts: usize,
    pub compile_cases: usize,
    pub simulation_cases: usize,
    pub scheduler_cases: usize,
    pub total_test_scripts: usize,
    pub migrated_test_scripts: usize,
    pub remaining_test_scripts: usize,
}

pub fn check_alignment() -> Result<AlignmentSummary, String> {
    let project_root = locate_project_root()?;
    let scripts = check_upstream_cases(&project_root)?;
    let scheduler_cases = check_scheduler_sat(&project_root)?;
    let total_test_scripts = count_test_scripts(&project_root)?;
    let migrated_test_scripts = scripts + 1;
    let remaining_test_scripts = total_test_scripts
        .checked_sub(migrated_test_scripts)
        .ok_or_else(|| "migrated test script count exceeds testsuite script count".to_owned())?;
    Ok(AlignmentSummary {
        scripts,
        compile_cases: CASES.len(),
        simulation_cases: SIMULATION_CASES.len(),
        scheduler_cases,
        total_test_scripts,
        migrated_test_scripts,
        remaining_test_scripts,
    })
}

fn count_test_scripts(project_root: &Path) -> Result<usize, String> {
    let testsuite = project_root.join("testsuite");
    let infrastructure = [
        project_root.join("testsuite/config/unix.exp"),
        project_root.join("testsuite/lib/bsc.exp"),
        project_root.join("testsuite/site.exp"),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let mut directories = vec![testsuite];
    let mut count = 0;

    while let Some(directory) = directories.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            format!("read testsuite directory {}: {error}", directory.display())
        })?;
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("read entry in {}: {error}", directory.display()))?;
            let file_type = entry.file_type().map_err(|error| {
                format!("read file type for {}: {error}", entry.path().display())
            })?;
            let path = entry.path();
            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file()
                && path.extension().is_some_and(|extension| extension == "exp")
                && !infrastructure.contains(&path)
            {
                count += 1;
            }
        }
    }

    Ok(count)
}

fn check_upstream_cases(project_root: &Path) -> Result<usize, String> {
    let mut names = BTreeSet::new();
    let mut registered = BTreeMap::<&str, Counts>::new();

    for case in CASES {
        if !names.insert(case.name) {
            return Err(format!("duplicate Rust case name: {}", case.name));
        }
        check_declared_fixtures(project_root, case.fixture_dir, case.fixtures, case.name)?;
        add_count(
            &mut registered.entry(case.fixture_dir).or_default().contracts,
            contract_key("compile", case.source),
        );
        if case.golden.is_some() {
            add_count(
                &mut registered.entry(case.fixture_dir).or_default().goldens,
                case.source.to_owned(),
            );
        }
    }

    for case in SIMULATION_CASES {
        if !names.insert(case.name) {
            return Err(format!("duplicate Rust case name: {}", case.name));
        }
        check_declared_fixtures(project_root, case.fixture_dir, case.fixtures, case.name)?;
        let backend = match case.backend {
            SimulationBackend::Bluesim => "bluesim",
            SimulationBackend::Icarus => "icarus",
        };
        add_count(
            &mut registered.entry(case.fixture_dir).or_default().contracts,
            contract_key(backend, case.source),
        );
    }

    for (fixture_dir, actual) in &registered {
        let origin = find_sole_exp(project_root, fixture_dir)?;
        let source = fs::read_to_string(&origin)
            .map_err(|error| format!("read origin {}: {error}", origin.display()))?;
        let expected = parse_exp_contracts(&source, &origin)?;
        compare_counts(&origin, "contracts", &expected.contracts, &actual.contracts)?;
        compare_counts(
            &origin,
            "golden comparisons",
            &expected.goldens,
            &actual.goldens,
        )?;
    }

    Ok(registered.len())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    contracts: BTreeMap<String, usize>,
    goldens: BTreeMap<String, usize>,
}

fn check_declared_fixtures(
    project_root: &Path,
    fixture_dir: &str,
    fixtures: &[&str],
    case_name: &str,
) -> Result<(), String> {
    let directory = project_root.join(fixture_dir);
    if !directory.is_dir() {
        return Err(format!(
            "fixture directory for {case_name} does not exist: {}",
            directory.display()
        ));
    }
    for fixture in fixtures {
        let path = directory.join(fixture);
        if !path.is_file() {
            return Err(format!(
                "fixture declared by {case_name} does not exist: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn find_sole_exp(project_root: &Path, fixture_dir: &str) -> Result<PathBuf, String> {
    let directory = project_root.join(fixture_dir);
    let mut scripts = fs::read_dir(&directory)
        .map_err(|error| format!("read fixture directory {}: {error}", directory.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "exp"))
        .collect::<Vec<_>>();
    scripts.sort();
    match scripts.as_slice() {
        [script] => Ok(script.clone()),
        [] => Err(format!(
            "fixture directory has no origin .exp: {}",
            directory.display()
        )),
        _ => Err(format!(
            "fixture directory has multiple .exp origins; add explicit origin metadata before migrating it: {} ({})",
            directory.display(),
            scripts
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn parse_exp_contracts(source: &str, origin: &Path) -> Result<Counts, String> {
    let mut counts = Counts::default();
    for (line_index, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words = line.split_whitespace().collect::<Vec<_>>();
        let Some(command) = words.first().copied() else {
            continue;
        };
        match command {
            "compile_pass"
            | "compile_fail"
            | "compile_fail_error"
            | "compile_verilog_pass"
            | "compile_verilog_fail"
            | "compile_verilog_fail_error"
            | "compile_verilog_pass_warning" => {
                let source = required_word(&words, 1, origin, line_index)?;
                add_count(&mut counts.contracts, contract_key("compile", source));
            }
            "test_c_veri_bsv" | "test_c_veri_bsv_modules_options" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bsv");
                add_count(&mut counts.contracts, contract_key("bluesim", &source));
                add_count(&mut counts.contracts, contract_key("icarus", &source));
            }
            "test_c_only_bsv" => {
                let module = required_word(&words, 1, origin, line_index)?;
                add_count(
                    &mut counts.contracts,
                    contract_key("bluesim", &format!("{module}.bsv")),
                );
            }
            "test_veri_only_bsv" => {
                let module = required_word(&words, 1, origin, line_index)?;
                add_count(
                    &mut counts.contracts,
                    contract_key("icarus", &format!("{module}.bsv")),
                );
            }
            "compare_file" => {
                let output = required_word(&words, 1, origin, line_index)?;
                let source = if output == "[make_bsc_output_name" {
                    required_word(&words, 2, origin, line_index)?
                } else {
                    output
                        .strip_suffix(".bsc-vcomp-out")
                        .or_else(|| output.strip_suffix(".bsc-out"))
                        .ok_or_else(|| {
                            format!(
                                "unsupported compare_file target at {}:{}: {output}",
                                origin.display(),
                                line_index + 1
                            )
                        })?
                };
                add_count(&mut counts.goldens, source.to_owned());
            }
            _ => {}
        }
    }
    Ok(counts)
}

fn required_word<'a>(
    words: &'a [&str],
    index: usize,
    origin: &Path,
    line_index: usize,
) -> Result<&'a str, String> {
    words
        .get(index)
        .map(|word| word.trim_matches(['"', '{', '}', ']']))
        .filter(|word| !word.is_empty())
        .ok_or_else(|| {
            format!(
                "missing argument for migrated command at {}:{}",
                origin.display(),
                line_index + 1
            )
        })
}

fn contract_key(kind: &str, source: &str) -> String {
    format!("{kind}:{source}")
}

fn add_count(counts: &mut BTreeMap<String, usize>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn compare_counts(
    origin: &Path,
    label: &str,
    expected: &BTreeMap<String, usize>,
    actual: &BTreeMap<String, usize>,
) -> Result<(), String> {
    if expected == actual {
        return Ok(());
    }
    let missing = count_difference(expected, actual);
    let unexpected = count_difference(actual, expected);
    Err(format!(
        "Rust tests are not aligned with {} ({label})\n  missing: {}\n  unexpected: {}",
        origin.display(),
        render_counts(&missing),
        render_counts(&unexpected)
    ))
}

fn count_difference(
    left: &BTreeMap<String, usize>,
    right: &BTreeMap<String, usize>,
) -> BTreeMap<String, usize> {
    left.iter()
        .filter_map(|(key, left_count)| {
            let difference = left_count.saturating_sub(right.get(key).copied().unwrap_or_default());
            (difference > 0).then(|| (key.clone(), difference))
        })
        .collect()
}

fn render_counts(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "none".to_owned();
    }
    counts
        .iter()
        .map(|(key, count)| {
            if *count == 1 {
                key.clone()
            } else {
                format!("{key} x{count}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn check_scheduler_sat(project_root: &Path) -> Result<usize, String> {
    let origin = project_root.join("testsuite/bsc.scheduler/sat/sat.exp");
    let origin_source = fs::read_to_string(&origin)
        .map_err(|error| format!("read scheduler origin {}: {error}", origin.display()))?;
    let expected = parse_scheduler_sources(&origin_source)?;

    let rust_path = project_root.join("rust-tests/tests/scheduler_sat.rs");
    let rust_source = fs::read_to_string(&rust_path)
        .map_err(|error| format!("read scheduler Rust tests {}: {error}", rust_path.display()))?;
    let actual = parse_rust_scheduler_cases(&rust_source);
    if expected != actual {
        return Err(format!(
            "scheduler Rust cases are not aligned with {}\n  expected: {}\n  actual: {}",
            origin.display(),
            expected.join(", "),
            actual.join(", ")
        ));
    }

    let directory = origin.parent().expect("sat.exp has a parent");
    for case in &actual {
        for file_name in [
            format!("{case}.bsv"),
            format!("{case}_sat-yices.bsv.bsc-sched-out.expected"),
        ] {
            let path = directory.join(file_name);
            if !path.is_file() {
                return Err(format!(
                    "scheduler fixture declared by sat.exp is missing: {}",
                    path.display()
                ));
            }
        }
    }
    Ok(actual.len())
}

fn parse_scheduler_sources(source: &str) -> Result<Vec<String>, String> {
    let mut cases = Vec::new();
    let mut in_sources = false;
    for raw_line in source.lines() {
        let mut line = raw_line.trim();
        if !in_sources {
            let Some((_, remainder)) = line.split_once("set sources [list") else {
                continue;
            };
            in_sources = true;
            line = remainder.trim();
        }
        let finished = line.contains(']');
        let content = line
            .split('#')
            .next()
            .unwrap_or_default()
            .replace(['\\', ']'], " ");
        cases.extend(content.split_whitespace().map(str::to_owned));
        if finished {
            return Ok(cases);
        }
    }
    Err("could not parse `set sources [list ...]` from scheduler sat.exp".to_owned())
}

fn parse_rust_scheduler_cases(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let (_, value) = line.split_once("=>")?;
            let value = value.trim();
            let value = value.strip_prefix('"')?;
            let (case, _) = value.split_once('"')?;
            Some(case.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_tcl_contracts() {
        let source = concat!(
            "compile_pass Good.bsv\n",
            "compile_fail_error Bad.bsv T0001\n",
            "compare_file Bad.bsv.bsc-out\n",
            "test_c_veri_bsv Both\n",
            "test_c_only_bsv COnly expected\n",
            "test_veri_only_bsv VOnly expected\n",
        );
        let actual = parse_exp_contracts(source, Path::new("sample.exp")).unwrap();
        let expected_contracts = [
            ("compile:Good.bsv", 1),
            ("compile:Bad.bsv", 1),
            ("bluesim:Both.bsv", 1),
            ("icarus:Both.bsv", 1),
            ("bluesim:COnly.bsv", 1),
            ("icarus:VOnly.bsv", 1),
        ]
        .into_iter()
        .map(|(key, count)| (key.to_owned(), count))
        .collect();
        assert_eq!(actual.contracts, expected_contracts);
        assert_eq!(actual.goldens, BTreeMap::from([("Bad.bsv".to_owned(), 1)]));
    }

    #[test]
    fn parses_scheduler_source_list() {
        let source = "set sources [list \\\n  One Two \\\n  Three]\n";
        assert_eq!(
            parse_scheduler_sources(source).unwrap(),
            ["One", "Two", "Three"]
        );
    }
}
