use crate::locate_project_root;
use crate::upstream::{
    compile_case_modules, compile_cases, simulation_scenario_modules, simulation_scenarios,
    validate_simulation_scenario, ArtifactAssertion, ArtifactNormalization, CaseModule,
    DiagnosticKind, GenerationStrategy, SimulationBackend, TextAssertion,
};
use bsc_testsuite_manifest::model::{
    AssertionContract as ManifestAssertion, ComparisonContract as ManifestComparison,
    Contract as ManifestContract, ExternalContractKind,
    GenerationStrategy as ManifestGenerationStrategy, ScriptManifest,
    SimulationBackend as ManifestBackend, TestsuiteManifest, UnsupportedReason,
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
    pub total_contracts: usize,
    pub migrated_contracts: usize,
    pub remaining_contracts: usize,
    pub scripts_without_contracts: usize,
}

pub fn check_alignment() -> Result<AlignmentSummary, String> {
    let project_root = locate_project_root()?;
    let manifest = load_manifest(&project_root)?;
    let compile_cases = compile_cases();
    let simulation_scenarios = simulation_scenarios();
    let simulation_contracts = simulation_scenarios
        .iter()
        .map(|scenario| scenario.contracts.len())
        .sum::<usize>();
    let scripts = check_upstream_cases(&project_root, &manifest)?;
    let scheduler_cases = check_scheduler_sat(&project_root, &manifest)?;
    let inventory = inventory_testsuite(&manifest);
    let migrated_test_scripts = scripts + SCHEDULER_ORIGINS.len();
    let remaining_test_scripts = inventory
        .total_test_scripts
        .checked_sub(migrated_test_scripts)
        .ok_or_else(|| "migrated test script count exceeds testsuite script count".to_owned())?;
    let migrated_contracts = compile_cases.len() + simulation_contracts + scheduler_cases;
    let remaining_contracts = inventory
        .total_contracts
        .checked_sub(migrated_contracts)
        .ok_or_else(|| {
            "migrated contract count exceeds typed testsuite contract count".to_owned()
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
        total_contracts: inventory.total_contracts,
        migrated_contracts,
        remaining_contracts,
        scripts_without_contracts: inventory.scripts_without_contracts,
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
    pub contract_count: usize,
    pub readiness: MigrationReadiness,
    pub unsupported_commands: Vec<UnsupportedTclCommand>,
    pub known_blocker: Option<String>,
}

pub fn remaining_inventory() -> Result<Vec<RemainingTestScript>, String> {
    let summary = check_alignment()?;
    let project_root = locate_project_root()?;
    let manifest = load_manifest(&project_root)?;
    let migrated = collect_migrated_origins(&project_root)?;
    let remaining = collect_testsuite_scripts(&manifest)
        .into_iter()
        .filter(|script| !migrated.contains(&script.origin))
        .map(|script| {
            let known_blocker = known_migration_blocker(&script.origin).map(str::to_owned);
            let readiness = migration_readiness(
                &script.origin,
                script.contract_count,
                &script.unsupported_commands,
            );
            RemainingTestScript {
                origin: script.origin,
                contract_count: script.contract_count,
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
        .map(|script| script.contract_count)
        .sum::<usize>();
    if remaining.len() != summary.remaining_test_scripts
        || remaining_contracts != summary.remaining_contracts
    {
        return Err(format!(
            "remaining inventory does not match alignment summary: {} scripts/{remaining_contracts} contracts, expected {}/{}",
            remaining.len(), summary.remaining_test_scripts, summary.remaining_contracts
        ));
    }
    Ok(remaining)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TestsuiteInventory {
    total_test_scripts: usize,
    total_contracts: usize,
    scripts_without_contracts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestsuiteScript {
    origin: String,
    contract_count: usize,
    unsupported_commands: Vec<UnsupportedTclCommand>,
}

fn load_manifest(project_root: &Path) -> Result<TestsuiteManifest, String> {
    bsc_testsuite_manifest::build_manifest(project_root)
        .map_err(|error| format!("build typed contract manifest: {error}"))
}

fn inventory_testsuite(manifest: &TestsuiteManifest) -> TestsuiteInventory {
    let mut inventory = TestsuiteInventory::default();
    for script in collect_testsuite_scripts(manifest) {
        inventory.total_test_scripts += 1;
        inventory.total_contracts += script.contract_count;
        inventory.scripts_without_contracts += usize::from(script.contract_count == 0);
    }
    inventory
}

fn collect_testsuite_scripts(manifest: &TestsuiteManifest) -> Vec<TestsuiteScript> {
    manifest
        .scripts
        .iter()
        .map(|script| TestsuiteScript {
            origin: script.origin.clone(),
            contract_count: script
                .contracts
                .iter()
                .map(ManifestContract::effective_count)
                .sum(),
            unsupported_commands: unsupported_manifest_commands(script),
        })
        .collect()
}

fn unsupported_manifest_commands(script: &ScriptManifest) -> Vec<UnsupportedTclCommand> {
    let mut counts = BTreeMap::<String, usize>::new();
    for unsupported in &script.unsupported {
        let name = unsupported
            .command
            .clone()
            .unwrap_or_else(|| unsupported_reason_label(unsupported.reason).to_owned());
        *counts.entry(name).or_default() += 1;
    }
    for action in &script.workflow_actions {
        *counts.entry(action.helper_name().to_owned()).or_default() += 1;
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

fn unsupported_reason_label(reason: UnsupportedReason) -> &'static str {
    match reason {
        UnsupportedReason::DynamicAssignment => "dynamic_assignment",
        UnsupportedReason::DynamicArguments => "dynamic_arguments",
        UnsupportedReason::UnsupportedCommand => "unsupported_command",
        UnsupportedReason::UnsupportedControlFlow => "unsupported_control_flow",
        UnsupportedReason::UnsupportedSyntax => "unsupported_syntax",
    }
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
            | "dynamic_assignment"
            | "dynamic_arguments"
            | "unsupported_control_flow"
            | "unsupported_syntax"
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
    contract_count: usize,
    unsupported_commands: &[UnsupportedTclCommand],
) -> MigrationReadiness {
    if known_migration_blocker(origin).is_some() {
        MigrationReadiness::Blocked
    } else if contract_count == 0 {
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

fn check_upstream_cases(
    project_root: &Path,
    manifest: &TestsuiteManifest,
) -> Result<usize, String> {
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
        let origin_key = project_relative_unix_path(project_root, &origin)?;
        let script = manifest
            .scripts
            .iter()
            .find(|script| script.origin == origin_key)
            .ok_or_else(|| format!("typed manifest is missing {origin_key}"))?;
        let expected = manifest_counts(script, &origin)?;
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

fn manifest_counts(script: &ScriptManifest, origin: &Path) -> Result<Counts, String> {
    let mut counts = Counts::default();
    let mut generations = BTreeSet::new();

    for contract in &script.contracts {
        match contract {
            ManifestContract::Compile(contract) => {
                add_count(
                    &mut counts.contracts,
                    contract_key("compile", &contract.source),
                );
            }
            ManifestContract::Simulation(contract) => {
                let backend = match contract.backend {
                    ManifestBackend::Bluesim => "bluesim",
                    ManifestBackend::Icarus => "icarus",
                };
                add_count(
                    &mut counts.contracts,
                    contract_key(backend, &contract.source),
                );
                let strategy = match contract.generation {
                    ManifestGenerationStrategy::Shared => "shared",
                    ManifestGenerationStrategy::Bluesim => "bluesim",
                    ManifestGenerationStrategy::Icarus => "icarus",
                };
                let key = generation_key_name(strategy, &contract.source);
                let instance = format!(
                    "{}:{}:{:?}:{key}",
                    contract.span.start_byte, contract.span.end_byte, contract.expansion
                );
                if generations.insert(instance) {
                    add_count(&mut counts.generations, key);
                }
            }
            ManifestContract::ExternalSet(_) => {}
        }
    }

    for comparison in &script.comparisons {
        add_manifest_comparison(&mut counts, comparison, origin)?;
    }
    for assertion in &script.assertions {
        add_count(
            &mut counts.assertions,
            manifest_assertion_key(assertion, origin)?,
        );
    }
    Ok(counts)
}

fn add_manifest_comparison(
    counts: &mut Counts,
    comparison: &ManifestComparison,
    origin: &Path,
) -> Result<(), String> {
    let output = comparison.arguments.first().ok_or_else(|| {
        format!(
            "{} comparison at line {} has no output path",
            comparison.helper, comparison.span.start_line
        )
    })?;
    match comparison.helper.as_str() {
        "compare_file" => {
            if let Some(source) = output
                .strip_suffix(".bsc-vcomp-out")
                .or_else(|| output.strip_suffix(".bsc-sched-out"))
                .or_else(|| output.strip_suffix(".bsc-out"))
            {
                add_count(&mut counts.goldens, source.to_owned());
            } else {
                let expected = comparison
                    .arguments
                    .get(1)
                    .filter(|value| !value.is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("{output}.expected"));
                add_count(
                    &mut counts.assertions,
                    matches_assertion_key(output, &expected, ArtifactNormalization::GoldenOutput),
                );
            }
        }
        "compare_verilog" => {
            let expected = comparison
                .arguments
                .get(1)
                .filter(|value| !value.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{output}.expected"));
            add_count(
                &mut counts.assertions,
                matches_assertion_key(output, &expected, ArtifactNormalization::Verilog),
            );
        }
        helper => {
            return Err(format!(
                "unsupported typed comparison {helper:?} in {}",
                origin.display()
            ));
        }
    }
    Ok(())
}

fn manifest_assertion_key(assertion: &ManifestAssertion, origin: &Path) -> Result<String, String> {
    let argument = |index: usize| {
        assertion
            .arguments
            .get(index)
            .map(String::as_str)
            .ok_or_else(|| {
                format!(
                    "missing argument {index} for {} at {}:{}",
                    assertion.helper,
                    origin.display(),
                    assertion.span.start_line,
                )
            })
    };
    let path = normalize_assertion_path(argument(0)?)?;
    match assertion.helper.as_str() {
        "find_n_strings" => Ok(line_count_assertion_key(
            &path,
            argument(1)?,
            parse_assertion_count(argument(2)?, origin)?,
        )),
        "string_occurs" => Ok(contains_assertion_key(&path, argument(1)?)),
        "string_does_not_occur" => Ok(does_not_contain_assertion_key(&path, argument(1)?)),
        "find_regexp" => Ok(regex_assertion_key(&path, argument(1)?)),
        "find_regexp_fail" => Ok(regex_does_not_match_assertion_key(&path, argument(1)?)),
        "find_n_regexp" => Ok(regex_count_assertion_key(
            &path,
            argument(1)?,
            parse_assertion_count(argument(2)?, origin)?,
        )),
        "find_n_emsg" => Ok(diagnostic_assertion_key(
            &path,
            parse_diagnostic_kind(argument(1)?, origin)?,
            argument(2)?,
            parse_assertion_count(argument(3)?, origin)?,
        )),
        helper => Err(format!(
            "unsupported typed assertion {helper:?} in {}",
            origin.display()
        )),
    }
}

fn normalize_assertion_path(path: &str) -> Result<String, String> {
    if let Some(source) = path
        .strip_prefix("[make_bsc_vcomp_output_name ")
        .and_then(|value| value.strip_suffix(']'))
    {
        if source.is_empty() {
            return Err("make_bsc_vcomp_output_name assertion has no source".to_owned());
        }
        Ok(format!("{source}.bsc-out"))
    } else if let Some(source) = path
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
        ArtifactNormalization::Exact => "exact".to_owned(),
        ArtifactNormalization::GoldenOutput => "golden-output".to_owned(),
        ArtifactNormalization::Verilog => "verilog".to_owned(),
        ArtifactNormalization::DecimalTolerance { .. } => "golden-output".to_owned(),
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
    format!("regex:{path:?}:{:?}", canonical_regex_pattern(pattern))
}

fn regex_does_not_match_assertion_key(path: &str, pattern: &str) -> String {
    format!(
        "regex-does-not-match:{path:?}:{:?}",
        canonical_regex_pattern(pattern)
    )
}

fn regex_count_assertion_key(path: &str, pattern: &str, count: usize) -> String {
    format!(
        "regex-count:{path:?}:{:?}:{count}",
        canonical_regex_pattern(pattern)
    )
}

fn canonical_regex_pattern(pattern: &str) -> String {
    let mut canonical = String::with_capacity(pattern.len());
    let mut characters = pattern.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\'
            && characters
                .peek()
                .copied()
                .is_some_and(is_redundantly_escaped_regex_punctuation)
        {
            canonical.push(characters.next().expect("peeked regex character"));
        } else {
            canonical.push(character);
        }
    }
    canonical
}

fn is_redundantly_escaped_regex_punctuation(character: char) -> bool {
    character.is_ascii_punctuation()
        && !matches!(
            character,
            '\\' | '.'
                | '+'
                | '*'
                | '?'
                | '('
                | ')'
                | '|'
                | '['
                | ']'
                | '{'
                | '}'
                | '^'
                | '$'
                | '-'
        )
}

fn diagnostic_assertion_key(path: &str, kind: DiagnosticKind, tag: &str, count: usize) -> String {
    let kind = match kind {
        DiagnosticKind::Error => "Error",
        DiagnosticKind::Warning => "Warning",
    };
    format!("diagnostic-count:{path:?}:{kind}:{tag:?}:{count}")
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

fn check_scheduler_sat(project_root: &Path, manifest: &TestsuiteManifest) -> Result<usize, String> {
    let origin = project_root.join(SCHEDULER_ORIGINS[0]);
    let script = manifest
        .scripts
        .iter()
        .find(|script| script.origin == SCHEDULER_ORIGINS[0])
        .ok_or_else(|| format!("typed manifest is missing {}", SCHEDULER_ORIGINS[0]))?;
    let mut external_sets = script
        .contracts
        .iter()
        .filter_map(|contract| match contract {
            ManifestContract::ExternalSet(contract)
                if contract.kind == ExternalContractKind::SchedulerSat =>
            {
                Some(&contract.cases)
            }
            _ => None,
        });
    let expected = external_sets
        .next()
        .ok_or_else(|| "typed manifest has no scheduler SAT contract set".to_owned())?;
    if external_sets.next().is_some() {
        return Err("typed manifest has multiple scheduler SAT contract sets".to_owned());
    }

    let rust_path = project_root.join("rust-tests/tests/scheduler_sat.rs");
    let rust_source = fs::read_to_string(&rust_path)
        .map_err(|error| format!("read scheduler Rust tests {}: {error}", rust_path.display()))?;
    let actual = parse_rust_scheduler_cases(&rust_source);
    if expected.as_slice() != actual.as_slice() {
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
    fn canonicalizes_redundant_regex_punctuation_escapes() {
        assert_eq!(
            regex_assertion_key("Generated.v", r"input  \[7 \: 0\] VAL\;"),
            regex_assertion_key("Generated.v", r"input  \[7 : 0\] VAL;")
        );
        assert_ne!(
            regex_assertion_key("Generated.v", r"literal\.dot"),
            regex_assertion_key("Generated.v", r"literal.dot")
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
    fn known_blocker_overrides_typed_candidate_status() {
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
    fn alignment_uses_the_typed_manifest_contract_total() {
        let project_root = locate_project_root().expect("locate project root");
        let manifest = load_manifest(&project_root).expect("build typed manifest");
        let expected = manifest
            .scripts
            .iter()
            .flat_map(|script| &script.contracts)
            .map(ManifestContract::effective_count)
            .sum::<usize>();
        assert_eq!(
            check_alignment()
                .expect("alignment should remain valid")
                .total_contracts,
            expected
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
