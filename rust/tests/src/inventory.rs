use crate::locate_project_root;
use bsc_testsuite_manifest::model::{
    BluesimSequence as ManifestBluesimSequence, BluesimWorkflow as ManifestBluesimWorkflow,
    Contract as ManifestContract, ScriptManifest, TestsuiteManifest, UnsupportedReason,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const KNOWN_RUNTIME_BLOCKERS: &[(&str, &str)] = &[
    (
        "testsuite/bsc.bugs/bluespec_inc/b260/b260.exp",
        "Verilog compilation cannot resolve the legacy UInt package required by Sub.bsv",
    ),
    (
        "testsuite/bsc.interra/libraries/Environment/Environment.exp",
        "Bluesim link for mkTestbench_Env4 triggers an internal compiler error: quoting a character value",
    ),
    (
        "testsuite/bsc.evaluator/dynamic/strings/dynamic_strings.exp",
        "Icarus 11.0 passes upstream-excluded StringInteger and StringIntegerWithNull cases, so their faithful known-failure contracts report XPASS",
    ),
    (
        "testsuite/bsc.bsv_examples/MacTestBench/mac_testbench.exp",
        "Compilation of TbEnvConfigs.bsv fails a type check (Bit#(32) vs Bit#(16) at max_len) under the pinned compiler, so the Verilog and Bluesim scenarios cannot build",
    ),
    (
        "testsuite/bsc.preprocessor/include/include.exp",
        "The pinned compiler XPASSes the upstream IncludeVendor known-failure contract; on Windows it also rejects the upstream-normalized absolute include path as ./D:/...",
    ),
    (
        "testsuite/bsc.options/options.exp",
        "The canonical plan executes, including all four flag-preflight scenarios, but the pinned Windows toolchain still has 18 existing output, path, and generated-artifact runtime mismatches",
    ),
    (
        "testsuite/bsc.driver/cpp/cpp.exp",
        "On Windows the pinned compiler passes shell redirection to cc for the quoted -Xcpp macro (`cc: error: >: Invalid argument`) and the line-directive case cannot remove its temporary preprocessor output (S0084 permission denied)",
    ),
    (
        "testsuite/bsc.bluetcl/packages/makedepend/makedepend.exp",
        "On Windows the pinned Bluetcl renders makedepend usage text with the executable name bluetcl.exe instead of the upstream golden name bluetcl",
    ),
    (
        "testsuite/bsc.bsv_examples/mesa/course_lab/course_lab.exp",
        "Compilation of MesaCircLpmQ.bsv fails in CompletionBuffer.bsv with T0030 and T0020 type errors under the pinned compiler",
    ),
];

const KNOWN_MIGRATION_BLOCKERS: &[(&str, &str)] = &[
    (
        "testsuite/bsc.bluetcl/packages/InstSynth/InstSynth.exp",
        "the active Bluetcl BH-mode golden is missing",
    ),
    (
        "testsuite/bsc.bugs/bluespec_inc/b437/b437.exp",
        "the active Bug437BSV.bsv source is missing; only a .bs file exists",
    ),
    (
        "testsuite/bsc.verilog/foreign_module/foreign_module.exp",
        "the active failure source is missing",
    ),
];

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

pub fn runtime_blockers() -> &'static [(&'static str, &'static str)] {
    KNOWN_RUNTIME_BLOCKERS
}

pub fn remaining_inventory() -> Result<Vec<RemainingTestScript>, String> {
    let project_root = locate_project_root()?;
    let manifest = load_manifest(&project_root)?;
    let generated =
        bsc_testsuite_manifest::build_test_plans_from_manifest(&project_root, &manifest)
            .map_err(|error| format!("build Test Plans for remaining inventory: {error}"))?;
    let plan_status = generated
        .plans
        .iter()
        .map(|generated| (generated.plan.origin.path.as_str(), generated.plan.status))
        .collect::<BTreeMap<_, _>>();
    let blocked_origins = plan_status
        .iter()
        .filter_map(|(origin, status)| {
            (*status == bsc_test_plan::PlanStatus::Blocked).then_some(*origin)
        })
        .collect::<BTreeSet<_>>();
    let remaining = collect_testsuite_scripts(&manifest)
        .into_iter()
        .filter(|script| blocked_origins.contains(script.origin.as_str()))
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
                "known migration blocker is no longer a blocked Test Plan; remove or update it: {origin}"
            ));
        }
    }
    for (origin, _) in KNOWN_RUNTIME_BLOCKERS {
        match plan_status.get(origin) {
            Some(bsc_test_plan::PlanStatus::Complete) => {}
            Some(status) => {
                return Err(format!(
                    "runtime blocker must reference a complete Test Plan, found {status:?}: {origin}"
                ));
            }
            None => return Err(format!("runtime blocker has no Test Plan: {origin}")),
        }
    }
    if remaining.len() != generated.summary().blocked {
        return Err(format!(
            "remaining inventory has {} scripts, expected {} blocked Test Plans",
            remaining.len(),
            generated.summary().blocked
        ));
    }
    Ok(remaining)
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
                .sum::<usize>()
                + script
                    .bluesim_sequences
                    .iter()
                    .map(ManifestBluesimSequence::effective_count)
                    .sum::<usize>()
                + script
                    .bluesim_workflows
                    .iter()
                    .map(ManifestBluesimWorkflow::effective_count)
                    .sum::<usize>(),
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
    for sequence in &script.bluesim_sequences {
        *counts.entry("bluesim_sequence".to_owned()).or_default() += sequence.effective_count();
    }
    for workflow in &script.bluesim_workflows {
        *counts.entry("bluesim_workflow".to_owned()).or_default() += workflow.effective_count();
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
        || matches!(
            command,
            "bluesim_sequence" | "bluesim_workflow" | "bluetcl" | "exec" | "source" | "test_ovl"
        )
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
