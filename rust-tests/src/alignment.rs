use crate::locate_project_root;
use crate::upstream::{
    compile_case_modules, compile_cases, simulation_scenario_modules, simulation_scenarios,
    validate_simulation_scenario, ArtifactAssertion, ArtifactNormalization, CaseModule,
    DiagnosticKind, GenerationStrategy, SimulationBackend, TextAssertion,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEDULER_ORIGINS: &[&str] = &["testsuite/bsc.scheduler/sat/sat.exp"];

const KNOWN_MIGRATION_BLOCKERS: &[(&str, &str)] = &[
    (
        "testsuite/bsc.evaluator/performance/performance.exp",
        "native Windows code generation exceeds 300 seconds",
    ),
    (
        "testsuite/bsc.lib/FloatingPoint/FloatTest.exp",
        "shared native Windows elaboration exceeds 600 seconds",
    ),
    (
        "testsuite/bsc.lib/BRAM/BRAM0Test/BRAM0Test.exp",
        "shared native Windows elaboration exceeds 300 seconds",
    ),
    (
        "testsuite/bsc.bugs/bluespec_inc/b925/b925.exp",
        "backend-specific XFAIL and bug gate are not modeled",
    ),
    (
        "testsuite/bsc.bluesim/operators/operators.exp",
        "Bluesim and Verilog bug gates are not modeled",
    ),
    (
        "testsuite/bsc.if/split-execution/2x2-switch-split/switch.exp",
        "manual interactive Bluesim and cycle assertions",
    ),
    (
        "testsuite/bsc.if/split-execution/2x2-switch/switch.exp",
        "manual interactive Bluesim and cycle assertions",
    ),
    (
        "testsuite/bsc.lib/DefaultValue/DefaultValue.exp",
        "compile_pass_warning is not modeled",
    ),
    (
        "testsuite/bsc.lib/FShow/FShow.exp",
        "compile_pass_warning is not modeled",
    ),
    (
        "testsuite/bsc.lib/oint/oint.exp",
        "compile_verilog_pass_no_warning_bug is not modeled",
    ),
    (
        "testsuite/bsc.bugs/bluespec_inc/b1666/b1666.exp",
        "expected Verilog link failure is not modeled",
    ),
    (
        "testsuite/bsc.lib/getput/getput.exp",
        "dynamic Icarus probing and additional assertions",
    ),
    (
        "testsuite/bsc.bsv_examples/bsvfifo/bsvfifo.exp",
        "manual copy, erase, link, and simulation flow",
    ),
    (
        "testsuite/bsc.bugs/bluespec_inc/b535/b535.exp",
        "manual copy, erase, link, and simulation flow",
    ),
    (
        "testsuite/bsc.arrays/arrays.exp",
        "conditional branches and compile_verilog_fail_bug",
    ),
    (
        "testsuite/bsc.mcd/ModArgs/ModArgs.exp",
        "no-warning and no-internal-error contracts are not modeled",
    ),
    (
        "testsuite/bsc.driver/gensign/gensign.exp",
        "dumpbi/dumpbo and string-count workflow",
    ),
    (
        "testsuite/bsc.mcd/Reset/Reset.exp",
        "dynamic branches, regular expressions, and simulation flow",
    ),
    (
        "testsuite/bsc.names/portRenaming/enableTests/enableTests.exp",
        "no-main link contract is not modeled",
    ),
    (
        "testsuite/bsc.compile/compile.exp",
        "dynamic fixture replacement and delayed workflow",
    ),
    (
        "testsuite/bsc.verilog/foreign_module/foreign_module.exp",
        "active failure source is missing",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentSummary {
    pub scripts: usize,
    pub compile_cases: usize,
    pub simulation_scenarios: usize,
    pub simulation_contracts: usize,
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
    let simulation_scenarios = simulation_scenarios();
    let simulation_contracts = simulation_scenarios
        .iter()
        .map(|scenario| scenario.contracts.len())
        .sum::<usize>();
    let scripts = check_upstream_cases(&project_root)?;
    let scheduler_cases = check_scheduler_sat(&project_root)?;
    let inventory = inventory_testsuite(&project_root, scheduler_cases)?;
    let migrated_test_scripts = scripts + SCHEDULER_ORIGINS.len();
    let remaining_test_scripts = inventory
        .total_test_scripts
        .checked_sub(migrated_test_scripts)
        .ok_or_else(|| "migrated test script count exceeds testsuite script count".to_owned())?;
    let migrated_contracts = compile_cases.len() + simulation_contracts + scheduler_cases;
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
        simulation_scenarios: simulation_scenarios.len(),
        simulation_contracts,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationReadiness {
    Candidate,
    Review,
    Blocked,
    Dynamic,
}

impl MigrationReadiness {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Review => "review",
            Self::Blocked => "blocked",
            Self::Dynamic => "dynamic/custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TclCommandCategory {
    ControlState,
    FileSystem,
    ManualToolchain,
    UnsupportedContract,
    UnsupportedAssertion,
    Custom,
}

impl TclCommandCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ControlState => "control/state",
            Self::FileSystem => "filesystem",
            Self::ManualToolchain => "manual toolchain",
            Self::UnsupportedContract => "unsupported contract",
            Self::UnsupportedAssertion => "unsupported assertion",
            Self::Custom => "custom helper",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedTclCommand {
    pub name: String,
    pub count: usize,
    pub category: TclCommandCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingTestScript {
    pub origin: String,
    pub statically_declared_contracts: usize,
    pub readiness: MigrationReadiness,
    pub unsupported_commands: Vec<UnsupportedTclCommand>,
    pub known_blocker: Option<String>,
}

pub fn remaining_inventory() -> Result<Vec<RemainingTestScript>, String> {
    let summary = check_alignment()?;
    let project_root = locate_project_root()?;
    let migrated = collect_migrated_origins(&project_root)?;
    let remaining = collect_testsuite_scripts(&project_root, summary.scheduler_cases)?
        .into_iter()
        .filter(|script| !migrated.contains(&script.origin))
        .map(|script| {
            let known_blocker = known_migration_blocker(&script.origin).map(str::to_owned);
            let readiness = migration_readiness(
                &script.origin,
                script.statically_declared_contracts,
                &script.unsupported_commands,
            );
            RemainingTestScript {
                origin: script.origin,
                statically_declared_contracts: script.statically_declared_contracts,
                readiness,
                unsupported_commands: script.unsupported_commands,
                known_blocker,
            }
        })
        .collect::<Vec<_>>();

    let remaining_origins = remaining
        .iter()
        .map(|script| script.origin.as_str())
        .collect::<BTreeSet<_>>();
    for (origin, _) in KNOWN_MIGRATION_BLOCKERS {
        if !remaining_origins.contains(origin) {
            return Err(format!(
                "known migration blocker is no longer in the remaining inventory; remove or update it: {origin}"
            ));
        }
    }

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
    unsupported_commands: Vec<UnsupportedTclCommand>,
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
                let source = fs::read(&path)
                    .map_err(|error| format!("read test script {}: {error}", path.display()))?;
                let source = String::from_utf8_lossy(&source);
                let statically_declared_contracts = if path == scheduler_origin {
                    scheduler_cases
                } else {
                    count_statically_declared_contracts(&source)
                };
                scripts.push(TestsuiteScript {
                    origin: project_relative_unix_path(project_root, &path)?,
                    statically_declared_contracts,
                    unsupported_commands: unsupported_tcl_commands(&source),
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
    for fixture_dir in compile_cases().iter().map(|case| case.fixture_dir).chain(
        simulation_scenarios()
            .iter()
            .map(|scenario| scenario.fixture_dir),
    ) {
        let origin = find_sole_exp(project_root, fixture_dir)?;
        origins.insert(project_relative_unix_path(project_root, &origin)?);
    }
    Ok(origins)
}

fn count_statically_declared_contracts(source: &str) -> usize {
    logical_tcl_commands(source)
        .iter()
        .filter_map(|command| statically_declared_contract_count(command))
        .sum()
}

fn statically_declared_contract_count(command: &str) -> Option<usize> {
    let name = tcl_command_name(command)?;
    if matches!(
        name,
        "test_c_veri_bsv_multi_options" | "test_c_veri_bsv_multi_options_separately"
    ) {
        let words = tcl_words(command).ok()?;
        let (bluesim, icarus) = multi_backend_flags(&words)?;
        return Some(usize::from(bluesim) + usize::from(icarus));
    }
    statically_declared_contract_weight(name)
}

fn statically_declared_contract_weight(command: &str) -> Option<usize> {
    match command {
        "compile_pass"
        | "compile_fail"
        | "compile_fail_error"
        | "compile_verilog_pass"
        | "compile_verilog_fail"
        | "compile_verilog_fail_error"
        | "compile_verilog_pass_warning"
        | "compile_verilog_schedule_pass"
        | "test_c_only_bsv"
        | "test_c_only_bsv_modules"
        | "test_c_only_bsv_modules_options"
        | "test_veri_only_bsv"
        | "test_veri_only_bsv_modules"
        | "test_veri_only_bsv_modules_options"
        | "test_c_only_bsv_multi"
        | "test_veri_only_bsv_multi"
        | "test_c_only_bsv_multi_options"
        | "test_veri_only_bsv_multi_options" => Some(1),
        "test_c_veri"
        | "test_c_veri_bs_modules"
        | "test_c_veri_bs_modules_options"
        | "test_c_veri_bsv"
        | "test_c_veri_bsv_modules"
        | "test_c_veri_bsv_modules_options"
        | "test_c_veri_bsv_separately"
        | "test_c_veri_bsv_modules_options_separately"
        | "test_c_veri_bsv_multi"
        | "test_c_veri_bsv_multi_options_separately" => Some(2),
        _ => None,
    }
}

fn unsupported_tcl_commands(source: &str) -> Vec<UnsupportedTclCommand> {
    let mut counts = BTreeMap::<String, usize>::new();
    for command in logical_tcl_commands(source) {
        let Some(name) = tcl_command_name(&command) else {
            continue;
        };
        if !is_supported_inventory_command(&command) {
            *counts.entry(name.to_owned()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(name, count)| UnsupportedTclCommand {
            category: unsupported_tcl_command_category(&name),
            name,
            count,
        })
        .collect()
}

fn tcl_command_name(command: &str) -> Option<&str> {
    let mut command = command.trim();
    if command.is_empty() || command.starts_with('#') {
        return None;
    }
    while let Some(rest) = command.strip_prefix('}') {
        command = rest.trim_start();
    }
    let name = command.split_whitespace().next()?;
    if matches!(name, "" | "{" | "}" | "else" | "elseif") || name.starts_with('#') {
        None
    } else {
        Some(name.trim_end_matches(';'))
    }
}

fn is_supported_inventory_command(command: &str) -> bool {
    let Some(name) = tcl_command_name(command) else {
        return true;
    };
    if is_multi_simulation_command(name) {
        return multi_command_is_statically_migratable(command);
    }
    statically_declared_contract_weight(name).is_some()
        || is_supported_tcl_assertion(name)
        || matches!(name, "compare_file" | "compare_verilog")
}

fn is_multi_simulation_command(command: &str) -> bool {
    matches!(
        command,
        "test_c_veri_bsv_multi"
            | "test_c_veri_bsv_multi_options"
            | "test_c_veri_bsv_multi_options_separately"
            | "test_c_only_bsv_multi"
            | "test_veri_only_bsv_multi"
            | "test_c_only_bsv_multi_options"
            | "test_veri_only_bsv_multi_options"
    )
}

fn multi_command_is_statically_migratable(command: &str) -> bool {
    let Ok(words) = tcl_words(command) else {
        return false;
    };
    let Some(name) = words.first().map(String::as_str) else {
        return false;
    };
    let bug_indexes: &[usize] = match name {
        "test_c_veri_bsv_multi" => &[5, 6],
        "test_c_veri_bsv_multi_options" | "test_c_veri_bsv_multi_options_separately" => &[6, 7],
        "test_c_only_bsv_multi" | "test_veri_only_bsv_multi" => &[5],
        "test_c_only_bsv_multi_options" | "test_veri_only_bsv_multi_options" => &[6],
        _ => return false,
    };
    let has_bug_gate = bug_indexes
        .iter()
        .filter_map(|index| words.get(*index))
        .any(|value| !value.is_empty());
    if has_bug_gate {
        return false;
    }
    if matches!(
        name,
        "test_c_veri_bsv_multi_options" | "test_c_veri_bsv_multi_options_separately"
    ) {
        return multi_backend_flags(&words).is_some();
    }
    true
}

fn multi_backend_flags(words: &[String]) -> Option<(bool, bool)> {
    let parse = |index: usize| match words.get(index).map(String::as_str) {
        None | Some("") | Some("1") => Some(true),
        Some("0") => Some(false),
        Some(_) => None,
    };
    let bluesim = parse(8)?;
    let icarus = parse(9)?;
    (bluesim || icarus).then_some((bluesim, icarus))
}

fn unsupported_tcl_command_category(command: &str) -> TclCommandCategory {
    if matches!(
        command,
        "if" | "foreach"
            | "for"
            | "while"
            | "switch"
            | "proc"
            | "return"
            | "break"
            | "continue"
            | "catch"
            | "try"
            | "throw"
            | "error"
            | "eval"
            | "expr"
            | "incr"
            | "set"
            | "unset"
            | "append"
            | "lappend"
            | "global"
            | "variable"
            | "namespace"
            | "upvar"
            | "uplevel"
    ) {
        TclCommandCategory::ControlState
    } else if matches!(
        command,
        "copy"
            | "erase"
            | "move"
            | "mkdir"
            | "touch"
            | "file"
            | "glob"
            | "open"
            | "close"
            | "read"
            | "gets"
            | "puts"
            | "cd"
            | "pwd"
    ) {
        TclCommandCategory::FileSystem
    } else if command.starts_with("link_")
        || command.starts_with("sim_")
        || command.starts_with("run_")
        || command.contains("worker")
        || matches!(command, "bluetcl" | "exec" | "source" | "test_ovl")
    {
        TclCommandCategory::ManualToolchain
    } else if command.starts_with("compile_") || command.starts_with("test_") {
        TclCommandCategory::UnsupportedContract
    } else if command.starts_with("find_")
        || command.starts_with("string_")
        || command.starts_with("compare_")
    {
        TclCommandCategory::UnsupportedAssertion
    } else {
        TclCommandCategory::Custom
    }
}

fn known_migration_blocker(origin: &str) -> Option<&'static str> {
    KNOWN_MIGRATION_BLOCKERS
        .iter()
        .find_map(|(blocked_origin, reason)| (*blocked_origin == origin).then_some(*reason))
}

fn migration_readiness(
    origin: &str,
    statically_declared_contracts: usize,
    unsupported_commands: &[UnsupportedTclCommand],
) -> MigrationReadiness {
    if known_migration_blocker(origin).is_some() {
        MigrationReadiness::Blocked
    } else if statically_declared_contracts == 0 {
        MigrationReadiness::Dynamic
    } else if unsupported_commands.is_empty() {
        MigrationReadiness::Candidate
    } else {
        MigrationReadiness::Review
    }
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
        simulation_scenario_modules(),
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
        for key in case
            .assertions
            .iter()
            .filter_map(|assertion| artifact_assertion_key(*assertion))
        {
            add_count(
                &mut registered.entry(case.fixture_dir).or_default().assertions,
                key,
            );
        }
    }

    let mut scenario_names = BTreeSet::new();
    for scenario in simulation_scenarios() {
        validate_simulation_scenario(scenario)?;
        if !scenario_names.insert(scenario.name) {
            return Err(format!(
                "duplicate Rust simulation scenario name: {}",
                scenario.name
            ));
        }
        check_declared_fixtures(
            project_root,
            scenario.fixture_dir,
            scenario.fixtures,
            scenario.name,
        )?;
        add_count(
            &mut registered
                .entry(scenario.fixture_dir)
                .or_default()
                .generations,
            generation_key(scenario.generation, scenario.source),
        );
        for contract in scenario.contracts {
            if !names.insert(contract.name) {
                return Err(format!("duplicate Rust contract name: {}", contract.name));
            }
            let backend = match contract.backend {
                SimulationBackend::Bluesim => "bluesim",
                SimulationBackend::Icarus => "icarus",
            };
            add_count(
                &mut registered
                    .entry(scenario.fixture_dir)
                    .or_default()
                    .contracts,
                contract_key(backend, scenario.source),
            );
            for key in contract
                .assertions
                .iter()
                .filter_map(|assertion| artifact_assertion_key(*assertion))
            {
                add_count(
                    &mut registered
                        .entry(scenario.fixture_dir)
                        .or_default()
                        .assertions,
                    key,
                );
            }
        }
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
        compare_counts(
            &origin,
            "generation strategies",
            &expected.generations,
            &actual.generations,
        )?;
        compare_counts(
            &origin,
            "artifact assertions",
            &expected.assertions,
            &actual.assertions,
        )?;
    }

    Ok(registered.len())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    contracts: BTreeMap<String, usize>,
    goldens: BTreeMap<String, usize>,
    generations: BTreeMap<String, usize>,
    assertions: BTreeMap<String, usize>,
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
    for (line_index, logical_command) in logical_tcl_commands(source).iter().enumerate() {
        let line = logical_command.trim();
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
            | "compile_verilog_pass_warning"
            | "compile_verilog_schedule_pass" => {
                let source = required_word(&words, 1, origin, line_index)?;
                add_count(&mut counts.contracts, contract_key("compile", source));
            }
            "test_c_veri_bsv" | "test_c_veri_bsv_modules" | "test_c_veri_bsv_modules_options" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bsv");
                add_simulation_counts(&mut counts, &source, true, true, false);
            }
            "test_c_veri_bsv_multi" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bsv");
                add_simulation_counts(&mut counts, &source, true, true, false);
            }
            "test_c_veri_bsv_multi_options" | "test_c_veri_bsv_multi_options_separately" => {
                let parsed = tcl_words(line).map_err(|error| {
                    format!("parse multi helper in {}: {error}", origin.display())
                })?;
                let module = parsed.get(1).ok_or_else(|| {
                    format!("missing multi helper source in {}", origin.display())
                })?;
                let (bluesim, icarus) = multi_backend_flags(&parsed).ok_or_else(|| {
                    format!(
                        "dynamic or disabled multi helper backends in {}",
                        origin.display()
                    )
                })?;
                let source = format!("{module}.bsv");
                add_simulation_counts(
                    &mut counts,
                    &source,
                    bluesim,
                    icarus,
                    command == "test_c_veri_bsv_multi_options_separately",
                );
            }
            "test_c_veri" | "test_c_veri_bs_modules" | "test_c_veri_bs_modules_options" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bs");
                add_count(&mut counts.contracts, contract_key("bluesim", &source));
                add_count(&mut counts.contracts, contract_key("icarus", &source));
                add_count(
                    &mut counts.generations,
                    generation_key_name("shared", &source),
                );
            }
            "test_c_veri_bsv_separately" | "test_c_veri_bsv_modules_options_separately" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bsv");
                add_count(&mut counts.contracts, contract_key("bluesim", &source));
                add_count(&mut counts.contracts, contract_key("icarus", &source));
                add_count(
                    &mut counts.generations,
                    generation_key_name("bluesim", &source),
                );
                add_count(
                    &mut counts.generations,
                    generation_key_name("icarus", &source),
                );
            }
            "test_c_only_bsv"
            | "test_c_only_bsv_modules"
            | "test_c_only_bsv_modules_options"
            | "test_c_only_bsv_multi"
            | "test_c_only_bsv_multi_options" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bsv");
                add_simulation_counts(&mut counts, &source, true, false, true);
            }
            "test_veri_only_bsv"
            | "test_veri_only_bsv_modules"
            | "test_veri_only_bsv_modules_options"
            | "test_veri_only_bsv_multi"
            | "test_veri_only_bsv_multi_options" => {
                let module = required_word(&words, 1, origin, line_index)?;
                let source = format!("{module}.bsv");
                add_simulation_counts(&mut counts, &source, false, true, true);
            }
            "compare_file" => {
                let output = required_word(&words, 1, origin, line_index)?;
                if output == "[make_bsc_output_name" {
                    let source = required_word(&words, 2, origin, line_index)?;
                    add_count(&mut counts.goldens, source.to_owned());
                } else if let Some(source) = output
                    .strip_suffix(".bsc-vcomp-out")
                    .or_else(|| output.strip_suffix(".bsc-out"))
                {
                    add_count(&mut counts.goldens, source.to_owned());
                } else {
                    let expected = words
                        .get(2)
                        .map(|value| value.trim_matches(['"', '{', '}']))
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("{output}.expected"));
                    add_count(
                        &mut counts.assertions,
                        matches_assertion_key(
                            output,
                            &expected,
                            ArtifactNormalization::GoldenOutput,
                        ),
                    );
                }
            }
            "compare_verilog" => {
                let output = required_word(&words, 1, origin, line_index)?;
                let expected = words
                    .get(2)
                    .map(|value| value.trim_matches(['"', '{', '}']))
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("{output}.expected"));
                add_count(
                    &mut counts.assertions,
                    matches_assertion_key(output, &expected, ArtifactNormalization::Verilog),
                );
            }
            _ => {}
        }
    }
    parse_exp_assertions(source, origin, &mut counts)?;
    Ok(counts)
}

fn add_simulation_counts(
    counts: &mut Counts,
    source: &str,
    bluesim: bool,
    icarus: bool,
    separate_generation: bool,
) {
    if bluesim {
        add_count(&mut counts.contracts, contract_key("bluesim", source));
    }
    if icarus {
        add_count(&mut counts.contracts, contract_key("icarus", source));
    }
    if bluesim && icarus && !separate_generation {
        add_count(
            &mut counts.generations,
            generation_key_name("shared", source),
        );
    } else {
        if bluesim {
            add_count(
                &mut counts.generations,
                generation_key_name("bluesim", source),
            );
        }
        if icarus {
            add_count(
                &mut counts.generations,
                generation_key_name("icarus", source),
            );
        }
    }
}

fn parse_exp_assertions(source: &str, origin: &Path, counts: &mut Counts) -> Result<(), String> {
    for command in logical_tcl_commands(source) {
        let name = command.split_whitespace().next().unwrap_or_default();
        if !is_supported_tcl_assertion(name) {
            continue;
        }
        let words = tcl_words(&command)
            .map_err(|error| format!("parse assertion in {}: {error}", origin.display()))?;
        let argument = |index: usize| {
            words.get(index).map(String::as_str).ok_or_else(|| {
                format!(
                    "missing argument {index} for assertion in {}: {command}",
                    origin.display()
                )
            })
        };
        let path = normalize_assertion_path(argument(1)?)?;
        let key = match name {
            "find_n_strings" => line_count_assertion_key(
                &path,
                argument(2)?,
                parse_assertion_count(argument(3)?, origin)?,
            ),
            "string_occurs" => contains_assertion_key(&path, argument(2)?),
            "string_does_not_occur" => does_not_contain_assertion_key(&path, argument(2)?),
            "find_regexp" => regex_assertion_key(&path, argument(2)?),
            "find_regexp_fail" => regex_does_not_match_assertion_key(&path, argument(2)?),
            "find_n_regexp" => regex_count_assertion_key(
                &path,
                argument(2)?,
                parse_assertion_count(argument(3)?, origin)?,
            ),
            "find_n_emsg" => diagnostic_assertion_key(
                &path,
                parse_diagnostic_kind(argument(2)?, origin)?,
                argument(3)?,
                parse_assertion_count(argument(4)?, origin)?,
            ),
            _ => unreachable!(),
        };
        add_count(&mut counts.assertions, key);
    }
    Ok(())
}

fn is_supported_tcl_assertion(name: &str) -> bool {
    matches!(
        name,
        "find_n_strings"
            | "string_occurs"
            | "string_does_not_occur"
            | "find_regexp"
            | "find_regexp_fail"
            | "find_n_regexp"
            | "find_n_emsg"
    )
}

fn logical_tcl_commands(source: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut current = String::new();
    let mut separator = None;
    for raw_line in source.lines() {
        if current.is_empty()
            && (raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#'))
        {
            continue;
        }
        let preserve_multiline_pattern = separator == Some('\n');
        let line = if preserve_multiline_pattern {
            raw_line
        } else {
            raw_line.trim_start()
        };
        let (part, continued) = match line.trim_end().strip_suffix('\\') {
            Some(part) => (part.trim_end(), true),
            None => (line, false),
        };
        if let Some(separator) = separator.take() {
            current.push(separator);
        }
        current.push_str(part);

        let name = current.split_whitespace().next().unwrap_or_default();
        let grouped_assertion =
            is_supported_tcl_assertion(name) && !tcl_groups_are_balanced(&current);
        if continued || grouped_assertion {
            separator = Some(if continued { ' ' } else { '\n' });
        } else if !current.is_empty() && !current.starts_with('#') {
            commands.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
}

fn tcl_groups_are_balanced(command: &str) -> bool {
    let mut closing = Vec::new();
    let mut escaped = false;
    for character in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        match closing.last().copied() {
            Some('}') => match character {
                '{' => closing.push('}'),
                '}' => {
                    closing.pop();
                }
                _ => {}
            },
            Some('"') => match character {
                '"' => {
                    closing.pop();
                }
                '[' => closing.push(']'),
                _ => {}
            },
            Some(']') => match character {
                '{' => closing.push('}'),
                '"' => closing.push('"'),
                '[' => closing.push(']'),
                ']' => {
                    closing.pop();
                }
                _ => {}
            },
            None => match character {
                '{' => closing.push('}'),
                '"' => closing.push('"'),
                '[' => closing.push(']'),
                _ => {}
            },
            Some(_) => unreachable!(),
        }
    }
    closing.is_empty()
}

fn tcl_words(command: &str) -> Result<Vec<String>, String> {
    let chars = command.chars().collect::<Vec<_>>();
    let mut words = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index == chars.len() {
            break;
        }
        let opening = chars[index];
        if matches!(opening, '{' | '[' | '"') {
            let closing = match opening {
                '{' => '}',
                '[' => ']',
                '"' => '"',
                _ => unreachable!(),
            };
            let preserve_delimiters = opening == '[';
            let start = index;
            index += 1;
            let content_start = index;
            let mut depth = 1;
            while index < chars.len() {
                if opening != '"' && chars[index] == opening {
                    depth += 1;
                } else if chars[index] == closing {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                index += 1;
            }
            if index == chars.len() {
                return Err(format!("unterminated {opening} group: {command}"));
            }
            let word = if preserve_delimiters {
                chars[start..=index].iter().collect()
            } else {
                chars[content_start..index].iter().collect()
            };
            words.push(word);
            index += 1;
        } else {
            let start = index;
            while index < chars.len() && !chars[index].is_whitespace() {
                index += 1;
            }
            words.push(chars[start..index].iter().collect());
        }
    }
    Ok(words)
}

fn normalize_assertion_path(path: &str) -> Result<String, String> {
    if let Some(source) = path
        .strip_prefix("[make_bsc_output_name ")
        .and_then(|value| value.strip_suffix(']'))
    {
        if source.is_empty() {
            return Err("make_bsc_output_name assertion has no source".to_owned());
        }
        Ok(format!("{source}.bsc-out"))
    } else if let Some(source) = path.strip_suffix(".bsc-vcomp-out") {
        Ok(format!("{source}.bsc-out"))
    } else {
        Ok(path.to_owned())
    }
}

fn parse_assertion_count(value: &str, origin: &Path) -> Result<usize, String> {
    value.parse().map_err(|error| {
        format!(
            "invalid assertion count {value:?} in {}: {error}",
            origin.display()
        )
    })
}

fn parse_diagnostic_kind(value: &str, origin: &Path) -> Result<DiagnosticKind, String> {
    match value.trim_matches('"') {
        "Error" => Ok(DiagnosticKind::Error),
        "Warning" => Ok(DiagnosticKind::Warning),
        other => Err(format!(
            "unsupported diagnostic kind {other:?} in {}",
            origin.display()
        )),
    }
}

fn artifact_assertion_key(assertion: ArtifactAssertion) -> Option<String> {
    let key = match assertion {
        ArtifactAssertion::Exists { path } => format!("exists:{path:?}"),
        ArtifactAssertion::Text { path, assertion } => match assertion {
            TextAssertion::Contains { text } => contains_assertion_key(path, text),
            TextAssertion::DoesNotContain { text } => does_not_contain_assertion_key(path, text),
            TextAssertion::LineCount { text, count } => line_count_assertion_key(path, text, count),
            TextAssertion::Regex { pattern } => regex_assertion_key(path, pattern),
            TextAssertion::RegexDoesNotMatch { pattern } => {
                regex_does_not_match_assertion_key(path, pattern)
            }
            TextAssertion::RegexCount { pattern, count } => {
                regex_count_assertion_key(path, pattern, count)
            }
            TextAssertion::DiagnosticCount { kind, tag, count } => {
                diagnostic_assertion_key(path, kind, tag, count)
            }
        },
        ArtifactAssertion::Matches {
            actual,
            expected,
            normalization,
        } => matches_assertion_key(actual, expected, normalization),
        ArtifactAssertion::ParsesAsSystemVerilog { .. } => return None,
    };
    Some(key)
}

fn matches_assertion_key(
    actual: &str,
    expected: &str,
    normalization: ArtifactNormalization,
) -> String {
    let normalization = match normalization {
        ArtifactNormalization::Exact => "exact",
        ArtifactNormalization::GoldenOutput => "golden-output",
        ArtifactNormalization::Verilog => "verilog",
    };
    format!("matches:{actual:?}:{expected:?}:{normalization}")
}

fn contains_assertion_key(path: &str, text: &str) -> String {
    format!("contains:{path:?}:{text:?}")
}

fn does_not_contain_assertion_key(path: &str, text: &str) -> String {
    format!("does-not-contain:{path:?}:{text:?}")
}

fn line_count_assertion_key(path: &str, text: &str, count: usize) -> String {
    format!("line-count:{path:?}:{text:?}:{count}")
}

fn regex_assertion_key(path: &str, pattern: &str) -> String {
    format!("regex:{path:?}:{pattern:?}")
}

fn regex_does_not_match_assertion_key(path: &str, pattern: &str) -> String {
    format!("regex-does-not-match:{path:?}:{pattern:?}")
}

fn regex_count_assertion_key(path: &str, pattern: &str, count: usize) -> String {
    format!("regex-count:{path:?}:{pattern:?}:{count}")
}

fn diagnostic_assertion_key(path: &str, kind: DiagnosticKind, tag: &str, count: usize) -> String {
    let kind = match kind {
        DiagnosticKind::Error => "Error",
        DiagnosticKind::Warning => "Warning",
    };
    format!("diagnostic-count:{path:?}:{kind}:{tag:?}:{count}")
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

fn generation_key(strategy: GenerationStrategy, source: &str) -> String {
    let strategy = match strategy {
        GenerationStrategy::BackendSpecific(SimulationBackend::Bluesim) => "bluesim",
        GenerationStrategy::BackendSpecific(SimulationBackend::Icarus) => "icarus",
        GenerationStrategy::SharedElaboration => "shared",
    };
    generation_key_name(strategy, source)
}

fn generation_key_name(strategy: &str, source: &str) -> String {
    format!("{strategy}:{source}")
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
            "test_c_veri_bsv_separately Separate\n",
            "test_c_only_bsv_modules_options COnly {} {} expected\n",
            "test_veri_only_bsv_modules VOnly {} expected\n",
        );
        let actual = parse_exp_contracts(source, Path::new("sample.exp")).unwrap();
        let expected_contracts = [
            ("compile:Good.bsv", 1),
            ("compile:Bad.bsv", 1),
            ("bluesim:Both.bsv", 1),
            ("icarus:Both.bsv", 1),
            ("bluesim:Separate.bsv", 1),
            ("icarus:Separate.bsv", 1),
            ("bluesim:COnly.bsv", 1),
            ("icarus:VOnly.bsv", 1),
        ]
        .into_iter()
        .map(|(key, count)| (key.to_owned(), count))
        .collect();
        assert_eq!(actual.contracts, expected_contracts);
        assert_eq!(actual.goldens, BTreeMap::from([("Bad.bsv".to_owned(), 1)]));
        assert_eq!(
            actual.generations,
            BTreeMap::from([
                ("shared:Both.bsv".to_owned(), 1),
                ("bluesim:Separate.bsv".to_owned(), 1),
                ("icarus:Separate.bsv".to_owned(), 1),
                ("bluesim:COnly.bsv".to_owned(), 1),
                ("icarus:VOnly.bsv".to_owned(), 1),
            ])
        );
    }

    #[test]
    fn parses_supported_tcl_artifact_assertions() {
        let source = concat!(
            "find_n_strings Output.bsc-out {argument 2} 1\n",
            "string_occurs Generated.v {input  CLK;}\n",
            "string_does_not_occur Generated.v {input  GATE;}\n",
            "find_regexp [make_bsc_output_name Source.bsv] \\\n",
            "    {Source\\.bsv\", line 2, column 8:}\n",
            "find_n_regexp Output.bsc-out {Error:} 2\n",
            "find_n_emsg Output.bsc-out \"Error\" G0055 1\n",
            "compare_file Generated.dat\n",
            "compare_file Other.dat Custom.expected\n",
            "compare_verilog Generated.v\n",
            "# string_occurs Ignored.v {ignored}\n",
        );
        let actual = parse_exp_contracts(source, Path::new("sample.exp")).unwrap();
        assert_eq!(
            actual.assertions,
            BTreeMap::from([
                (
                    line_count_assertion_key("Output.bsc-out", "argument 2", 1),
                    1
                ),
                (contains_assertion_key("Generated.v", "input  CLK;"), 1),
                (
                    does_not_contain_assertion_key("Generated.v", "input  GATE;"),
                    1,
                ),
                (
                    regex_assertion_key("Source.bsv.bsc-out", r#"Source\.bsv", line 2, column 8:"#,),
                    1,
                ),
                (regex_count_assertion_key("Output.bsc-out", "Error:", 2), 1),
                (
                    diagnostic_assertion_key("Output.bsc-out", DiagnosticKind::Error, "G0055", 1,),
                    1,
                ),
                (
                    matches_assertion_key(
                        "Generated.dat",
                        "Generated.dat.expected",
                        ArtifactNormalization::GoldenOutput,
                    ),
                    1,
                ),
                (
                    matches_assertion_key(
                        "Other.dat",
                        "Custom.expected",
                        ArtifactNormalization::GoldenOutput,
                    ),
                    1,
                ),
                (
                    matches_assertion_key(
                        "Generated.v",
                        "Generated.v.expected",
                        ArtifactNormalization::Verilog,
                    ),
                    1,
                ),
            ])
        );
    }

    #[test]
    fn parses_negative_regex_assertions_with_tcl_word_forms_and_multiplicity() {
        let source = concat!(
            "find_regexp_fail Generated.bsv.bsc-vcomp-out {forbidden.*port}\n",
            "find_regexp_fail \"Quoted Output.log\" \"^Internal Error$\"\n",
            "find_regexp_fail [make_bsc_output_name Source.bsv] \\\n",
            "    {Internal.*Error}\n",
            "find_regexp_fail BracedMultiline.log {^first$\n",
            "    second$}\n",
            "find_regexp_fail QuotedMultiline.log \"^alpha$\n",
            "  omega$\"\n",
            "find_regexp_fail Generated.bsv.bsc-vcomp-out {forbidden.*port}\n",
        );
        let actual = parse_exp_contracts(source, Path::new("sample.exp")).unwrap();
        assert_eq!(
            actual.assertions,
            BTreeMap::from([
                (
                    regex_does_not_match_assertion_key("Generated.bsv.bsc-out", "forbidden.*port",),
                    2,
                ),
                (
                    regex_does_not_match_assertion_key("Quoted Output.log", "^Internal Error$",),
                    1,
                ),
                (
                    regex_does_not_match_assertion_key("Source.bsv.bsc-out", "Internal.*Error",),
                    1,
                ),
                (
                    regex_does_not_match_assertion_key(
                        "BracedMultiline.log",
                        "^first$\n    second$",
                    ),
                    1,
                ),
                (
                    regex_does_not_match_assertion_key("QuotedMultiline.log", "^alpha$\n  omega$",),
                    1,
                ),
            ])
        );
        assert_eq!(
            artifact_assertion_key(ArtifactAssertion::Text {
                path: "Generated.bsv.bsc-out",
                assertion: TextAssertion::RegexDoesNotMatch {
                    pattern: "forbidden.*port",
                },
            }),
            Some(regex_does_not_match_assertion_key(
                "Generated.bsv.bsc-out",
                "forbidden.*port"
            ))
        );
    }

    #[test]
    fn excludes_rust_only_parser_assertions_from_upstream_multiplicity() {
        let assertions = [
            ArtifactAssertion::ParsesAsSystemVerilog {
                path: "Generated.v",
            },
            ArtifactAssertion::Text {
                path: "Generated.v",
                assertion: TextAssertion::Contains { text: "module" },
            },
        ];
        let mut aligned = BTreeMap::new();
        for key in assertions.into_iter().filter_map(artifact_assertion_key) {
            add_count(&mut aligned, key);
        }

        assert_eq!(
            aligned,
            BTreeMap::from([(contains_assertion_key("Generated.v", "module"), 1)])
        );
    }

    #[test]
    fn counts_statically_declared_contract_multiplicity() {
        let source = concat!(
            "compile_pass Good.bsv\n",
            "test_c_veri_bsv Both\n",
            "test_c_veri ClassicBoth\n",
            "test_c_only_bsv COnly expected\n",
            "test_veri_only_bsv VOnly expected\n",
            "test_c_veri_bsv_separately Separate\n",
            "test_c_only_bsv_modules CModules {}\n",
            "# compile_fail Ignored.bsv\n",
            "compare_file Good.bsv.bsc-out\n",
            "foreach item $items {\n",
        );
        assert_eq!(count_statically_declared_contracts(source), 10);
    }

    #[test]
    fn counts_multi_simulation_helpers_by_enabled_backend() {
        let source = concat!(
            "test_c_veri_bsv_multi Dual mkDual {mkChild}\n",
            "test_c_veri_bsv_multi_options Bluesim mkBluesim {} {} {} {} {} 1 0\n",
            "test_c_veri_bsv_multi_options Icarus mkIcarus {} {} {} {} {} 0 1\n",
            "test_c_veri_bsv_multi_options_separately Separate mkSeparate {}\n",
            "test_c_only_bsv_multi COnly mkCOnly {}\n",
            "test_veri_only_bsv_multi VOnly mkVOnly {}\n",
        );
        assert_eq!(count_statically_declared_contracts(source), 8);

        let parsed = parse_exp_contracts(source, Path::new("multi.exp")).unwrap();
        assert_eq!(
            parsed.generations,
            BTreeMap::from([
                ("bluesim:Bluesim.bsv".to_owned(), 1),
                ("bluesim:COnly.bsv".to_owned(), 1),
                ("bluesim:Separate.bsv".to_owned(), 1),
                ("icarus:Icarus.bsv".to_owned(), 1),
                ("icarus:Separate.bsv".to_owned(), 1),
                ("icarus:VOnly.bsv".to_owned(), 1),
                ("shared:Dual.bsv".to_owned(), 1),
            ])
        );
    }

    #[test]
    fn multi_inventory_rejects_bug_gates_and_dynamic_backend_flags() {
        let source = concat!(
            "test_c_veri_bsv_multi Clean mkClean {}\n",
            "test_c_veri_bsv_multi Bugged mkBugged {} {} 123\n",
            "test_c_veri_bsv_multi_options Dynamic mkDynamic {} {} {} {} {} $doC 1\n",
        );
        assert_eq!(
            unsupported_tcl_commands(source),
            vec![
                UnsupportedTclCommand {
                    name: "test_c_veri_bsv_multi".to_owned(),
                    count: 1,
                    category: TclCommandCategory::UnsupportedContract,
                },
                UnsupportedTclCommand {
                    name: "test_c_veri_bsv_multi_options".to_owned(),
                    count: 1,
                    category: TclCommandCategory::UnsupportedContract,
                },
            ]
        );
    }

    #[test]
    fn treats_supported_inventory_vocabulary_as_migration_ready() {
        let source = concat!(
            "compile_pass Good.bsv\n",
            "test_c_veri_bsv Both\n",
            "find_n_strings Both.bsc-out {success} 1\n",
            "compare_file Both.out\n",
            "compare_verilog Both.v\n",
        );
        assert!(unsupported_tcl_commands(source).is_empty());
    }

    #[test]
    fn classifies_unsupported_inventory_commands() {
        let source = concat!(
            "compile_pass Good.bsv\n",
            "if {$vtest} {\n",
            "copy A B\n",
            "link_verilog_no_main_pass Top\n",
            "find_n_error out 1\n",
            "}\n",
        );
        assert_eq!(
            unsupported_tcl_commands(source),
            vec![
                UnsupportedTclCommand {
                    name: "copy".to_owned(),
                    count: 1,
                    category: TclCommandCategory::FileSystem,
                },
                UnsupportedTclCommand {
                    name: "find_n_error".to_owned(),
                    count: 1,
                    category: TclCommandCategory::UnsupportedAssertion,
                },
                UnsupportedTclCommand {
                    name: "if".to_owned(),
                    count: 1,
                    category: TclCommandCategory::ControlState,
                },
                UnsupportedTclCommand {
                    name: "link_verilog_no_main_pass".to_owned(),
                    count: 1,
                    category: TclCommandCategory::ManualToolchain,
                },
            ]
        );
    }

    #[test]
    fn known_blocker_overrides_lexical_candidate_status() {
        assert_eq!(
            migration_readiness(
                "testsuite/bsc.evaluator/performance/performance.exp",
                1,
                &[],
            ),
            MigrationReadiness::Blocked
        );
        assert_eq!(
            migration_readiness("testsuite/example/example.exp", 1, &[]),
            MigrationReadiness::Candidate
        );
    }

    #[test]
    fn preserves_upstream_static_contract_denominator_with_multi_helpers() {
        assert_eq!(
            check_alignment()
                .expect("alignment should remain valid")
                .total_statically_declared_contracts,
            5_269
        );
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
