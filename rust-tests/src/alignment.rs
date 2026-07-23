use crate::locate_project_root;
use crate::upstream::{
    compile_case_modules, compile_cases, simulation_case_modules, simulation_cases, CaseModule,
    SimulationBackend,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEDULER_ORIGINS: &[&str] = &["testsuite/bsc.scheduler/sat/sat.exp"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentSummary {
    pub scripts: usize,
    pub compile_cases: usize,
    pub simulation_cases: usize,
    pub scheduler_cases: usize,
    pub total_test_scripts: usize,
    pub migrated_test_scripts: usize,
    pub remaining_test_scripts: usize,
    pub total_statically_declared_contracts: usize,
    pub migrated_contracts: usize,
    pub remaining_statically_declared_contracts: usize,
    pub unclassified_test_scripts: usize,
}

pub fn check_alignment() -> Result<AlignmentSummary, String> {
    let project_root = locate_project_root()?;
    let compile_cases = compile_cases();
    let simulation_cases = simulation_cases();
    let scripts = check_upstream_cases(&project_root)?;
    let scheduler_cases = check_scheduler_sat(&project_root)?;
    let inventory = inventory_testsuite(&project_root, scheduler_cases)?;
    let migrated_test_scripts = scripts + SCHEDULER_ORIGINS.len();
    let remaining_test_scripts = inventory
        .total_test_scripts
        .checked_sub(migrated_test_scripts)
        .ok_or_else(|| "migrated test script count exceeds testsuite script count".to_owned())?;
    let migrated_contracts = compile_cases.len() + simulation_cases.len() + scheduler_cases;
    let remaining_statically_declared_contracts = inventory
        .total_statically_declared_contracts
        .checked_sub(migrated_contracts)
        .ok_or_else(|| {
            "migrated contract count exceeds statically declared testsuite contract count"
                .to_owned()
        })?;
    Ok(AlignmentSummary {
        scripts,
        compile_cases: compile_cases.len(),
        simulation_cases: simulation_cases.len(),
        scheduler_cases,
        total_test_scripts: inventory.total_test_scripts,
        migrated_test_scripts,
        remaining_test_scripts,
        total_statically_declared_contracts: inventory.total_statically_declared_contracts,
        migrated_contracts,
        remaining_statically_declared_contracts,
        unclassified_test_scripts: inventory.unclassified_test_scripts,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingTestScript {
    pub origin: String,
    pub statically_declared_contracts: usize,
}

pub fn remaining_inventory() -> Result<Vec<RemainingTestScript>, String> {
    let summary = check_alignment()?;
    let project_root = locate_project_root()?;
    let migrated = collect_migrated_origins(&project_root)?;
    let remaining = collect_testsuite_scripts(&project_root, summary.scheduler_cases)?
        .into_iter()
        .filter(|script| !migrated.contains(&script.origin))
        .map(|script| RemainingTestScript {
            origin: script.origin,
            statically_declared_contracts: script.statically_declared_contracts,
        })
        .collect::<Vec<_>>();

    let remaining_contracts = remaining
        .iter()
        .map(|script| script.statically_declared_contracts)
        .sum::<usize>();
    if remaining.len() != summary.remaining_test_scripts
        || remaining_contracts != summary.remaining_statically_declared_contracts
    {
        return Err(format!(
            "remaining inventory does not match alignment summary: {} scripts/{remaining_contracts} contracts, expected {}/{}",
            remaining.len(),
            summary.remaining_test_scripts,
            summary.remaining_statically_declared_contracts
        ));
    }
    Ok(remaining)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TestsuiteInventory {
    total_test_scripts: usize,
    total_statically_declared_contracts: usize,
    unclassified_test_scripts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestsuiteScript {
    origin: String,
    statically_declared_contracts: usize,
}

fn inventory_testsuite(
    project_root: &Path,
    scheduler_cases: usize,
) -> Result<TestsuiteInventory, String> {
    let mut inventory = TestsuiteInventory::default();
    for script in collect_testsuite_scripts(project_root, scheduler_cases)? {
        inventory.total_test_scripts += 1;
        inventory.total_statically_declared_contracts += script.statically_declared_contracts;
        if script.statically_declared_contracts == 0 {
            inventory.unclassified_test_scripts += 1;
        }
    }
    Ok(inventory)
}

fn collect_testsuite_scripts(
    project_root: &Path,
    scheduler_cases: usize,
) -> Result<Vec<TestsuiteScript>, String> {
    let testsuite = project_root.join("testsuite");
    let infrastructure = [
        project_root.join("testsuite/config/unix.exp"),
        project_root.join("testsuite/lib/bsc.exp"),
        project_root.join("testsuite/site.exp"),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let scheduler_origin = project_root.join(SCHEDULER_ORIGINS[0]);
    let mut directories = vec![testsuite];
    let mut scripts = Vec::new();

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
                let statically_declared_contracts = if path == scheduler_origin {
                    scheduler_cases
                } else {
                    let source = fs::read(&path)
                        .map_err(|error| format!("read test script {}: {error}", path.display()))?;
                    count_statically_declared_contracts(&String::from_utf8_lossy(&source))
                };
                scripts.push(TestsuiteScript {
                    origin: project_relative_unix_path(project_root, &path)?,
                    statically_declared_contracts,
                });
            }
        }
    }
    scripts.sort_by(|left, right| left.origin.cmp(&right.origin));
    Ok(scripts)
}

fn collect_migrated_origins(project_root: &Path) -> Result<BTreeSet<String>, String> {
    let mut origins = SCHEDULER_ORIGINS
        .iter()
        .map(|origin| (*origin).to_owned())
        .collect::<BTreeSet<_>>();
    for fixture_dir in compile_cases()
        .iter()
        .map(|case| case.fixture_dir)
        .chain(simulation_cases().iter().map(|case| case.fixture_dir))
    {
        let origin = find_sole_exp(project_root, fixture_dir)?;
        origins.insert(project_relative_unix_path(project_root, &origin)?);
    }
    Ok(origins)
}

fn count_statically_declared_contracts(source: &str) -> usize {
    source
        .lines()
        .filter_map(|raw_line| {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            line.split_whitespace().next()
        })
        .map(|command| match command {
            "compile_pass"
            | "compile_fail"
            | "compile_fail_error"
            | "compile_verilog_pass"
            | "compile_verilog_fail"
            | "compile_verilog_fail_error"
            | "compile_verilog_pass_warning"
            | "test_c_only_bsv"
            | "test_veri_only_bsv" => 1,
            "test_c_veri_bsv" | "test_c_veri_bsv_modules_options" => 2,
            _ => 0,
        })
        .sum()
}

fn check_case_modules<C: 'static>(
    project_root: &Path,
    relative_directory: &str,
    modules: &[CaseModule<C>],
    fixture_dir: impl Fn(&C) -> &'static str,
) -> Result<(), String> {
    let directory = project_root.join(relative_directory);
    let mut disk_modules = BTreeSet::new();
    for entry in fs::read_dir(&directory).map_err(|error| {
        format!(
            "read case module directory {}: {error}",
            directory.display()
        )
    })? {
        let entry =
            entry.map_err(|error| format!("read entry in case module directory: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", entry.path().display()))?;
        let path = entry.path();
        if !file_type.is_file() || path.extension().is_none_or(|extension| extension != "rs") {
            return Err(format!(
                "case module directory may contain only Rust module files: {}",
                path.display()
            ));
        }
        let module_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("case module file name is not UTF-8: {}", path.display()))?;
        disk_modules.insert(module_name.to_owned());
    }

    let mut registered_modules = BTreeSet::new();
    let mut previous_module = None;
    for module in modules {
        if previous_module.is_some_and(|previous| previous >= module.name) {
            return Err(format!(
                "case modules must be registered once in ascending name order: {}",
                module.name
            ));
        }
        previous_module = Some(module.name);
        if !is_stable_module_name(module.name) {
            return Err(format!(
                "case module name must be stable ASCII snake_case without migration terms: {}",
                module.name
            ));
        }
        if !registered_modules.insert(module.name.to_owned()) {
            return Err(format!(
                "duplicate case module registration: {}",
                module.name
            ));
        }
        if module.cases.is_empty() {
            return Err(format!("case module is empty: {}", module.name));
        }

        let module_path = directory.join(format!("{}.rs", module.name));
        let module_source = fs::read_to_string(&module_path)
            .map_err(|error| format!("read case module {}: {error}", module_path.display()))?;
        let declared_origins = parse_module_origins(&module_source, &module_path)?;
        let mut actual_origins = BTreeSet::new();
        for case in module.cases {
            let origin = find_sole_exp(project_root, fixture_dir(case))?;
            actual_origins.insert(project_relative_unix_path(project_root, &origin)?);
        }
        if declared_origins != actual_origins {
            return Err(format!(
                "case module origins are not aligned with registered fixtures: {}\n  declared: {}\n  actual: {}",
                module_path.display(),
                render_set(&declared_origins),
                render_set(&actual_origins)
            ));
        }
    }

    if registered_modules != disk_modules {
        let unregistered = disk_modules
            .difference(&registered_modules)
            .cloned()
            .collect::<BTreeSet<_>>();
        let missing = registered_modules
            .difference(&disk_modules)
            .cloned()
            .collect::<BTreeSet<_>>();
        return Err(format!(
            "case module registry does not match {}\n  unregistered files: {}\n  missing files: {}",
            directory.display(),
            render_set(&unregistered),
            render_set(&missing)
        ));
    }

    Ok(())
}

fn is_stable_module_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    let valid_snake_case = first.is_ascii_lowercase()
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !name.ends_with('_')
        && !name.contains("__");
    let forbidden = ["batch", "large", "other", "four", "five"];
    valid_snake_case && !name.split('_').any(|part| forbidden.contains(&part))
}

fn parse_module_origins(source: &str, module_path: &Path) -> Result<BTreeSet<String>, String> {
    let lines = source.lines().collect::<Vec<_>>();
    let Some(first) = lines.first().copied() else {
        return Err(format!("empty case module: {}", module_path.display()));
    };

    let origins = if let Some(value) = first.strip_prefix("//! Origin: ") {
        if lines.get(1).is_some_and(|line| line.starts_with("//!")) {
            return Err(format!(
                "single-origin module has additional origin comment lines: {}",
                module_path.display()
            ));
        }
        vec![parse_backticked_origin(value, module_path)?]
    } else if first == "//! Origins:" {
        let mut values = Vec::new();
        for line in lines
            .iter()
            .skip(1)
            .take_while(|line| line.starts_with("//!"))
        {
            let value = line.strip_prefix("//! - ").ok_or_else(|| {
                format!(
                    "multi-origin module must use `//! - `path`` lines: {}",
                    module_path.display()
                )
            })?;
            values.push(parse_backticked_origin(value, module_path)?);
        }
        if values.len() < 2 {
            return Err(format!(
                "multi-origin module must declare at least two origins: {}",
                module_path.display()
            ));
        }
        values
    } else {
        return Err(format!(
            "case module must start with `//! Origin:` or `//! Origins:`: {}",
            module_path.display()
        ));
    };

    let mut unique = BTreeSet::new();
    for origin in origins {
        if !origin.starts_with("testsuite/")
            || !origin.ends_with(".exp")
            || origin.contains(['\\', '<', '>'])
            || origin
                .split('/')
                .any(|component| component.is_empty() || component == "." || component == "..")
        {
            return Err(format!(
                "invalid case module origin `{origin}` in {}",
                module_path.display()
            ));
        }
        if !unique.insert(origin.clone()) {
            return Err(format!(
                "duplicate case module origin `{origin}` in {}",
                module_path.display()
            ));
        }
    }
    Ok(unique)
}

fn parse_backticked_origin(value: &str, module_path: &Path) -> Result<String, String> {
    let Some(value) = value.strip_prefix('`') else {
        return Err(format!(
            "origin path is not backticked: {}",
            module_path.display()
        ));
    };
    let Some((origin, trailing)) = value.split_once('`') else {
        return Err(format!(
            "unterminated origin path: {}",
            module_path.display()
        ));
    };
    if !matches!(trailing.trim(), "" | ".") {
        return Err(format!(
            "unexpected text after origin path in {}: {}",
            module_path.display(),
            trailing.trim()
        ));
    }
    Ok(origin.to_owned())
}

fn project_relative_unix_path(project_root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(project_root)
        .map_err(|_| format!("origin is outside the project root: {}", path.display()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn render_set(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

fn check_upstream_cases(project_root: &Path) -> Result<usize, String> {
    check_case_modules(
        project_root,
        "rust-tests/src/upstream/cases_compile",
        compile_case_modules(),
        |case| case.fixture_dir,
    )?;
    check_case_modules(
        project_root,
        "rust-tests/src/upstream/cases_simulation",
        simulation_case_modules(),
        |case| case.fixture_dir,
    )?;

    let mut names = BTreeSet::new();
    let mut registered = BTreeMap::<&str, Counts>::new();

    for case in compile_cases() {
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

    for case in simulation_cases() {
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
    let mut scripts = Vec::new();
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("read fixture directory {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("read entry in fixture directory: {error}"))?
            .path();
        if path.extension().is_some_and(|extension| extension == "exp") {
            scripts.push(path);
        }
    }
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
    let origin = project_root.join(SCHEDULER_ORIGINS[0]);
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
    fn counts_statically_declared_contract_multiplicity() {
        let source = concat!(
            "compile_pass Good.bsv\n",
            "test_c_veri_bsv Both\n",
            "test_c_only_bsv COnly expected\n",
            "test_veri_only_bsv VOnly expected\n",
            "# compile_fail Ignored.bsv\n",
            "compare_file Good.bsv.bsc-out\n",
            "foreach item $items {\n",
        );
        assert_eq!(count_statically_declared_contracts(source), 5);
    }

    #[test]
    fn parses_scheduler_source_list() {
        let source = "set sources [list \\\n  One Two \\\n  Three]\n";
        assert_eq!(
            parse_scheduler_sources(source).unwrap(),
            ["One", "Two", "Three"]
        );
    }

    #[test]
    fn parses_strict_module_origin_headers() {
        let single = "//! Origin: `testsuite/one/one.exp`.\n\nconst VALUE: usize = 1;\n";
        assert_eq!(
            parse_module_origins(single, Path::new("single.rs")).unwrap(),
            BTreeSet::from(["testsuite/one/one.exp".to_owned()])
        );

        let multiple = concat!(
            "//! Origins:\n",
            "//! - `testsuite/one/one.exp`\n",
            "//! - `testsuite/two/two.exp`\n",
            "\nconst VALUE: usize = 1;\n",
        );
        assert_eq!(
            parse_module_origins(multiple, Path::new("multiple.rs")).unwrap(),
            BTreeSet::from([
                "testsuite/one/one.exp".to_owned(),
                "testsuite/two/two.exp".to_owned(),
            ])
        );
    }

    #[test]
    fn rejects_template_origins_and_migration_module_names() {
        let template = "//! Origin: `testsuite/<bug>/<bug>.exp`.\n";
        assert!(parse_module_origins(template, Path::new("template.rs")).is_err());

        assert!(is_stable_module_name("cross_suite_multi"));
        assert!(!is_stable_module_name("large_compile_batch_five"));
        assert!(!is_stable_module_name("other_regressions"));
        assert!(!is_stable_module_name("not__snake_case"));
    }
}
