use crate::bsv::resolve_local_dependency_closures;
use crate::model::{
    ArtifactTransferOperation, AssertionContract, BasicOptionsContract, Capability,
    ComparisonContract, CompileContract, CompileObjectAction, Contract, ExternalContractKind,
    ExternalSetContract, GoldenMacroValue, Guard, MakeTestDataAction, NoSourceCompileContract,
    OvlContract, RenderGoldenContract, ScriptManifest, SimulationBackend, SimulationContract,
    SourceSpan as ManifestSourceSpan, SystemcLinkAction, UnsupportedConstruct, UnsupportedReason,
    WorkflowAction, WorkflowOperation,
};
use crate::{build_manifest, parse_static_tcl_list, ManifestError};
use bsc_test_plan::{
    generation_package_artifacts, generation_static_dump_artifacts, path_requires_non_windows,
    simulation_executable_artifact, simulation_vcd_outputs, Action, ArtifactContract,
    BluetclInstalledScript, BluetclPackage, BscCompileEnvironment, BscCompileMode,
    BscFlagPreflightMode, BscLinkMode, DependencyMode, DiagnosticKind, DiagnosticSeverity,
    ExpectedExit, ExpectedExitSet, Fixture, FixtureRole, GoldenNormalization, GoldenReplacement,
    ImportDiagnostic, InterraOperatorSuite, OperationExpectation, OperationRecord, Origin,
    PlanStatus, Provenance, Requirement, ResourceClass, Scenario,
    SimulationBackend as PlanSimulationBackend, SimulationGenerationMode, SourceSpan, Stage,
    TestPlan, TestPlanIndex, TestPlanIndexEntry, TextNormalization, Timeouts, UndeterminedValue,
    TEST_PLAN_INDEX_SCHEMA_VERSION, TEST_PLAN_SCHEMA_VERSION,
};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlanBuildError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("generated plan {id} is invalid: {message}")]
    InvalidPlan { id: String, message: String },
    #[error("generated plan index is invalid: {0}")]
    InvalidIndex(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedTestPlan {
    pub relative_path: PathBuf,
    pub plan: TestPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedTestPlans {
    pub plans: Vec<GeneratedTestPlan>,
    pub index: TestPlanIndex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanSummary {
    pub plans: usize,
    pub complete: usize,
    pub disabled: usize,
    pub blocked: usize,
    pub scenarios: usize,
    pub stages: usize,
    pub operations: usize,
    pub diagnostics: usize,
}

impl GeneratedTestPlans {
    pub fn summary(&self) -> PlanSummary {
        let mut summary = PlanSummary {
            plans: self.plans.len(),
            ..PlanSummary::default()
        };
        for generated in &self.plans {
            match generated.plan.status {
                PlanStatus::Complete => summary.complete += 1,
                PlanStatus::Disabled => summary.disabled += 1,
                PlanStatus::Blocked => summary.blocked += 1,
            }
            summary.scenarios += generated.plan.scenarios.len();
            summary.stages += generated
                .plan
                .scenarios
                .iter()
                .map(|scenario| scenario.stages.len())
                .sum::<usize>();
            summary.operations += generated
                .plan
                .scenarios
                .iter()
                .flat_map(|scenario| &scenario.stages)
                .map(|stage| stage.operations.len())
                .sum::<usize>();
            summary.diagnostics += generated.plan.diagnostics.len();
        }
        summary
    }
}

pub fn build_test_plans(project_root: &Path) -> Result<GeneratedTestPlans, PlanBuildError> {
    let manifest = build_manifest(project_root)?;
    build_test_plans_from_manifest(project_root, &manifest)
}

pub fn build_test_plans_from_manifest(
    project_root: &Path,
    manifest: &crate::model::TestsuiteManifest,
) -> Result<GeneratedTestPlans, PlanBuildError> {
    let mut plans = manifest
        .scripts
        .iter()
        .map(|script| plan_from_script(project_root, script))
        .collect::<Vec<_>>();
    plans.sort_by(|left, right| left.plan.id.cmp(&right.plan.id));

    for generated in &plans {
        generated
            .plan
            .validate()
            .map_err(|error| PlanBuildError::InvalidPlan {
                id: generated.plan.id.clone(),
                message: error.to_string(),
            })?;
    }

    let index = TestPlanIndex {
        schema_version: TEST_PLAN_INDEX_SCHEMA_VERSION,
        plans: plans
            .iter()
            .map(|generated| TestPlanIndexEntry {
                id: generated.plan.id.clone(),
                path: unix_path(&generated.relative_path),
                origin: generated.plan.origin.clone(),
                status: generated.plan.status,
                scenario_count: generated.plan.scenarios.len(),
                stage_count: generated
                    .plan
                    .scenarios
                    .iter()
                    .map(|scenario| scenario.stages.len())
                    .sum(),
                operation_count: generated
                    .plan
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .map(|stage| stage.operations.len())
                    .sum(),
                diagnostic_count: generated.plan.diagnostics.len(),
            })
            .collect(),
    };
    index
        .validate()
        .map_err(|error| PlanBuildError::InvalidIndex(error.to_string()))?;
    Ok(GeneratedTestPlans { plans, index })
}

const TINY_M0_SIMIR_ORIGIN: &str = "testsuite/bsc.bluesim/interactive/interactive.exp";
const TINY_M0_SIMIR_SHA256: &str =
    "9d3ec0fbb8fd0de5fc024703c64ef8d282570cf9b3875191352a32aed4d59fda";
const MCD_M2_SIMIR_SHA256: &str =
    "9d3ec0fbb8fd0de5fc024703c64ef8d282570cf9b3875191352a32aed4d59fda";
const TBGCD_M3_SIMIR_SHA256: &str = MCD_M2_SIMIR_SHA256;
const CLKTEST_M0_SIMIR_ORIGIN: &str = "testsuite/bsc.bluesim/misc/misc.exp";
const CLKTEST_M0_SIMIR_SHA256: &str =
    "21fabedb180b9e0d87af131a9f14d1710834a2ae60ebbc980b06e50d241d28ce";

const OPTIONS_PLAN_ORIGIN: &str = "testsuite/bsc.options/options.exp";
const OPTIONS_PLAN_SHA256: &str =
    "636b8c7a49224cf3737a679dd3f5b04989fb63f868528a9e41e77fe50e7aebcd";

const DISABLED_UPSTREAM_SCRIPTS: &[(&str, &str)] = &[
    (
        "testsuite/bsc.assertions/sequences/sequences.exp",
        "e7bb8ef1dee20191c0aaebdfdfe5c3c780f9f9ef6143c7695452e80a4ea10b4b",
    ),
    (
        "testsuite/bsc.interra/bugs/bugID239/bugID239.exp",
        "c5a9646e024485a33bccfbcc5cdad4d96ba42485953f4fd10e5b0eb002c63603",
    ),
    (
        "testsuite/bsc.interra/bugs/bugID403/bugID403.exp",
        "03edd383de5dc31b3ffddab536351a6b3cd1dfa0f3a9ac00fb66f8f47ecd386d",
    ),
];

fn is_pinned_disabled_upstream_script(script: &ScriptManifest) -> bool {
    DISABLED_UPSTREAM_SCRIPTS
        .iter()
        .any(|(origin, sha256)| script.origin == *origin && script.source_sha256 == *sha256)
}

fn is_pinned_options_plan(script: &ScriptManifest) -> bool {
    script.origin == OPTIONS_PLAN_ORIGIN && script.source_sha256 == OPTIONS_PLAN_SHA256
}

fn pair_pinned_options_render_chains(script: &mut ScriptManifest) {
    if !is_pinned_options_plan(script) {
        return;
    }

    let original = script.contracts.clone();
    let render_by_output = original
        .iter()
        .enumerate()
        .filter_map(|(index, contract)| match contract {
            Contract::RenderGolden(render) => Some((normalize_path(&render.output), index)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut chains = BTreeMap::<usize, Vec<usize>>::new();
    let mut paired_renders = BTreeSet::new();
    for (basic_index, contract) in original.iter().enumerate() {
        let Contract::BasicOptions(options) = contract else {
            continue;
        };
        let mut current = normalize_path(&options.expected);
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        while let Some(&render_index) = render_by_output.get(&current) {
            if render_index >= basic_index || !seen.insert(render_index) {
                break;
            }
            let Contract::RenderGolden(render) = &original[render_index] else {
                unreachable!("render output index must reference a render contract");
            };
            chain.push(render_index);
            current = normalize_path(&render.template);
        }
        if !chain.is_empty() {
            chain.reverse();
            paired_renders.extend(chain.iter().copied());
            chains.insert(basic_index, chain);
        }
    }

    let mut reordered = Vec::with_capacity(original.len());
    for (index, contract) in original.iter().enumerate() {
        if paired_renders.contains(&index) {
            continue;
        }
        if let Some(chain) = chains.get(&index) {
            reordered.extend(
                chain
                    .iter()
                    .map(|render_index| original[*render_index].clone()),
            );
        }
        reordered.push(contract.clone());
    }
    script.contracts = reordered;
}

fn plan_from_script(project_root: &Path, script: &ScriptManifest) -> GeneratedTestPlan {
    let mut script = script.clone();
    let id = script
        .origin
        .strip_prefix("testsuite/")
        .unwrap_or(&script.origin)
        .strip_suffix(".exp")
        .unwrap_or(&script.origin)
        .to_owned();
    let fixture_dir = if script.origin == SAL_PLAN_ORIGIN {
        "testsuite/bsc.misc".to_owned()
    } else if script
        .contracts
        .iter()
        .any(|contract| matches!(contract, Contract::Ovl(_)))
    {
        "testsuite/bsc.interra/OVL".to_owned()
    } else {
        script
            .origin
            .rsplit_once('/')
            .map_or("testsuite", |(directory, _)| directory)
            .to_owned()
    };
    let fixture_root = project_root.join(&fixture_dir);
    resolve_extensionless_contract_sources(&mut script, &fixture_root);
    pair_pinned_options_render_chains(&mut script);
    let mut diagnostics = Vec::new();
    let pinned_batch = match prepare_pinned_batch(&mut script, &fixture_root) {
        Ok(batch) => batch,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            None
        }
    };
    let mut assembly = PlanAssembly::default();

    for (sequence_index, sequence) in script.bluesim_sequences.iter().enumerate() {
        match sequence_scenario(sequence_index, sequence) {
            Ok(scenario) => assembly.scenarios.push(scenario),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    let check_bindings = check_bindings(&script, &fixture_root);
    let static_fixture_sources = static_fixture_sources(&script, &fixture_root);
    synthesize_hierarchy2_case_set(&mut script, &fixture_root);
    let closed_target_batch = match closed_target_bluetcl_scenarios(&script, &fixture_root) {
        Ok(Some(imported)) => {
            for scenario in imported {
                assembly.push(scenario);
            }
            true
        }
        Ok(None) => false,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            true
        }
    };
    let closed_bluetcl_batch = !closed_target_batch
        && match closed_bsc_compile_bluetcl_scenario(&script, &fixture_root) {
            Ok(Some(imported)) => {
                assembly.push(imported);
                true
            }
            Ok(None) => false,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                true
            }
        };
    let mut contract_index = if closed_target_batch || closed_bluetcl_batch {
        script.contracts.len()
    } else {
        0
    };
    while contract_index < script.contracts.len() {
        let contract = &script.contracts[contract_index];
        match contract {
            Contract::RenderGolden(render) => {
                let mut chain_end = contract_index + 1;
                while matches!(
                    script.contracts.get(chain_end),
                    Some(Contract::RenderGolden(_))
                ) {
                    chain_end += 1;
                }
                if let Some(Contract::BasicOptions(options)) = script.contracts.get(chain_end) {
                    let renders = script.contracts[contract_index..chain_end]
                        .iter()
                        .filter_map(|contract| match contract {
                            Contract::RenderGolden(render) => Some(render),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    match rendered_basic_options_scenario(&renders, options, &script, &fixture_root)
                    {
                        Ok(imported) => assembly.push(imported),
                        Err(diagnostic) => diagnostics.push(diagnostic),
                    }
                    contract_index = chain_end + 1;
                    continue;
                }
                if let Some(Contract::Simulation(simulation)) =
                    script.contracts.get(contract_index + 1)
                {
                    let simulation_index = contract_index + 1;
                    let mut group_end = simulation_index + 1;
                    if simulation.generation == crate::model::GenerationStrategy::Shared {
                        while let Some(Contract::Simulation(candidate)) =
                            script.contracts.get(group_end)
                        {
                            if !same_simulation_invocation(simulation, candidate) {
                                break;
                            }
                            group_end += 1;
                        }
                    }
                    let group = script.contracts[simulation_index..group_end]
                        .iter()
                        .filter_map(|contract| match contract {
                            Contract::Simulation(contract) => Some(contract),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    let previous_contract_order = script.contracts[..contract_index]
                        .last()
                        .map(contract_order_key);
                    match simulation_scenario(
                        &group,
                        previous_contract_order.as_ref(),
                        &script.workflow_actions,
                        &assembly.consumed_actions,
                        check_bindings.workflow_actions(&ProducerKey::Simulation(simulation_index)),
                        &script.assertions,
                        &script.comparisons,
                        check_bindings.get(&ProducerKey::Simulation(simulation_index)),
                        &fixture_root,
                    ) {
                        Ok(Some(imported)) => match prepend_rendered_simulation_golden(
                            render,
                            simulation,
                            &script,
                            &fixture_root,
                            imported,
                        ) {
                            Ok(mut imported) => {
                                uniquify_scenario_id(&mut imported.scenario, &assembly.scenarios);
                                assembly.push_simulation(simulation_index, imported);
                            }
                            Err(diagnostic) => diagnostics.push(diagnostic),
                        },
                        Ok(None) => diagnostics.extend(
                            script.contracts[simulation_index..group_end]
                                .iter()
                                .map(unconverted_contract),
                        ),
                        Err(diagnostic) => diagnostics.push(diagnostic),
                    }
                    contract_index = group_end;
                    continue;
                }
                diagnostics.push(unconverted_contract(contract));
            }
            Contract::NoSourceCompile(contract) => match no_source_compile_scenario(contract) {
                Ok(imported) => assembly.push(imported),
                Err(diagnostic) => diagnostics.push(diagnostic),
            },
            Contract::BasicOptions(contract) => {
                match basic_options_scenario(contract, &[], &fixture_root) {
                    Ok(imported) => assembly.push(imported),
                    Err(diagnostic) => diagnostics.push(diagnostic),
                }
            }
            Contract::Ovl(contract) => match ovl_scenario(contract, &fixture_root) {
                Ok(imported) => assembly.push(imported),
                Err(diagnostic) => diagnostics.push(diagnostic),
            },
            Contract::Compile(contract) => {
                let previous_contract_order = script.contracts[..contract_index]
                    .last()
                    .map(contract_order_key);
                let next_contract_order = script
                    .contracts
                    .get(contract_index + 1)
                    .map(contract_order_key);
                match compile_scenario(
                    contract_index,
                    contract,
                    is_pinned_options_plan(&script),
                    matches!(
                        script.origin.as_str(),
                        "testsuite/bsc.driver/depend/depend.exp"
                            | "testsuite/bsc.driver/imports/imports.exp"
                            | "testsuite/bsc.preprocessor/include/include.exp"
                            | OPTIONS_PLAN_ORIGIN
                    ),
                    previous_contract_order.as_ref(),
                    &script.workflow_actions,
                    &assembly.consumed_actions,
                    next_contract_order.as_ref(),
                    &script.assertions,
                    &script.comparisons,
                    check_bindings.get(&ProducerKey::Compile(contract_index)),
                    check_bindings.workflow_actions(&ProducerKey::Compile(contract_index)),
                    &static_fixture_sources,
                    &script.unsupported,
                    &fixture_root,
                ) {
                    Ok(imported) => assembly.push_compile(contract_index, imported),
                    Err(diagnostic) => diagnostics.push(diagnostic),
                }
            }
            Contract::Simulation(simulation) => {
                let mut group_end = contract_index + 1;
                if simulation.generation == crate::model::GenerationStrategy::Shared {
                    while let Some(Contract::Simulation(candidate)) =
                        script.contracts.get(group_end)
                    {
                        if !same_simulation_invocation(simulation, candidate) {
                            break;
                        }
                        group_end += 1;
                    }
                }
                let group = script.contracts[contract_index..group_end]
                    .iter()
                    .filter_map(|contract| match contract {
                        Contract::Simulation(contract) => Some(contract),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let previous_contract_order = script.contracts[..contract_index]
                    .last()
                    .map(contract_order_key);
                match simulation_scenario(
                    &group,
                    previous_contract_order.as_ref(),
                    &script.workflow_actions,
                    &assembly.consumed_actions,
                    check_bindings.workflow_actions(&ProducerKey::Simulation(contract_index)),
                    &script.assertions,
                    &script.comparisons,
                    check_bindings.get(&ProducerKey::Simulation(contract_index)),
                    &fixture_root,
                ) {
                    Ok(Some(mut imported)) => {
                        uniquify_scenario_id(&mut imported.scenario, &assembly.scenarios);
                        assembly.push_simulation(contract_index, imported);
                    }
                    Ok(None) => diagnostics.extend(
                        script.contracts[contract_index..group_end]
                            .iter()
                            .map(unconverted_contract),
                    ),
                    Err(diagnostic) => diagnostics.push(diagnostic),
                }
                contract_index = group_end;
                continue;
            }
            Contract::ExternalSet(contract) => match external_set_scenarios(contract) {
                Ok(imported) => {
                    for scenario in imported {
                        assembly.push(scenario);
                    }
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            },
        }
        contract_index += 1;
    }
    compose_ordered_workspace_compile_episodes(&script, &fixture_root, &mut assembly);
    compose_paired_compile_dump_comparisons(&script, &mut assembly);
    compose_stateful_simulation_episodes(&script, &mut assembly);
    compose_fixture_replacement_compile_episode(&fixture_root, &script, &mut assembly);
    compose_stateful_compile_chains(&fixture_root, &script, &mut assembly);
    for (workflow_index, workflow) in script.bluesim_workflows.iter().enumerate() {
        match workflow_scenario(
            workflow_index,
            workflow,
            &script.workflow_actions,
            &assembly.consumed_actions,
            &script.unsupported,
            &script.assertions,
            &script.comparisons,
            &check_bindings,
            &fixture_root,
        ) {
            Ok(imported) => assembly.push(imported),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    match tiny_m0_simir_scenario(&script, &fixture_root) {
        Ok(Some(scenario)) => assembly.scenarios.push(scenario),
        Ok(None) => {}
        Err(diagnostic) => diagnostics.push(diagnostic),
    }
    match mcd_m2_simir_scenario(&script, &fixture_root) {
        Ok(Some(scenario)) => assembly.scenarios.push(scenario),
        Ok(None) => {}
        Err(diagnostic) => diagnostics.push(diagnostic),
    }
    match tbgcd_m3_simir_scenario(&script, &fixture_root) {
        Ok(Some(scenario)) => assembly.scenarios.push(scenario),
        Ok(None) => {}
        Err(diagnostic) => diagnostics.push(diagnostic),
    }
    match clktest_m0_simir_scenario(&script, &fixture_root, &assembly.scenarios) {
        Ok(Some(scenario)) => assembly.scenarios.push(scenario),
        Ok(None) => {}
        Err(diagnostic) => diagnostics.push(diagnostic),
    }
    for (workflow_index, workflow) in script.systemc_workflows.iter().enumerate() {
        match systemc_workflow_scenario(workflow_index, workflow) {
            Ok(imported) => assembly.push(imported),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    compose_pinned_options_typed_episodes(&script, &mut assembly);
    compose_b1595_workspace_episodes(&script, &mut assembly);
    compose_cpp_darwin_normalization_episode(&script, &mut assembly);
    compose_ordered_repeated_bluesim_episodes(&script, &mut assembly);
    compose_pinned_options_split_if_episode(&script, &mut assembly);
    for (action_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&action_index) {
            continue;
        }
        let imported = match action {
            WorkflowAction::CompileObject(generation) => standalone_generation_scenario(
                action_index,
                generation,
                &script.assertions,
                &script.comparisons,
                check_bindings.get(&ProducerKey::WorkflowAction(action_index)),
            ),
            WorkflowAction::BluetclRun(run) if run.artifact_inputs.is_empty() => {
                standalone_bluetcl_scenario(
                    action_index,
                    run,
                    &script.assertions,
                    &script.comparisons,
                    check_bindings.get(&ProducerKey::WorkflowAction(action_index)),
                )
            }
            WorkflowAction::Bsc2Bsv(action) => standalone_bsc2bsv_scenario(action_index, action),
            WorkflowAction::BscParsePretty(action) => {
                standalone_bsc_parse_pretty_scenario(action_index, action)
            }
            _ => continue,
        };
        assembly.consumed_actions.insert(action_index);
        match imported {
            Ok(imported) => assembly.push(imported),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    declare_dependency_generation_artifacts(&mut assembly.scenarios, &fixture_root);
    declare_showrules_design_inputs(&mut assembly.scenarios, &fixture_root);
    compose_persistent_generated_artifact_producers(&mut assembly.scenarios, &fixture_root);
    activate_proven_capability_disjunction_assertions(&mut script, &assembly);
    loop {
        let consumed_before = assembly.consumed_actions.len()
            + assembly.consumed_assertions.len()
            + assembly.consumed_comparisons.len();
        compose_multi_compile_verilog_workflows(&fixture_root, &script, &mut assembly);
        compose_ordered_bluesim_links(&fixture_root, &script, &mut assembly);
        compose_ordered_intermediate_dumps(&fixture_root, &script, &mut assembly);
        compose_trailing_filesystem_actions(&script, &mut assembly);
        compose_idempotent_cleanup_actions(&script, &mut assembly);
        compose_ordered_checks(&script, &mut assembly);
        compose_ordered_simulation_runs(&script, &mut assembly);
        let consumed_after = assembly.consumed_actions.len()
            + assembly.consumed_assertions.len()
            + assembly.consumed_comparisons.len();
        if consumed_after == consumed_before {
            break;
        }
    }
    compose_static_fixture_vcd_checks(&script, &fixture_root, &mut assembly);
    inject_interra_operator_vectors(&mut script, &mut assembly);
    inject_make_test_data_actions(&script, &mut assembly, &mut diagnostics);
    apply_bsc_options_overlays(&script, &mut assembly, &mut diagnostics);
    compose_persistent_c_object_builds(&script, &mut assembly);
    let preflight_inputs = compose_pinned_options_flag_preflights(&script, &mut assembly);
    compose_persistent_fixture_aliases(&script, &fixture_root, &mut assembly);
    compose_missing_bug_golden_xfails(&fixture_root, &mut assembly);
    if let Some(batch) = pinned_batch {
        if let Err(diagnostic) = apply_pinned_batch(batch, &script, &mut assembly) {
            diagnostics.push(diagnostic);
        }
    }

    for (assertion_index, assertion) in script.assertions.iter().enumerate() {
        if assembly.consumed_assertions.contains(&assertion_index) {
            continue;
        }
        diagnostics.push(error_diagnostic(
            "import.unbound_assertion",
            format!(
                "assertion helper {} is not yet attached to an executable operation stream",
                assertion.helper
            ),
            assertion.span,
            &assertion.expansion,
        ));
    }
    for (comparison_index, comparison) in script.comparisons.iter().enumerate() {
        if assembly.consumed_comparisons.contains(&comparison_index) {
            continue;
        }
        diagnostics.push(error_diagnostic(
            "import.unbound_comparison",
            format!(
                "comparison helper {} is not yet attached to an executable operation stream",
                comparison.helper
            ),
            comparison.span,
            &comparison.expansion,
        ));
    }

    for (action_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&action_index) {
            continue;
        }
        diagnostics.push(error_diagnostic(
            "import.uncomposed_action",
            format!(
                "uncomposed workflow action {} requires importer support",
                action.helper_name()
            ),
            action_span(action),
            action_expansion(action),
        ));
    }
    diagnostics.extend(script.unsupported.iter().map(unsupported_diagnostic));

    let generated_source_paths = workflow_generated_destinations(&script);
    let ovl_runtime_sources = script
        .contracts
        .iter()
        .filter_map(|contract| match contract {
            Contract::Ovl(contract) => Some(format!("{}.bsv", contract.top)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    append_local_verilog_search_dependencies(&mut assembly.scenarios, &fixture_root);
    let mut source_paths = collect_source_paths(&script);
    source_paths.retain(|path| !preflight_inputs.contains(path));
    if is_pinned_options_plan(&script) && !preflight_inputs.contains("m.ba") {
        source_paths.insert("m.ba".to_owned());
    }
    if script.origin == COURSE_LAB_PLAN_ORIGIN && script.source_sha256 == COURSE_LAB_PLAN_SHA256 {
        source_paths.extend(
            COURSE_LAB_COMMON_CLOSURE
                .iter()
                .map(|path| (*path).to_owned()),
        );
        source_paths.extend(
            COURSE_LAB_VARIANT_CLOSURES
                .iter()
                .flat_map(|(_, closure)| closure.iter())
                .map(|path| (*path).to_owned()),
        );
    }
    if script.origin == SAL_PLAN_ORIGIN && script.source_sha256 == SAL_PLAN_SHA256 {
        for member in SAL_LAMBDA_MEMBERS {
            source_paths.remove(&format!("sal/{member}"));
            source_paths.insert(format!("lambda_calculus/{member}"));
        }
    }
    if script.origin == MAKEDEPEND_PLAN_ORIGIN {
        for source in [
            "Dep1.bsv",
            "Foo.bsv",
            "IncDep1.bsv",
            "IncDep2.bsv",
            "Test.bsv",
        ] {
            source_paths.remove(&format!("makedepend/{source}"));
            source_paths.insert(source.to_owned());
        }
    }
    for contract in &script.contracts {
        let Contract::Ovl(contract) = contract else {
            continue;
        };
        source_paths.extend([
            format!("{}/{}.bsv", contract.case_dir, contract.top),
            format!("std_ovl/{}", contract.library),
        ]);
        assembly.golden_paths.insert(format!(
            "{}/{}.out.expected",
            contract.case_dir, contract.top
        ));
    }
    source_paths.retain(|path| {
        (!generated_source_paths.contains(path) || fixture_root.join(path).is_file())
            && !ovl_runtime_sources.contains(path)
    });
    source_paths.extend(
        assembly
            .scenarios
            .iter()
            .flat_map(|scenario| local_link_fixture_paths(&fixture_root, scenario)),
    );

    let dependency_roots = assembly
        .scenarios
        .iter()
        .map(|scenario| {
            let mut roots = scenario_dependency_roots(scenario);
            roots.retain(|path| !ovl_runtime_sources.contains(path));
            roots
        })
        .collect::<Vec<_>>();
    let dependency_resolution =
        resolve_local_dependency_closures(&project_root.join(&fixture_dir), &dependency_roots);

    append_foreign_link_dependencies(
        &mut assembly.scenarios,
        &dependency_resolution.foreign_link_paths,
    );
    prepend_prior_compile_prerequisites(&mut assembly.scenarios, &dependency_resolution.paths);
    reconcile_expected_failure_link_inputs(&mut assembly.scenarios, &fixture_root);
    if script.origin == INOUT_PLAN_ORIGIN && script.source_sha256 == INOUT_PLAN_SHA256 {
        if let Err(diagnostic) =
            enforce_inout_closed_postconditions(&script, &mut assembly.scenarios)
        {
            diagnostics.push(diagnostic);
        }
    }
    let build_input_paths = assembly
        .scenarios
        .iter()
        .flat_map(|scenario| scenario.stages.iter())
        .flat_map(|stage| &stage.operations)
        .flat_map(|operation| match &operation.action {
            Action::MakeTestData | Action::InterraOperatorVectors { .. } => {
                operation.artifacts.inputs.clone()
            }
            Action::CObjectBuild { makefile, .. } => vec![makefile.clone()],
            Action::M4CurdirRender { template, .. } => vec![template.clone()],
            _ => Vec::new(),
        })
        .collect::<BTreeSet<_>>();
    let mut data_paths = assembly
        .scenarios
        .iter()
        .flat_map(|scenario| local_operation_data_paths(&fixture_root, scenario))
        .chain(
            assembly
                .scenarios
                .iter()
                .flat_map(|scenario| local_transfer_fixture_paths(&fixture_root, scenario)),
        )
        .collect::<BTreeSet<_>>();
    for (dependencies, scenario_data_paths) in dependency_resolution
        .paths
        .iter()
        .zip(&dependency_resolution.data_paths)
    {
        source_paths.extend(dependencies.difference(scenario_data_paths).cloned());
        data_paths.extend(scenario_data_paths.iter().cloned());
    }
    source_paths.retain(|path| {
        (!generated_source_paths.contains(path) || fixture_root.join(path).is_file())
            && !ovl_runtime_sources.contains(path)
    });

    diagnostics.extend(
        dependency_resolution
            .diagnostics
            .iter()
            .cloned()
            .map(|message| global_error("fixture.bsv_dependency", message)),
    );
    let script_paths = script
        .workflow_actions
        .iter()
        .filter_map(|action| match action {
            WorkflowAction::BluetclRun(run) => Some(run),
            _ => None,
        })
        .flat_map(|run| {
            let primary = match &run.invocation {
                crate::model::BluetclInvocation::Script { script, .. }
                | crate::model::BluetclInvocation::Exec { script, .. } => {
                    Some(normalize_path(script))
                }
                crate::model::BluetclInvocation::InstalledScript { .. }
                | crate::model::BluetclInvocation::Makedepend { .. } => None,
            };
            primary.into_iter().chain(
                run.artifact_inputs
                    .iter()
                    .map(|path| normalize_path(path))
                    .filter(|path| path.ends_with(".tcl")),
            )
        })
        .chain(script.workflow_actions.iter().flat_map(|action| {
            match action {
                WorkflowAction::VerilogFilter(filter) => filter
                    .profiles
                    .iter()
                    .filter_map(|profile| profile.fixture_path())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            }
        }))
        .collect();
    assembly
        .golden_paths
        .retain(|path| !generated_source_paths.contains(path));
    let fixtures = collect_fixtures(
        project_root,
        &fixture_dir,
        source_paths,
        std::mem::take(&mut assembly.golden_paths),
        data_paths,
        build_input_paths,
        script_paths,
        &mut diagnostics,
    );

    let fixture_paths = fixtures
        .iter()
        .map(|fixture| fixture.path.as_str())
        .collect::<BTreeSet<_>>();
    for (scenario, dependencies) in assembly
        .scenarios
        .iter_mut()
        .zip(dependency_resolution.paths)
    {
        let mut inputs = dependencies;
        inputs.extend(
            scenario_declared_fixture_inputs(scenario)
                .filter(|path| fixture_paths.contains(path.as_str())),
        );
        inputs.retain(|path| fixture_paths.contains(path.as_str()));
        scenario.fixtures = inputs.into_iter().collect();
    }
    if script.origin == COURSE_LAB_PLAN_ORIGIN && script.source_sha256 == COURSE_LAB_PLAN_SHA256 {
        append_course_lab_variant_fixtures(&mut assembly.scenarios, &fixture_paths);
    }
    let disabled_upstream = assembly.scenarios.is_empty()
        && diagnostics.is_empty()
        && is_pinned_disabled_upstream_script(&script);
    if assembly.scenarios.is_empty() && diagnostics.is_empty() {
        if disabled_upstream {
            diagnostics.push(global_warning(
                "import.disabled",
                "upstream script contains no active contracts; every historical test command is intentionally commented out"
                    .to_owned(),
            ));
        } else {
            diagnostics.push(global_error(
                "import.empty",
                "the static lowerer found no executable contracts or explicit unsupported constructs"
                    .to_owned(),
            ));
        }
    }

    diagnostics.sort_by(|left, right| {
        left.provenance
            .span
            .start_byte
            .cmp(&right.provenance.span.start_byte)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.message.cmp(&right.message))
    });
    let status = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        PlanStatus::Blocked
    } else if disabled_upstream {
        PlanStatus::Disabled
    } else {
        PlanStatus::Complete
    };
    let plan = TestPlan {
        schema_version: TEST_PLAN_SCHEMA_VERSION,
        id: id.clone(),
        origin: Origin {
            path: script.origin.clone(),
            sha256: script.source_sha256.clone(),
        },
        status,
        fixture_dir,
        fixtures,
        scenarios: assembly.scenarios,
        diagnostics,
    };
    GeneratedTestPlan {
        relative_path: PathBuf::from(format!("{id}.test.json")),
        plan,
    }
}

fn tiny_m0_simir_scenario(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<Option<Scenario>, ImportDiagnostic> {
    if script.origin != TINY_M0_SIMIR_ORIGIN {
        return Ok(None);
    }
    if script.source_sha256 != TINY_M0_SIMIR_SHA256 {
        return Err(global_error(
            "import.tiny_m0_simir_pin",
            "the interactive Bluesim origin changed; review the M0 SimIR workflow shape".to_owned(),
        ));
    }
    let workflows = script
        .bluesim_workflows
        .iter()
        .filter(|workflow| {
            workflow.top == "mkTest"
                && workflow.link.top == "mkTest"
                && workflow.generations.len() == 1
                && workflow.generations[0].source == "tiny.bsv"
                && workflow.generations[0].module.as_deref() == Some("mkTest")
        })
        .collect::<Vec<_>>();
    let [workflow] = workflows.as_slice() else {
        return Err(global_error(
            "import.tiny_m0_simir_shape",
            "expected exactly one mkTest Bluesim workflow generated from tiny.bsv".to_owned(),
        ));
    };
    if !fixture_root.join("tiny.bsv").is_file()
        || !fixture_root.join("mkTest_step.out.expected").is_file()
    {
        return Err(global_error(
            "import.tiny_m0_simir_fixture",
            "tiny M0 SimIR fixture or its step golden is missing".to_owned(),
        ));
    }

    let generation = &workflow.generations[0];
    let generation_provenance = provenance(generation.span, &generation.expansion);
    let link_provenance = provenance(workflow.link.span, &workflow.link.expansion);
    let step_provenance = workflow
        .runs
        .iter()
        .find(|run| run.action.stdout == "mkTest.out")
        .map(|run| provenance(run.action.span, &run.action.expansion))
        .unwrap_or_else(|| link_provenance.clone());
    Ok(Some(Scenario {
        id: "simir-m0-mkTest".to_owned(),
        resource: ResourceClass::Normal,
        fixtures: Vec::new(),
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![
            Stage {
                id: "export-m0".to_owned(),
                operations: vec![
                    OperationRecord::new(
                        Action::BscGenerate {
                            source: "tiny.bsv".to_owned(),
                            mode: SimulationGenerationMode::Bluesim,
                            module: Some("mkTest".to_owned()),
                            args: Vec::new(),
                        },
                        OperationExpectation::Required,
                        generation_provenance,
                    ),
                    OperationRecord::new(
                        Action::BscSimirExport {
                            top: "mkTest".to_owned(),
                            output: "mkTest.m0.bsim.json".to_owned(),
                        },
                        OperationExpectation::Required,
                        link_provenance,
                    ),
                ],
            },
            Stage {
                id: "step-m0".to_owned(),
                operations: vec![
                    OperationRecord::new(
                        Action::SimirM0Step {
                            model: "mkTest.m0.bsim.json".to_owned(),
                            cycles: 10,
                            stdout: "mkTest_m0_step.out".to_owned(),
                            expected_finish: None,
                        },
                        OperationExpectation::Required,
                        step_provenance.clone(),
                    ),
                    OperationRecord::new(
                        Action::AssertGolden {
                            actual: "mkTest_m0_step.out".to_owned(),
                            expected: "mkTest_step.out.expected".to_owned(),
                        },
                        OperationExpectation::Required,
                        step_provenance,
                    ),
                ],
            },
        ],
    }))
}

fn mcd_m2_simir_scenario(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<Option<Scenario>, ImportDiagnostic> {
    if script.origin != TINY_M0_SIMIR_ORIGIN {
        return Ok(None);
    }
    if script.source_sha256 != MCD_M2_SIMIR_SHA256 {
        return Err(global_error(
            "import.mcd_m2_simir_pin",
            "the interactive Bluesim origin changed; review the M2 SimIR workflow shape".to_owned(),
        ));
    }
    let workflows = script
        .bluesim_workflows
        .iter()
        .filter(|workflow| {
            workflow.top == "mkMCDTest"
                && workflow.link.top == "mkMCDTest"
                && workflow.generations.len() == 1
                && workflow.generations[0].source == "MCDTest.bsv"
                && workflow.generations[0].module.as_deref() == Some("mkMCDTest")
        })
        .collect::<Vec<_>>();
    let [workflow] = workflows.as_slice() else {
        return Err(global_error(
            "import.mcd_m2_simir_shape",
            "expected exactly one mkMCDTest Bluesim workflow generated from MCDTest.bsv".to_owned(),
        ));
    };
    if !fixture_root.join("MCDTest.bsv").is_file() {
        return Err(global_error(
            "import.mcd_m2_simir_fixture",
            "MCDTest M2 SimIR fixture is missing".to_owned(),
        ));
    }

    let generation = &workflow.generations[0];
    let generation_provenance = provenance(generation.span, &generation.expansion);
    let link_provenance = provenance(workflow.link.span, &workflow.link.expansion);
    let run_provenance = workflow
        .runs
        .iter()
        .find(|run| run.action.stdout == "mkMCDTest.out")
        .map(|run| provenance(run.action.span, &run.action.expansion))
        .unwrap_or_else(|| link_provenance.clone());
    Ok(Some(Scenario {
        id: "simir-m2-mkMCDTest".to_owned(),
        resource: ResourceClass::Normal,
        fixtures: Vec::new(),
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![
            Stage {
                id: "export-m2".to_owned(),
                operations: vec![
                    OperationRecord::new(
                        Action::BscGenerate {
                            source: "MCDTest.bsv".to_owned(),
                            mode: SimulationGenerationMode::Bluesim,
                            module: Some("mkMCDTest".to_owned()),
                            args: Vec::new(),
                        },
                        OperationExpectation::Required,
                        generation_provenance,
                    ),
                    OperationRecord::new(
                        Action::BscSimirExport {
                            top: "mkMCDTest".to_owned(),
                            output: "mkMCDTest.m2.bsim.json".to_owned(),
                        },
                        OperationExpectation::Required,
                        link_provenance,
                    ),
                ],
            },
            Stage {
                id: "run-m2".to_owned(),
                operations: vec![OperationRecord::new(
                    Action::SimirM2Run {
                        model: "mkMCDTest.m2.bsim.json".to_owned(),
                        max_events: 100,
                        expected_finish: 0,
                        expected_time: 163,
                        stdout: "mkMCDTest_m2_run.out".to_owned(),
                    },
                    OperationExpectation::Required,
                    run_provenance,
                )],
            },
        ],
    }))
}

fn tbgcd_m3_simir_scenario(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<Option<Scenario>, ImportDiagnostic> {
    if script.origin != TINY_M0_SIMIR_ORIGIN {
        return Ok(None);
    }
    if script.source_sha256 != TBGCD_M3_SIMIR_SHA256 {
        return Err(global_error(
            "import.tbgcd_m3_simir_pin",
            "the interactive Bluesim origin changed; review the M3 SimIR workflow shape".to_owned(),
        ));
    }
    let workflows = script
        .bluesim_workflows
        .iter()
        .filter(|workflow| {
            workflow.top == "mkTbGCD"
                && workflow.link.top == "mkTbGCD"
                && workflow.generations.len() == 1
                && workflow.generations[0].source == "TbGCD.bsv"
                && workflow.generations[0].module.as_deref() == Some("mkTbGCD")
        })
        .collect::<Vec<_>>();
    let [workflow] = workflows.as_slice() else {
        return Err(global_error(
            "import.tbgcd_m3_simir_shape",
            "expected exactly one mkTbGCD Bluesim workflow generated from TbGCD.bsv".to_owned(),
        ));
    };
    if !fixture_root.join("TbGCD.bsv").is_file() || !fixture_root.join("GCD.bsv").is_file() {
        return Err(global_error(
            "import.tbgcd_m3_simir_fixture",
            "TbGCD M3 SimIR fixtures are missing".to_owned(),
        ));
    }

    let generation = &workflow.generations[0];
    let generation_provenance = provenance(generation.span, &generation.expansion);
    let link_provenance = provenance(workflow.link.span, &workflow.link.expansion);
    let run_provenance = workflow
        .runs
        .first()
        .map(|run| provenance(run.action.span, &run.action.expansion))
        .unwrap_or_else(|| link_provenance.clone());
    Ok(Some(Scenario {
        id: "simir-m3-mkTbGCD".to_owned(),
        resource: ResourceClass::Normal,
        fixtures: vec!["GCD.bsv".to_owned()],
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![
            Stage {
                id: "export-m3".to_owned(),
                operations: vec![
                    OperationRecord::new(
                        Action::BscGenerate {
                            source: "TbGCD.bsv".to_owned(),
                            mode: SimulationGenerationMode::Bluesim,
                            module: Some("mkTbGCD".to_owned()),
                            args: Vec::new(),
                        },
                        OperationExpectation::Required,
                        generation_provenance,
                    ),
                    OperationRecord::new(
                        Action::BscSimirExport {
                            top: "mkTbGCD".to_owned(),
                            output: "mkTbGCD.m3.bsim.json".to_owned(),
                        },
                        OperationExpectation::Required,
                        link_provenance,
                    ),
                ],
            },
            Stage {
                id: "run-m3".to_owned(),
                operations: vec![OperationRecord::new(
                    Action::SimirM3Run {
                        model: "mkTbGCD.m3.bsim.json".to_owned(),
                        max_events: 1_000,
                        expected_finish: 0,
                        expected_time: 4_760,
                        stdout: "mkTbGCD_m3_run.out".to_owned(),
                    },
                    OperationExpectation::Required,
                    run_provenance,
                )],
            },
        ],
    }))
}

fn clktest_m0_simir_scenario(
    script: &ScriptManifest,
    fixture_root: &Path,
    scenarios: &[Scenario],
) -> Result<Option<Scenario>, ImportDiagnostic> {
    if script.origin != CLKTEST_M0_SIMIR_ORIGIN {
        return Ok(None);
    }
    if script.source_sha256 != CLKTEST_M0_SIMIR_SHA256 {
        return Err(global_error(
            "import.clktest_m0_simir_pin",
            "the misc Bluesim origin changed; review the ClkTest SimIR workflow shape".to_owned(),
        ));
    }
    if !fixture_root.join("ClkTest.bsv").is_file()
        || !fixture_root.join("sysClkTest.out.expected").is_file()
    {
        return Err(global_error(
            "import.clktest_m0_simir_fixture",
            "ClkTest M0 SimIR fixture or stdout golden is missing".to_owned(),
        ));
    }
    let legacy = scenarios
        .iter()
        .filter(|scenario| scenario.id == "simulation-sysClkTest")
        .collect::<Vec<_>>();
    let [legacy] = legacy.as_slice() else {
        return Err(global_error(
            "import.clktest_m0_simir_shape",
            "expected exactly one simulation-sysClkTest scenario".to_owned(),
        ));
    };
    let operations = legacy
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .collect::<Vec<_>>();
    let generation = operations.iter().find(|operation| {
        matches!(
            &operation.action,
            Action::BscGenerate { source, module, .. }
                if source == "ClkTest.bsv" && module.as_deref() == Some("sysClkTest")
        )
    });
    let run = operations.iter().find(|operation| {
        matches!(
            &operation.action,
            Action::SimulationRun { backend: bsc_test_plan::SimulationBackend::Bluesim, stdout, .. }
                if stdout == "sysClkTest.c.out"
        )
    });
    let golden = operations.iter().find(|operation| {
        matches!(
            &operation.action,
            Action::AssertGolden { actual, expected }
                if actual == "sysClkTest.c.out" && expected == "sysClkTest.out.expected"
        )
    });
    let (Some(generation), Some(run), Some(golden)) = (generation, run, golden) else {
        return Err(global_error(
            "import.clktest_m0_simir_shape",
            "simulation-sysClkTest no longer has the expected generate/run/golden operations"
                .to_owned(),
        ));
    };

    Ok(Some(Scenario {
        id: "simir-m0-sysClkTest".to_owned(),
        resource: ResourceClass::Normal,
        fixtures: Vec::new(),
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![
            Stage {
                id: "export-m0".to_owned(),
                operations: vec![
                    OperationRecord::new(
                        Action::BscGenerate {
                            source: "ClkTest.bsv".to_owned(),
                            mode: SimulationGenerationMode::Bluesim,
                            module: Some("sysClkTest".to_owned()),
                            args: Vec::new(),
                        },
                        OperationExpectation::Required,
                        generation.provenance.clone(),
                    ),
                    OperationRecord::new(
                        Action::BscSimirExport {
                            top: "sysClkTest".to_owned(),
                            output: "sysClkTest.m0.bsim.json".to_owned(),
                        },
                        OperationExpectation::Required,
                        generation.provenance.clone(),
                    ),
                ],
            },
            Stage {
                id: "run-m0".to_owned(),
                operations: vec![
                    OperationRecord::new(
                        Action::SimirM0Step {
                            model: "sysClkTest.m0.bsim.json".to_owned(),
                            cycles: 102,
                            stdout: "sysClkTest_m0_run.out".to_owned(),
                            expected_finish: Some(0),
                        },
                        OperationExpectation::Required,
                        run.provenance.clone(),
                    ),
                    OperationRecord::new(
                        Action::AssertGolden {
                            actual: "sysClkTest_m0_run.out".to_owned(),
                            expected: "sysClkTest.out.expected".to_owned(),
                        },
                        OperationExpectation::Required,
                        golden.provenance.clone(),
                    ),
                ],
            },
        ],
    }))
}

fn external_set_scenarios(
    contract: &ExternalSetContract,
) -> Result<Vec<ImportedScenario>, ImportDiagnostic> {
    let fail = |message| {
        error_diagnostic(
            "import.external_contract",
            message,
            contract.span,
            &contract.expansion,
        )
    };
    match contract.external_kind {
        ExternalContractKind::SchedulerSat => {
            let mut requirements = BTreeSet::new();
            collect_requirements(&contract.guard, &mut requirements).map_err(&fail)?;
            let mut seen = BTreeSet::new();
            contract
                .cases
                .iter()
                .map(|case| {
                    if case.is_empty()
                        || !case
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                        || !seen.insert(case)
                    {
                        return Err(fail(format!(
                            "scheduler SAT case must be a unique portable identifier: {case:?}"
                        )));
                    }
                    let source = format!("{case}.bsv");
                    let renamed_source = format!("{case}_sat-z3.bsv");
                    let actual = format!("{renamed_source}.bsc-sched-out");
                    let expected = format!("{case}_sat-yices.bsv.bsc-sched-out.expected");
                    let provenance = provenance(contract.span, &contract.expansion);
                    let operations = vec![
                        OperationRecord::new(
                            Action::FsCopy {
                                source: source.clone(),
                                destination: renamed_source.clone(),
                            },
                            OperationExpectation::Required,
                            provenance.clone(),
                        ),
                        OperationRecord::new(
                            Action::BscCompile {
                                source: renamed_source,
                                working_directory: None,
                                mode: BscCompileMode::VerilogSchedule,
                                module: None,
                                args: vec!["-sat-z3".to_owned()],
                                absolute_import_paths: Vec::new(),
                                dependency_mode: DependencyMode::Update,
                                expected_exit: ExpectedExit::Success,
                                unexpected_success_forbidden_regex: None,
                                environment: None,
                                stdout: actual.clone(),
                            },
                            OperationExpectation::Required,
                            provenance.clone(),
                        ),
                        OperationRecord::new(
                            Action::AssertGoldenNormalized {
                                actual,
                                expected: expected.clone(),
                                normalizations: vec![
                                    GoldenNormalization::GeneratedIds,
                                    GoldenNormalization::SatSolverNames,
                                ],
                            },
                            OperationExpectation::Required,
                            provenance,
                        ),
                    ];
                    Ok(ImportedScenario {
                        scenario: Scenario {
                            id: format!("scheduler-sat-{case}"),
                            resource: ResourceClass::Heavy,
                            fixtures: Vec::new(),
                            requires: requirements.iter().copied().collect(),
                            bsc_options_append: None,
                            timeouts: Timeouts::default(),
                            stages: vec![Stage {
                                id: format!("schedule-{case}"),
                                operations,
                            }],
                        },
                        consumption: ImportConsumption {
                            golden_paths: vec![expected],
                            ..ImportConsumption::default()
                        },
                    })
                })
                .collect()
        }
    }
}

fn declare_link_intermediate_transfer_source(
    operations: &mut [OperationRecord],
    transfer: &crate::model::ArtifactTransferAction,
) {
    let source = normalize_path(&transfer.source);
    let Some(link) = operations.iter_mut().rev().find(|operation| {
        matches!(
            operation.action,
            Action::BscLink {
                backend: PlanSimulationBackend::Bluesim,
                expected_exit: ExpectedExit::Success,
                ..
            }
        )
    }) else {
        return;
    };
    let Action::BscLink { top, .. } = &link.action else {
        unreachable!("link intermediate producer must be a Bluesim link")
    };
    let known_intermediate = [
        format!("{top}.cxx"),
        format!("model_{top}.cxx"),
        format!("{top}.o"),
        format!("model_{top}.o"),
    ];
    if known_intermediate.contains(&source) && !link.artifacts.outputs.contains(&source) {
        link.artifacts.outputs.push(source);
    }
}

fn sequence_scenario(
    sequence_index: usize,
    sequence: &crate::model::BluesimSequence,
) -> Result<Scenario, ImportDiagnostic> {
    let mut requirements = BTreeSet::new();
    let mut stage_names = BTreeMap::<String, usize>::new();
    let mut stages = Vec::new();
    for contract in &sequence.contracts {
        let mut operations = Vec::new();
        let mut link_top = None;
        for operation in &contract.operations {
            collect_requirements(operation.guard(), &mut requirements).map_err(|message| {
                error_diagnostic(
                    "import.guard",
                    message,
                    operation_span(operation),
                    operation_expansion(operation),
                )
            })?;
            if let WorkflowOperation::Action(WorkflowAction::TransferArtifact(transfer)) = operation
            {
                declare_link_intermediate_transfer_source(&mut operations, transfer);
            }
            let record = match operation {
                WorkflowOperation::Action(WorkflowAction::EraseArtifact(erase))
                    if !operations.iter().any(|operation: &OperationRecord| {
                        operation
                            .artifacts
                            .outputs
                            .contains(&normalize_path(&erase.path))
                    }) =>
                {
                    Ok(OperationRecord::new(
                        map_erase(erase, EraseMode::EnsureAbsent),
                        OperationExpectation::Required,
                        provenance(erase.span, &erase.expansion),
                    ))
                }
                WorkflowOperation::Action(action) => map_action(action),
                WorkflowOperation::Assertion(assertion) => map_assertion(assertion),
            }
            .map_err(|message| {
                error_diagnostic(
                    "import.operation",
                    message,
                    operation_span(operation),
                    operation_expansion(operation),
                )
            })?;
            if let Action::BscLink { top, .. } = &record.action {
                link_top = Some(top.clone());
            }
            if let Some(path) = record.action.asserted_path().map(normalize_path) {
                declare_bound_output(&mut operations, path);
            }
            operations.push(record);
            if let WorkflowOperation::Action(WorkflowAction::LinkObjects(link)) = operation {
                if let Some(diagnostic) =
                    link_error_diagnostic_operation(link).map_err(|message| {
                        error_diagnostic("import.operation", message, link.span, &link.expansion)
                    })?
                {
                    operations.push(diagnostic);
                }
            }
        }
        let base = link_top.unwrap_or_else(|| "stage".to_owned());
        let occurrence = stage_names.entry(base.clone()).or_default();
        *occurrence += 1;
        let stage_id = if *occurrence == 1 {
            base
        } else {
            format!("{base}-{occurrence}")
        };
        stages.push(Stage {
            id: stage_id,
            operations,
        });
    }
    if !requirements.contains(&Requirement::Bluesim) {
        requirements.insert(Requirement::Bluesim);
    }
    Ok(Scenario {
        id: if sequence_index == 0 {
            "bluesim-sequence".to_owned()
        } else {
            format!("bluesim-sequence-{}", sequence_index + 1)
        },
        resource: ResourceClass::Heavy,
        fixtures: Vec::new(),
        requires: requirements.into_iter().collect(),
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExecutionOrderKey(Vec<usize>);

#[derive(Clone, Copy)]
struct ProvenanceWindow<'a> {
    after: Option<&'a ExecutionOrderKey>,
    before: Option<&'a ExecutionOrderKey>,
}

impl ProvenanceWindow<'_> {
    fn contains(self, order: &ExecutionOrderKey) -> bool {
        self.after.is_none_or(|after| order > after)
            && self.before.is_none_or(|before| order < before)
    }
}

struct OrderedBindingEvent {
    order: ExecutionOrderKey,
    kind: BindingEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WorkflowStageKey {
    Build,
    Run(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ProducerKey {
    Compile(usize),
    Simulation(usize),
    WorkflowAction(usize),
    Workflow {
        index: usize,
        stage: WorkflowStageKey,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundCheck {
    Assertion(usize),
    Comparison(usize),
}

#[derive(Default)]
struct CheckBindings {
    checks: BTreeMap<ProducerKey, Vec<BoundCheck>>,
    workflow_actions: BTreeMap<ProducerKey, BTreeSet<usize>>,
}

impl CheckBindings {
    fn get(&self, producer: &ProducerKey) -> Option<&Vec<BoundCheck>> {
        self.checks.get(producer)
    }

    fn workflow_actions(&self, producer: &ProducerKey) -> Option<&BTreeSet<usize>> {
        self.workflow_actions.get(producer)
    }
}

#[derive(Debug, Clone, Default)]
struct GeneratedArtifactProfile {
    verilog: bool,
    schedule: bool,
    dynamic_output: bool,
    sal: bool,
    working_directory: Option<String>,
    file_output_directories: BTreeSet<String>,
}

impl GeneratedArtifactProfile {
    fn matches(&self, path: &str) -> bool {
        let local = self.working_directory.as_ref().map_or(path, |directory| {
            path.strip_prefix(directory)
                .and_then(|suffix| suffix.strip_prefix('/'))
                .unwrap_or(path)
        });
        (self.verilog && is_local_generated_artifact(local, "v"))
            || (self.schedule && is_local_generated_artifact(local, "sched"))
            || (self.dynamic_output && is_dynamic_generated_output(local))
            || (self.sal && is_local_generated_artifact(local, "sal"))
            || self.file_output_directories.iter().any(|directory| {
                path.strip_prefix(directory)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            })
    }
}

struct ActiveProducer {
    target: ProducerKey,
    guard: Guard,
    artifacts: ArtifactFlow,
    generated: GeneratedArtifactProfile,
}

enum BindingEvent {
    Producer {
        target: ProducerKey,
        guard: Guard,
        paths: BTreeSet<String>,
        generated: GeneratedArtifactProfile,
    },
    LinkVerilog(usize),
    RunVerilog(usize),
    ShowRules(usize),
    DumpIntermediate(usize),
    BluetclRun(usize),
    Transfer(usize),
    RenderGolden(usize),
    RenderM4Curdir(usize),
    TextNormalize(usize),
    VerilogFilter(usize),
    Assertion(usize),
    Comparison(usize),
    Barrier,
}

fn check_bindings(script: &ScriptManifest, fixture_root: &Path) -> CheckBindings {
    let mut events = Vec::new();
    for (contract_index, contract) in script.contracts.iter().enumerate() {
        match contract {
            Contract::BasicOptions(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
            Contract::NoSourceCompile(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
            Contract::Ovl(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
            Contract::RenderGolden(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
            Contract::Compile(contract) if contract.guard.is_resolved() => {
                let kind = match compile_shape(contract) {
                    Ok(shape) => BindingEvent::Producer {
                        target: ProducerKey::Compile(contract_index),
                        guard: contract.guard.clone(),
                        paths: compile_artifact_paths(&shape, &contract.source, fixture_root)
                            .into_iter()
                            .map(|path| compile_contract_path(contract, &path))
                            .collect(),
                        generated: shape
                            .generated_artifact_profile(contract.working_directory.as_deref()),
                    },
                    Err(_) => BindingEvent::Barrier,
                };
                events.push(OrderedBindingEvent {
                    order: execution_order_key(contract.span, &contract.expansion),
                    kind,
                });
            }
            Contract::Compile(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
            Contract::Simulation(contract)
                if contract.guard.is_resolved()
                    && (contract.generation != crate::model::GenerationStrategy::Shared
                        || contract.backend == SimulationBackend::Icarus) =>
            {
                let target = if contract.generation == crate::model::GenerationStrategy::Shared {
                    script.contracts[..=contract_index]
                        .iter()
                        .position(|candidate| {
                            matches!(candidate, Contract::Simulation(candidate)
                                if same_simulation_invocation(contract, candidate))
                        })
                        .unwrap_or(contract_index)
                } else {
                    contract_index
                };
                events.push(OrderedBindingEvent {
                    order: execution_order_key(contract.span, &contract.expansion),
                    kind: BindingEvent::Producer {
                        target: ProducerKey::Simulation(target),
                        guard: contract.guard.clone(),
                        paths: simulation_binding_artifact_paths(script, contract, contract_index),
                        generated: GeneratedArtifactProfile {
                            verilog: matches!(
                                contract.generation,
                                crate::model::GenerationStrategy::Shared
                                    | crate::model::GenerationStrategy::Icarus
                            ),
                            dynamic_output: true,
                            ..GeneratedArtifactProfile::default()
                        },
                    },
                });
            }
            Contract::Simulation(contract) if contract.guard.is_resolved() => {}
            Contract::Simulation(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
            Contract::ExternalSet(contract) => {
                events.push(binding_barrier(contract.span, &contract.expansion));
            }
        }
    }
    events.extend(
        script
            .unsupported
            .iter()
            .map(|unsupported| binding_barrier(unsupported.span, &unsupported.expansion)),
    );
    events.extend(
        script
            .workflow_actions
            .iter()
            .enumerate()
            .map(|(action_index, action)| OrderedBindingEvent {
                order: execution_order_key(action_span(action), action_expansion(action)),
                kind: match action {
                    WorkflowAction::CompileObject(generation) if generation.guard.is_resolved() => {
                        match generation_artifact_paths(generation, None) {
                            Ok(paths) => BindingEvent::Producer {
                                target: ProducerKey::WorkflowAction(action_index),
                                guard: generation.guard.clone(),
                                paths,
                                generated: GeneratedArtifactProfile::default(),
                            },
                            Err(_) => BindingEvent::Barrier,
                        }
                    }
                    WorkflowAction::LinkVerilog(link) if link.guard.is_resolved() => {
                        BindingEvent::LinkVerilog(action_index)
                    }
                    WorkflowAction::RunVerilog(run) if run.guard.is_resolved() => {
                        BindingEvent::RunVerilog(action_index)
                    }
                    WorkflowAction::ShowRules(action) if action.guard.is_resolved() => {
                        BindingEvent::ShowRules(action_index)
                    }
                    WorkflowAction::DumpIntermediate(dump) if dump.guard.is_resolved() => {
                        BindingEvent::DumpIntermediate(action_index)
                    }
                    WorkflowAction::BluetclRun(run) if run.guard.is_resolved() => {
                        if run.artifact_inputs.is_empty() {
                            BindingEvent::Producer {
                                target: ProducerKey::WorkflowAction(action_index),
                                guard: run.guard.clone(),
                                paths: std::iter::once(normalize_path(&run.stdout))
                                    .chain(
                                        run.artifact_outputs
                                            .iter()
                                            .map(|path| normalize_path(path)),
                                    )
                                    .collect(),
                                generated: GeneratedArtifactProfile::default(),
                            }
                        } else {
                            BindingEvent::BluetclRun(action_index)
                        }
                    }
                    WorkflowAction::TransferArtifact(transfer) if transfer.guard.is_resolved() => {
                        BindingEvent::Transfer(action_index)
                    }
                    WorkflowAction::RenderGolden(render) if render.guard.is_resolved() => {
                        BindingEvent::RenderGolden(action_index)
                    }
                    WorkflowAction::RenderM4Curdir(render) if render.guard.is_resolved() => {
                        BindingEvent::RenderM4Curdir(action_index)
                    }
                    WorkflowAction::TextNormalize(action) if action.guard.is_resolved() => {
                        BindingEvent::TextNormalize(action_index)
                    }
                    WorkflowAction::VerilogFilter(action) if action.guard.is_resolved() => {
                        BindingEvent::VerilogFilter(action_index)
                    }
                    _ => BindingEvent::Barrier,
                },
            }),
    );
    events.extend(
        script
            .bluesim_sequences
            .iter()
            .flat_map(|sequence| &sequence.contracts)
            .flat_map(|contract| &contract.operations)
            .map(|operation| OrderedBindingEvent {
                order: execution_order_key(
                    operation_span(operation),
                    operation_expansion(operation),
                ),
                kind: BindingEvent::Barrier,
            }),
    );

    for (workflow_index, workflow) in script.bluesim_workflows.iter().enumerate() {
        for generation in &workflow.generations {
            events.push(binding_barrier(generation.span, &generation.expansion));
        }
        let mut build_order = execution_order_key(workflow.link.span, &workflow.link.expansion);
        for transfer in &workflow.link_transfers {
            build_order = build_order.max(execution_order_key(transfer.span, &transfer.expansion));
        }
        let kind = match link_artifact_paths(workflow) {
            Ok(paths) => BindingEvent::Producer {
                target: ProducerKey::Workflow {
                    index: workflow_index,
                    stage: WorkflowStageKey::Build,
                },
                guard: workflow.link.guard.clone(),
                paths,
                generated: GeneratedArtifactProfile::default(),
            },
            Err(_) => BindingEvent::Barrier,
        };
        events.push(OrderedBindingEvent {
            order: build_order,
            kind,
        });
        for (run_index, run) in workflow.runs.iter().enumerate() {
            let mut order = execution_order_key(run.action.span, &run.action.expansion);
            for transfer in &run.transfers {
                order = order.max(execution_order_key(transfer.span, &transfer.expansion));
            }
            events.push(OrderedBindingEvent {
                order,
                kind: BindingEvent::Producer {
                    target: ProducerKey::Workflow {
                        index: workflow_index,
                        stage: WorkflowStageKey::Run(run_index),
                    },
                    guard: run.action.guard.clone(),
                    paths: run_artifact_paths(run),
                    generated: GeneratedArtifactProfile::default(),
                },
            });
        }
    }
    events.extend(
        script
            .assertions
            .iter()
            .enumerate()
            .map(|(index, assertion)| OrderedBindingEvent {
                order: execution_order_key(assertion.span, &assertion.expansion),
                kind: BindingEvent::Assertion(index),
            }),
    );
    events.extend(
        script
            .comparisons
            .iter()
            .enumerate()
            .map(|(index, comparison)| OrderedBindingEvent {
                order: execution_order_key(comparison.span, &comparison.expansion),
                kind: BindingEvent::Comparison(index),
            }),
    );
    events.sort_by(|left, right| left.order.cmp(&right.order));

    let mut bindings = CheckBindings::default();
    let mut active = Vec::<ActiveProducer>::new();
    let mut checks_started = false;
    for event in events {
        match event.kind {
            BindingEvent::Producer {
                target,
                guard,
                paths,
                generated,
            } => {
                let extends_producer_batch = match target {
                    ProducerKey::Compile(_) => active
                        .iter()
                        .all(|producer| matches!(producer.target, ProducerKey::Compile(_))),
                    ProducerKey::Simulation(_) => active
                        .iter()
                        .all(|producer| matches!(producer.target, ProducerKey::Simulation(_))),
                    ProducerKey::WorkflowAction(_) => active
                        .iter()
                        .all(|producer| matches!(producer.target, ProducerKey::WorkflowAction(_))),
                    ProducerKey::Workflow { .. } => false,
                };
                if checks_started || !extends_producer_batch {
                    active.clear();
                }
                let artifacts = ArtifactFlow::new(paths);
                active.retain(|producer| {
                    producer.guard != guard || !producer.artifacts.overlaps(&artifacts)
                });
                active.push(ActiveProducer {
                    target,
                    guard,
                    artifacts,
                    generated,
                });
                checks_started = false;
            }
            BindingEvent::LinkVerilog(action_index) => {
                let WorkflowAction::LinkVerilog(link) = &script.workflow_actions[action_index]
                else {
                    unreachable!("Verilog link event must reference a Verilog link action");
                };
                let mut transformed = Vec::new();
                for producer in &mut active {
                    if matches!(producer.target, ProducerKey::Compile(_))
                        && guard_covers(&producer.guard, &link.guard)
                        && verilog_link_extends_flow(
                            &mut producer.artifacts,
                            &producer.generated,
                            link,
                        )
                        .is_ok()
                    {
                        transformed.push(producer.target);
                    }
                }
                if transformed.len() == 1 {
                    bindings
                        .workflow_actions
                        .entry(transformed[0])
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::RunVerilog(action_index) => {
                let WorkflowAction::RunVerilog(run) = &script.workflow_actions[action_index] else {
                    unreachable!("Verilog run event must reference a Verilog run action");
                };
                let mut transformed = Vec::new();
                for producer in &mut active {
                    if matches!(producer.target, ProducerKey::Compile(_))
                        && guard_covers(&producer.guard, &run.guard)
                        && verilog_run_extends_flow(&mut producer.artifacts, run).is_ok()
                    {
                        transformed.push(producer.target);
                    }
                }
                if transformed.len() == 1 {
                    bindings
                        .workflow_actions
                        .entry(transformed[0])
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::ShowRules(action_index) => {
                let WorkflowAction::ShowRules(action) = &script.workflow_actions[action_index]
                else {
                    unreachable!("showrules binding event must reference a showrules action");
                };
                let mut transformed = Vec::new();
                for producer in &mut active {
                    if guard_covers(&producer.guard, &action.guard)
                        && showrules_extends_flow(&mut producer.artifacts, action)
                    {
                        transformed.push(producer.target);
                    }
                }
                if transformed.len() == 1 {
                    bindings
                        .workflow_actions
                        .entry(transformed[0])
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::DumpIntermediate(action_index) => {
                let WorkflowAction::DumpIntermediate(dump) = &script.workflow_actions[action_index]
                else {
                    unreachable!("dump binding event must reference a dump action");
                };
                let input = normalize_path(&dump.input);
                let exact = active
                    .iter()
                    .enumerate()
                    .filter(|(_, producer)| {
                        guard_covers(&producer.guard, &dump.guard)
                            && producer.artifacts.contains(&input)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                let source_matched = (!exact.is_empty()).then_some(exact).unwrap_or_else(|| {
                    active
                        .iter()
                        .enumerate()
                        .filter(|(_, producer)| {
                            guard_covers(&producer.guard, &dump.guard)
                                && producer_source_matches_dump_input(
                                    script,
                                    fixture_root,
                                    producer.target,
                                    &input,
                                )
                        })
                        .map(|(index, _)| index)
                        .collect()
                });
                if !checks_started && source_matched.len() == 1 {
                    let producer = &mut active[source_matched[0]];
                    producer.artifacts.insert(input);
                    producer.artifacts.insert(normalize_path(&dump.output));
                    bindings
                        .workflow_actions
                        .entry(producer.target)
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::BluetclRun(action_index) => {
                let WorkflowAction::BluetclRun(run) = &script.workflow_actions[action_index] else {
                    unreachable!("Bluetcl binding event must reference a Bluetcl action");
                };
                let inputs = run
                    .artifact_inputs
                    .iter()
                    .map(|path| normalize_path(path))
                    .collect::<Vec<_>>();
                let matched = active
                    .iter()
                    .enumerate()
                    .filter(|(_, producer)| {
                        guard_covers(&producer.guard, &run.guard)
                            && inputs
                                .iter()
                                .all(|input| producer.artifacts.contains(input))
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if matched.len() == 1 {
                    let producer = &mut active[matched[0]];
                    producer.artifacts.insert(normalize_path(&run.stdout));
                    for output in &run.artifact_outputs {
                        producer.artifacts.insert(normalize_path(output));
                    }
                    bindings
                        .workflow_actions
                        .entry(producer.target)
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::Transfer(action_index) => {
                let WorkflowAction::TransferArtifact(transfer) =
                    &script.workflow_actions[action_index]
                else {
                    unreachable!("transfer binding event must reference a transfer action");
                };
                let mut transformed = Vec::new();
                for producer in &mut active {
                    if guard_covers(&producer.guard, &transfer.guard)
                        && producer.artifacts.apply(transfer)
                    {
                        transformed.push(producer.target);
                    }
                }
                if transformed.len() == 1 {
                    bindings
                        .workflow_actions
                        .entry(transformed[0])
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::RenderGolden(action_index) => {
                let WorkflowAction::RenderGolden(render) = &script.workflow_actions[action_index]
                else {
                    unreachable!("golden render event must reference its typed action");
                };
                let matched = if checks_started {
                    Vec::new()
                } else {
                    active
                        .iter_mut()
                        .filter(|producer| guard_covers(&producer.guard, &render.guard))
                        .collect::<Vec<_>>()
                };
                if matched.len() == 1 {
                    let producer = matched.into_iter().next().expect("one render owner");
                    producer.artifacts.insert(normalize_path(&render.output));
                    bindings
                        .workflow_actions
                        .entry(producer.target)
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::RenderM4Curdir(action_index) => {
                let WorkflowAction::RenderM4Curdir(render) = &script.workflow_actions[action_index]
                else {
                    unreachable!("M4 CURDIR event must reference its typed render action");
                };
                let matched = if checks_started {
                    Vec::new()
                } else {
                    active
                        .iter_mut()
                        .filter(|producer| guard_covers(&producer.guard, &render.guard))
                        .collect::<Vec<_>>()
                };
                if matched.len() == 1 {
                    let producer = matched.into_iter().next().expect("one renderer producer");
                    producer.artifacts.insert(normalize_path(&render.output));
                    bindings
                        .workflow_actions
                        .entry(producer.target)
                        .or_default()
                        .insert(action_index);
                } else if !active.is_empty() {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::TextNormalize(action_index) => {
                let WorkflowAction::TextNormalize(action) = &script.workflow_actions[action_index]
                else {
                    unreachable!("text normalization event must reference its typed action");
                };
                let source = normalize_path(&action.source);
                let destination = normalize_path(&action.destination);
                let matched = active
                    .iter_mut()
                    .filter(|producer| {
                        guard_covers(&producer.guard, &action.guard)
                            && producer.artifacts.contains(&source)
                            && !producer.artifacts.contains(&destination)
                    })
                    .collect::<Vec<_>>();
                if matched.len() == 1 {
                    let producer = matched
                        .into_iter()
                        .next()
                        .expect("one text transform owner");
                    producer.artifacts.insert(destination);
                    bindings
                        .workflow_actions
                        .entry(producer.target)
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::VerilogFilter(action_index) => {
                let WorkflowAction::VerilogFilter(action) = &script.workflow_actions[action_index]
                else {
                    unreachable!("Verilog filter event must reference its typed action");
                };
                let path = normalize_path(&action.path);
                let matched = active
                    .iter()
                    .filter(|producer| {
                        guard_covers(&producer.guard, &action.guard)
                            && producer.artifacts.contains(&path)
                    })
                    .map(|producer| producer.target)
                    .collect::<Vec<_>>();
                if matched.len() == 1 {
                    bindings
                        .workflow_actions
                        .entry(matched[0])
                        .or_default()
                        .insert(action_index);
                } else {
                    active.clear();
                }
                checks_started = false;
            }
            BindingEvent::Assertion(assertion_index) => {
                let assertion = &script.assertions[assertion_index];
                if bind_check(
                    &mut bindings,
                    &active,
                    &assertion.guard,
                    assertion.arguments.first(),
                    BoundCheck::Assertion(assertion_index),
                ) {
                    checks_started = true;
                    continue;
                }
                active.clear();
                checks_started = false;
            }
            BindingEvent::Comparison(comparison_index) => {
                let comparison = &script.comparisons[comparison_index];
                if bind_check(
                    &mut bindings,
                    &active,
                    &comparison.guard,
                    comparison.arguments.first(),
                    BoundCheck::Comparison(comparison_index),
                ) {
                    checks_started = true;
                    continue;
                }
                active.clear();
                checks_started = false;
            }
            BindingEvent::Barrier => {
                active.clear();
                checks_started = false;
            }
        }
    }
    bindings
}

fn producer_source_matches_dump_input(
    script: &ScriptManifest,
    fixture_root: &Path,
    producer: ProducerKey,
    input: &str,
) -> bool {
    let ProducerKey::Compile(index) = producer else {
        return false;
    };
    let Some(Contract::Compile(contract)) = script.contracts.get(index) else {
        return false;
    };
    source_matches_dump_input(&contract.source, fixture_root, input)
}

fn source_matches_dump_input(source: &str, fixture_root: &Path, input: &str) -> bool {
    let Some(object_stem) = input.strip_suffix(".bo") else {
        return false;
    };
    let roots = BTreeSet::from([normalize_path(source)]);
    let resolution = resolve_local_dependency_closures(fixture_root, &[roots]);
    if !resolution.diagnostics.is_empty() {
        return false;
    }
    resolution.paths.first().is_some_and(|closure| {
        closure
            .iter()
            .filter_map(|path| Path::new(path).file_stem().and_then(|stem| stem.to_str()))
            .filter(|stem| *stem == object_stem)
            .count()
            == 1
    })
}

fn bind_check(
    bindings: &mut CheckBindings,
    active: &[ActiveProducer],
    check_guard: &Guard,
    path: Option<&String>,
    check: BoundCheck,
) -> bool {
    let Some(path) = path.map(|path| normalize_path(path)) else {
        return false;
    };
    let eligible = |producer: &&ActiveProducer| guard_covers(&producer.guard, check_guard);
    let exact = active
        .iter()
        .filter(eligible)
        .filter(|producer| producer.artifacts.contains(&path))
        .collect::<Vec<_>>();
    let producer = if exact.len() == 1 {
        exact[0]
    } else if exact.is_empty() {
        let derived = implicit_dumpbo_input(&path)
            .filter(|_| guard_has_capability(check_guard, Capability::InternalChecks))
            .map(|input| {
                active
                    .iter()
                    .filter(eligible)
                    .filter(|producer| matches!(producer.target, ProducerKey::Compile(_)))
                    .filter(|producer| producer.artifacts.contains(&input))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if derived.len() == 1 {
            derived[0]
        } else if derived.is_empty() {
            let Some(producer) = active
                .iter()
                .filter(eligible)
                .rev()
                .find(|producer| producer.generated.matches(&path))
            else {
                return false;
            };
            producer
        } else {
            return false;
        }
    } else {
        return false;
    };
    bindings
        .checks
        .entry(producer.target)
        .or_default()
        .push(check);
    true
}

fn implicit_dumpbo_input(path: &str) -> Option<String> {
    let input = path.strip_suffix(".dumpbo-out")?;
    Path::new(input)
        .extension()
        .is_some_and(|extension| extension == "bo")
        .then(|| input.to_owned())
}

fn guard_has_capability(guard: &Guard, expected: Capability) -> bool {
    match guard {
        Guard::Capability { capability } => *capability == expected,
        Guard::All { guards } => guards
            .iter()
            .any(|guard| guard_has_capability(guard, expected)),
        Guard::Always | Guard::Not { .. } | Guard::UnsupportedExpression { .. } => false,
    }
}

fn simulation_binding_artifact_paths(
    script: &ScriptManifest,
    contract: &SimulationContract,
    contract_index: usize,
) -> BTreeSet<String> {
    let mut paths = simulation_artifact_paths(contract);
    let has_shared_bluesim_link = contract.generation == crate::model::GenerationStrategy::Shared
        && script.contracts[..=contract_index].iter().any(|candidate| {
            matches!(candidate, Contract::Simulation(candidate)
                if candidate.backend == SimulationBackend::Bluesim
                    && same_simulation_invocation(contract, candidate))
        });
    if has_shared_bluesim_link {
        let icarus_executable = paths.iter().find_map(|path| path.strip_suffix(".vexe"));
        if let Some(top) = icarus_executable {
            paths.insert(format!("{top}.bsc-ccomp-out"));
        }
    }
    paths
}

fn simulation_artifact_paths(contract: &SimulationContract) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let is_multi = contract.helper.contains("_multi") || contract.helper == "test_c_veri_worker";
    let top = if is_multi {
        contract.arguments.get(1).cloned()
    } else {
        contract
            .arguments
            .first()
            .map(|source| format!("sys{source}"))
    };
    let Some(top) = top.filter(|top| !top.is_empty()) else {
        return paths;
    };

    let generation_mode = match contract.generation {
        crate::model::GenerationStrategy::Shared => SimulationGenerationMode::SharedElaboration,
        crate::model::GenerationStrategy::Bluesim => SimulationGenerationMode::Bluesim,
        crate::model::GenerationStrategy::Icarus => SimulationGenerationMode::Verilog,
    };
    paths.insert(generation_mode.compiler_output_path(&normalize_path(&contract.source)));
    match contract.backend {
        SimulationBackend::Bluesim => {
            paths.insert(format!("{top}.c.out"));
        }
        SimulationBackend::Icarus => {
            paths.insert(simulation_executable_artifact(
                PlanSimulationBackend::Icarus,
                &top,
            ));
            paths.insert(format!("{top}.v.out"));
        }
    }
    if contract.generation != crate::model::GenerationStrategy::Bluesim {
        paths.insert(format!("{top}.v"));
        paths.insert(format!("{top}.cxx"));
        let module_index = if is_multi {
            Some(2)
        } else if contract.helper.contains("_modules") {
            Some(1)
        } else {
            None
        };
        if let Some(module_list) = module_index.and_then(|index| contract.arguments.get(index)) {
            if let Ok(modules) = parse_static_tcl_list(module_list) {
                paths.extend(modules.into_iter().map(|module| {
                    let module = normalize_path(&module);
                    if Path::new(&module).extension().is_some() {
                        module
                    } else {
                        format!("{module}.v")
                    }
                }));
            }
        }
    }
    paths
}

fn is_local_generated_artifact(path: &str, extension: &str) -> bool {
    let path = Path::new(path);
    path.components().count() == 1
        && path.file_stem().is_some_and(|stem| !stem.is_empty())
        && path.extension().and_then(|value| value.to_str()) == Some(extension)
}

fn is_dynamic_generated_output(path: &str) -> bool {
    is_local_generated_artifact(path, "log")
        || (is_local_generated_artifact(path, "out")
            && !path.ends_with(".c.out")
            && !path.ends_with(".v.out"))
}

fn guard_covers(producer: &Guard, consumer: &Guard) -> bool {
    if !producer.is_resolved() || !consumer.is_resolved() {
        return false;
    }
    producer == &Guard::Always
        || producer == consumer
        || matches!(
            consumer,
            Guard::All { guards } if guards.iter().any(|guard| guard == producer)
        )
}

fn binding_barrier(
    span: ManifestSourceSpan,
    expansion: &[ManifestSourceSpan],
) -> OrderedBindingEvent {
    OrderedBindingEvent {
        order: execution_order_key(span, expansion),
        kind: BindingEvent::Barrier,
    }
}

#[derive(Debug, Clone)]
struct ArtifactFlow {
    available: BTreeSet<String>,
}

impl ArtifactFlow {
    fn new(paths: BTreeSet<String>) -> Self {
        Self { available: paths }
    }

    fn contains(&self, path: &str) -> bool {
        self.available.contains(path)
    }

    fn insert(&mut self, path: String) {
        self.available.insert(path);
    }

    fn apply(&mut self, transfer: &crate::model::ArtifactTransferAction) -> bool {
        let source = normalize_path(&transfer.source);
        let destination = normalize_path(&transfer.destination);
        if source == destination
            || !self.available.contains(&source)
            || self.available.contains(&destination)
        {
            return false;
        }
        if transfer.operation == ArtifactTransferOperation::Move {
            self.available.remove(&source);
        }
        self.available.insert(destination);
        true
    }

    fn remove(&mut self, path: &str) -> bool {
        self.available.remove(&normalize_path(path))
    }

    fn overlaps(&self, other: &Self) -> bool {
        !self.available.is_disjoint(&other.available)
    }

    fn apply_all(&mut self, transfers: &[crate::model::ArtifactTransferAction]) -> bool {
        let mut valid = true;
        for transfer in transfers {
            valid &= self.apply(transfer);
        }
        valid
    }

    fn into_paths(self) -> BTreeSet<String> {
        self.available
    }
}

fn systemc_link_error_diagnostic_operation(
    link: &SystemcLinkAction,
) -> Result<Option<OperationRecord>, String> {
    let Some(diagnostic) = &link.error_diagnostic else {
        return Ok(None);
    };
    if link.expected_exit != ExpectedExit::Failure {
        return Err("create_systemc_objects_fail_error requires expected failure".to_owned());
    }
    let count = diagnostic.count.parse::<usize>().map_err(|error| {
        format!(
            "create_systemc_objects_fail_error has invalid diagnostic count {:?}: {error}",
            diagnostic.count
        )
    })?;
    Ok(Some(OperationRecord::new(
        Action::AssertDiagnosticCount {
            path: format!("{}.bsc-ccomp-out", normalize_path(&link.top)),
            kind: DiagnosticKind::Error,
            code: Some(diagnostic.code.clone()),
            count,
        },
        OperationExpectation::Required,
        provenance(link.span, &link.expansion),
    )))
}

fn link_error_diagnostic_operation(
    link: &crate::model::LinkObjectsAction,
) -> Result<Option<OperationRecord>, String> {
    let Some(diagnostic) = &link.error_diagnostic else {
        return Ok(None);
    };
    if link.expected_exit != ExpectedExit::Failure {
        return Err("link_objects_fail_error requires expected failure".to_owned());
    }
    if diagnostic.code.trim().is_empty() {
        return Err("link_objects_fail_error requires a non-empty diagnostic code".to_owned());
    }
    let count = diagnostic.count.parse::<usize>().map_err(|error| {
        format!(
            "link_objects_fail_error has invalid diagnostic count {:?}: {error}",
            diagnostic.count
        )
    })?;
    Ok(Some(OperationRecord::new(
        Action::AssertDiagnosticCount {
            path: format!("{}.bsc-ccomp-out", normalize_path(&link.top)),
            kind: DiagnosticKind::Error,
            code: Some(diagnostic.code.clone()),
            count,
        },
        OperationExpectation::Required,
        provenance(link.span, &link.expansion),
    )))
}

fn link_initial_artifact_paths(link: &crate::model::LinkObjectsAction) -> BTreeSet<String> {
    let mut paths = BTreeSet::from([format!("{}.bsc-ccomp-out", link.top)]);
    if link.expected_exit == ExpectedExit::Success {
        paths.extend([
            format!("{}.cxx", link.top),
            format!("model_{}.cxx", link.top),
            format!("{}.o", link.top),
            format!("model_{}.o", link.top),
        ]);
    }
    paths
}

fn compile_artifact_paths(
    shape: &CompileShape,
    source: &str,
    fixture_root: &Path,
) -> BTreeSet<String> {
    let mut paths = shape.artifact_paths(source);
    if !shape.produces_verilog_outputs() || !shape.uses_verilog_backend() {
        return relocate_compile_artifact_paths(paths, &shape.args);
    }
    let source_path = fixture_root.join(normalize_path(source));
    let Ok(contents) = fs::read_to_string(source_path) else {
        return paths;
    };
    let synthesize = synthesize_module_regex();
    for module in synthesize
        .captures_iter(&contents)
        .filter_map(|capture| capture.get(1))
        .map(|module| module.as_str())
    {
        paths.insert(format!("{module}.v"));
        if shape.produces_elaboration_outputs() {
            paths.insert(format!("{module}.ba"));
        }
        paths.extend(compile_dump_paths(&shape.args, module));
    }
    let noinline_function = Regex::new(
        r"(?s)\(\*\s*noinline\s*\*\)(?:\s*\(\*.*?\*\))*\s*function(?:\s+[^;()]+)?\s+([A-Za-z_][A-Za-z0-9_$]*)\s*\(",
    )
    .expect("valid noinline function regex");
    paths.extend(
        noinline_function
            .captures_iter(&contents)
            .filter_map(|capture| capture.get(1))
            .map(|function| format!("module_{}.v", function.as_str())),
    );
    relocate_compile_artifact_paths(paths, &shape.args)
}

fn compile_output_directories(arguments: &[String]) -> BTreeSet<String> {
    ["-bdir", "-vdir", "-fdir"]
        .into_iter()
        .filter_map(|option| {
            let values = option_values(arguments, option).ok()?;
            (values.len() == 1)
                .then(|| normalize_path(&values[0]))
                .filter(|directory| is_safe_relative(directory))
        })
        .collect()
}

fn relocate_compile_artifact_paths(
    paths: BTreeSet<String>,
    arguments: &[String],
) -> BTreeSet<String> {
    let output_directory = |option| {
        let values = option_values(arguments, option).ok()?;
        (values.len() == 1)
            .then(|| normalize_path(&values[0]))
            .filter(|directory| is_safe_relative(directory))
    };
    let bdir = output_directory("-bdir");
    let vdir = output_directory("-vdir");
    paths
        .into_iter()
        .map(|path| {
            let extension = Path::new(&path)
                .extension()
                .and_then(|extension| extension.to_str());
            let directory = match extension {
                Some("ba" | "bo") => bdir.as_deref(),
                Some("v") => vdir.as_deref(),
                _ => None,
            };
            directory.map_or(path.clone(), |directory| {
                path.strip_prefix(directory)
                    .is_some_and(|suffix| suffix.starts_with('/'))
                    .then_some(path.clone())
                    .unwrap_or_else(|| format!("{directory}/{path}"))
            })
        })
        .collect()
}

fn verilog_link_extends_flow(
    flow: &mut ArtifactFlow,
    generated: &GeneratedArtifactProfile,
    link: &crate::model::LinkVerilogAction,
) -> Result<(), String> {
    let objects = parse_arguments(&link.objects, "Verilog link objects")?;
    if objects
        .iter()
        .any(|object| object.contains(['*', '?', '[', ']']))
    {
        return Err("link_verilog_pass object globs require shell expansion and are not statically executable".to_owned());
    }
    if !link.no_main {
        let options = parse_arguments(&link.options, "Verilog link options")?;
        if options.iter().any(|argument| {
            Path::new(argument).is_absolute() || argument.contains("=/") || argument.contains("=\\")
        }) {
            return Err("link_verilog_pass options contain an unsafe absolute path".to_owned());
        }
    }
    let top = normalize_path(&link.top);
    if !is_safe_relative(&top) || top.contains('/') {
        return Err(format!(
            "link_verilog_pass top must be a portable file-name segment: {top:?}"
        ));
    }
    let normalized_objects = objects
        .iter()
        .map(|object| normalize_path(object))
        .collect::<Vec<_>>();
    let is_generated = |path: &str| {
        flow.contains(path)
            || (!link.no_main && generated.verilog && is_local_generated_artifact(path, "v"))
    };
    let generated_top = format!("{top}.v");
    let has_link_input = normalized_objects.iter().any(|object| is_generated(object));
    let connected = if link.no_main {
        has_link_input
    } else {
        is_generated(&generated_top) && (normalized_objects.is_empty() || has_link_input)
    };
    if !connected {
        return Err(format!(
            "{} for {top:?} is not connected to the active compile artifacts",
            if link.no_main {
                "link_verilog_no_main_pass"
            } else {
                "link_verilog_pass"
            }
        ));
    }
    flow.insert(format!("{top}.bsc-vcomp-out"));
    if link.expected_exit == ExpectedExit::Success && link.simulator.produces_executable() {
        flow.insert(simulation_executable_artifact(
            PlanSimulationBackend::Icarus,
            &top,
        ));
    }
    Ok(())
}

fn verilog_run_extends_flow(
    flow: &mut ArtifactFlow,
    run: &crate::model::RunVerilogAction,
) -> Result<(), String> {
    let executable = normalize_path(&run.executable);
    if !flow.contains(&simulation_executable_artifact(
        PlanSimulationBackend::Icarus,
        &executable,
    )) {
        return Err(format!(
            "{} executable {executable:?} was not produced by a preceding Verilog link",
            if run.vcd {
                "sim_verilog_vcd"
            } else if run.expected_exits.is_empty() {
                "sim_verilog"
            } else {
                "sim_verilog_status"
            }
        ));
    }
    parse_arguments(&run.options, "Icarus simulation options")?;
    flow.insert(normalize_path(&run.stdout));
    if run.vcd {
        flow.insert("dump.vcd".to_owned());
    }
    Ok(())
}

fn showrules_extends_flow(flow: &mut ArtifactFlow, action: &crate::model::ShowRulesAction) -> bool {
    let input = normalize_path(&action.input);
    let output = normalize_path(&action.output);
    let stdout = normalize_path(&action.stdout);
    if input == output
        || input == stdout
        || output == stdout
        || !flow.contains(&input)
        || flow.contains(&output)
        || flow.contains(&stdout)
    {
        return false;
    }
    flow.insert(output);
    flow.insert(stdout);
    true
}

fn link_artifact_paths(
    workflow: &crate::model::BluesimWorkflow,
) -> Result<BTreeSet<String>, String> {
    let mut paths = link_initial_artifact_paths(&workflow.link);
    for generation in &workflow.generations {
        paths.extend(generation_artifact_paths(generation, Some(&workflow.top))?);
    }
    let mut flow = ArtifactFlow::new(paths);
    if !flow.apply_all(&workflow.pre_link_transfers) || !flow.apply_all(&workflow.link_transfers) {
        return Err("workflow artifact transfers do not form a valid ordered flow".to_owned());
    }
    Ok(flow.into_paths())
}

fn bluesim_vcd_paths(arguments: &[String]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] != "-V" {
            index += 1;
            continue;
        }
        let explicit = arguments
            .get(index + 1)
            .filter(|value| !value.starts_with('-'));
        paths.insert(normalize_path(explicit.map_or("dump.vcd", String::as_str)));
        index += usize::from(explicit.is_some()) + 1;
    }
    paths
}

fn run_initial_artifact_paths(run: &crate::model::BluesimRun) -> BTreeSet<String> {
    let mut paths = BTreeSet::from([normalize_path(&run.action.stdout)]);
    if let Ok(arguments) = parse_arguments(&run.action.options, "Bluesim options") {
        paths.extend(bluesim_vcd_paths(&arguments));
    }
    paths
}

fn run_artifact_paths(run: &crate::model::BluesimRun) -> BTreeSet<String> {
    let mut flow = ArtifactFlow::new(run_initial_artifact_paths(run));
    flow.apply_all(&run.transfers);
    flow.into_paths()
}

#[derive(Default)]
struct ImportConsumption {
    actions: Vec<usize>,
    assertions: Vec<usize>,
    comparisons: Vec<usize>,
    golden_paths: Vec<String>,
}

struct ImportedScenario {
    scenario: Scenario,
    consumption: ImportConsumption,
}

#[derive(Default)]
struct PlanAssembly {
    scenarios: Vec<Scenario>,
    compile_scenarios: BTreeMap<usize, usize>,
    simulation_scenarios: BTreeMap<usize, usize>,
    consumed_actions: BTreeSet<usize>,
    consumed_assertions: BTreeSet<usize>,
    consumed_comparisons: BTreeSet<usize>,
    golden_paths: BTreeSet<String>,
}

impl PlanAssembly {
    fn push_compile(&mut self, contract_index: usize, imported: ImportedScenario) {
        self.compile_scenarios
            .insert(contract_index, self.scenarios.len());
        self.push(imported);
    }

    fn push_simulation(&mut self, contract_index: usize, imported: ImportedScenario) {
        self.simulation_scenarios
            .insert(contract_index, self.scenarios.len());
        self.push(imported);
    }

    fn push(&mut self, imported: ImportedScenario) {
        self.scenarios.push(imported.scenario);
        self.consumed_actions.extend(imported.consumption.actions);
        self.consumed_assertions
            .extend(imported.consumption.assertions);
        self.consumed_comparisons
            .extend(imported.consumption.comparisons);
        self.golden_paths.extend(imported.consumption.golden_paths);
    }
}

const COURSE_LAB_PLAN_ORIGIN: &str = "testsuite/bsc.bsv_examples/mesa/course_lab/course_lab.exp";
const COURSE_LAB_PLAN_SHA256: &str =
    "dea11a2a650740b8d63311addcd10a902948a96fa43727bb75fda598ea1fbbb7";
const SAL_PLAN_ORIGIN: &str = "testsuite/bsc.misc/sal/sal.exp";
const SAL_PLAN_SHA256: &str = "f8e36e7bc6ed53ae4347c6a0cd86bac0b49910e41ac63bc27eaa286602bce918";
const INOUT_PLAN_ORIGIN: &str = "testsuite/bsc.verilog/inout/inout.exp";
const INOUT_PLAN_SHA256: &str = "390aae7815c23392243a1b118a9a330479deed303c880b30add18826903e6fe7";

const COURSE_LAB_COMMON_CLOSURE: &[&str] = &[
    "ClientServerLib.bsv",
    "MesaDefs.bsv",
    "MesaIDefs.bsv",
    "Mesa_Dm.bsv",
    "Mesa_Mif.bsv",
    "Mesa_Vff.bsv",
];

const COURSE_LAB_VARIANT_CLOSURES: &[(&str, &[&str])] = &[
    ("MesaTx.bsv", &["MesaTxLpm.bsv", "ShiftRegisters.bsv"]),
    (
        "MesaStatic.bsv",
        &["MesaStaticLpm.bsv", "ShiftRegisters.bsv"],
    ),
    ("MesaFlex.bsv", &["MesaFlexLpm.bsv", "Replicator.bsv"]),
    ("MesaCirc.bsv", &["MesaCircLpm.bsv", "CompletionBuffer.bsv"]),
];

const SAL_LAMBDA_MEMBERS: &[&str] = &[
    "AVMethod_UnusedValue.bsv",
    "DynArrSelWithImplCond.bsv",
    "DynamicInstArg.bsv",
    "Extract.bsv",
    "MergeIf.bsv",
    "MergeIf2.bsv",
    "MergeIf3.bsv",
    "MethodReturn_AMethValue.bsv",
    "MethodReturn_ATaskValue.bsv",
    "Method_Split.bsv",
    "Methods.bsv",
    "MultiArityConcat.bsv",
    "NoInline.bsv",
    "PrimMods.bsv",
    "RealInstArg.bsv",
    "StringInstArg.bsv",
    "Structs.bsv",
    "Tb.bsv",
];

const INOUT_BO_CLOSURE: &[&str] = &[
    "ArgToIfc.bo",
    "BothSendAndReceive.bo",
    "Cond.bo",
    "Cond_expr.bo",
    "Connect_wrapped.bo",
    "Connect_wrapped2.bo",
    "EnabledReceiver.bo",
    "FunctionInout.bo",
    "HigherFunction.bo",
    "InoutUsed.bo",
    "LineConnect.bo",
    "ManyLineConnect1.bo",
    "ManyLineConnect2.bo",
    "ManyLineConnectArray.bo",
    "RegEnConnect.bo",
    "RegEnConnect2.bo",
    "RegisteredSender.bo",
    "SenderReceiver.bo",
    "SimpleConnect1.bo",
    "SimpleConnect2.bo",
    "TbBoth.bo",
    "WrapConnection.bo",
    "WrapReceiver.bo",
    "WrapSender.bo",
];

const INOUT_ARCHIVE_OUTPUTS: &[&str] = &[
    "sysSimpleConnect1.v.out",
    "sysSimpleConnect2.v.out",
    "sysRegEnConnect.v.out",
    "sysRegEnConnect2.v.out",
    "sysManyLineConnectArray.v.out",
    "sysManyLineConnect1.v.out",
    "sysManyLineConnect2.v.out",
    "sysLineConnect.v.out",
    "sysConnect_wrapped.v.out",
    "sysConnect_wrapped2.v.out",
    "sysHigherFunction.v.out",
    "sysFunctionInout.v.out",
    "sysCond_expr.v.out",
    "sysCond.v.out",
    "sysTbBoth.v.out",
    "sysInoutUsed.v.out",
];

#[derive(Clone)]
enum PinnedBatch {
    CourseLab {
        variants: Vec<(String, ManifestSourceSpan)>,
    },
    Sal {
        setup_span: ManifestSourceSpan,
        cleanup_span: ManifestSourceSpan,
    },
    Inout {
        erase_spans: [ManifestSourceSpan; 2],
        archive_span: ManifestSourceSpan,
    },
}

fn prepare_pinned_batch(
    script: &mut ScriptManifest,
    fixture_root: &Path,
) -> Result<Option<PinnedBatch>, ImportDiagnostic> {
    let expected_sha256 = match script.origin.as_str() {
        COURSE_LAB_PLAN_ORIGIN => COURSE_LAB_PLAN_SHA256,
        SAL_PLAN_ORIGIN => SAL_PLAN_SHA256,
        INOUT_PLAN_ORIGIN => INOUT_PLAN_SHA256,
        _ => return Ok(None),
    };
    let span = script
        .contracts
        .first()
        .map(contract_source_span)
        .or_else(|| {
            script
                .unsupported
                .first()
                .map(|unsupported| unsupported.span)
        })
        .unwrap_or_else(empty_manifest_span);
    let fail = |message: String| error_diagnostic("import.pinned_batch", message, span, &[]);
    if script.source_sha256 != expected_sha256 {
        return Err(fail(format!(
            "{} changed from audited SHA-256 {expected_sha256}; closed expansion refused",
            script.origin
        )));
    }

    let batch = match script.origin.as_str() {
        COURSE_LAB_PLAN_ORIGIN => {
            require_pinned_shape(script, 10, 0, 4, 68, 15).map_err(&fail)?;
            let mut variants = Vec::new();
            for expected in [
                "MesaTx.bsv",
                "MesaStatic.bsv",
                "MesaFlex.bsv",
                "MesaCirc.bsv",
            ] {
                let matches = script
                    .workflow_actions
                    .iter()
                    .filter_map(|action| match action {
                        WorkflowAction::TransferArtifact(transfer)
                            if transfer.operation == ArtifactTransferOperation::Copy
                                && transfer.source == expected
                                && transfer.destination == "Mesa.bsv" =>
                        {
                            Some(transfer.span)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err(fail(format!(
                        "course_lab requires one exact {expected} -> Mesa.bsv copy, found {}",
                        matches.len()
                    )));
                }
                variants.push((expected.to_owned(), matches[0]));
            }
            script.unsupported.clear();
            PinnedBatch::CourseLab { variants }
        }
        SAL_PLAN_ORIGIN => {
            require_pinned_shape(script, 16, 8, 20, 0, 5).map_err(&fail)?;
            audit_pinned_regular_membership(fixture_root, "lambda_calculus", SAL_LAMBDA_MEMBERS)
                .map_err(&fail)?;
            let setup_span = script
                .unsupported
                .iter()
                .find(|unsupported| unsupported.span.start_line == 17)
                .map(|unsupported| unsupported.span)
                .ok_or_else(|| fail("SAL setup foreach shape changed".to_owned()))?;
            let cleanup_span = script
                .unsupported
                .iter()
                .find(|unsupported| unsupported.span.start_line == 209)
                .map(|unsupported| unsupported.span)
                .ok_or_else(|| fail("SAL cleanup foreach shape changed".to_owned()))?;
            for contract in &mut script.contracts {
                let Contract::Compile(contract) = contract else {
                    return Err(fail("SAL contains a non-compile contract".to_owned()));
                };
                if contract.working_directory.is_some()
                    || !SAL_LAMBDA_MEMBERS.contains(&contract.source.as_str())
                {
                    return Err(fail(format!(
                        "SAL compile source shape changed: {:?}",
                        contract.source
                    )));
                }
                contract.working_directory = Some("sal".to_owned());
            }
            for comparison in &mut script.comparisons {
                prefix_check_path(&mut comparison.arguments, "sal").map_err(&fail)?;
            }
            for assertion in &mut script.assertions {
                prefix_check_path(&mut assertion.arguments, "sal").map_err(&fail)?;
            }
            script.unsupported.clear();
            PinnedBatch::Sal {
                setup_span,
                cleanup_span,
            }
        }
        INOUT_PLAN_ORIGIN => {
            require_pinned_shape(script, 53, 3, 9, 4, 10).map_err(&fail)?;
            let erase_many = script
                .unsupported
                .iter()
                .filter(|unsupported| unsupported.command.as_deref() == Some("erase_many"))
                .collect::<Vec<_>>();
            if erase_many.len() != 2
                || erase_many
                    .iter()
                    .any(|unsupported| unsupported.expansion.len() != 1)
            {
                return Err(fail("inout erase_many invocation shape changed".to_owned()));
            }
            let archive_span = script
                .unsupported
                .iter()
                .find(|unsupported| unsupported.span.start_line == 75)
                .map(|unsupported| unsupported.span)
                .ok_or_else(|| fail("inout output archival foreach shape changed".to_owned()))?;
            let erase_spans = [erase_many[0].expansion[0], erase_many[1].expansion[0]];
            script.unsupported.clear();
            PinnedBatch::Inout {
                erase_spans,
                archive_span,
            }
        }
        _ => unreachable!("pinned origin was selected above"),
    };
    Ok(Some(batch))
}

fn require_pinned_shape(
    script: &ScriptManifest,
    contracts: usize,
    assertions: usize,
    comparisons: usize,
    actions: usize,
    unsupported: usize,
) -> Result<(), String> {
    let actual = (
        script.contracts.len(),
        script.assertions.len(),
        script.comparisons.len(),
        script.workflow_actions.len(),
        script.unsupported.len(),
    );
    let expected = (contracts, assertions, comparisons, actions, unsupported);
    if actual != expected
        || !script.bluesim_sequences.is_empty()
        || !script.bluesim_workflows.is_empty()
        || !script.systemc_workflows.is_empty()
        || !script.make_test_data_actions.is_empty()
        || !script.bsc_options_overlays.is_empty()
    {
        return Err(format!(
            "{} lowered shape changed: expected {expected:?}, found {actual:?}",
            script.origin
        ));
    }
    Ok(())
}

fn prefix_check_path(arguments: &mut [String], directory: &str) -> Result<(), String> {
    let Some(path) = arguments.first_mut() else {
        return Err("pinned check has no actual path".to_owned());
    };
    if !is_safe_relative(path) || Path::new(path).components().count() != 1 {
        return Err(format!(
            "pinned check path is not a local basename: {path:?}"
        ));
    }
    *path = format!("{directory}/{path}");
    Ok(())
}

fn audit_pinned_regular_membership(
    fixture_root: &Path,
    relative_directory: &str,
    expected: &[&str],
) -> Result<(), String> {
    if !is_safe_relative(relative_directory) {
        return Err(format!(
            "pinned membership directory is not a safe relative path: {relative_directory:?}"
        ));
    }
    let directory = fixture_root.join(relative_directory);
    let metadata = fs::symlink_metadata(&directory)
        .map_err(|error| format!("inspect pinned membership directory: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "pinned membership path is not a regular directory: {}",
            directory.display()
        ));
    }
    let mut actual = Vec::new();
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("read pinned membership directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("read pinned membership entry: {error}"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "pinned membership contains a non-Unicode name".to_owned())?;
        if !name.ends_with("bsv") {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| format!("inspect pinned member {name:?}: {error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "pinned member is not a regular non-link file: {name:?}"
            ));
        }
        actual.push(name);
    }
    actual.sort();
    let mut folded = BTreeSet::new();
    if actual
        .iter()
        .any(|name| !folded.insert(name.to_ascii_lowercase()))
    {
        return Err("pinned membership contains a case-colliding name".to_owned());
    }
    let expected = expected
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(format!(
            "pinned membership changed: expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn apply_pinned_batch(
    batch: PinnedBatch,
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) -> Result<(), ImportDiagnostic> {
    match batch {
        PinnedBatch::CourseLab { variants } => apply_course_lab_batch(script, assembly, &variants),
        PinnedBatch::Sal {
            setup_span,
            cleanup_span,
        } => apply_sal_batch(script, assembly, setup_span, cleanup_span),
        PinnedBatch::Inout {
            erase_spans,
            archive_span,
        } => apply_inout_batch(script, assembly, erase_spans, archive_span),
    }
}

fn pinned_batch_error(script: &ScriptManifest, message: String) -> ImportDiagnostic {
    let span = script
        .contracts
        .first()
        .map(contract_source_span)
        .unwrap_or_else(empty_manifest_span);
    error_diagnostic("import.pinned_batch", message, span, &[])
}

fn apply_course_lab_batch(
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
    variants: &[(String, ManifestSourceSpan)],
) -> Result<(), ImportDiagnostic> {
    let mut mesa_index = 0usize;
    for scenario in &mut assembly.scenarios {
        let is_mesa = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    &operation.action,
                    Action::BscCompile { source, .. } | Action::BscGenerate { source, .. }
                        if source == "TestMesa.bsv"
                )
            });
        if !is_mesa {
            continue;
        }
        let Some((variant, span)) = variants.get(mesa_index / 2) else {
            return Err(pinned_batch_error(
                script,
                "course_lab produced more than eight TestMesa scenarios".to_owned(),
            ));
        };
        for stage in &mut scenario.stages {
            stage.operations.retain(|operation| {
                !matches!(
                    &operation.action,
                    Action::FsCopy { source, .. } if source.starts_with("sysTestMesa.")
                ) && !matches!(
                    &operation.action,
                    Action::FsMove { source, .. } if source == "sysTestMesa.out.bak"
                )
            });
        }
        let copy = OperationRecord::new(
            Action::FsCopy {
                source: variant.clone(),
                destination: "Mesa.bsv".to_owned(),
            },
            OperationExpectation::Required,
            provenance(*span, &[]),
        );
        scenario
            .stages
            .first_mut()
            .expect("imported scenario has a stage")
            .operations
            .insert(0, copy);
        mesa_index += 1;
    }
    if mesa_index != 8 {
        return Err(pinned_batch_error(
            script,
            format!("course_lab expected eight TestMesa scenarios, found {mesa_index}"),
        ));
    }
    assembly
        .consumed_actions
        .extend(0..script.workflow_actions.len());
    Ok(())
}

fn apply_sal_batch(
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
    setup_span: ManifestSourceSpan,
    cleanup_span: ManifestSourceSpan,
) -> Result<(), ImportDiagnostic> {
    if assembly.compile_scenarios.len() != 16
        || assembly.consumed_assertions.len() != 8
        || assembly.consumed_comparisons.len() != 20
    {
        return Err(pinned_batch_error(
            script,
            format!(
                "SAL producer binding changed: compiles={}, assertions={}, comparisons={}",
                assembly.compile_scenarios.len(),
                assembly.consumed_assertions.len(),
                assembly.consumed_comparisons.len()
            ),
        ));
    }
    let mut requirements = BTreeSet::new();
    let mut operations = SAL_LAMBDA_MEMBERS
        .iter()
        .map(|member| {
            OperationRecord::new(
                Action::FsCopy {
                    source: format!("lambda_calculus/{member}"),
                    destination: format!("sal/{member}"),
                },
                OperationExpectation::Required,
                provenance(setup_span, &[]),
            )
        })
        .collect::<Vec<_>>();
    for contract_index in 0..16 {
        let Some(&scenario_index) = assembly.compile_scenarios.get(&contract_index) else {
            return Err(pinned_batch_error(
                script,
                format!("SAL compile contract {contract_index} has no scenario"),
            ));
        };
        let scenario = &assembly.scenarios[scenario_index];
        requirements.extend(scenario.requires.iter().copied());
        operations.extend(
            scenario
                .stages
                .iter()
                .flat_map(|stage| stage.operations.iter().cloned()),
        );
    }
    operations.extend(SAL_LAMBDA_MEMBERS.iter().map(|member| {
        OperationRecord::new(
            Action::FsEnsureAbsent {
                path: format!("sal/{member}"),
            },
            OperationExpectation::Required,
            provenance(cleanup_span, &[]),
        )
    }));
    assembly.scenarios = vec![Scenario {
        id: "sal-closed-workspace".to_owned(),
        resource: ResourceClass::Normal,
        fixtures: Vec::new(),
        requires: requirements.into_iter().collect(),
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![Stage {
            id: "sal-compile-and-check".to_owned(),
            operations,
        }],
    }];
    assembly.compile_scenarios.clear();
    Ok(())
}

fn apply_inout_batch(
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
    erase_spans: [ManifestSourceSpan; 2],
    archive_span: ManifestSourceSpan,
) -> Result<(), ImportDiagnostic> {
    let episode_indices = assembly
        .scenarios
        .iter()
        .enumerate()
        .filter(|(_, scenario)| scenario.id.starts_with("stateful-simulation-"))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if episode_indices.len() != 2
        || episode_indices.iter().any(|index| {
            assembly.scenarios[*index]
                .stages
                .iter()
                .filter(|stage| {
                    stage
                        .operations
                        .iter()
                        .any(|operation| matches!(operation.action, Action::BscGenerate { .. }))
                })
                .count()
                != 17
        })
    {
        return Err(pinned_batch_error(
            script,
            format!(
                "inout expected two closed 17-simulation episodes, found indices {episode_indices:?}"
            ),
        ));
    }
    for (invocation, scenario_index) in episode_indices.into_iter().enumerate() {
        let scenario = &mut assembly.scenarios[scenario_index];
        scenario.id = if invocation == 0 {
            "inout-no-inline-episode"
        } else {
            "inout-inline-episode"
        }
        .to_owned();
        scenario.stages.insert(
            0,
            Stage {
                id: "erase-bo-closure".to_owned(),
                operations: INOUT_BO_CLOSURE
                    .iter()
                    .map(|path| {
                        OperationRecord::new(
                            Action::FsEnsureAbsent {
                                path: (*path).to_owned(),
                            },
                            OperationExpectation::Required,
                            provenance(erase_spans[invocation], &[]),
                        )
                    })
                    .collect(),
            },
        );
        if invocation == 0 {
            scenario.stages.push(Stage {
                id: "archive-no-inline-outputs".to_owned(),
                operations: INOUT_ARCHIVE_OUTPUTS
                    .iter()
                    .map(|source| {
                        OperationRecord::new(
                            Action::FsMove {
                                source: (*source).to_owned(),
                                destination: format!("{source}.no-inline-inout"),
                            },
                            OperationExpectation::Required,
                            provenance(archive_span, &[]),
                        )
                    })
                    .collect(),
            });
        }
    }
    assembly
        .consumed_actions
        .extend(0..script.workflow_actions.len());
    Ok(())
}

fn append_course_lab_variant_fixtures(scenarios: &mut [Scenario], registered: &BTreeSet<&str>) {
    for scenario in scenarios {
        let variant = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .find_map(|operation| match &operation.action {
                Action::FsCopy {
                    source,
                    destination,
                } if destination == "Mesa.bsv" => Some(source.as_str()),
                _ => None,
            });
        let Some((_, closure)) = COURSE_LAB_VARIANT_CLOSURES
            .iter()
            .find(|(candidate, _)| Some(*candidate) == variant)
        else {
            continue;
        };
        scenario.fixtures.extend(
            COURSE_LAB_COMMON_CLOSURE
                .iter()
                .chain(closure.iter())
                .filter(|path| registered.contains(**path))
                .map(|path| (*path).to_owned()),
        );
        scenario.fixtures.sort();
        scenario.fixtures.dedup();
    }
}

fn enforce_inout_closed_postconditions(
    script: &ScriptManifest,
    scenarios: &mut [Scenario],
) -> Result<(), ImportDiagnostic> {
    let fail = |message| pinned_batch_error(script, message);
    let episode_indices = ["inout-no-inline-episode", "inout-inline-episode"]
        .map(|id| {
            let matches = scenarios
                .iter()
                .enumerate()
                .filter(|(_, scenario)| scenario.id == id)
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [index] => Ok(*index),
                _ => Err(fail(format!(
                    "inout requires exactly one {id} scenario, found {}",
                    matches.len()
                ))),
            }
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    for (invocation, scenario_index) in episode_indices.into_iter().enumerate() {
        let scenario = &mut scenarios[scenario_index];
        let bsc_operations = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .filter(|operation| {
                matches!(
                    operation.action,
                    Action::BscCompile { .. } | Action::BscGenerate { .. } | Action::BscLink { .. }
                )
            })
            .count();
        if bsc_operations != 34 {
            return Err(fail(format!(
                "inout episode {} expected 34 generation/link operations, found {bsc_operations}",
                invocation + 1
            )));
        }
        for operation in scenario
            .stages
            .iter_mut()
            .flat_map(|stage| &mut stage.operations)
        {
            let args = match &mut operation.action {
                Action::BscCompile { args, .. }
                | Action::BscGenerate { args, .. }
                | Action::BscLink { args, .. } => args,
                _ => continue,
            };
            if invocation == 0 {
                if args.first().map(String::as_str) != Some("-no-inline-inout-connect") {
                    args.insert(0, "-no-inline-inout-connect".to_owned());
                }
            } else if args.iter().any(|arg| arg == "-no-inline-inout-connect") {
                return Err(fail(
                    "inout inline episode unexpectedly contains -no-inline-inout-connect"
                        .to_owned(),
                ));
            }
            operation.artifacts = bsc_test_plan::ArtifactContract::for_action(&operation.action);
        }

        let vcd_runs = scenario
            .stages
            .iter()
            .enumerate()
            .flat_map(|(stage_index, stage)| {
                stage.operations.iter().enumerate().filter_map(
                    move |(operation_index, operation)| {
                        matches!(
                            &operation.action,
                            Action::SimulationRun {
                                backend: PlanSimulationBackend::Icarus,
                                executable,
                                vcd: Some(vcd),
                                ..
                            } if executable == "sysSimpleConnect1"
                                && vcd == "sysSimpleConnect1.v.vcd"
                        )
                        .then_some((stage_index, operation_index))
                    },
                )
            })
            .collect::<Vec<_>>();
        let [_, (stage_index, operation_index)] = vcd_runs.as_slice() else {
            return Err(fail(format!(
                "inout episode {} expected two SimpleConnect1 VCD producers, found {}",
                invocation + 1,
                vcd_runs.len()
            )));
        };
        let operation = &mut scenario.stages[*stage_index].operations[*operation_index];
        let provenance = operation.provenance.clone();
        let Action::SimulationRun { vcd, .. } = &mut operation.action else {
            unreachable!("the pinned VCD producer shape was checked above")
        };
        *vcd = Some("dump.vcd".to_owned());
        operation.artifacts = bsc_test_plan::ArtifactContract::for_action(&operation.action);
        scenario.stages[*stage_index].operations.insert(
            *operation_index + 1,
            OperationRecord::new(
                Action::FsMoveReplace {
                    source: "dump.vcd".to_owned(),
                    destination: "sysSimpleConnect1.v.vcd".to_owned(),
                },
                OperationExpectation::Required,
                provenance,
            ),
        );
    }

    let scenario_index = |id: &str| {
        let matches = scenarios
            .iter()
            .enumerate()
            .filter(|(_, scenario)| scenario.id == id)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [index] => Ok(*index),
            _ => Err(fail(format!(
                "inout requires exactly one {id} scenario, found {}",
                matches.len()
            ))),
        }
    };
    let producer_index = scenario_index("compile-45-CheckResets_ArgToIfc_DiffReset")?;
    let stale_index = scenario_index("compile-52-FourInoutBuses")?;

    let assertion_positions =
        scenarios[stale_index]
            .stages
            .iter()
            .enumerate()
            .flat_map(|(stage_index, stage)| {
                stage.operations.iter().enumerate().filter_map(
                    move |(operation_index, operation)| {
                        matches!(
                            &operation.action,
                            Action::AssertTextContains { path, text }
                                if path == "sysArgToIfc.v" && text == "inout  [31 : 0]"
                        )
                        .then_some((stage_index, operation_index))
                    },
                )
            })
            .collect::<Vec<_>>();
    let [(assertion_stage, assertion_index)] = assertion_positions.as_slice() else {
        return Err(fail(format!(
            "inout expected one late sysArgToIfc.v assertion on FourInoutBuses, found {}",
            assertion_positions.len()
        )));
    };
    let assertion = scenarios[stale_index].stages[*assertion_stage]
        .operations
        .remove(*assertion_index);

    let stale_producers =
        scenarios[stale_index]
            .stages
            .iter()
            .enumerate()
            .flat_map(|(stage_index, stage)| {
                stage.operations.iter().enumerate().filter_map(
                    move |(operation_index, operation)| {
                        operation
                            .artifacts
                            .outputs
                            .iter()
                            .any(|path| path == "sysArgToIfc.v")
                            .then_some((stage_index, operation_index))
                    },
                )
            })
            .collect::<Vec<_>>();
    let [(stale_stage, stale_operation)] = stale_producers.as_slice() else {
        return Err(fail(format!(
            "inout expected one false sysArgToIfc.v producer on FourInoutBuses, found {}",
            stale_producers.len()
        )));
    };
    let stale_producer =
        &mut scenarios[stale_index].stages[*stale_stage].operations[*stale_operation];
    if !matches!(
        stale_producer.action,
        Action::BscCompile { ref source, .. } if source == "FourInoutBuses.bsv"
    ) {
        return Err(fail(
            "inout false sysArgToIfc.v owner is not the FourInoutBuses compile".to_owned(),
        ));
    }
    stale_producer
        .artifacts
        .outputs
        .retain(|path| path != "sysArgToIfc.v");

    let real_producers = scenarios[producer_index]
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .filter(|operation| {
            operation
                .artifacts
                .outputs
                .iter()
                .any(|path| path == "sysArgToIfc.v")
        })
        .collect::<Vec<_>>();
    let [real_producer] = real_producers.as_slice() else {
        return Err(fail(format!(
            "inout expected one final real sysArgToIfc.v producer on CheckResets_ArgToIfc_DiffReset, found {}",
            real_producers.len()
        )));
    };
    if !matches!(
        real_producer.action,
        Action::BscCompile { ref source, .. } if source == "CheckResets_ArgToIfc_DiffReset.bsv"
    ) {
        return Err(fail(
            "inout final sysArgToIfc.v owner is not CheckResets_ArgToIfc_DiffReset".to_owned(),
        ));
    }
    scenarios[producer_index]
        .stages
        .last_mut()
        .expect("pinned compile scenario has one stage")
        .operations
        .push(assertion);
    Ok(())
}

fn pinned_interra_operator_suite(script: &ScriptManifest) -> Option<InterraOperatorSuite> {
    match (script.origin.as_str(), script.source_sha256.as_str()) {
        (
            "testsuite/bsc.interra/operators/Arith/arith.exp",
            "a47a094161752aacdd527b73b296c5eb9013bd353ed7ed24d043e65f9fa6dde5",
        ) => Some(InterraOperatorSuite::Arith),
        (
            "testsuite/bsc.interra/operators/BitSel/bitsel.exp",
            "2586f74c5b7ac835a878aed0b3019a5d03a2c3b0dcaec039560c724808fba039",
        ) => Some(InterraOperatorSuite::BitSel),
        (
            "testsuite/bsc.interra/operators/Logic/logic.exp",
            "6dfc136ad7aadc9e6408399e2c0d5430f29b1454a96f65261951faf62d892f3a",
        ) => Some(InterraOperatorSuite::Logic),
        _ => None,
    }
}

fn inject_interra_operator_vectors(script: &mut ScriptManifest, assembly: &mut PlanAssembly) {
    let Some(suite) = pinned_interra_operator_suite(script) else {
        return;
    };
    let expected_unsupported = ["make_pass", "set", "set", "verbose", "note"];
    if script.unsupported.len() != expected_unsupported.len()
        || !script
            .unsupported
            .iter()
            .zip(expected_unsupported)
            .all(|(unsupported, expected)| {
                unsupported.command.as_deref() == Some(expected) && unsupported.span.start_line <= 5
            })
    {
        return;
    }
    let transfer_indices = script
        .workflow_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| match action {
            WorkflowAction::TransferArtifact(transfer)
                if transfer.operation == ArtifactTransferOperation::Copy
                    && transfer.source.trim_start_matches("./") == "generate/Vectors.bsv"
                    && transfer.destination.trim_start_matches("./") == "Vectors.bsv" =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let [transfer_index] = transfer_indices.as_slice() else {
        return;
    };
    let transfer = &script.workflow_actions[*transfer_index];
    let mut operation = OperationRecord::new(
        Action::InterraOperatorVectors { suite },
        OperationExpectation::Required,
        provenance(action_span(transfer), action_expansion(transfer)),
    );
    operation.requires = vec![Requirement::Verilog, Requirement::Icarus];
    for scenario in &mut assembly.scenarios {
        let Some(stage) = scenario.stages.first_mut() else {
            continue;
        };
        stage.operations.insert(0, operation.clone());
    }
    assembly.consumed_actions.insert(*transfer_index);
    script.unsupported.clear();
}

fn inject_make_test_data_actions(
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
    diagnostics: &mut Vec<ImportDiagnostic>,
) {
    for action in &script.make_test_data_actions {
        let order = execution_order_key(action.span, &action.expansion);
        if let Err(message) = collect_make_test_data_requirements(action, assembly, &order) {
            diagnostics.push(error_diagnostic(
                "import.make_test_data",
                message,
                action.span,
                &action.expansion,
            ));
            continue;
        }
        for scenario in &mut assembly.scenarios {
            if scenario_start_order(scenario).is_some_and(|start| order < start) {
                let operation = OperationRecord::new(
                    Action::MakeTestData,
                    OperationExpectation::Required,
                    provenance(action.span, &action.expansion),
                );
                if let Some(stage) = scenario.stages.first_mut() {
                    stage.operations.insert(0, operation);
                }
            }
        }
    }
}

fn collect_make_test_data_requirements(
    action: &MakeTestDataAction,
    assembly: &mut PlanAssembly,
    order: &ExecutionOrderKey,
) -> Result<(), String> {
    let mut requirements = BTreeSet::new();
    collect_requirements(&action.guard, &mut requirements)?;
    for scenario in &mut assembly.scenarios {
        if scenario_start_order(scenario).is_some_and(|start| order < &start) {
            scenario.requires.extend(requirements.iter().copied());
            scenario.requires.sort();
            scenario.requires.dedup();
        }
    }
    Ok(())
}

fn activate_proven_capability_disjunction_assertions(
    script: &mut ScriptManifest,
    assembly: &PlanAssembly,
) {
    let mut resolved_spans = Vec::new();
    for assertion in &mut script.assertions {
        let Guard::UnsupportedExpression { source, span } = &assertion.guard else {
            continue;
        };
        let condition = source
            .trim()
            .strip_prefix('{')
            .and_then(|value| value.strip_suffix('}'))
            .unwrap_or(source)
            .split_whitespace()
            .collect::<String>();
        if condition != "$ctest||$vtest"
            || assertion.helper != "find_regexp"
            || assertion.arguments.len() != 2
        {
            continue;
        }
        let path = normalize_path(&assertion.arguments[0]);
        let assertion_order = execution_order_key(assertion.span, &assertion.expansion);
        let producers = assembly
            .scenarios
            .iter()
            .filter(|scenario| {
                scenario.requires.contains(&Requirement::Bluesim)
                    && scenario.requires.contains(&Requirement::Verilog)
            })
            .filter_map(|scenario| artifact_producer_order(scenario, &path))
            .filter(|order| order < &assertion_order)
            .collect::<Vec<_>>();
        if producers.len() == 1 {
            resolved_spans.push(*span);
            assertion.guard = Guard::Always;
        }
    }
    script.unsupported.retain(|unsupported| {
        unsupported.command.as_deref() != Some("if")
            || unsupported.reason != UnsupportedReason::UnsupportedControlFlow
            || !resolved_spans.contains(&unsupported.span)
    });
}

fn compose_persistent_c_object_builds(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    for scenario in &mut assembly.scenarios {
        let Some(start) = scenario_start_order(scenario) else {
            continue;
        };
        let required_inputs = scenario_external_inputs(scenario);
        let mut active = BTreeMap::<String, (usize, &WorkflowAction)>::new();
        let mut events = script
            .workflow_actions
            .iter()
            .enumerate()
            .filter(|(_, action)| {
                execution_order_key(action_span(action), action_expansion(action)) < start
            })
            .collect::<Vec<_>>();
        events.sort_by_key(|(_, action)| {
            execution_order_key(action_span(action), action_expansion(action))
        });
        for (index, action) in events {
            match action {
                WorkflowAction::BuildCObject(build) => {
                    active.insert(normalize_path(&build.output), (index, action));
                }
                WorkflowAction::TransferArtifact(transfer) => {
                    active.remove(&normalize_path(&transfer.destination));
                    if transfer.operation == ArtifactTransferOperation::Move {
                        active.remove(&normalize_path(&transfer.source));
                    }
                }
                WorkflowAction::EraseArtifact(action) => {
                    active.remove(&normalize_path(&action.path));
                }
                WorkflowAction::TouchArtifact(action) => {
                    active.remove(&normalize_path(&action.path));
                }
                _ => {}
            }
        }
        let mut operations = Vec::new();
        for (output, (index, action)) in active {
            if !required_inputs.contains(&output)
                || !guard_applies_to_scenario(action.guard(), scenario)
            {
                continue;
            }
            let Ok(operation) = map_action(action) else {
                continue;
            };
            operations.push((operation_order(&operation), operation));
            assembly.consumed_actions.insert(index);
        }
        operations.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some(stage) = scenario.stages.first_mut() {
            for (_, operation) in operations.into_iter().rev() {
                stage.operations.insert(0, operation);
            }
        }
    }
}

#[derive(Clone)]
struct ActiveFixtureAlias<'a> {
    action_index: usize,
    action: &'a WorkflowAction,
    source: String,
    destination: String,
}

fn compose_persistent_fixture_aliases(
    script: &ScriptManifest,
    fixture_root: &Path,
    assembly: &mut PlanAssembly,
) {
    for scenario in &mut assembly.scenarios {
        let Some(start) = scenario_start_order(scenario) else {
            continue;
        };
        let mut active = BTreeMap::<String, ActiveFixtureAlias<'_>>::new();
        let mut events = script
            .workflow_actions
            .iter()
            .enumerate()
            .filter(|(_, action)| {
                execution_order_key(action_span(action), action_expansion(action)) < start
            })
            .collect::<Vec<_>>();
        events.sort_by_key(|(_, action)| {
            execution_order_key(action_span(action), action_expansion(action))
        });
        for (index, action) in events {
            match action {
                WorkflowAction::TransferArtifact(transfer) => {
                    let source = normalize_path(&transfer.source);
                    let destination = normalize_path(&transfer.destination);
                    active.remove(&destination);
                    if transfer.operation == ArtifactTransferOperation::Move {
                        active.remove(&source);
                    }
                    if is_checked_in_fixture_alias(transfer, fixture_root) {
                        active.insert(
                            destination.clone(),
                            ActiveFixtureAlias {
                                action_index: index,
                                action,
                                source,
                                destination,
                            },
                        );
                    }
                }
                WorkflowAction::EraseArtifact(action) => {
                    active.remove(&normalize_path(&action.path));
                }
                WorkflowAction::TouchArtifact(action) => {
                    active.remove(&normalize_path(&action.path));
                }
                _ => {}
            }
        }
        let required_inputs = scenario_external_inputs(scenario);
        let needed = required_fixture_aliases(fixture_root, &required_inputs, &active);
        let mut aliases = active
            .into_values()
            .filter(|alias| {
                needed.contains(&alias.destination)
                    && guard_applies_to_scenario(alias.action.guard(), scenario)
            })
            .collect::<Vec<_>>();
        aliases.sort_by_key(|alias| {
            execution_order_key(action_span(alias.action), action_expansion(alias.action))
        });
        if let Some(stage) = scenario.stages.first_mut() {
            for alias in aliases.into_iter().rev() {
                let Ok(operation) = map_action(alias.action) else {
                    continue;
                };
                stage.operations.insert(0, operation);
                assembly.consumed_actions.insert(alias.action_index);
            }
        }
    }
}

fn is_checked_in_fixture_alias(
    transfer: &crate::model::ArtifactTransferAction,
    fixture_root: &Path,
) -> bool {
    if transfer.operation != ArtifactTransferOperation::Copy {
        return false;
    }
    let source = normalize_path(&transfer.source);
    let destination = normalize_path(&transfer.destination);
    if !is_safe_relative(&source)
        || !is_safe_relative(&destination)
        || source.strip_suffix(".keep") != Some(destination.as_str())
        || fs::symlink_metadata(fixture_root.join(&destination)).is_ok()
    {
        return false;
    }
    fs::symlink_metadata(fixture_root.join(source))
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn guard_applies_to_scenario(guard: &Guard, scenario: &Scenario) -> bool {
    let mut requirements = BTreeSet::new();
    collect_requirements(guard, &mut requirements).is_ok()
        && requirements
            .iter()
            .all(|requirement| scenario.requires.contains(requirement))
}

fn scenario_external_inputs(scenario: &Scenario) -> BTreeSet<String> {
    let mut produced = BTreeSet::new();
    let mut inputs = BTreeSet::new();
    for operation in scenario.stages.iter().flat_map(|stage| &stage.operations) {
        for input in &operation.artifacts.inputs {
            let input = normalize_path(input);
            if !produced.contains(&input) {
                inputs.insert(input);
            }
        }
        for removed in &operation.artifacts.removes {
            produced.remove(&normalize_path(removed));
        }
        produced.extend(
            operation
                .artifacts
                .outputs
                .iter()
                .map(|path| normalize_path(path)),
        );
        produced.extend(
            operation
                .artifacts
                .output_alternatives
                .iter()
                .flatten()
                .map(|path| normalize_path(path)),
        );
    }
    inputs
}

fn required_fixture_aliases(
    fixture_root: &Path,
    roots: &BTreeSet<String>,
    aliases: &BTreeMap<String, ActiveFixtureAlias<'_>>,
) -> BTreeSet<String> {
    let include = Regex::new(r#"(?m)^\s*#\s*include\s*\"([^\"\r\n]+)\""#)
        .expect("valid literal C include regex");
    let mut needed = roots
        .intersection(&aliases.keys().cloned().collect())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut pending = roots.iter().cloned().collect::<VecDeque<_>>();
    let mut visited = BTreeSet::new();
    while let Some(logical_path) = pending.pop_front() {
        if !visited.insert(logical_path.clone()) || !is_native_source_or_header(&logical_path) {
            continue;
        }
        let physical_path = aliases
            .get(&logical_path)
            .map_or(logical_path.as_str(), |alias| alias.source.as_str());
        let absolute = fixture_root.join(physical_path);
        let Ok(metadata) = fs::symlink_metadata(&absolute) else {
            continue;
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&absolute) else {
            continue;
        };
        for capture in include.captures_iter(&contents) {
            let Some(path) = resolve_local_c_include(
                &logical_path,
                capture.get(1).expect("include capture").as_str(),
            ) else {
                continue;
            };
            if aliases.contains_key(&path) {
                needed.insert(path.clone());
                pending.push_back(path);
            } else if fs::symlink_metadata(fixture_root.join(&path))
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
            {
                pending.push_back(path);
            }
        }
    }
    needed
}

fn is_native_source_or_header(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx"
            )
        })
}

fn resolve_local_c_include(including: &str, included: &str) -> Option<String> {
    let joined = Path::new(including)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(included);
    is_safe_relative(&unix_path(&joined)).then(|| unix_path(&joined))
}

fn apply_bsc_options_overlays(
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
    diagnostics: &mut Vec<ImportDiagnostic>,
) {
    for overlay in &script.bsc_options_overlays {
        let start = execution_order_key(overlay.start, &[]);
        let end = execution_order_key(overlay.end, &[]);
        let mut covered = 0;
        for scenario in &mut assembly.scenarios {
            let Some(scenario_start) = scenario_start_order(scenario) else {
                continue;
            };
            let Some(scenario_end) = scenario_end_order(scenario) else {
                continue;
            };
            if start < scenario_start && scenario_end < end {
                scenario.bsc_options_append = Some(overlay.append.clone());
                covered += 1;
            } else if scenario_start < end && start < scenario_end {
                diagnostics.push(error_diagnostic(
                    "import.bsc_options_overlay",
                    "a BSC_OPTIONS overlay only supports scenarios wholly contained by its static save/append/restore scope".to_owned(),
                    overlay.start,
                    &[],
                ));
            }
        }
        if covered == 0 {
            diagnostics.push(error_diagnostic(
                "import.bsc_options_overlay",
                "a BSC_OPTIONS overlay did not enclose an executable scenario".to_owned(),
                overlay.start,
                &[],
            ));
        }
    }
}

struct CompileChainNode<'a> {
    contract_index: usize,
    scenario_index: usize,
    contract: &'a CompileContract,
    shape: CompileShape,
    dependencies: BTreeSet<String>,
}

struct CompileChainLink {
    left_scenario: usize,
    right_scenario: usize,
    transitions: Vec<(usize, OperationRecord)>,
}

struct CompileChainGroup {
    first_contract_index: usize,
    members: Vec<usize>,
    links: Vec<CompileChainLink>,
}

fn compose_stateful_simulation_episodes(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    let mut episodes = BTreeMap::<ExecutionOrderKey, Vec<(usize, usize)>>::new();
    let mut top_level_windows =
        BTreeMap::<ExecutionOrderKey, (ExecutionOrderKey, ExecutionOrderKey)>::new();
    for (&contract_index, &scenario_index) in &assembly.simulation_scenarios {
        let Some(Contract::Simulation(contract)) = script.contracts.get(contract_index) else {
            continue;
        };
        let Some(key) = simulation_episode_key(contract.span, &contract.expansion) else {
            continue;
        };
        episodes
            .entry(key)
            .or_default()
            .push((contract_index, scenario_index));
    }
    let simulations = assembly
        .simulation_scenarios
        .iter()
        .filter_map(|(&contract_index, &scenario_index)| {
            let Contract::Simulation(contract) = script.contracts.get(contract_index)? else {
                return None;
            };
            contract
                .expansion
                .is_empty()
                .then_some((contract_index, scenario_index, contract))
        })
        .collect::<Vec<_>>();
    let mut pair_index = 0;
    while pair_index + 1 < simulations.len() {
        let (left_index, left_scenario, left) = simulations[pair_index];
        let (right_index, right_scenario, right) = simulations[pair_index + 1];
        if left.source != right.source {
            pair_index += 1;
            continue;
        }
        let after = contract_order_key(&script.contracts[left_index]);
        let before = contract_order_key(&script.contracts[right_index]);
        let actions = script
            .workflow_actions
            .iter()
            .filter(|action| {
                let order = execution_order_key(action_span(action), action_expansion(action));
                after < order && order < before
            })
            .collect::<Vec<_>>();
        if actions.is_empty()
            || !actions
                .iter()
                .any(|action| matches!(action, WorkflowAction::TransferArtifact(_)))
            || actions.iter().any(|action| {
                !matches!(
                    action,
                    WorkflowAction::TransferArtifact(_) | WorkflowAction::EraseArtifact(_)
                )
            })
        {
            pair_index += 1;
            continue;
        }
        let key = ExecutionOrderKey(vec![usize::MAX, left_index]);
        top_level_windows.insert(key.clone(), (after, before));
        episodes.insert(
            key,
            vec![(left_index, left_scenario), (right_index, right_scenario)],
        );
        pair_index += 2;
    }

    let mut replacements = BTreeMap::<usize, (BTreeSet<usize>, Scenario, BTreeSet<usize>)>::new();
    for (episode_key, mut members) in episodes {
        members.sort_by_key(|(contract_index, _)| *contract_index);
        if members.len() < 2 {
            continue;
        }
        let scenario_indices = members
            .iter()
            .map(|(_, scenario_index)| *scenario_index)
            .collect::<BTreeSet<_>>();
        let top_level_window = top_level_windows.get(&episode_key);
        let actions = script
            .workflow_actions
            .iter()
            .enumerate()
            .filter(|(_, action)| {
                let order = execution_order_key(action_span(action), action_expansion(action));
                top_level_window.map_or_else(
                    || {
                        simulation_episode_key(action_span(action), action_expansion(action))
                            .as_ref()
                            == Some(&episode_key)
                    },
                    |(after, before)| after < &order && &order < before,
                )
            })
            .collect::<Vec<_>>();
        if actions.is_empty()
            || script.unsupported.iter().any(|unsupported| {
                simulation_episode_key(unsupported.span, &unsupported.expansion).as_ref()
                    == Some(&episode_key)
            })
            || script
                .assertions
                .iter()
                .enumerate()
                .any(|(index, assertion)| {
                    !assembly.consumed_assertions.contains(&index)
                        && simulation_episode_key(assertion.span, &assertion.expansion).as_ref()
                            == Some(&episode_key)
                })
            || script
                .comparisons
                .iter()
                .enumerate()
                .any(|(index, comparison)| {
                    !assembly.consumed_comparisons.contains(&index)
                        && simulation_episode_key(comparison.span, &comparison.expansion).as_ref()
                            == Some(&episode_key)
                })
        {
            continue;
        }

        let mut transitions = Vec::new();
        let mut valid = true;
        for (action_index, action) in actions {
            let (source, destination, action_order, mut operation) = match action {
                WorkflowAction::TransferArtifact(transfer)
                    if is_safe_relative(&normalize_path(&transfer.source))
                        && is_safe_relative(&normalize_path(&transfer.destination)) =>
                {
                    (
                        normalize_path(&transfer.source),
                        Some(normalize_path(&transfer.destination)),
                        execution_order_key(transfer.span, &transfer.expansion),
                        OperationRecord::new(
                            map_transfer(transfer),
                            OperationExpectation::Required,
                            provenance(transfer.span, &transfer.expansion),
                        ),
                    )
                }
                WorkflowAction::EraseArtifact(erase)
                    if is_safe_relative(&normalize_path(&erase.path)) =>
                {
                    (
                        normalize_path(&erase.path),
                        None,
                        execution_order_key(erase.span, &erase.expansion),
                        OperationRecord::new(
                            map_erase(erase, EraseMode::RequirePresent),
                            OperationExpectation::Required,
                            provenance(erase.span, &erase.expansion),
                        ),
                    )
                }
                _ => {
                    valid = false;
                    break;
                }
            };
            let producers = scenario_indices
                .iter()
                .filter_map(|scenario_index| {
                    let scenario = &assembly.scenarios[*scenario_index];
                    artifact_producer_order(scenario, &source)
                        .filter(|order| order < &action_order)
                        .map(|order| (*scenario_index, order))
                })
                .collect::<Vec<_>>();
            let optional_cleanup = destination.is_none() && producers.is_empty();
            if (!optional_cleanup && producers.len() != 1)
                || destination.as_ref().is_some_and(|destination| {
                    scenario_indices.iter().any(|scenario_index| {
                        artifact_producer_order(&assembly.scenarios[*scenario_index], destination)
                            .is_some_and(|order| order < action_order)
                    })
                })
            {
                valid = false;
                break;
            }
            if optional_cleanup {
                let WorkflowAction::EraseArtifact(erase) = action else {
                    unreachable!("only erase actions can be optional cleanup")
                };
                operation = OperationRecord::new(
                    map_erase(erase, EraseMode::EnsureAbsent),
                    OperationExpectation::Required,
                    provenance(erase.span, &erase.expansion),
                );
            }
            transitions.push((action_order, action_index, operation));
        }
        if !valid {
            continue;
        }

        let mut scenarios = scenario_indices
            .iter()
            .map(|index| assembly.scenarios[*index].clone())
            .collect::<Vec<_>>();
        let Some(first) = scenarios.first() else {
            continue;
        };
        let first_id = first.id.clone();
        let timeouts = first.timeouts.clone();
        if scenarios
            .iter()
            .any(|scenario| scenario.timeouts != timeouts)
        {
            continue;
        }
        let erase_actions = transitions
            .iter()
            .filter_map(|(order, _, operation)| match &operation.action {
                Action::FsRemove { path } => Some((order, path)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut represented_actions = BTreeSet::new();
        for scenario in &mut scenarios {
            for operation in scenario
                .stages
                .iter_mut()
                .flat_map(|stage| &mut stage.operations)
            {
                let order = operation_order(operation);
                let Action::FsEnsureAbsent { path } = &operation.action else {
                    continue;
                };
                let path = path.clone();
                let provenance = operation.provenance.clone();
                if !erase_actions
                    .iter()
                    .any(|(erase_order, erase_path)| &order == *erase_order && path == **erase_path)
                {
                    continue;
                }
                *operation = OperationRecord::new(
                    Action::FsRemove { path: path.clone() },
                    OperationExpectation::Required,
                    provenance,
                );
                if let Some((_, action_index, _)) = transitions.iter().find(|(erase_order, _, transition)| {
                    erase_order == &order
                        && matches!(&transition.action, Action::FsRemove { path: erase_path } if erase_path == &path)
                }) {
                    represented_actions.insert(*action_index);
                }
            }
        }
        for (_, action_index, operation) in &transitions {
            if scenarios
                .iter()
                .any(|scenario| scenario_contains_operation(scenario, operation))
            {
                represented_actions.insert(*action_index);
            }
        }
        let mut requirements = BTreeSet::new();
        let mut resource = ResourceClass::Normal;
        let mut stages = Vec::<(ExecutionOrderKey, Stage)>::new();
        for scenario in &scenarios {
            requirements.extend(scenario.requires.iter().copied());
            if scenario.resource == ResourceClass::Heavy {
                resource = ResourceClass::Heavy;
            }
            for stage in &scenario.stages {
                let Some(order) = stage.operations.iter().map(operation_order).min() else {
                    valid = false;
                    break;
                };
                stages.push((order, stage.clone()));
            }
        }
        if !valid {
            continue;
        }
        for (order, action_index, operation) in &transitions {
            if represented_actions.contains(action_index) {
                continue;
            }
            stages.push((
                order.clone(),
                Stage {
                    id: match operation.action {
                        Action::FsCopy { .. } => "state-copy",
                        Action::FsMove { .. } => "state-move",
                        Action::FsRemove { .. } => "state-remove",
                        _ => unreachable!("episode transitions are filesystem actions"),
                    }
                    .to_owned(),
                    operations: vec![operation.clone()],
                },
            ));
        }
        stages.sort_by(|left, right| left.0.cmp(&right.0));
        let mut stages = stages
            .into_iter()
            .map(|(_, stage)| stage)
            .collect::<Vec<_>>();
        uniquify_stage_ids(&mut stages);
        let first_index = *scenario_indices.first().expect("episode has scenarios");
        replacements.insert(
            first_index,
            (
                scenario_indices,
                Scenario {
                    id: format!("stateful-simulation-{first_id}"),
                    resource,
                    fixtures: Vec::new(),
                    requires: requirements.into_iter().collect(),
                    bsc_options_append: None,
                    timeouts,
                    stages,
                },
                transitions.into_iter().map(|(_, index, _)| index).collect(),
            ),
        );
    }
    if replacements.is_empty() {
        return;
    }
    let mut scenarios = std::mem::take(&mut assembly.scenarios)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    for (first_index, (members, merged, actions)) in replacements {
        for member in &members {
            scenarios[*member] = None;
        }
        scenarios[first_index] = Some(merged);
        assembly.consumed_actions.extend(actions);
    }
    let mut remapped = BTreeMap::new();
    assembly.scenarios = scenarios
        .into_iter()
        .enumerate()
        .filter_map(|(old_index, scenario)| {
            scenario.map(|scenario| {
                let new_index = remapped.len();
                remapped.insert(old_index, new_index);
                scenario
            })
        })
        .collect();
    assembly.compile_scenarios.retain(|_, scenario_index| {
        let Some(new_index) = remapped.get(scenario_index).copied() else {
            return false;
        };
        *scenario_index = new_index;
        true
    });
    assembly.simulation_scenarios.clear();
}

fn simulation_episode_key(
    span: ManifestSourceSpan,
    expansion: &[ManifestSourceSpan],
) -> Option<ExecutionOrderKey> {
    expansion
        .first()
        .copied()
        .map(|invocation| execution_order_key(invocation, &[]))
        .or_else(|| (span.start_byte != 0).then(|| execution_order_key(span, &[])))
}

fn compose_ordered_repeated_bluesim_episodes(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    let mut groups = BTreeMap::<(String, String), Vec<usize>>::new();
    for (scenario_index, scenario) in assembly.scenarios.iter().enumerate() {
        let generations = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::BscGenerate {
                    source,
                    mode: SimulationGenerationMode::Bluesim,
                    ..
                } => Some(source.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let links = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::BscLink {
                    backend: PlanSimulationBackend::Bluesim,
                    top,
                    ..
                } => Some(top.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if generations.len() != 1 || links.len() != 1 {
            continue;
        }
        let source = generations.iter().next().expect("one generation").clone();
        let top = links.iter().next().expect("one link").clone();
        groups
            .entry((source, top))
            .or_default()
            .push(scenario_index);
    }

    for ((source, top), mut scenario_indices) in groups {
        if scenario_indices.len() < 2 {
            continue;
        }
        scenario_indices.sort_by_key(|index| scenario_start_order(&assembly.scenarios[*index]));
        let Some(first_order) = scenario_start_order(&assembly.scenarios[scenario_indices[0]])
        else {
            continue;
        };
        let Some(last_order) = scenario_end_order(
            &assembly.scenarios[*scenario_indices.last().expect("repeated scenarios")],
        ) else {
            continue;
        };
        let in_window = |order: ExecutionOrderKey| first_order <= order && order <= last_order;
        let pending_actions = script
            .workflow_actions
            .iter()
            .enumerate()
            .filter(|(index, action)| {
                !assembly.consumed_actions.contains(index)
                    && in_window(execution_order_key(
                        action_span(action),
                        action_expansion(action),
                    ))
            })
            .collect::<Vec<_>>();
        if pending_actions.is_empty()
            || !pending_actions
                .iter()
                .any(|(_, action)| matches!(action, WorkflowAction::Delay(_)))
            || pending_actions.iter().any(|(_, action)| {
                !matches!(
                    action,
                    WorkflowAction::Delay(_) | WorkflowAction::TouchArtifact(_)
                )
            })
            || script.unsupported.iter().any(|unsupported| {
                in_window(execution_order_key(
                    unsupported.span,
                    &unsupported.expansion,
                ))
            })
            || script
                .assertions
                .iter()
                .enumerate()
                .any(|(index, assertion)| {
                    !assembly.consumed_assertions.contains(&index)
                        && in_window(execution_order_key(assertion.span, &assertion.expansion))
                })
            || script
                .comparisons
                .iter()
                .enumerate()
                .any(|(index, comparison)| {
                    !assembly.consumed_comparisons.contains(&index)
                        && in_window(execution_order_key(comparison.span, &comparison.expansion))
                })
        {
            continue;
        }
        let scenarios = scenario_indices
            .iter()
            .map(|index| &assembly.scenarios[*index])
            .collect::<Vec<_>>();
        let first = scenarios[0];
        if scenarios.iter().any(|scenario| {
            scenario.resource != first.resource
                || scenario.requires != first.requires
                || scenario.timeouts != first.timeouts
                || scenario.bsc_options_append != first.bsc_options_append
        }) {
            continue;
        }

        let mut ordered_operations = scenarios
            .iter()
            .flat_map(|scenario| scenario.stages.iter())
            .flat_map(|stage| stage.operations.iter().cloned())
            .map(|operation| (operation_order(&operation), operation))
            .collect::<Vec<_>>();
        let mut consumed = Vec::new();
        let mut valid = true;
        for (action_index, action) in &pending_actions {
            match map_action(action) {
                Ok(mut operation) => {
                    let mut action_requirements = BTreeSet::new();
                    if collect_check_requirements(
                        action.guard(),
                        &mut action_requirements,
                        &mut operation.requires,
                    )
                    .is_err()
                        || !action_requirements
                            .iter()
                            .all(|requirement| first.requires.contains(requirement))
                    {
                        valid = false;
                        break;
                    }
                    ordered_operations.push((
                        execution_order_key(action_span(action), action_expansion(action)),
                        operation,
                    ));
                    consumed.push(*action_index);
                }
                Err(_) => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid {
            continue;
        }
        ordered_operations.sort_by(|left, right| left.0.cmp(&right.0));
        let fixtures = scenarios
            .iter()
            .flat_map(|scenario| scenario.fixtures.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let merged = Scenario {
            id: format!("stateful-bluesim-{top}"),
            resource: first.resource,
            fixtures,
            requires: first.requires.clone(),
            bsc_options_append: first.bsc_options_append.clone(),
            timeouts: first.timeouts.clone(),
            stages: vec![Stage {
                id: format!("ordered-{source}"),
                operations: ordered_operations
                    .into_iter()
                    .map(|(_, operation)| operation)
                    .collect(),
            }],
        };

        let members = scenario_indices.iter().copied().collect::<BTreeSet<_>>();
        let first_index = *members.first().expect("repeated Bluesim scenarios");
        let old_scenarios = std::mem::take(&mut assembly.scenarios);
        let mut remapped = BTreeMap::new();
        assembly.scenarios = old_scenarios
            .into_iter()
            .enumerate()
            .filter_map(|(old_index, scenario)| {
                if old_index == first_index {
                    let new_index = remapped.len();
                    remapped.insert(old_index, new_index);
                    Some(merged.clone())
                } else if members.contains(&old_index) {
                    None
                } else {
                    let new_index = remapped.len();
                    remapped.insert(old_index, new_index);
                    Some(scenario)
                }
            })
            .collect();
        let merged_index = remapped[&first_index];
        for scenario_index in assembly.compile_scenarios.values_mut() {
            if members.contains(scenario_index) {
                *scenario_index = merged_index;
            } else if let Some(new_index) = remapped.get(scenario_index) {
                *scenario_index = *new_index;
            }
        }
        for scenario_index in assembly.simulation_scenarios.values_mut() {
            if members.contains(scenario_index) {
                *scenario_index = merged_index;
            } else if let Some(new_index) = remapped.get(scenario_index) {
                *scenario_index = *new_index;
            }
        }
        assembly.consumed_actions.extend(consumed);
        return;
    }
}

fn compose_missing_bug_golden_xfails(fixture_root: &Path, assembly: &mut PlanAssembly) {
    let mut missing = BTreeSet::new();
    for operation in assembly
        .scenarios
        .iter_mut()
        .flat_map(|scenario| &mut scenario.stages)
        .flat_map(|stage| &mut stage.operations)
    {
        let OperationExpectation::Xfail { reason } = &operation.expectation else {
            continue;
        };
        let Action::AssertGolden { actual, expected } = &operation.action else {
            continue;
        };
        if !reason.starts_with("upstream bug ")
            || !matches!(
                fs::symlink_metadata(fixture_root.join(expected)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        {
            continue;
        }
        let action = Action::AssertGoldenMissingXfail {
            actual: actual.clone(),
            expected: expected.clone(),
            reason: reason.clone(),
        };
        missing.insert(expected.clone());
        operation.action = action;
        operation.artifacts = bsc_test_plan::ArtifactContract::for_action(&operation.action);
        operation.expectation = OperationExpectation::Required;
    }
    for path in missing {
        assembly.golden_paths.remove(&path);
    }
}

fn compose_b1595_workspace_episodes(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    const ORIGIN: &str = "testsuite/bsc.bugs/bluespec_inc/b1595/b1595.exp";
    const SHA256: &str = "916d217320503b58d8e94a57b0e7b8b81929b6680ba21faa51b3776f2e2d6854";
    if script.origin != ORIGIN
        || script.source_sha256 != SHA256
        || !script.contracts.is_empty()
        || !script.assertions.is_empty()
        || !script.unsupported.is_empty()
        || script.bluesim_workflows.len() != 1
        || script.comparisons.len() != 2
        || script.workflow_actions.len() != 9
    {
        return;
    }
    let workflow = &script.bluesim_workflows[0];
    if workflow.top != "mkTbGCD"
        || workflow.generations.len() != 1
        || workflow.generations[0].source != "TbGCD.bsv"
        || workflow.link.top != "mkTbGCD"
        || workflow.link.options != "-p libdir1:+:libdir2"
        || workflow.link.expected_exit != ExpectedExit::Success
        || !workflow.runs.is_empty()
        || !matches!(
            workflow.generations[0].guard,
            Guard::Capability {
                capability: Capability::Bluesim
            }
        )
        || !matches!(
            workflow.link.guard,
            Guard::Capability {
                capability: Capability::Bluesim
            }
        )
        || !matches!(script.comparisons[0].arguments.as_slice(), [actual] if actual == "mkTbGCD.bsc-ccomp-out")
        || !matches!(script.comparisons[1].arguments.as_slice(), [actual] if actual == "mkWrongTop.bsc-ccomp-out")
    {
        return;
    }
    let expected_actions = [
        "mkdir",
        "mkdir",
        "copy",
        "copy",
        "chmod_u_minus_r",
        "compile_object_pass",
        "compile_object_pass",
        "move",
        "link_objects_fail",
    ];
    if script
        .workflow_actions
        .iter()
        .map(WorkflowAction::helper_name)
        .ne(expected_actions)
        || script.workflow_actions.iter().any(|action| {
            !matches!(
                action.guard(),
                Guard::Capability {
                    capability: Capability::Bluesim
                }
            )
        })
    {
        return;
    }
    let exact_actions = matches!(
        script.workflow_actions.as_slice(),
        [
            WorkflowAction::CreateDirectory(first),
            WorkflowAction::CreateDirectory(second),
            WorkflowAction::TransferArtifact(copy_first),
            WorkflowAction::TransferArtifact(copy_second),
            WorkflowAction::RemoveUserRead(unreadable),
            WorkflowAction::CompileObject(wrong_mod),
            WorkflowAction::CompileObject(wrong_top),
            WorkflowAction::TransferArtifact(rename),
            WorkflowAction::LinkObjects(link),
        ] if first.path == "libdir1"
            && second.path == "libdir2"
            && copy_first.operation == ArtifactTransferOperation::Copy
            && copy_first.source == "mkGCD.ba"
            && copy_first.destination == "libdir1"
            && copy_second.operation == ArtifactTransferOperation::Copy
            && copy_second.source == "mkGCD.ba"
            && copy_second.destination == "libdir2"
            && unreadable.path == "libdir1/mkGCD.ba"
            && wrong_mod.source == "WrongMod.bsv"
            && wrong_top.source == "WrongTop.bsv"
            && rename.operation == ArtifactTransferOperation::Move
            && rename.source == "mkWrongMod.ba"
            && rename.destination == "mkRightMod.ba"
            && link.top == "mkWrongTop"
            && link.expected_exit == ExpectedExit::Failure
    );
    if !exact_actions {
        return;
    }

    let matching = assembly
        .scenarios
        .iter()
        .enumerate()
        .filter(|(_, scenario)| {
            scenario.stages.iter().flat_map(|stage| &stage.operations).any(|operation| {
                matches!(&operation.action, Action::BscLink { top, .. } if top == "mkTbGCD")
            })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [first_index] = matching.as_slice() else {
        return;
    };
    let mut first_operations = assembly.scenarios[*first_index]
        .stages
        .iter()
        .flat_map(|stage| stage.operations.iter().cloned())
        .collect::<Vec<_>>();
    for (index, action) in script.workflow_actions[..5].iter().enumerate() {
        let Ok(mut operation) = map_action(action) else {
            return;
        };
        if let Action::FsCopy { destination, .. } = &mut operation.action {
            *destination = if index == 2 {
                "libdir1/mkGCD.ba".to_owned()
            } else if index == 3 {
                "libdir2/mkGCD.ba".to_owned()
            } else {
                return;
            };
            operation.artifacts = bsc_test_plan::ArtifactContract::for_action(&operation.action);
        }
        first_operations.push(operation);
    }
    first_operations.sort_by_key(operation_order);
    for operation in &mut first_operations {
        if matches!(&operation.action, Action::FsRemoveUserRead { .. })
            || matches!(&operation.action, Action::BscLink { top, .. } if top == "mkTbGCD")
            || matches!(
                &operation.action,
                Action::AssertGolden { actual, .. } if actual == "mkTbGCD.bsc-ccomp-out"
            )
        {
            if !operation
                .requires
                .contains(&Requirement::PosixUnreadability)
            {
                operation.requires.push(Requirement::PosixUnreadability);
            }
        }
    }
    assembly.scenarios[*first_index] = Scenario {
        id: "ordered-workspace-unreadable-import".to_owned(),
        resource: ResourceClass::Heavy,
        fixtures: Vec::new(),
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![Stage {
            id: "unreadable-import".to_owned(),
            operations: first_operations,
        }],
    };

    let mut second_operations = Vec::new();
    for (offset, action) in script.workflow_actions[5..].iter().enumerate() {
        let Ok(mut operation) = map_action(action) else {
            return;
        };
        if offset == 2 {
            let Action::FsMove {
                source,
                destination,
            } = operation.action
            else {
                return;
            };
            operation.action = Action::FsMoveReplace {
                source,
                destination,
            };
            operation.artifacts = bsc_test_plan::ArtifactContract::for_action(&operation.action);
        }
        second_operations.push(operation);
    }
    let Ok(comparison) = map_comparison(&script.comparisons[1]) else {
        return;
    };
    second_operations.push(comparison);
    second_operations.sort_by_key(operation_order);
    assembly.scenarios.push(Scenario {
        id: "ordered-workspace-wrong-module".to_owned(),
        resource: ResourceClass::Heavy,
        fixtures: Vec::new(),
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![Stage {
            id: "wrong-module-link".to_owned(),
            operations: second_operations,
        }],
    });
    assembly
        .consumed_actions
        .extend(0..script.workflow_actions.len());
    assembly.consumed_comparisons.extend([0, 1]);
    assembly
        .golden_paths
        .insert("mkWrongTop.bsc-ccomp-out.expected".to_owned());
}

fn compose_cpp_darwin_normalization_episode(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    const ORIGIN: &str = "testsuite/bsc.driver/cpp/cpp.exp";
    const SHA256: &str = "ff0764bdcf5d57315d61c7ed6f1669bd620f4c45f4e38f7b9ca51db413dea3ae";
    if script.origin != ORIGIN
        || script.source_sha256 != SHA256
        || !script.unsupported.is_empty()
        || script.workflow_actions.len() != 2
        || script.comparisons.len() != 1
        || !matches!(
            script.workflow_actions.as_slice(),
            [
                WorkflowAction::RewriteDarwinCppIncludePath(rewrite),
                WorkflowAction::TransferArtifact(rename),
            ] if rewrite.source == "Cpreprocess_line.bsv.bsc-out"
                && rewrite.destination == "Cpreprocess_line.bsv.bsc-out.filtered"
                && rename.operation == ArtifactTransferOperation::Move
                && rename.source == "Cpreprocess_line.bsv.bsc-out.filtered"
                && rename.destination == "Cpreprocess_line.bsv.bsc-out"
                && matches!(rewrite.guard, Guard::Capability { capability: Capability::Darwin })
                && matches!(rename.guard, Guard::Capability { capability: Capability::Darwin })
        )
        || !matches!(
            script.comparisons[0].arguments.as_slice(),
            [actual] if actual == "Cpreprocess_line.bsv.bsc-out"
        )
    {
        return;
    }
    let matching = assembly
        .scenarios
        .iter()
        .enumerate()
        .filter(|(_, scenario)| {
            scenario.stages.iter().flat_map(|stage| &stage.operations).any(|operation| {
                matches!(&operation.action, Action::BscCompile { source, .. } if source == "Cpreprocess_line.bsv")
            })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [scenario_index] = matching.as_slice() else {
        return;
    };
    let mut operations = assembly.scenarios[*scenario_index]
        .stages
        .iter()
        .flat_map(|stage| stage.operations.iter().cloned())
        .collect::<Vec<_>>();
    let [compile] = operations.as_mut_slice() else {
        return;
    };
    if !matches!(
        &compile.action,
        Action::BscCompile {
            source,
            args,
            stdout,
            ..
        } if source == "Cpreprocess_line.bsv"
            && args == &["-cpp"]
            && stdout == "Cpreprocess_line.bsv.bsc-out"
    ) || compile.artifacts.inputs != ["Cpreprocess_line.bsv"]
    {
        return;
    }
    compile.artifacts.inputs.push("more.bsv".to_owned());
    let Ok(mut rewrite) = map_action(&script.workflow_actions[0]) else {
        return;
    };
    rewrite.requires.push(Requirement::Darwin);
    operations.push(rewrite);
    let Ok(mut rename) = map_action(&script.workflow_actions[1]) else {
        return;
    };
    rename.action = match rename.action {
        Action::FsMove {
            source,
            destination,
        } => Action::FsMoveReplace {
            source,
            destination,
        },
        _ => return,
    };
    rename.artifacts = bsc_test_plan::ArtifactContract::for_action(&rename.action);
    rename.requires.push(Requirement::Darwin);
    operations.push(rename);
    let Ok(comparison) = map_comparison(&script.comparisons[0]) else {
        return;
    };
    operations.push(comparison);
    operations.sort_by_key(operation_order);
    assembly.scenarios[*scenario_index].stages = vec![Stage {
        id: "compile-normalize-compare".to_owned(),
        operations,
    }];
    assembly.consumed_actions.extend([0, 1]);
    assembly.consumed_comparisons.insert(0);
    assembly
        .golden_paths
        .insert("Cpreprocess_line.bsv.bsc-out.expected".to_owned());
}

fn compose_pinned_options_typed_episodes(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    if !is_pinned_options_plan(script)
        || script.workflow_actions.len() < 54
        || script.assertions.len() < 24
        || script.comparisons.len() < 10
    {
        return;
    }

    let assertion_operation = |index: usize| map_assertion(&script.assertions[index]).ok();
    let action_operation = |index: usize| map_action(&script.workflow_actions[index]).ok();
    let Some(expected_bluespecdir) = assertion_operation(0) else {
        return;
    };
    let Some(expected_raw_bluespecdir) = assertion_operation(1) else {
        return;
    };
    let Some(touch_dummy) = action_operation(9) else {
        return;
    };
    let Some(create_simfiles) = action_operation(10) else {
        return;
    };
    let mut missing_input_links = Vec::new();
    for index in [48, 49] {
        let WorkflowAction::LinkObjects(link) = &script.workflow_actions[index] else {
            return;
        };
        let Some(diagnostic) = link_error_diagnostic_operation(link).ok().flatten() else {
            return;
        };
        let Some(operation) = action_operation(index) else {
            return;
        };
        missing_input_links.push((link.top.clone(), operation, diagnostic));
    }
    let Some(quiet_iverilog) = action_operation(51) else {
        return;
    };
    let Some(parallel_link) = action_operation(52) else {
        return;
    };
    let Some(parallel_normalize) = action_operation(53) else {
        return;
    };
    let Ok(pre_parallel_comparison) = map_comparison(&script.comparisons[7]) else {
        return;
    };
    let Ok(quiet_iverilog_comparison) = map_comparison(&script.comparisons[8]) else {
        return;
    };
    let Ok(parallel_comparison) = map_comparison(&script.comparisons[9]) else {
        return;
    };
    let bluesim_file_checks = (9..=14)
        .map(assertion_operation)
        .collect::<Option<Vec<_>>>();
    let Some(bluesim_file_checks) = bluesim_file_checks else {
        return;
    };
    let first_vpi_checks = (15..=16)
        .map(assertion_operation)
        .collect::<Option<Vec<_>>>();
    let Some(first_vpi_checks) = first_vpi_checks else {
        return;
    };
    let first_link_checks = (19..=20)
        .map(assertion_operation)
        .collect::<Option<Vec<_>>>();
    let Some(first_link_checks) = first_link_checks else {
        return;
    };
    let second_link_checks = (21..=22)
        .map(assertion_operation)
        .collect::<Option<Vec<_>>>();
    let Some(second_link_checks) = second_link_checks else {
        return;
    };
    let Some(create_incfiles) = action_operation(43) else {
        return;
    };
    let Some(touch_include) = action_operation(44) else {
        return;
    };
    let Some(mut first_link) = action_operation(33) else {
        return;
    };
    let Some(mut second_link) = action_operation(42) else {
        return;
    };
    if !matches!(first_link.action, Action::BscLink { .. })
        || !matches!(second_link.action, Action::BscLink { .. })
    {
        return;
    }
    first_link.artifacts.outputs.extend([
        "vfiles/vpi_wrapper_my_time.o".to_owned(),
        "vfiles_link/vpi_startup_array.o".to_owned(),
    ]);
    second_link.artifacts.outputs.extend([
        "vfiles/vpi_wrapper_my_time.o".to_owned(),
        "vpi_startup_array.o".to_owned(),
    ]);
    let mut between_links = Vec::new();
    for index in 34..=41 {
        let WorkflowAction::EraseArtifact(erase) = &script.workflow_actions[index] else {
            return;
        };
        between_links.push(OperationRecord::new(
            map_erase(erase, EraseMode::EnsureAbsent),
            OperationExpectation::Required,
            provenance(erase.span, &erase.expansion),
        ));
    }

    let first_vpi_index =
        assembly.scenarios.iter().position(|scenario| {
            scenario.stages.iter().flat_map(|stage| &stage.operations).any(|operation| {
            matches!(
                &operation.action,
                Action::BscCompile { source, args, .. }
                    if source == "srcfiles/GCD.bsv" && !args.iter().any(|arg| arg == "-vdir")
            )
        })
        });
    let second_vpi_index = assembly.scenarios.iter().position(|scenario| {
        scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    &operation.action,
                    Action::BscCompile { source, args, .. }
                        if source == "srcfiles/GCD.bsv" && args.iter().any(|arg| arg == "-vdir")
                )
            })
    });
    let include_index = assembly.scenarios.iter().position(|scenario| {
        scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    &operation.action,
                    Action::BscCompile {
                        source,
                        absolute_import_paths,
                        ..
                    } if source == "IncludeTest.bsv"
                        && absolute_import_paths == &["incfiles"]
                )
            })
    });
    let bluesim_index = assembly
        .scenarios
        .iter()
        .position(|scenario| scenario.id == "bluesim-workflow-mkDummyModule");
    let bluesim_simulation_index = assembly.scenarios.iter().position(|scenario| {
        scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    operation.action,
                    Action::SimulationRun {
                        backend: PlanSimulationBackend::Bluesim,
                        ..
                    }
                )
            })
    });
    let icarus_simulation_index = assembly.scenarios.iter().position(|scenario| {
        scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    operation.action,
                    Action::SimulationRun {
                        backend: PlanSimulationBackend::Icarus,
                        ..
                    }
                )
            })
    });
    let (
        Some(first_vpi_index),
        Some(second_vpi_index),
        Some(include_index),
        Some(bluesim_index),
        Some(bluesim_simulation_index),
        Some(icarus_simulation_index),
    ) = (
        first_vpi_index,
        second_vpi_index,
        include_index,
        bluesim_index,
        bluesim_simulation_index,
        icarus_simulation_index,
    )
    else {
        return;
    };

    for (scenario_index, directory) in [(first_vpi_index, "srcfiles"), (second_vpi_index, "vfiles")]
    {
        let Some(compile) = assembly.scenarios[scenario_index].stages[0]
            .operations
            .iter_mut()
            .find(|operation| {
                matches!(
                    &operation.action,
                    Action::BscCompile { source, .. } if source == "srcfiles/GCD.bsv"
                )
            })
        else {
            return;
        };
        for output in [
            "srcfiles/my_time.ba".to_owned(),
            format!("{directory}/vpi_wrapper_my_time.h"),
            format!("{directory}/vpi_wrapper_my_time.c"),
        ] {
            if !compile.artifacts.outputs.contains(&output) {
                compile.artifacts.outputs.push(output);
            }
        }
    }

    assembly.scenarios[include_index].stages[0]
        .operations
        .splice(0..0, [create_incfiles, touch_include]);

    for (top, link, diagnostic) in missing_input_links {
        assembly.scenarios.push(Scenario {
            id: format!("options-reject-link-{top}"),
            resource: ResourceClass::Heavy,
            fixtures: Vec::new(),
            requires: vec![Requirement::Bluesim],
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("reject-link-{top}"),
                operations: vec![link, diagnostic],
            }],
        });
    }

    assembly.scenarios.push(Scenario {
        id: "options-static-golden-integrity".to_owned(),
        resource: ResourceClass::Normal,
        fixtures: Vec::new(),
        requires: Vec::new(),
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![Stage {
            id: "count-unexpanded-bluespecdir-markers".to_owned(),
            operations: vec![expected_bluespecdir, expected_raw_bluespecdir],
        }],
    });

    let first_vpi_stage = &mut assembly.scenarios[first_vpi_index].stages[0];
    first_vpi_stage.operations.extend(first_vpi_checks);

    if !assembly.scenarios[second_vpi_index]
        .requires
        .contains(&Requirement::Icarus)
    {
        assembly.scenarios[second_vpi_index]
            .requires
            .push(Requirement::Icarus);
    }
    let second_vpi_stage = &mut assembly.scenarios[second_vpi_index].stages[0];
    second_vpi_stage.operations.push(first_link);
    second_vpi_stage.operations.extend(first_link_checks);
    second_vpi_stage.operations.extend(between_links);
    second_vpi_stage.operations.push(second_link);
    second_vpi_stage.operations.extend(second_link_checks);

    let bluesim_stage = &mut assembly.scenarios[bluesim_index].stages[0];
    let Some(link_position) = bluesim_stage
        .operations
        .iter()
        .position(|operation| matches!(operation.action, Action::BscLink { .. }))
    else {
        return;
    };
    for output in [
        "simfiles/mkDummyModule.cxx",
        "simfiles/mkDummyModule.h",
        "simfiles/mkDummyModule.o",
        "simfiles/model_mkDummyModule.cxx",
        "simfiles/model_mkDummyModule.h",
        "simfiles/model_mkDummyModule.o",
    ] {
        let output = output.to_owned();
        if !bluesim_stage.operations[link_position]
            .artifacts
            .outputs
            .contains(&output)
        {
            bluesim_stage.operations[link_position]
                .artifacts
                .outputs
                .push(output);
        }
    }
    let Some(create_bfiles) = action_operation(7) else {
        return;
    };
    bluesim_stage.operations.insert(0, create_bfiles);
    bluesim_stage.operations.insert(1, touch_dummy);
    bluesim_stage
        .operations
        .insert(link_position + 2, create_simfiles);
    bluesim_stage.operations.extend(bluesim_file_checks);

    assembly.scenarios[bluesim_simulation_index].stages[0]
        .operations
        .extend([
            pre_parallel_comparison,
            parallel_link,
            parallel_normalize,
            parallel_comparison,
        ]);
    assembly.scenarios[icarus_simulation_index].stages[0]
        .operations
        .extend([quiet_iverilog, quiet_iverilog_comparison]);

    for scenario in &mut assembly.scenarios {
        for stage in &mut scenario.stages {
            stage.operations.retain(|operation| {
                !matches!(
                    &operation.action,
                    Action::FsCreateDirAll { path }
                        if matches!(path.as_str(), "foo" | "bar" | "baz")
                )
            });
        }
    }
    let path_directories = (45..=47).map(action_operation).collect::<Option<Vec<_>>>();
    let Some(path_directories) = path_directories else {
        return;
    };
    for scenario in &mut assembly.scenarios {
        if matches!(
            scenario.id.as_str(),
            "basic-options-bsc.path_dup_no_bdir"
                | "basic-options-bsc.path_dup_bdir"
                | "basic-options-bsc.path_nonidentical_dup_no_bdir"
        ) {
            scenario.stages[0]
                .operations
                .splice(0..0, path_directories.iter().cloned());
        }
    }

    assembly
        .consumed_actions
        .extend([9, 10, 33, 42, 43, 44, 48, 49, 51, 52, 53]);
    assembly.consumed_actions.extend(34..=41);
    assembly.consumed_assertions.extend([0, 1]);
    assembly.consumed_assertions.extend(9..=16);
    assembly.consumed_assertions.extend(19..=22);
    assembly.consumed_comparisons.extend([7, 8, 9]);
}

fn compose_pinned_options_flag_preflights(
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) -> BTreeSet<String> {
    if !is_pinned_options_plan(script) {
        return BTreeSet::new();
    }

    let mut exempted = BTreeSet::new();
    for scenario in &mut assembly.scenarios {
        let s0075_paths = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::AssertDiagnosticCount {
                    path,
                    kind: DiagnosticKind::Error,
                    code: Some(code),
                    count: 1,
                } if code == "S0075" => Some(path.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let s0043_paths = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::AssertDiagnosticCount {
                    path,
                    kind: DiagnosticKind::Error,
                    code: Some(code),
                    count: 1,
                } if code == "S0043" => Some(path.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();

        for stage in &mut scenario.stages {
            for operation in &mut stage.operations {
                let replacement = match &operation.action {
                    Action::BscCompile {
                        source,
                        working_directory: None,
                        mode: BscCompileMode::Frontend,
                        module: None,
                        args,
                        absolute_import_paths,
                        dependency_mode: DependencyMode::Update,
                        expected_exit: ExpectedExit::Failure,
                        environment: None,
                        stdout,
                        ..
                    } if matches!(
                        (source.as_str(), args.as_slice()),
                        (
                            "NoOptUndet_UnspecToX.bsv",
                            [verilog, no_opt, unspecified, value]
                        ) if verilog == "-verilog"
                            && no_opt == "-no-opt-undetermined-vals"
                            && unspecified == "-unspecified-to"
                            && value == "X"
                    ) || matches!(
                        (source.as_str(), args.as_slice()),
                        (
                            "NoOptUndet_UnspecToZ.bsv",
                            [verilog, no_opt, unspecified, value]
                        ) if verilog == "-verilog"
                            && no_opt == "-no-opt-undetermined-vals"
                            && unspecified == "-unspecified-to"
                            && value == "Z"
                    ) =>
                    {
                        if !absolute_import_paths.is_empty() || !s0075_paths.contains(stdout) {
                            None
                        } else {
                            let value = if source.ends_with("ToX.bsv") {
                                UndeterminedValue::X
                            } else {
                                UndeterminedValue::Z
                            };
                            exempted.insert(source.clone());
                            Some(Action::BscFlagPreflight {
                                mode: BscFlagPreflightMode::VerilogNoOptUndetermined,
                                input: source.clone(),
                                top: None,
                                unspecified_to: value,
                                stdout: stdout.clone(),
                            })
                        }
                    }
                    Action::BscLink {
                        backend: PlanSimulationBackend::Bluesim,
                        mode: BscLinkMode::Standard,
                        objects,
                        top,
                        args,
                        expected_exit: ExpectedExit::Failure,
                        simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                        missing_objects,
                    } if objects == &["m.ba"]
                        && missing_objects.is_empty()
                        && matches!(args.as_slice(), [flag, value] if flag == "-unspecified-to" && matches!(value.as_str(), "x" | "z")) =>
                    {
                        let stdout = format!("{top}.bsc-ccomp-out");
                        if !s0043_paths.contains(&stdout) {
                            None
                        } else {
                            let value = if args[1] == "x" {
                                UndeterminedValue::X
                            } else {
                                UndeterminedValue::Z
                            };
                            exempted.insert("m.ba".to_owned());
                            Some(Action::BscFlagPreflight {
                                mode: BscFlagPreflightMode::BluesimLink,
                                input: "m.ba".to_owned(),
                                top: Some(top.clone()),
                                unspecified_to: value,
                                stdout,
                            })
                        }
                    }
                    _ => None,
                };
                if let Some(action) = replacement {
                    operation.action = action;
                    operation.artifacts = ArtifactContract::for_action(&operation.action);
                }
            }
        }
    }
    exempted
}

fn compose_pinned_options_split_if_episode(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    if !is_pinned_options_plan(script)
        || script.workflow_actions.len() < 7
        || script.comparisons.len() < 2
    {
        return;
    }
    let actions = &script.workflow_actions[..7];
    if !matches!(
        actions,
        [
            WorkflowAction::CompileObject(first),
            WorkflowAction::CompileObject(second),
            WorkflowAction::TextNormalize(first_render),
            WorkflowAction::TouchCreateArtifact(touch),
            WorkflowAction::CompileObject(third),
            WorkflowAction::CompileObject(fourth),
            WorkflowAction::TextNormalize(second_render),
        ] if first.source == "SplitIfNested.bs"
            && second.source == "IfNested.bs"
            && first_render.transform == TextNormalization::IfNestedToSplitIfNested
            && touch.path == "IfNested.bs"
            && third.source == "NoSplitIfNested.bs"
            && fourth.source == "IfNested.bs"
            && second_render.transform == TextNormalization::IfNestedToNoSplitIfNested
    ) {
        return;
    }
    if !matches!(
        script.comparisons[..2],
        [
            ComparisonContract { ref arguments, .. },
            ComparisonContract { arguments: ref second_arguments, .. },
        ] if arguments == &["SplitIfNested.bs.expandif.atsexpand"]
            && second_arguments == &["NoSplitIfNested.bs.noexpandif.atsexpand"]
    ) {
        return;
    }

    let mut operations = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        let Ok(operation) = map_action(action) else {
            return;
        };
        operations.push(operation);
        if index == 2 {
            let Ok(comparison) = map_comparison(&script.comparisons[0]) else {
                return;
            };
            operations.push(comparison);
        } else if index == 6 {
            let Ok(comparison) = map_comparison(&script.comparisons[1]) else {
                return;
            };
            operations.push(comparison);
        }
    }
    assembly.scenarios.push(Scenario {
        id: "options-split-if-ordered".to_owned(),
        resource: ResourceClass::Heavy,
        fixtures: Vec::new(),
        requires: vec![Requirement::Bluesim],
        bsc_options_append: None,
        timeouts: Timeouts::default(),
        stages: vec![Stage {
            id: "split-if-generate-render-compare".to_owned(),
            operations,
        }],
    });
    assembly.consumed_actions.extend(0..7);
    assembly.consumed_comparisons.extend([0, 1]);
    assembly.golden_paths.extend([
        "SplitIfNested.bs.expandif.atsexpand.expected".to_owned(),
        "NoSplitIfNested.bs.noexpandif.atsexpand.expected".to_owned(),
    ]);
}

fn compose_ordered_workspace_compile_episodes(
    script: &ScriptManifest,
    fixture_root: &Path,
    assembly: &mut PlanAssembly,
) {
    if !matches!(
        script.origin.as_str(),
        "testsuite/bsc.driver/depend/depend.exp"
            | "testsuite/bsc.driver/imports/imports.exp"
            | "testsuite/bsc.preprocessor/include/include.exp"
            | "testsuite/bsc.verilog/filter/filter.exp"
            | OPTIONS_PLAN_ORIGIN
    ) {
        return;
    }

    #[derive(Clone, Copy)]
    enum Event {
        Compile {
            contract_index: usize,
            scenario_index: usize,
        },
        Action(usize),
        Barrier,
    }

    let supported_action = |action: &WorkflowAction| {
        matches!(
            action,
            WorkflowAction::TransferArtifact(_)
                | WorkflowAction::EraseArtifact(_)
                | WorkflowAction::EnsureDirectoryAbsent(_)
                | WorkflowAction::CreateDirectory(_)
                | WorkflowAction::TouchCreateArtifact(_)
                | WorkflowAction::RemoveUserRead(_)
                | WorkflowAction::RenderGolden(_)
                | WorkflowAction::RenderM4Curdir(_)
                | WorkflowAction::TextNormalize(_)
                | WorkflowAction::VerilogFilter(_)
                | WorkflowAction::Delay(_)
        )
    };
    let mut events = Vec::<(ExecutionOrderKey, Event)>::new();
    for (&contract_index, &scenario_index) in &assembly.compile_scenarios {
        let Some(Contract::Compile(contract)) = script.contracts.get(contract_index) else {
            continue;
        };
        events.push((
            execution_order_key(contract.span, &contract.expansion),
            Event::Compile {
                contract_index,
                scenario_index,
            },
        ));
    }
    for (contract_index, contract) in script.contracts.iter().enumerate() {
        if !matches!(contract, Contract::Compile(_)) {
            let _ = contract_index;
            events.push((contract_order_key(contract), Event::Barrier));
        }
    }
    for (action_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&action_index) {
            continue;
        }
        events.push((
            execution_order_key(action_span(action), action_expansion(action)),
            if supported_action(action) && action.guard().is_resolved() {
                Event::Action(action_index)
            } else {
                Event::Barrier
            },
        ));
    }
    events.extend(script.unsupported.iter().map(|unsupported| {
        (
            execution_order_key(unsupported.span, &unsupported.expansion),
            Event::Barrier,
        )
    }));
    events.extend(
        script
            .assertions
            .iter()
            .enumerate()
            .filter(|(index, _)| !assembly.consumed_assertions.contains(index))
            .map(|(_, assertion)| {
                (
                    execution_order_key(assertion.span, &assertion.expansion),
                    Event::Barrier,
                )
            }),
    );
    events.extend(
        script
            .comparisons
            .iter()
            .enumerate()
            .filter(|(index, _)| !assembly.consumed_comparisons.contains(index))
            .map(|(_, comparison)| {
                (
                    execution_order_key(comparison.span, &comparison.expansion),
                    Event::Barrier,
                )
            }),
    );
    events.sort_by(|left, right| left.0.cmp(&right.0));

    let mut blocks = Vec::<Vec<Event>>::new();
    let mut current = Vec::new();
    let mut current_guard = None::<Guard>;
    let mut compatibility = None::<(ResourceClass, Vec<Requirement>, Option<String>, Timeouts)>;
    let finish = |current: &mut Vec<Event>, blocks: &mut Vec<Vec<Event>>| {
        if current
            .iter()
            .any(|event| matches!(event, Event::Action(_)))
            && current
                .iter()
                .any(|event| matches!(event, Event::Compile { .. }))
        {
            blocks.push(std::mem::take(current));
        } else {
            current.clear();
        }
    };
    for (_, event) in events {
        if matches!(event, Event::Barrier) {
            finish(&mut current, &mut blocks);
            current_guard = None;
            compatibility = None;
            continue;
        }
        let guard = match event {
            Event::Compile { contract_index, .. } => match &script.contracts[contract_index] {
                Contract::Compile(contract) => contract.guard.clone(),
                _ => unreachable!("compile event references a compile contract"),
            },
            Event::Action(index) => script.workflow_actions[index].guard().clone(),
            Event::Barrier => unreachable!(),
        };
        if current_guard
            .as_ref()
            .is_some_and(|active| active != &guard)
        {
            finish(&mut current, &mut blocks);
            compatibility = None;
        }
        current_guard = Some(guard);
        if let Event::Compile { scenario_index, .. } = event {
            let scenario = &assembly.scenarios[scenario_index];
            let key = (
                scenario.resource,
                scenario.requires.clone(),
                scenario.bsc_options_append.clone(),
                scenario.timeouts,
            );
            if compatibility.as_ref().is_some_and(|active| active != &key) {
                finish(&mut current, &mut blocks);
            }
            compatibility = Some(key);
        }
        current.push(event);
    }
    finish(&mut current, &mut blocks);

    let mut replacements = Vec::<(BTreeSet<usize>, usize, Scenario, Vec<usize>)>::new();
    for block in blocks {
        let scenario_indices = block
            .iter()
            .filter_map(|event| match event {
                Event::Compile { scenario_index, .. } => Some(*scenario_index),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let action_indices = block
            .iter()
            .filter_map(|event| match event {
                Event::Action(index) => Some(*index),
                _ => None,
            })
            .collect::<Vec<_>>();
        let Some(&first_scenario_index) = scenario_indices.first() else {
            continue;
        };
        let first = &assembly.scenarios[first_scenario_index];
        let mut ordered = scenario_indices
            .iter()
            .flat_map(|index| &assembly.scenarios[*index].stages)
            .flat_map(|stage| stage.operations.iter().cloned())
            .map(|operation| (operation_order(&operation), operation))
            .collect::<Vec<_>>();
        for index in &action_indices {
            let action = &script.workflow_actions[*index];
            let operation = match action {
                WorkflowAction::EraseArtifact(erase) => OperationRecord::new(
                    map_erase(erase, EraseMode::EnsureAbsent),
                    OperationExpectation::Required,
                    provenance(erase.span, &erase.expansion),
                ),
                WorkflowAction::CreateDirectory(directory) => OperationRecord::new(
                    Action::FsCreateDirAll {
                        path: normalize_path(&directory.path),
                    },
                    OperationExpectation::Required,
                    provenance(directory.span, &directory.expansion),
                ),
                _ => match map_action(action) {
                    Ok(operation) => operation,
                    Err(_) => {
                        ordered.clear();
                        break;
                    }
                },
            };
            ordered.push((
                execution_order_key(action_span(action), action_expansion(action)),
                operation,
            ));
        }
        if ordered.is_empty() {
            continue;
        }
        ordered.sort_by(|left, right| left.0.cmp(&right.0));

        let fixture_is_file = |path: &str| {
            fs::symlink_metadata(fixture_root.join(path))
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        };
        let mut directories = BTreeSet::new();
        let mut files = BTreeSet::new();
        let mut unreadability_active = false;
        let mut unreadability_compile_seen = false;
        let mut valid = true;
        let mut operations = Vec::new();
        for (_, mut operation) in ordered {
            match &mut operation.action {
                Action::FsCreateDirAll { path } => {
                    let normalized = normalize_path(path).trim_end_matches('/').to_owned();
                    if !is_safe_relative(&normalized) {
                        valid = false;
                        break;
                    }
                    *path = normalized.clone();
                    directories.insert(normalized);
                }
                Action::FsEnsureDirectoryAbsent { path } => {
                    directories.remove(path.trim_end_matches('/'));
                }
                Action::FsCopy {
                    source,
                    destination,
                } => {
                    let directory = destination.trim_end_matches('/').to_owned();
                    if directories.contains(&directory) {
                        let Some(name) =
                            Path::new(source).file_name().and_then(|name| name.to_str())
                        else {
                            valid = false;
                            break;
                        };
                        *destination = format!("{directory}/{name}");
                    } else if destination.ends_with('/') {
                        valid = false;
                        break;
                    }
                    if !files.contains(source) && !fixture_is_file(source) {
                        valid = false;
                        break;
                    }
                    let source = source.clone();
                    let destination = destination.clone();
                    let action = if files.contains(&destination) || fixture_is_file(&destination) {
                        Action::FsCopyReplace {
                            source,
                            destination: destination.clone(),
                        }
                    } else {
                        Action::FsCopy {
                            source,
                            destination: destination.clone(),
                        }
                    };
                    operation = OperationRecord::new(
                        action,
                        OperationExpectation::Required,
                        operation.provenance.clone(),
                    );
                    files.insert(destination);
                }
                Action::FsMove {
                    source,
                    destination,
                } => {
                    if !files.contains(source) && !fixture_is_file(source) {
                        valid = false;
                        break;
                    }
                    files.remove(source);
                    files.insert(destination.clone());
                }
                Action::FsEnsureAbsent { path } => {
                    files.remove(path);
                }
                Action::FsTouchCreate { path, .. } => {
                    files.insert(path.clone());
                }
                Action::M4CurdirRender { template, output } => {
                    if !files.contains(template) && !fixture_is_file(template) {
                        valid = false;
                        break;
                    }
                    files.insert(output.clone());
                }
                Action::FsRemoveUserRead { path } => {
                    if !files.contains(path) && !fixture_is_file(path) {
                        valid = false;
                        break;
                    }
                    unreadability_active = true;
                    unreadability_compile_seen = false;
                }
                Action::BscCompile {
                    source,
                    working_directory,
                    args,
                    ..
                } => {
                    let Ok(option_directories) = compile_directory_options(args) else {
                        valid = false;
                        break;
                    };
                    if option_directories
                        .iter()
                        .any(|directory| !directories.contains(directory))
                    {
                        valid = false;
                        break;
                    }
                    if working_directory
                        .as_ref()
                        .is_some_and(|directory| !directories.contains(directory))
                    {
                        valid = false;
                        break;
                    }
                    let input = working_directory.as_ref().map_or_else(
                        || source.clone(),
                        |directory| format!("{directory}/{source}"),
                    );
                    if !files.contains(&input) && !fixture_is_file(&input) {
                        valid = false;
                        break;
                    }
                    if unreadability_active {
                        if !operation
                            .requires
                            .contains(&Requirement::PosixUnreadability)
                        {
                            operation.requires.push(Requirement::PosixUnreadability);
                        }
                        unreadability_compile_seen = true;
                    }
                }
                _ => {}
            }
            for removed in &operation.artifacts.removes {
                files.remove(removed);
            }
            files.extend(operation.artifacts.outputs.iter().cloned());
            if unreadability_active && operation.action.is_assertion() {
                if !operation
                    .requires
                    .contains(&Requirement::PosixUnreadability)
                {
                    operation.requires.push(Requirement::PosixUnreadability);
                }
                if unreadability_compile_seen {
                    unreadability_active = false;
                    unreadability_compile_seen = false;
                }
            }
            operations.push(operation);
        }
        if !valid {
            continue;
        }
        let first_contract = block.iter().find_map(|event| match event {
            Event::Compile { contract_index, .. } => Some(*contract_index),
            _ => None,
        });
        let scenario = Scenario {
            id: format!(
                "ordered-workspace-compile-{}",
                first_contract.map_or(1, |index| index + 1)
            ),
            resource: first.resource,
            fixtures: Vec::new(),
            requires: first.requires.clone(),
            bsc_options_append: first.bsc_options_append.clone(),
            timeouts: first.timeouts,
            stages: vec![Stage {
                id: "ordered-workspace".to_owned(),
                operations,
            }],
        };
        replacements.push((
            scenario_indices,
            first_scenario_index,
            scenario,
            action_indices,
        ));
    }

    if replacements.is_empty() {
        return;
    }
    let old = std::mem::take(&mut assembly.scenarios);
    assembly.scenarios = old
        .into_iter()
        .enumerate()
        .filter_map(|(index, scenario)| {
            for (members, first, replacement, actions) in &replacements {
                if index == *first {
                    assembly.consumed_actions.extend(actions);
                    return Some(replacement.clone());
                }
                if members.contains(&index) {
                    return None;
                }
            }
            Some(scenario)
        })
        .collect();
    assembly.compile_scenarios.clear();
}

fn compose_fixture_replacement_compile_episode(
    fixture_root: &Path,
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) {
    let pending_actions = script
        .workflow_actions
        .iter()
        .enumerate()
        .filter(|(index, _)| !assembly.consumed_actions.contains(index))
        .collect::<Vec<_>>();
    for actions in pending_actions.windows(3) {
        let [(first_copy_index, WorkflowAction::TransferArtifact(first_copy)), (delay_index, WorkflowAction::Delay(delay)), (second_copy_index, WorkflowAction::TransferArtifact(second_copy))] =
            actions
        else {
            continue;
        };
        if first_copy.operation != ArtifactTransferOperation::Copy
            || second_copy.operation != ArtifactTransferOperation::Copy
            || first_copy.guard != delay.guard
            || first_copy.guard != second_copy.guard
        {
            continue;
        }
        let first_source = normalize_path(&first_copy.source);
        let second_source = normalize_path(&second_copy.source);
        let destination = normalize_path(&first_copy.destination);
        if destination != normalize_path(&second_copy.destination)
            || first_source == second_source
            || !is_safe_relative(&first_source)
            || !is_safe_relative(&second_source)
            || !is_safe_relative(&destination)
            || ![&first_source, &second_source].iter().all(|source| {
                fs::symlink_metadata(fixture_root.join(source))
                    .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
            })
            || !matches!(
                fs::symlink_metadata(fixture_root.join(&destination)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        {
            continue;
        }

        let first_copy_order = execution_order_key(first_copy.span, &first_copy.expansion);
        let delay_order = execution_order_key(delay.span, &delay.expansion);
        let second_copy_order = execution_order_key(second_copy.span, &second_copy.expansion);
        if !(first_copy_order < delay_order && delay_order < second_copy_order) {
            continue;
        }
        let mut compile_nodes = script
            .contracts
            .iter()
            .enumerate()
            .filter_map(|(contract_index, contract)| {
                let Contract::Compile(contract) = contract else {
                    return None;
                };
                let scenario_index = *assembly.compile_scenarios.get(&contract_index)?;
                Some((
                    contract_index,
                    scenario_index,
                    contract,
                    execution_order_key(contract.span, &contract.expansion),
                ))
            })
            .collect::<Vec<_>>();
        compile_nodes.sort_by(|left, right| left.3.cmp(&right.3));
        let Some(final_position) = compile_nodes
            .iter()
            .position(|(_, _, _, order)| *order > second_copy_order)
        else {
            continue;
        };
        let episode_end = compile_nodes[final_position].3.clone();
        let mut members = compile_nodes
            .iter()
            .filter(|(_, _, _, order)| first_copy_order < *order && *order <= episode_end)
            .collect::<Vec<_>>();
        if members.len() < 2
            || members.first().is_none_or(|(_, _, contract, order)| {
                normalize_path(&contract.source) != destination || *order >= delay_order
            })
            || members.last().is_none_or(|(_, _, contract, order)| {
                normalize_path(&contract.source) != destination || *order <= second_copy_order
            })
            || members
                .iter()
                .any(|(_, _, contract, _)| contract.guard != first_copy.guard)
        {
            continue;
        }

        let mut episode_sources = members
            .iter()
            .map(|(_, _, contract, _)| normalize_path(&contract.source))
            .collect::<BTreeSet<_>>();
        let mut next_contract_index = members
            .last()
            .expect("episode has a final replacement compile")
            .0
            + 1;
        for candidate in compile_nodes.iter().filter(|node| node.3 > episode_end) {
            if candidate.0 != next_contract_index || candidate.2.guard != first_copy.guard {
                break;
            }
            let dependency_resolution = resolve_local_dependency_closures(
                fixture_root,
                &[BTreeSet::from([normalize_path(&candidate.2.source)])],
            );
            if !dependency_resolution.diagnostics.is_empty()
                || dependency_resolution.paths[0].is_disjoint(&episode_sources)
            {
                break;
            }
            episode_sources.insert(normalize_path(&candidate.2.source));
            members.push(candidate);
            next_contract_index += 1;
        }

        let member_contracts = members
            .iter()
            .map(|(contract_index, _, _, _)| *contract_index)
            .collect::<Vec<_>>();
        if member_contracts
            .windows(2)
            .any(|pair| pair[1] != pair[0] + 1)
        {
            continue;
        }
        let final_order = members
            .last()
            .expect("episode has a final compile")
            .3
            .clone();
        let in_window =
            |order: ExecutionOrderKey| first_copy_order <= order && order <= final_order;
        if script.unsupported.iter().any(|unsupported| {
            in_window(execution_order_key(
                unsupported.span,
                &unsupported.expansion,
            ))
        }) || script
            .workflow_actions
            .iter()
            .enumerate()
            .any(|(index, action)| {
                !assembly.consumed_actions.contains(&index)
                    && ![*first_copy_index, *delay_index, *second_copy_index].contains(&index)
                    && in_window(execution_order_key(
                        action_span(action),
                        action_expansion(action),
                    ))
            })
            || script
                .assertions
                .iter()
                .enumerate()
                .any(|(index, assertion)| {
                    !assembly.consumed_assertions.contains(&index)
                        && in_window(execution_order_key(assertion.span, &assertion.expansion))
                })
            || script
                .comparisons
                .iter()
                .enumerate()
                .any(|(index, comparison)| {
                    !assembly.consumed_comparisons.contains(&index)
                        && in_window(execution_order_key(comparison.span, &comparison.expansion))
                })
        {
            continue;
        }

        let scenario_indices = members
            .iter()
            .map(|(_, scenario_index, _, _)| *scenario_index)
            .collect::<BTreeSet<_>>();
        if scenario_indices.len() != members.len() {
            continue;
        }
        let scenarios = scenario_indices
            .iter()
            .map(|index| &assembly.scenarios[*index])
            .collect::<Vec<_>>();
        let Some(first) = scenarios.first() else {
            continue;
        };
        if scenarios.iter().any(|scenario| {
            scenario.resource != first.resource
                || scenario.requires != first.requires
                || scenario.timeouts != first.timeouts
                || scenario.bsc_options_append != first.bsc_options_append
        }) {
            continue;
        }

        let mut ordered_operations = scenarios
            .iter()
            .flat_map(|scenario| scenario.stages.iter())
            .flat_map(|stage| stage.operations.iter().cloned())
            .map(|operation| (operation_order(&operation), operation))
            .collect::<Vec<_>>();
        let first_copy_operation = map_action(&script.workflow_actions[*first_copy_index]);
        let delay_operation = map_action(&script.workflow_actions[*delay_index]);
        let (Ok(first_copy_operation), Ok(delay_operation)) =
            (first_copy_operation, delay_operation)
        else {
            continue;
        };
        ordered_operations.push((first_copy_order.clone(), first_copy_operation));
        ordered_operations.push((delay_order, delay_operation));
        ordered_operations.push((
            second_copy_order,
            OperationRecord::new(
                Action::FsCopyReplace {
                    source: second_source,
                    destination: destination.clone(),
                },
                OperationExpectation::Required,
                provenance(second_copy.span, &second_copy.expansion),
            ),
        ));
        ordered_operations.sort_by(|left, right| left.0.cmp(&right.0));

        let first_scenario_index = *scenario_indices.first().expect("episode has scenarios");
        let merged = Scenario {
            id: format!(
                "fixture-replacement-compile-{}",
                Path::new(&destination)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("source")
            ),
            resource: first.resource,
            fixtures: Vec::new(),
            requires: first.requires.clone(),
            bsc_options_append: first.bsc_options_append.clone(),
            timeouts: first.timeouts.clone(),
            stages: vec![Stage {
                id: "ordered-compile-episode".to_owned(),
                operations: ordered_operations
                    .into_iter()
                    .map(|(_, operation)| operation)
                    .collect(),
            }],
        };

        let old_scenarios = std::mem::take(&mut assembly.scenarios);
        let mut remapped = BTreeMap::new();
        assembly.scenarios = old_scenarios
            .into_iter()
            .enumerate()
            .filter_map(|(old_index, scenario)| {
                if old_index == first_scenario_index {
                    let new_index = remapped.len();
                    remapped.insert(old_index, new_index);
                    Some(merged.clone())
                } else if scenario_indices.contains(&old_index) {
                    None
                } else {
                    let new_index = remapped.len();
                    remapped.insert(old_index, new_index);
                    Some(scenario)
                }
            })
            .collect();
        let merged_index = remapped[&first_scenario_index];
        for scenario_index in assembly.compile_scenarios.values_mut() {
            if scenario_indices.contains(scenario_index) {
                *scenario_index = merged_index;
            } else if let Some(new_index) = remapped.get(scenario_index) {
                *scenario_index = *new_index;
            }
        }
        assembly
            .consumed_actions
            .extend([*first_copy_index, *delay_index, *second_copy_index]);
        return;
    }
}

fn compose_stateful_compile_chains(
    fixture_root: &Path,
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) {
    let candidates = script
        .contracts
        .iter()
        .enumerate()
        .filter_map(|(contract_index, contract)| {
            let Contract::Compile(contract) = contract else {
                return None;
            };
            let scenario_index = *assembly.compile_scenarios.get(&contract_index)?;
            let shape = compile_shape(contract).ok()?;
            Some((contract_index, scenario_index, contract, shape))
        })
        .collect::<Vec<_>>();
    let dependency_roots = candidates
        .iter()
        .map(|(_, _, contract, _)| BTreeSet::from([normalize_path(&contract.source)]))
        .collect::<Vec<_>>();
    let dependency_resolution = resolve_local_dependency_closures(fixture_root, &dependency_roots);
    if !dependency_resolution.diagnostics.is_empty() {
        return;
    }
    let nodes = candidates
        .into_iter()
        .zip(dependency_resolution.paths)
        .map(
            |((contract_index, scenario_index, contract, shape), dependencies)| CompileChainNode {
                contract_index,
                scenario_index,
                contract,
                shape,
                dependencies,
            },
        )
        .collect::<Vec<_>>();

    let mut links = Vec::new();
    for pair in nodes.windows(2) {
        let [left, right] = pair else {
            unreachable!("compile chain windows always contain two nodes")
        };
        if right.contract_index != left.contract_index + 1
            || left.contract.guard != right.contract.guard
        {
            continue;
        }
        let left_scenario = &assembly.scenarios[left.scenario_index];
        let right_scenario = &assembly.scenarios[right.scenario_index];
        if left_scenario.resource != right_scenario.resource
            || left_scenario.requires != right_scenario.requires
            || left_scenario.timeouts != right_scenario.timeouts
        {
            continue;
        }

        let left_order = execution_order_key(left.contract.span, &left.contract.expansion);
        let right_order = execution_order_key(right.contract.span, &right.contract.expansion);
        let actions = script
            .workflow_actions
            .iter()
            .enumerate()
            .filter(|(_, action)| {
                action.guard() == &left.contract.guard && {
                    let order = execution_order_key(action_span(action), action_expansion(action));
                    left_order < order && order < right_order
                }
            })
            .collect::<Vec<_>>();
        let mut artifact_flow = ArtifactFlow::new(left.shape.artifact_paths(&left.contract.source));
        let mut transitions = Vec::new();
        let mut preserves_overwritten_output = false;
        let mut valid_actions = true;
        for (action_index, action) in actions {
            let operation = match action {
                WorkflowAction::TransferArtifact(transfer) => {
                    preserves_overwritten_output |=
                        normalize_path(&transfer.source) == left.shape.stdout;
                    if !artifact_flow.apply(transfer) {
                        valid_actions = false;
                        break;
                    }
                    let operation = OperationRecord::new(
                        map_transfer(transfer),
                        OperationExpectation::Required,
                        provenance(transfer.span, &transfer.expansion),
                    );
                    if assembly.consumed_actions.contains(&action_index)
                        && !scenario_contains_operation(left_scenario, &operation)
                    {
                        valid_actions = false;
                        break;
                    }
                    operation
                }
                WorkflowAction::EraseArtifact(erase) => {
                    let path = normalize_path(&erase.path);
                    if !is_safe_relative(&path) || !artifact_flow.remove(&path) {
                        valid_actions = false;
                        break;
                    }
                    OperationRecord::new(
                        map_erase(erase, EraseMode::EnsureAbsent),
                        OperationExpectation::Required,
                        provenance(erase.span, &erase.expansion),
                    )
                }
                _ => {
                    valid_actions = false;
                    break;
                }
            };
            if !assembly.consumed_actions.contains(&action_index) {
                transitions.push((action_index, operation));
            }
        }
        if !valid_actions {
            continue;
        }
        let poisoned_dependency = left.contract.source != right.contract.source
            && matches!(left.shape.expected_exit, ExpectedExit::Failure)
            && right
                .dependencies
                .contains(&normalize_path(&left.contract.source));
        let preserved_recompile = left.contract.source == right.contract.source
            && preserves_overwritten_output
            && !transitions.is_empty();
        if poisoned_dependency || preserved_recompile {
            links.push(CompileChainLink {
                left_scenario: left.scenario_index,
                right_scenario: right.scenario_index,
                transitions,
            });
        }
    }
    if links.is_empty() {
        return;
    }

    let mut groups = Vec::<CompileChainGroup>::new();
    for link in links {
        if let Some(group) = groups
            .last_mut()
            .filter(|group| group.members.last() == Some(&link.left_scenario))
        {
            group.members.push(link.right_scenario);
            group.links.push(link);
        } else {
            let first_contract_index = nodes
                .iter()
                .find(|node| node.scenario_index == link.left_scenario)
                .map_or(0, |node| node.contract_index);
            groups.push(CompileChainGroup {
                first_contract_index,
                members: vec![link.left_scenario, link.right_scenario],
                links: vec![link],
            });
        }
    }

    let mut scenarios = std::mem::take(&mut assembly.scenarios)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    for group in groups {
        let first_index = group.members[0];
        let mut merged = scenarios[first_index]
            .take()
            .expect("compile chain starts with an imported scenario");
        merged.id = format!(
            "compile-chain-{}-{}",
            group.first_contract_index + 1,
            Path::new(
                &nodes
                    .iter()
                    .find(|node| node.scenario_index == first_index)
                    .expect("compile chain node exists")
                    .contract
                    .source
            )
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("source")
        );
        for link in group.links {
            if let Some(stage) = merged.stages.last_mut() {
                stage.operations.extend(
                    link.transitions
                        .iter()
                        .map(|(_, operation)| operation.clone()),
                );
            }
            assembly
                .consumed_actions
                .extend(link.transitions.iter().map(|(index, _)| *index));
            let next = scenarios[link.right_scenario]
                .take()
                .expect("compile chain member is imported once");
            merged.stages.extend(next.stages);
        }
        uniquify_stage_ids(&mut merged.stages);
        scenarios[first_index] = Some(merged);
    }
    assembly.scenarios = scenarios.into_iter().flatten().collect();
    assembly.compile_scenarios.clear();
}

fn compose_ordered_intermediate_dumps(
    fixture_root: &Path,
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) {
    for (action_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&action_index) || !action.guard().is_resolved() {
            continue;
        }
        let WorkflowAction::DumpIntermediate(dump) = action else {
            continue;
        };
        let input = normalize_path(&dump.input);
        let output = normalize_path(&dump.output);
        if !is_safe_relative(&input) || !is_safe_relative(&output) {
            continue;
        }
        let dump_order = execution_order_key(dump.span, &dump.expansion);
        let mut scenario_requirements = BTreeSet::new();
        let mut operation_requirements = Vec::new();
        if collect_check_requirements(
            &dump.guard,
            &mut scenario_requirements,
            &mut operation_requirements,
        )
        .is_err()
        {
            continue;
        }
        let candidates = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(scenario_index, scenario)| {
                let end = scenario_end_order(scenario)?;
                if end >= dump_order
                    || !scenario_requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                    || script.unsupported.iter().any(|unsupported| {
                        let order = execution_order_key(unsupported.span, &unsupported.expansion);
                        end < order && order < dump_order
                    })
                {
                    return None;
                }
                let exact = scenario_artifact_flow(scenario).contains(&input);
                let dependency = scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .filter(|operation| match &operation.action {
                        Action::BscCompile {
                            source,
                            expected_exit: ExpectedExit::Success,
                            ..
                        }
                        | Action::BscGenerate { source, .. } => {
                            source_matches_dump_input(source, fixture_root, &input)
                        }
                        _ => false,
                    })
                    .count()
                    == 1;
                (exact || dependency).then_some((scenario_index, end))
            })
            .collect::<Vec<_>>();
        let Some((scenario_index, _)) = candidates
            .into_iter()
            .max_by(|left, right| left.1.cmp(&right.1))
        else {
            continue;
        };
        let scenario = &mut assembly.scenarios[scenario_index];
        if !scenario_artifact_flow(scenario).contains(&input) {
            let Some(producer) = scenario
                .stages
                .iter_mut()
                .flat_map(|stage| &mut stage.operations)
                .find(|operation| match &operation.action {
                    Action::BscCompile {
                        source,
                        expected_exit: ExpectedExit::Success,
                        ..
                    }
                    | Action::BscGenerate { source, .. } => {
                        source_matches_dump_input(source, fixture_root, &input)
                    }
                    _ => false,
                })
            else {
                continue;
            };
            if !producer.artifacts.outputs.contains(&input) {
                producer.artifacts.outputs.push(input.clone());
            }
        }
        let Ok(mut operation) = map_action(action) else {
            continue;
        };
        operation.requires = operation_requirements;
        scenario
            .stages
            .last_mut()
            .expect("imported scenario has a stage")
            .operations
            .push(operation);
        assembly.consumed_actions.insert(action_index);
    }
}

fn artifact_is_referenced_after(
    script: &ScriptManifest,
    path: &str,
    after: &ExecutionOrderKey,
) -> bool {
    if script
        .unsupported
        .iter()
        .any(|unsupported| execution_order_key(unsupported.span, &unsupported.expansion) > *after)
    {
        return true;
    }
    for action in &script.workflow_actions {
        if execution_order_key(action_span(action), action_expansion(action)) <= *after {
            continue;
        }
        let Ok(operation) = map_action(action) else {
            return true;
        };
        if operation.artifacts.inputs.iter().any(|input| input == path) {
            return true;
        }
    }
    for assertion in &script.assertions {
        if execution_order_key(assertion.span, &assertion.expansion) <= *after {
            continue;
        }
        let Ok(operation) = map_assertion(assertion) else {
            return true;
        };
        if operation.artifacts.inputs.iter().any(|input| input == path) {
            return true;
        }
    }
    for comparison in &script.comparisons {
        if execution_order_key(comparison.span, &comparison.expansion) <= *after {
            continue;
        }
        let Ok(operation) = map_comparison(comparison) else {
            return true;
        };
        if operation.artifacts.inputs.iter().any(|input| input == path) {
            return true;
        }
    }
    false
}

fn has_matching_later_vcd_check(
    script: &ScriptManifest,
    path: &str,
    after: &ExecutionOrderKey,
    guard: &Guard,
) -> bool {
    script.assertions.iter().any(|assertion| {
        guard_covers(guard, &assertion.guard)
            && execution_order_key(assertion.span, &assertion.expansion) > *after
            && map_assertion(assertion).is_ok_and(|operation| {
                matches!(
                    operation.action,
                    Action::VcdCheck {
                        path: checked_path,
                        ..
                    } if checked_path == path
                )
            })
    })
}

fn compose_trailing_filesystem_actions(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    let mut action_indices = script
        .workflow_actions
        .iter()
        .enumerate()
        .filter(|(index, action)| {
            !assembly.consumed_actions.contains(index)
                && matches!(
                    action,
                    WorkflowAction::TransferArtifact(_) | WorkflowAction::EraseArtifact(_)
                )
                && action.guard().is_resolved()
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    action_indices.sort_by_key(|index| {
        execution_order_key(
            action_span(&script.workflow_actions[*index]),
            action_expansion(&script.workflow_actions[*index]),
        )
    });

    for action_index in action_indices {
        if assembly.consumed_actions.contains(&action_index) {
            continue;
        }
        let action = &script.workflow_actions[action_index];
        let Ok(operation) = map_action(action) else {
            continue;
        };
        let Some(source) = operation.artifacts.inputs.first() else {
            continue;
        };
        let action_order = execution_order_key(action_span(action), action_expansion(action));
        let mut action_requirements = BTreeSet::new();
        if collect_requirements(action.guard(), &mut action_requirements).is_err() {
            continue;
        }
        let mut candidates = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(index, scenario)| {
                let flow = scenario_artifact_flow(scenario);
                let producer_order = artifact_producer_order(scenario, source)?;
                (scenario_start_order(scenario).is_some_and(|order| order < action_order)
                    && producer_order < action_order
                    && action_requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                    && flow.contains(source)
                    && no_filesystem_composition_barrier(
                        script,
                        assembly,
                        action_index,
                        producer_order.clone(),
                        &action_order,
                    ))
                .then_some((index, producer_order))
            })
            .collect::<Vec<_>>();
        let mut optional_missing_action = false;
        if candidates.is_empty()
            && source.ends_with(".out.bak")
            && matches!(&operation.action, Action::FsMove { .. })
        {
            candidates.extend(assembly.scenarios.iter().enumerate().filter_map(
                |(index, scenario)| {
                    let producer_order = scenario
                        .stages
                        .iter()
                        .flat_map(|stage| &stage.operations)
                        .filter_map(|operation| match &operation.action {
                            Action::SimulationRun { stdout, .. }
                                if source == &format!("{stdout}.bak") =>
                            {
                                Some(operation_order(operation))
                            }
                            _ => None,
                        })
                        .filter(|order| order < &action_order)
                        .max()?;
                    (action_requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                        && no_filesystem_composition_barrier(
                            script,
                            assembly,
                            action_index,
                            producer_order.clone(),
                            &action_order,
                        ))
                    .then_some((index, producer_order))
                },
            ));
            optional_missing_action = !candidates.is_empty();
        }
        if candidates.is_empty()
            && matches!(
                &operation.action,
                Action::FsCopy {
                    source,
                    destination,
                } if source.ends_with(".bi") && destination.starts_with(&format!("{source}."))
            )
        {
            let object = format!("{}.bo", source.trim_end_matches(".bi"));
            candidates.extend(assembly.scenarios.iter().enumerate().filter_map(
                |(index, scenario)| {
                    let producer_order = artifact_producer_order(scenario, &object)?;
                    (producer_order < action_order
                        && action_requirements
                            .iter()
                            .all(|requirement| scenario.requires.contains(requirement))
                        && no_filesystem_composition_barrier(
                            script,
                            assembly,
                            action_index,
                            producer_order.clone(),
                            &action_order,
                        ))
                    .then_some((index, producer_order))
                },
            ));
            optional_missing_action = !candidates.is_empty();
        }
        if candidates.is_empty()
            && matches!(
                &operation.action,
                Action::FsCopy {
                    source,
                    destination,
                } if source.ends_with(".diff-out")
                    && destination.starts_with(&format!("{source}."))
            )
        {
            candidates.extend(assembly.scenarios.iter().enumerate().filter_map(
                |(index, scenario)| {
                    let producer_order = scenario
                        .stages
                        .iter()
                        .flat_map(|stage| &stage.operations)
                        .filter(|operation| {
                            operation
                                .action
                                .asserted_path()
                                .is_some_and(|actual| source == &format!("{actual}.diff-out"))
                        })
                        .map(operation_order)
                        .filter(|order| order < &action_order)
                        .max()?;
                    (action_requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                        && no_filesystem_composition_barrier(
                            script,
                            assembly,
                            action_index,
                            producer_order.clone(),
                            &action_order,
                        ))
                    .then_some((index, producer_order))
                },
            ));
            optional_missing_action = !candidates.is_empty();
        }
        if candidates.is_empty()
            && matches!(
                &operation.action,
                Action::FsMove {
                    source,
                    destination,
                } if source == "dump.vcd"
                    && !artifact_is_referenced_after(script, destination, &action_order)
            )
        {
            candidates.extend(assembly.scenarios.iter().enumerate().filter_map(
                |(index, scenario)| {
                    if scenario.fixtures.iter().any(|fixture| fixture == source)
                        || scenario_artifact_flow(scenario).contains(source)
                        || !action_requirements
                            .iter()
                            .all(|requirement| scenario.requires.contains(requirement))
                    {
                        return None;
                    }
                    let producer_order = scenario
                        .stages
                        .iter()
                        .flat_map(|stage| &stage.operations)
                        .filter_map(|candidate| {
                            let Action::SimulationRun {
                                backend: PlanSimulationBackend::Bluesim,
                                args,
                                ..
                            } = &candidate.action
                            else {
                                return None;
                            };
                            let order = operation_order(candidate);
                            if order >= action_order
                                || !script.workflow_actions.iter().any(|workflow_action| {
                                    matches!(workflow_action, WorkflowAction::RunBluesim(_))
                                        && workflow_action.guard() == action.guard()
                                        && execution_order_key(
                                            action_span(workflow_action),
                                            action_expansion(workflow_action),
                                        ) == order
                                })
                            {
                                return None;
                            }
                            let explicit_outputs = simulation_vcd_outputs(args)
                                .into_iter()
                                .filter(|output| output != source)
                                .collect::<Vec<_>>();
                            let [explicit_output] = explicit_outputs.as_slice() else {
                                return None;
                            };
                            has_matching_later_vcd_check(
                                script,
                                explicit_output,
                                &action_order,
                                action.guard(),
                            )
                            .then_some(order)
                        })
                        .max()?;
                    no_filesystem_composition_barrier(
                        script,
                        assembly,
                        action_index,
                        producer_order.clone(),
                        &action_order,
                    )
                    .then_some((index, producer_order))
                },
            ));
            optional_missing_action = !candidates.is_empty();
        }
        candidates.sort_by(|left, right| right.1.cmp(&left.1));
        let Some((candidate, latest)) = candidates.first().cloned() else {
            continue;
        };
        if candidates.get(1).is_some_and(|(_, order)| order == &latest) {
            continue;
        }
        if optional_missing_action {
            assembly.consumed_actions.insert(action_index);
            continue;
        }
        let scenario = &mut assembly.scenarios[candidate];
        let flow = scenario_artifact_flow(scenario);
        match &operation.action {
            Action::FsCopy { destination, .. } | Action::FsMove { destination, .. }
                if flow.contains(destination) =>
            {
                continue;
            }
            Action::FsCopy { .. } | Action::FsMove { .. } | Action::FsRemove { .. } => {}
            _ => continue,
        }
        if operation.action.requires_non_windows() {
            scenario.requires.push(Requirement::NonWindows);
            scenario.requires.sort();
            scenario.requires.dedup();
        }
        scenario
            .stages
            .last_mut()
            .expect("imported scenario has a stage")
            .operations
            .push(operation);
        assembly.consumed_actions.insert(action_index);
    }
}

fn compose_idempotent_cleanup_actions(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    let mut action_indices = script
        .workflow_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            (!assembly.consumed_actions.contains(&index)
                && matches!(action, WorkflowAction::EraseArtifact(_))
                && action.guard().is_resolved())
            .then_some(index)
        })
        .collect::<Vec<_>>();
    action_indices.sort_by_key(|index| {
        execution_order_key(
            action_span(&script.workflow_actions[*index]),
            action_expansion(&script.workflow_actions[*index]),
        )
    });

    for action_index in action_indices {
        if assembly.consumed_actions.contains(&action_index) {
            continue;
        }
        let WorkflowAction::EraseArtifact(erase) = &script.workflow_actions[action_index] else {
            unreachable!("cleanup composer only selects erase actions");
        };
        let path = normalize_path(&erase.path);
        if !is_safe_relative(&path) {
            continue;
        }
        let action_order = execution_order_key(erase.span, &erase.expansion);
        let mut requirements = BTreeSet::new();
        if collect_requirements(&erase.guard, &mut requirements).is_err() {
            continue;
        }

        let mut candidates = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(index, scenario)| {
                let end = scenario_end_order(scenario)?;
                (end < action_order
                    && requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                    && no_filesystem_composition_barrier(
                        script,
                        assembly,
                        action_index,
                        end.clone(),
                        &action_order,
                    )
                    && no_unrepresented_execution_barrier(script, assembly, end, &action_order))
                .then_some((index, scenario_end_order(scenario)?))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| right.1.cmp(&left.1));
        let embedded = candidates.is_empty().then(|| {
            assembly
                .scenarios
                .iter()
                .enumerate()
                .filter_map(|(index, scenario)| {
                    let start = scenario_start_order(scenario)?;
                    let end = scenario_end_order(scenario)?;
                    let has_later_cleanup = scenario
                        .stages
                        .iter()
                        .flat_map(|stage| &stage.operations)
                        .any(|operation| {
                            operation_order(operation) > action_order
                                && matches!(
                                    operation.action,
                                    Action::FsRemove { .. } | Action::FsEnsureAbsent { .. }
                                )
                        });
                    (start < action_order
                        && action_order < end
                        && has_later_cleanup
                        && requirements
                            .iter()
                            .all(|requirement| scenario.requires.contains(requirement)))
                    .then_some(index)
                })
                .collect::<Vec<_>>()
        });
        let (candidate, latest, embedded) = if let Some(indices) = embedded {
            let [candidate] = indices.as_slice() else {
                continue;
            };
            (*candidate, action_order.clone(), true)
        } else {
            let Some((candidate, latest)) = candidates.first().cloned() else {
                continue;
            };
            if candidates.get(1).is_some_and(|(_, end)| end == &latest) {
                continue;
            }
            (candidate, latest, false)
        };
        let scenario = &mut assembly.scenarios[candidate];
        if path_requires_non_windows(&path) {
            scenario.requires.push(Requirement::NonWindows);
            scenario.requires.sort();
            scenario.requires.dedup();
        }
        let operation = OperationRecord::new(
            map_erase(erase, EraseMode::EnsureAbsent),
            OperationExpectation::Required,
            provenance(erase.span, &erase.expansion),
        );
        if embedded {
            let mut operation = Some(operation);
            for stage in &mut scenario.stages {
                if let Some(position) = stage
                    .operations
                    .iter()
                    .position(|candidate| operation_order(candidate) > latest)
                {
                    stage.operations.insert(
                        position,
                        operation
                            .take()
                            .expect("cleanup operation is inserted once"),
                    );
                    break;
                }
            }
            if let Some(operation) = operation {
                scenario
                    .stages
                    .last_mut()
                    .expect("imported scenario has a stage")
                    .operations
                    .push(operation);
            }
        } else {
            scenario
                .stages
                .last_mut()
                .expect("imported scenario has a stage")
                .operations
                .push(operation);
        }
        assembly.consumed_actions.insert(action_index);
    }
}

fn no_unrepresented_execution_barrier(
    script: &ScriptManifest,
    assembly: &PlanAssembly,
    after: ExecutionOrderKey,
    before: &ExecutionOrderKey,
) -> bool {
    let in_window = |order: ExecutionOrderKey| after < order && order < *before;
    !script
        .assertions
        .iter()
        .enumerate()
        .any(|(index, assertion)| {
            !assembly.consumed_assertions.contains(&index)
                && in_window(execution_order_key(assertion.span, &assertion.expansion))
        })
        && !script
            .comparisons
            .iter()
            .enumerate()
            .any(|(index, comparison)| {
                !assembly.consumed_comparisons.contains(&index)
                    && in_window(execution_order_key(comparison.span, &comparison.expansion))
            })
}

fn operation_order(operation: &OperationRecord) -> ExecutionOrderKey {
    execution_order_key(
        ManifestSourceSpan {
            start_byte: operation.provenance.span.start_byte,
            end_byte: operation.provenance.span.end_byte,
            start_line: operation.provenance.span.start_line,
            start_column: operation.provenance.span.start_column,
            end_line: operation.provenance.span.end_line,
            end_column: operation.provenance.span.end_column,
        },
        &operation
            .provenance
            .expansion
            .iter()
            .map(|span| ManifestSourceSpan {
                start_byte: span.start_byte,
                end_byte: span.end_byte,
                start_line: span.start_line,
                start_column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            })
            .collect::<Vec<_>>(),
    )
}

fn scenario_contains_order(scenario: &Scenario, expected: &ExecutionOrderKey) -> bool {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .any(|operation| operation_order(operation) == *expected)
}

fn scenario_start_order(scenario: &Scenario) -> Option<ExecutionOrderKey> {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .map(operation_order)
        .min()
}

fn scenario_end_order(scenario: &Scenario) -> Option<ExecutionOrderKey> {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .map(operation_order)
        .max()
}

fn artifact_producer_order(scenario: &Scenario, path: &str) -> Option<ExecutionOrderKey> {
    let path = normalize_path(path);
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .filter(|operation| {
            operation
                .artifacts
                .outputs
                .iter()
                .any(|output| output == &path)
                || operation
                    .artifacts
                    .output_alternatives
                    .iter()
                    .any(|alternatives| alternatives.iter().any(|output| output == &path))
        })
        .map(operation_order)
        .max()
}

/// Composes a pair of compile contracts and their comparison when all three
/// originate from one static Tcl procedure expansion and compare declared dumps.
///
/// This intentionally does not schedule arbitrary cross-scenario comparisons:
/// the two producers must be adjacent in the expansion, use the same resolved
/// guard, and have no intervening workflow constructs.
fn compose_paired_compile_dump_comparisons(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    for (comparison_index, comparison) in script.comparisons.iter().enumerate() {
        if !comparison.guard.is_resolved() || comparison.expansion.is_empty() {
            continue;
        }
        let Ok(comparison_operation) = map_comparison(comparison) else {
            continue;
        };
        let Some(actual) = comparison_operation
            .action
            .asserted_path()
            .map(normalize_path)
        else {
            continue;
        };
        let expected_paths = comparison_operation.action.expected_paths();
        let [expected] = expected_paths.as_slice() else {
            continue;
        };
        let expected = normalize_path(expected);
        let comparison_order = execution_order_key(comparison.span, &comparison.expansion);
        let candidates = script
            .contracts
            .iter()
            .enumerate()
            .filter_map(|(index, contract)| {
                let Contract::Compile(contract) = contract else {
                    return None;
                };
                (contract.expansion == comparison.expansion
                    && contract.guard == comparison.guard
                    && contract_order_key(&Contract::Compile(contract.clone())) < comparison_order)
                    .then_some((index, contract))
            })
            .collect::<Vec<_>>();
        let Some((first_contract_index, first_contract)) =
            candidates.iter().copied().find(|(index, _)| {
                assembly
                    .compile_scenarios
                    .get(index)
                    .is_some_and(|scenario_index| {
                        scenario_artifact_flow(&assembly.scenarios[*scenario_index])
                            .contains(&actual)
                    })
            })
        else {
            continue;
        };
        let first_order = contract_order_key(&Contract::Compile(first_contract.clone()));
        let Some((second_contract_index, second_contract)) =
            candidates.iter().copied().find(|(index, contract)| {
                contract_order_key(&Contract::Compile((*contract).clone())) > first_order
                    && assembly
                        .compile_scenarios
                        .get(index)
                        .is_some_and(|scenario_index| {
                            scenario_artifact_flow(&assembly.scenarios[*scenario_index])
                                .contains(&expected)
                        })
            })
        else {
            continue;
        };
        let first_order = contract_order_key(&Contract::Compile(first_contract.clone()));
        let second_order = contract_order_key(&Contract::Compile(second_contract.clone()));
        if first_contract_index == second_contract_index
            || !paired_compile_dump_comparison_is_closed(
                script,
                assembly,
                first_contract_index,
                second_contract_index,
                comparison_index,
                first_order,
                second_order,
                &comparison_order,
            )
        {
            continue;
        }
        let Some(&first_scenario_index) = assembly.compile_scenarios.get(&first_contract_index)
        else {
            continue;
        };
        let Some(&second_scenario_index) = assembly.compile_scenarios.get(&second_contract_index)
        else {
            continue;
        };
        if first_scenario_index >= second_scenario_index {
            continue;
        }

        let mut second = assembly.scenarios.remove(second_scenario_index);
        for stage in &mut second.stages {
            stage
                .operations
                .retain(|operation| operation != &comparison_operation);
        }
        let first = &mut assembly.scenarios[first_scenario_index];
        for stage in &mut first.stages {
            stage
                .operations
                .retain(|operation| operation != &comparison_operation);
        }
        first.id = format!(
            "compile-dump-comparison-{}-{}",
            comparison_index + 1,
            first_contract.source
        );
        if second.resource == ResourceClass::Heavy {
            first.resource = ResourceClass::Heavy;
        }
        first.requires.extend(second.requires);
        first.requires.sort();
        first.requires.dedup();
        first.fixtures.extend(second.fixtures);
        first.fixtures.sort();
        first.fixtures.dedup();
        first.stages.extend(second.stages);
        first
            .stages
            .last_mut()
            .expect("compiled scenario has a stage")
            .operations
            .push(comparison_operation);
        uniquify_stage_ids(&mut first.stages);

        for scenario_index in assembly.compile_scenarios.values_mut() {
            if *scenario_index == second_scenario_index {
                *scenario_index = first_scenario_index;
            } else if *scenario_index > second_scenario_index {
                *scenario_index -= 1;
            }
        }
        assembly.consumed_comparisons.insert(comparison_index);
        assembly.golden_paths.remove(&expected);
    }
}

fn paired_compile_dump_comparison_is_closed(
    script: &ScriptManifest,
    assembly: &PlanAssembly,
    first_contract_index: usize,
    second_contract_index: usize,
    comparison_index: usize,
    first_order: ExecutionOrderKey,
    second_order: ExecutionOrderKey,
    comparison_order: &ExecutionOrderKey,
) -> bool {
    if !(first_order < second_order && second_order < *comparison_order) {
        return false;
    }
    let in_window = |order: ExecutionOrderKey| first_order < order && order < *comparison_order;
    !script
        .contracts
        .iter()
        .enumerate()
        .any(|(index, contract)| {
            index != first_contract_index
                && index != second_contract_index
                && in_window(contract_order_key(contract))
        })
        && !script.unsupported.iter().any(|unsupported| {
            in_window(execution_order_key(
                unsupported.span,
                &unsupported.expansion,
            ))
        })
        && !script
            .workflow_actions
            .iter()
            .enumerate()
            .any(|(index, action)| {
                !assembly.consumed_actions.contains(&index)
                    && in_window(execution_order_key(
                        action_span(action),
                        action_expansion(action),
                    ))
            })
        && !script
            .assertions
            .iter()
            .any(|assertion| in_window(execution_order_key(assertion.span, &assertion.expansion)))
        && !script
            .comparisons
            .iter()
            .enumerate()
            .any(|(index, candidate)| {
                index != comparison_index
                    && in_window(execution_order_key(candidate.span, &candidate.expansion))
            })
}

fn map_verilog_link_with_static_globs(
    action: &WorkflowAction,
    link: &crate::model::LinkVerilogAction,
    fixture_root: &Path,
    scenarios: &[Scenario],
    link_order: &ExecutionOrderKey,
) -> Result<OperationRecord, String> {
    let objects = parse_arguments(&link.objects, "Verilog link objects")?;
    if !objects
        .iter()
        .any(|object| object.contains(['*', '?', '[', ']']))
    {
        return map_action(action);
    }
    if objects
        .iter()
        .any(|object| object.contains(['*', '?', '[', ']']) && object != "*.v")
    {
        return Err("only the static root-directory *.v Verilog link glob is supported".to_owned());
    }

    let mut expanded = objects
        .iter()
        .filter(|object| object.as_str() != "*.v")
        .map(|object| normalize_path(object))
        .collect::<BTreeSet<_>>();
    for scenario in scenarios {
        if scenario_end_order(scenario).is_none_or(|order| order >= *link_order) {
            continue;
        }
        let flow = scenario_artifact_flow(scenario);
        expanded.extend(
            flow.available
                .into_iter()
                .filter(|path| is_local_generated_artifact(path, "v")),
        );
    }
    if objects.iter().any(|object| object == "*.v") {
        if let Ok(entries) = fs::read_dir(fixture_root) {
            expanded.extend(entries.filter_map(Result::ok).filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_file() && !file_type.is_symlink())
                    .and_then(|_| entry.file_name().to_str().map(str::to_owned))
                    .filter(|path| is_local_generated_artifact(path, "v"))
            }));
        }
    }
    if expanded.is_empty() {
        return Err("Verilog link glob matched no declared artifacts or fixtures".to_owned());
    }

    Ok(OperationRecord::new(
        Action::BscLink {
            backend: PlanSimulationBackend::Icarus,
            mode: if link.no_main {
                BscLinkMode::NoMain
            } else {
                BscLinkMode::Standard
            },
            objects: expanded.into_iter().collect(),
            top: link.top.clone(),
            args: if link.no_main {
                Vec::new()
            } else {
                parse_arguments(&link.options, "Verilog link options")?
            },
            expected_exit: link.expected_exit,
            simulator: link.simulator,
            missing_objects: Vec::new(),
        },
        OperationExpectation::Required,
        provenance(link.span, &link.expansion),
    ))
}

fn compose_multi_compile_verilog_workflows(
    fixture_root: &Path,
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) {
    for (link_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&link_index) || !action.guard().is_resolved() {
            continue;
        }
        let WorkflowAction::LinkVerilog(link) = action else {
            continue;
        };
        let link_order = execution_order_key(action_span(action), action_expansion(action));
        let Ok(link_operation) = map_verilog_link_with_static_globs(
            action,
            link,
            fixture_root,
            &assembly.scenarios,
            &link_order,
        ) else {
            continue;
        };
        let mut sources = BTreeSet::new();
        let mut valid = true;
        for input in &link_operation.artifacts.inputs {
            let candidates = assembly
                .scenarios
                .iter()
                .enumerate()
                .filter(|(_, scenario)| {
                    scenario_start_order(scenario).is_some_and(|order| order < link_order)
                        && scenario_artifact_flow(scenario).contains(input)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            match candidates.as_slice() {
                [index] => {
                    sources.insert(*index);
                }
                [] if fixture_root.join(input).is_file() => {}
                _ => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid || sources.is_empty() {
            continue;
        }
        let source_indices = sources.into_iter().collect::<Vec<_>>();
        let Some(earliest) = source_indices
            .iter()
            .filter_map(|index| scenario_start_order(&assembly.scenarios[*index]))
            .min()
        else {
            continue;
        };
        let mut component_indices = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(index, scenario)| {
                let start = scenario_start_order(scenario)?;
                let end = scenario_end_order(scenario)?;
                (earliest <= start && end < link_order).then_some(index)
            })
            .collect::<Vec<_>>();
        if source_indices
            .iter()
            .any(|index| !component_indices.contains(index))
        {
            continue;
        }
        component_indices.sort_by_key(|index| scenario_start_order(&assembly.scenarios[*index]));
        let Some(timeouts) = component_indices
            .first()
            .map(|index| assembly.scenarios[*index].timeouts)
        else {
            continue;
        };
        if component_indices
            .iter()
            .any(|index| assembly.scenarios[*index].timeouts != timeouts)
            || !no_multi_compile_verilog_barrier(
                script,
                assembly,
                &component_indices,
                earliest,
                &link_order,
            )
        {
            continue;
        }

        let mut operations = vec![link_operation];
        let mut consumed_actions = vec![link_index];
        let mut cursor = link_order;
        for (run_index, run) in script.workflow_actions.iter().enumerate() {
            if assembly.consumed_actions.contains(&run_index)
                || !matches!(run, WorkflowAction::RunVerilog(_))
            {
                continue;
            }
            let run_order = execution_order_key(action_span(run), action_expansion(run));
            if run_order <= cursor || !guard_covers(&link.guard, run.guard()) {
                continue;
            }
            let Ok(run_operation) = map_action(run) else {
                continue;
            };
            let Action::SimulationRun {
                executable, stdout, ..
            } = &run_operation.action
            else {
                continue;
            };
            if executable != &link.top
                || !no_multi_compile_verilog_barrier(
                    script,
                    assembly,
                    &component_indices,
                    cursor.clone(),
                    &run_order,
                )
            {
                continue;
            }
            let stdout = stdout.clone();
            operations.push(run_operation);
            consumed_actions.push(run_index);
            cursor = run_order;
            for (comparison_index, comparison) in script.comparisons.iter().enumerate() {
                if assembly.consumed_comparisons.contains(&comparison_index)
                    || !guard_covers(run.guard(), &comparison.guard)
                    || comparison
                        .arguments
                        .first()
                        .is_none_or(|actual| normalize_path(actual) != *stdout)
                {
                    continue;
                }
                let comparison_order = execution_order_key(comparison.span, &comparison.expansion);
                if comparison_order <= cursor
                    || !no_verilog_followup_barrier(
                        script,
                        assembly,
                        cursor.clone(),
                        &comparison_order,
                    )
                {
                    continue;
                }
                let Ok(comparison_operation) = map_comparison(comparison) else {
                    continue;
                };
                operations.push(comparison_operation.clone());
                assembly.consumed_comparisons.insert(comparison_index);
                assembly.golden_paths.extend(
                    comparison_operation
                        .action
                        .expected_paths()
                        .into_iter()
                        .map(str::to_owned),
                );
                cursor = comparison_order;
            }
        }

        let mut scenarios = component_indices
            .iter()
            .map(|index| assembly.scenarios[*index].clone())
            .collect::<Vec<_>>();
        let mut removal_indices = component_indices.clone();
        removal_indices.sort_unstable_by(|left, right| right.cmp(left));
        for index in removal_indices {
            assembly.scenarios.remove(index);
        }
        let mut merged = scenarios.remove(0);
        merged.id = format!("verilog-workflow-{}-{}", link_index + 1, link.top);
        for scenario in scenarios {
            if scenario.resource == ResourceClass::Heavy {
                merged.resource = ResourceClass::Heavy;
            }
            merged.requires.extend(scenario.requires);
            merged.fixtures.extend(scenario.fixtures);
            merged.stages.extend(scenario.stages);
        }
        merged
            .requires
            .extend([Requirement::Icarus, Requirement::Verilog]);
        merged.requires.sort();
        merged.requires.dedup();
        merged.fixtures.sort();
        merged.fixtures.dedup();
        merged.stages.push(Stage {
            id: format!("verilog-link-{}", link.top),
            operations,
        });
        uniquify_stage_ids(&mut merged.stages);
        assembly.consumed_actions.extend(consumed_actions);
        assembly.scenarios.push(merged);
    }
}

fn compose_ordered_simulation_runs(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    for (action_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&action_index) || !action.guard().is_resolved() {
            continue;
        }
        let (backend, executable) = match action {
            WorkflowAction::RunBluesim(run) => (
                PlanSimulationBackend::Bluesim,
                normalize_path(&run.executable),
            ),
            WorkflowAction::RunVerilog(run) => (
                PlanSimulationBackend::Icarus,
                normalize_path(&run.executable),
            ),
            _ => continue,
        };
        let Ok(operation) = map_action(action) else {
            continue;
        };
        let input = simulation_executable_artifact(backend, &executable);
        let run_order = execution_order_key(action_span(action), action_expansion(action));
        let mut requirements = BTreeSet::new();
        if collect_requirements(action.guard(), &mut requirements).is_err() {
            continue;
        }
        match backend {
            PlanSimulationBackend::Bluesim => {
                requirements.insert(Requirement::Bluesim);
            }
            PlanSimulationBackend::Icarus => {
                requirements.extend([Requirement::Verilog, Requirement::Icarus]);
            }
        }

        let candidates = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(scenario_index, scenario)| {
                let end = scenario_end_order(scenario)?;
                (end < run_order
                    && requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                    && scenario_artifact_flow(scenario).contains(&input)
                    && no_verilog_followup_barrier(script, assembly, end.clone(), &run_order))
                .then_some((scenario_index, end))
            })
            .collect::<Vec<_>>();
        let [candidate] = candidates.as_slice() else {
            continue;
        };
        let scenario = &mut assembly.scenarios[candidate.0];
        scenario.resource = ResourceClass::Heavy;
        scenario.stages.push(Stage {
            id: format!("simulation-{}", executable),
            operations: vec![operation],
        });
        uniquify_stage_ids(&mut scenario.stages);
        assembly.consumed_actions.insert(action_index);
    }
}

fn compose_ordered_checks(script: &ScriptManifest, assembly: &mut PlanAssembly) {
    let mut checks = Vec::new();
    for (index, assertion) in script.assertions.iter().enumerate() {
        if assembly.consumed_assertions.contains(&index) || !assertion.guard.is_resolved() {
            continue;
        }
        if let Ok(operation) = map_assertion(assertion) {
            checks.push((
                execution_order_key(assertion.span, &assertion.expansion),
                BoundCheck::Assertion(index),
                assertion.guard.clone(),
                operation,
            ));
        }
    }
    for (index, comparison) in script.comparisons.iter().enumerate() {
        if assembly.consumed_comparisons.contains(&index) || !comparison.guard.is_resolved() {
            continue;
        }
        if let Ok(operation) = map_comparison(comparison) {
            checks.push((
                execution_order_key(comparison.span, &comparison.expansion),
                BoundCheck::Comparison(index),
                comparison.guard.clone(),
                operation,
            ));
        }
    }
    checks.sort_by(|left, right| left.0.cmp(&right.0));

    for (check_order, check, guard, mut operation) in checks {
        let Some(path) = operation.action.asserted_path().map(normalize_path) else {
            continue;
        };
        let mut requirements = BTreeSet::new();
        if collect_check_requirements(&guard, &mut requirements, &mut operation.requires).is_err() {
            continue;
        }
        let mut producers = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(index, scenario)| {
                let producer_order = artifact_producer_order(scenario, &path)?;
                (producer_order < check_order
                    && requirements
                        .iter()
                        .all(|requirement| scenario.requires.contains(requirement))
                    && no_ordered_check_barrier(
                        script,
                        assembly,
                        producer_order.clone(),
                        &check_order,
                    ))
                .then_some((index, producer_order))
            })
            .collect::<Vec<_>>();
        producers.sort_by(|left, right| right.1.cmp(&left.1));
        let Some((scenario_index, latest)) = producers.first().cloned() else {
            continue;
        };
        if producers.get(1).is_some_and(|(_, order)| order == &latest) {
            continue;
        }
        let scenario = &mut assembly.scenarios[scenario_index];
        if operation.action.requires_non_windows() {
            scenario.requires.push(Requirement::NonWindows);
            scenario.requires.sort();
            scenario.requires.dedup();
        }
        let mut pending = Some(operation.clone());
        for stage in &mut scenario.stages {
            if let Some(position) = stage
                .operations
                .iter()
                .position(|candidate| operation_order(candidate) > check_order)
            {
                stage.operations.insert(
                    position,
                    pending.take().expect("check operation is inserted once"),
                );
                break;
            }
        }
        if let Some(operation) = pending {
            scenario
                .stages
                .last_mut()
                .expect("imported scenario has a stage")
                .operations
                .push(operation);
        }
        match check {
            BoundCheck::Assertion(index) => {
                assembly.consumed_assertions.insert(index);
            }
            BoundCheck::Comparison(index) => {
                assembly.consumed_comparisons.insert(index);
                assembly.golden_paths.extend(
                    operation
                        .action
                        .expected_paths()
                        .into_iter()
                        .map(str::to_owned),
                );
            }
        }
    }
}

fn compose_static_fixture_vcd_checks(
    script: &ScriptManifest,
    fixture_root: &Path,
    assembly: &mut PlanAssembly,
) {
    let mut operations = Vec::new();
    let mut requirements = BTreeSet::new();
    let mut consumed = Vec::new();

    for (index, assertion) in script.assertions.iter().enumerate() {
        if assembly.consumed_assertions.contains(&index)
            || !matches!(assertion.helper.as_str(), "vcdcheck_pass" | "vcdcheck_fail")
            || !assertion.guard.is_resolved()
        {
            continue;
        }
        let Ok(mut operation) = map_assertion(assertion) else {
            continue;
        };
        let Action::VcdCheck { path, .. } = &operation.action else {
            continue;
        };
        if !is_safe_relative(path)
            || !fs::symlink_metadata(fixture_root.join(path))
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
            || assembly
                .scenarios
                .iter()
                .any(|scenario| scenario_artifact_flow(scenario).contains(path))
            || collect_check_requirements(
                &assertion.guard,
                &mut requirements,
                &mut operation.requires,
            )
            .is_err()
        {
            continue;
        }
        if operation.action.requires_non_windows() {
            requirements.insert(Requirement::NonWindows);
        }
        operations.push(operation);
        consumed.push(index);
    }

    if operations.is_empty() {
        return;
    }
    assembly.push(ImportedScenario {
        scenario: Scenario {
            id: "vcd-check-fixtures".to_owned(),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "check-vcd-fixtures".to_owned(),
                operations,
            }],
        },
        consumption: ImportConsumption {
            assertions: consumed,
            ..ImportConsumption::default()
        },
    });
}

fn no_ordered_check_barrier(
    script: &ScriptManifest,
    assembly: &PlanAssembly,
    after: ExecutionOrderKey,
    before: &ExecutionOrderKey,
) -> bool {
    let in_window = |order: ExecutionOrderKey| after < order && order < *before;
    !script.unsupported.iter().any(|unsupported| {
        in_window(execution_order_key(
            unsupported.span,
            &unsupported.expansion,
        ))
    }) && !script
        .workflow_actions
        .iter()
        .enumerate()
        .any(|(index, action)| {
            !assembly.consumed_actions.contains(&index)
                && in_window(execution_order_key(
                    action_span(action),
                    action_expansion(action),
                ))
        })
        && !script
            .assertions
            .iter()
            .enumerate()
            .any(|(index, assertion)| {
                !assembly.consumed_assertions.contains(&index)
                    && in_window(execution_order_key(assertion.span, &assertion.expansion))
            })
        && !script
            .comparisons
            .iter()
            .enumerate()
            .any(|(index, comparison)| {
                !assembly.consumed_comparisons.contains(&index)
                    && in_window(execution_order_key(comparison.span, &comparison.expansion))
            })
}

fn compose_ordered_bluesim_links(
    fixture_root: &Path,
    script: &ScriptManifest,
    assembly: &mut PlanAssembly,
) {
    for (link_index, action) in script.workflow_actions.iter().enumerate() {
        if assembly.consumed_actions.contains(&link_index) || !action.guard().is_resolved() {
            continue;
        }
        let WorkflowAction::LinkObjects(link) = action else {
            continue;
        };
        let Ok(mut link_operation) = map_action(action) else {
            continue;
        };
        let link_order = execution_order_key(link.span, &link.expansion);
        let default_artifact_producer = if link_operation.artifacts.inputs.is_empty() {
            let candidates = assembly
                .scenarios
                .iter()
                .enumerate()
                .filter(|(_, scenario)| {
                    scenario_end_order(scenario).is_some_and(|order| order < link_order)
                        && can_declare_default_bluesim_link_artifact(scenario, &link.top)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let [scenario_index] = candidates.as_slice() else {
                continue;
            };
            link_operation
                .artifacts
                .inputs
                .push(format!("{}.ba", normalize_path(&link.top)));
            Some(*scenario_index)
        } else {
            None
        };
        let inputs = link_operation.artifacts.inputs.clone();
        let default_artifact =
            default_artifact_producer.map(|_| format!("{}.ba", normalize_path(&link.top)));
        let mut source_indices = BTreeSet::new();
        let mut valid = true;
        for input in &inputs {
            if default_artifact.as_deref() == Some(input) {
                source_indices.insert(
                    default_artifact_producer.expect("default artifact has a unique producer"),
                );
                continue;
            }
            let producers = assembly
                .scenarios
                .iter()
                .enumerate()
                .filter(|(_, scenario)| {
                    scenario_end_order(scenario).is_some_and(|order| order < link_order)
                        && scenario_artifact_flow(scenario).contains(input)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            match producers.as_slice() {
                [index] => {
                    source_indices.insert(*index);
                }
                [] if fixture_root.join(input).is_file() => {}
                _ => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid || source_indices.is_empty() {
            continue;
        }
        let source_indices = source_indices.into_iter().collect::<Vec<_>>();
        let Some(earliest) = source_indices
            .iter()
            .filter_map(|index| scenario_start_order(&assembly.scenarios[*index]))
            .min()
        else {
            continue;
        };
        let component_indices = assembly
            .scenarios
            .iter()
            .enumerate()
            .filter_map(|(index, scenario)| {
                let start = scenario_start_order(scenario)?;
                let end = scenario_end_order(scenario)?;
                (earliest <= start && end < link_order).then_some(index)
            })
            .collect::<Vec<_>>();
        if component_indices.len() < source_indices.len()
            || source_indices
                .iter()
                .any(|index| !component_indices.contains(index))
        {
            continue;
        }
        let Some(timeouts) = component_indices
            .first()
            .map(|index| assembly.scenarios[*index].timeouts)
        else {
            continue;
        };
        if component_indices
            .iter()
            .any(|index| assembly.scenarios[*index].timeouts != timeouts)
            || !no_multi_compile_verilog_barrier(
                script,
                assembly,
                &component_indices,
                earliest,
                &link_order,
            )
        {
            continue;
        }
        let mut sources = component_indices
            .iter()
            .map(|index| assembly.scenarios[*index].clone())
            .collect::<Vec<_>>();
        if let Some(scenario_index) = default_artifact_producer {
            let Some(component_index) = component_indices
                .iter()
                .position(|index| *index == scenario_index)
            else {
                continue;
            };
            if !declare_default_bluesim_link_artifact(&mut sources[component_index], &link.top) {
                continue;
            }
        }
        let mut merged = sources.remove(0);
        merged.id = format!("bluesim-link-{}-{}", link_index + 1, link.top);
        for source in sources {
            if source.resource == ResourceClass::Heavy {
                merged.resource = ResourceClass::Heavy;
            }
            merged.requires.extend(source.requires);
            merged.fixtures.extend(source.fixtures);
            merged.stages.extend(source.stages);
        }
        merged.requires.push(Requirement::Bluesim);
        merged.requires.sort();
        merged.requires.dedup();
        merged.fixtures.sort();
        merged.fixtures.dedup();
        merged.stages.push(Stage {
            id: format!("bluesim-link-{}", link.top),
            operations: vec![link_operation],
        });
        uniquify_stage_ids(&mut merged.stages);
        for index in component_indices.iter().rev() {
            assembly.scenarios.remove(*index);
        }
        assembly.consumed_actions.insert(link_index);
        assembly.scenarios.push(merged);
    }
}

fn can_declare_default_bluesim_link_artifact(scenario: &Scenario, top: &str) -> bool {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .any(|operation| match &operation.action {
            Action::BscGenerate {
                source,
                mode: SimulationGenerationMode::Bluesim,
                module: None,
                ..
            }
            | Action::BscCompile {
                source,
                mode: BscCompileMode::BluesimObject,
                expected_exit: ExpectedExit::Success,
                ..
            } => Path::new(source)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| top == format!("sys{stem}")),
            _ => false,
        })
}

fn declare_default_bluesim_link_artifact(scenario: &mut Scenario, top: &str) -> bool {
    let artifact = format!("{}.ba", normalize_path(top));
    let Some(operation) = scenario
        .stages
        .iter_mut()
        .flat_map(|stage| &mut stage.operations)
        .find(|operation| match &operation.action {
            Action::BscGenerate {
                source,
                mode: SimulationGenerationMode::Bluesim,
                module: None,
                ..
            }
            | Action::BscCompile {
                source,
                mode: BscCompileMode::BluesimObject,
                expected_exit: ExpectedExit::Success,
                ..
            } => Path::new(source)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| top == format!("sys{stem}")),
            _ => false,
        })
    else {
        return false;
    };
    if !operation.artifacts.outputs.contains(&artifact) {
        operation.artifacts.outputs.push(artifact);
    }
    true
}

fn no_verilog_followup_barrier(
    script: &ScriptManifest,
    assembly: &PlanAssembly,
    after: ExecutionOrderKey,
    before: &ExecutionOrderKey,
) -> bool {
    let in_window = |order: ExecutionOrderKey| after < order && order < *before;
    !script.unsupported.iter().any(|unsupported| {
        in_window(execution_order_key(
            unsupported.span,
            &unsupported.expansion,
        ))
    }) && !script
        .workflow_actions
        .iter()
        .enumerate()
        .any(|(index, action)| {
            !assembly.consumed_actions.contains(&index)
                && in_window(execution_order_key(
                    action_span(action),
                    action_expansion(action),
                ))
        })
        && !script
            .assertions
            .iter()
            .enumerate()
            .any(|(index, assertion)| {
                !assembly.consumed_assertions.contains(&index)
                    && in_window(execution_order_key(assertion.span, &assertion.expansion))
            })
        && !script
            .comparisons
            .iter()
            .enumerate()
            .any(|(index, comparison)| {
                !assembly.consumed_comparisons.contains(&index)
                    && in_window(execution_order_key(comparison.span, &comparison.expansion))
            })
}

fn no_multi_compile_verilog_barrier(
    script: &ScriptManifest,
    assembly: &PlanAssembly,
    source_indices: &[usize],
    after: ExecutionOrderKey,
    before: &ExecutionOrderKey,
) -> bool {
    let in_window = |order: ExecutionOrderKey| after < order && order < *before;
    !script.unsupported.iter().any(|unsupported| {
        in_window(execution_order_key(
            unsupported.span,
            &unsupported.expansion,
        ))
    }) && !script
        .workflow_actions
        .iter()
        .enumerate()
        .any(|(index, action)| {
            !assembly.consumed_actions.contains(&index)
                && !matches!(action, WorkflowAction::RunVerilog(_))
                && in_window(execution_order_key(
                    action_span(action),
                    action_expansion(action),
                ))
        })
        && !script.contracts.iter().enumerate().any(|(_, contract)| {
            let order = contract_order_key(contract);
            in_window(order.clone())
                && !source_indices
                    .iter()
                    .any(|index| scenario_contains_order(&assembly.scenarios[*index], &order))
        })
        && !script
            .assertions
            .iter()
            .enumerate()
            .any(|(index, assertion)| {
                !assembly.consumed_assertions.contains(&index)
                    && in_window(execution_order_key(assertion.span, &assertion.expansion))
            })
        && !script
            .comparisons
            .iter()
            .enumerate()
            .any(|(index, comparison)| {
                !assembly.consumed_comparisons.contains(&index)
                    && in_window(execution_order_key(comparison.span, &comparison.expansion))
            })
}

fn scenario_artifact_flow(scenario: &Scenario) -> ArtifactFlow {
    let mut flow = ArtifactFlow::new(BTreeSet::new());
    for operation in scenario.stages.iter().flat_map(|stage| &stage.operations) {
        for output in &operation.artifacts.outputs {
            flow.insert(output.clone());
        }
        for alternative in &operation.artifacts.output_alternatives {
            for output in alternative {
                flow.insert(output.clone());
            }
        }
        for removed in &operation.artifacts.removes {
            flow.remove(removed);
        }
    }
    flow
}

fn no_filesystem_composition_barrier(
    script: &ScriptManifest,
    assembly: &PlanAssembly,
    target_action: usize,
    after: ExecutionOrderKey,
    before: &ExecutionOrderKey,
) -> bool {
    let in_window = |order: ExecutionOrderKey| after < order && order < *before;
    !script.unsupported.iter().any(|unsupported| {
        in_window(execution_order_key(
            unsupported.span,
            &unsupported.expansion,
        ))
    }) && !script
        .workflow_actions
        .iter()
        .enumerate()
        .any(|(index, action)| {
            index != target_action
                && !assembly.consumed_actions.contains(&index)
                && !matches!(
                    action,
                    WorkflowAction::TransferArtifact(_) | WorkflowAction::EraseArtifact(_)
                )
                && in_window(execution_order_key(
                    action_span(action),
                    action_expansion(action),
                ))
        })
}

fn scenario_contains_operation(scenario: &Scenario, expected: &OperationRecord) -> bool {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .any(|operation| operation == expected)
}

fn uniquify_stage_ids(stages: &mut [Stage]) {
    let mut used = BTreeSet::new();
    for stage in stages {
        let base = stage.id.clone();
        let mut suffix = 1;
        while !used.insert(stage.id.clone()) {
            suffix += 1;
            stage.id = format!("{base}-{suffix}");
        }
    }
}

#[derive(Default)]
struct RawSimulationShape<'a> {
    source: String,
    top: String,
    module_list: &'a str,
    generation_options: &'a str,
    expected_output: &'a str,
    bluesim_failure: &'a str,
    icarus_failure: &'a str,
    link_options: &'a str,
    simulation_options: &'a str,
    sort_output: &'a str,
    check_vcd: &'a str,
}

struct SimulationShape {
    source: String,
    top: String,
    modules: Vec<String>,
    generation_args: Vec<String>,
    link_args: Vec<String>,
    simulation_args: Vec<String>,
    expected: String,
    bluesim_xfail: Option<String>,
    icarus_xfail: Option<String>,
    sort_output: bool,
    check_vcd: bool,
}

fn parse_simulation_shape(
    raw: RawSimulationShape<'_>,
    backends: &[SimulationBackend],
    helper: &str,
) -> Result<SimulationShape, String> {
    let bluesim_xfail = if backends.contains(&SimulationBackend::Bluesim) {
        known_simulation_output_xfail(SimulationBackend::Bluesim, raw.bluesim_failure)?
    } else {
        None
    };
    let icarus_xfail = if backends.contains(&SimulationBackend::Icarus) {
        known_simulation_output_xfail(SimulationBackend::Icarus, raw.icarus_failure)?
    } else {
        None
    };
    let sort_output = match raw.sort_output {
        "" | "0" => false,
        "1" => true,
        value => return Err(format!("{helper} has invalid sort_output value {value:?}")),
    };
    if sort_output && (bluesim_xfail.is_some() || icarus_xfail.is_some()) {
        return Err(format!(
            "{helper} combines sorted output with a known-failure annotation"
        ));
    }
    let check_vcd = match raw.check_vcd {
        "" | "1" => true,
        "0" => false,
        value => return Err(format!("{helper} has invalid check_vcd value {value:?}")),
    };
    let source = normalize_path(&raw.source);
    let expected = if raw.expected_output.is_empty() {
        format!("{}.out.expected", raw.top)
    } else {
        normalize_path(raw.expected_output)
    };

    Ok(SimulationShape {
        source,
        top: raw.top,
        modules: parse_arguments(raw.module_list, "simulation module list")?,
        generation_args: parse_arguments(raw.generation_options, "generation options")?,
        link_args: parse_arguments(raw.link_options, "link options")?,
        simulation_args: parse_arguments(raw.simulation_options, "simulation options")?,
        expected,
        bluesim_xfail,
        icarus_xfail,
        sort_output,
        check_vcd,
    })
}

#[derive(Debug)]
struct CompileDiagnostic {
    action: Action,
    expectation: OperationExpectation,
}

#[derive(Debug)]
struct CompileShape {
    mode: BscCompileMode,
    module: Option<String>,
    args: Vec<String>,
    dependency_mode: DependencyMode,
    expected_exit: ExpectedExit,
    unexpected_success_forbidden_regex: Option<String>,
    expectation: OperationExpectation,
    stdout: String,
    diagnostics: Vec<CompileDiagnostic>,
}

impl CompileShape {
    fn artifact_paths(&self, source: &str) -> BTreeSet<String> {
        let source = normalize_path(source);
        let mut paths = BTreeSet::from([self.stdout.clone()]);
        if self.expected_exit != ExpectedExit::Failure {
            paths.extend(generation_package_artifacts(&source, &self.args));
        }
        paths.extend(compile_preprocessor_dump_paths(&self.args));
        if let Some(module) = self.module.as_deref() {
            paths.extend(compile_dump_paths(&self.args, module));
            if self.produces_verilog_outputs() && self.uses_verilog_backend() {
                paths.insert(format!("{module}.v"));
                if self.produces_elaboration_outputs() {
                    paths.insert(format!("{module}.ba"));
                }
            }
        }
        paths
    }

    fn produces_elaboration_outputs(&self) -> bool {
        self.expected_exit != ExpectedExit::Failure
            && self.uses_verilog_backend()
            && self.args.iter().any(|argument| argument == "-elab")
    }

    fn uses_verilog_backend(&self) -> bool {
        matches!(
            self.mode,
            BscCompileMode::Verilog | BscCompileMode::VerilogSchedule | BscCompileMode::Synthesize
        ) || self.args.iter().any(|argument| argument == "-verilog")
    }

    fn produces_verilog_outputs(&self) -> bool {
        !self
            .args
            .iter()
            .any(|argument| argument.starts_with("-KILL"))
            && (self.expected_exit == ExpectedExit::Success
                || self
                    .args
                    .iter()
                    .any(|argument| argument == "-continue-after-errors"))
    }

    fn generated_artifact_profile(
        &self,
        working_directory: Option<&str>,
    ) -> GeneratedArtifactProfile {
        let successful = !matches!(self.expected_exit, ExpectedExit::Failure);
        let file_output_directories = compile_output_directories(&self.args);
        let verilog = self.produces_verilog_outputs() && self.uses_verilog_backend();
        GeneratedArtifactProfile {
            verilog,
            schedule: verilog,
            dynamic_output: successful && !matches!(self.mode, BscCompileMode::BluesimObject),
            sal: successful
                && self
                    .args
                    .iter()
                    .any(|argument| argument.starts_with("-ddumpSAL=")),
            working_directory: working_directory.map(normalize_path),
            file_output_directories,
        }
    }
}

fn synthesize_hierarchy2_case_set(script: &mut ScriptManifest, fixture_root: &Path) {
    const ORIGIN: &str = "testsuite/bsc.bluetcl/hierarchy2/hierarchy2.exp";
    const AUDITED_SHA256: &str = "6b3da6f58931fb0727423cb2a4654e3e545fbbd024ad01889f70053824494ebd";
    if script.origin != ORIGIN || script.source_sha256 != AUDITED_SHA256 {
        return;
    }
    let audited_unsupported = script.unsupported.len() == 2
        && script
            .unsupported
            .iter()
            .any(|unsupported| unsupported.command.as_deref() == Some("set"))
        && script
            .unsupported
            .iter()
            .any(|unsupported| unsupported.command.as_deref() == Some("foreach"));
    let otherwise_empty = script.contracts.is_empty()
        && script.assertions.is_empty()
        && script.comparisons.is_empty()
        && script.bluesim_sequences.is_empty()
        && script.bluesim_workflows.is_empty()
        && script.systemc_workflows.is_empty()
        && script.workflow_actions.is_empty()
        && script.make_test_data_actions.is_empty();
    if !audited_unsupported || !otherwise_empty {
        return;
    }
    let span = script
        .unsupported
        .iter()
        .find(|unsupported| unsupported.command.as_deref() == Some("foreach"))
        .map(|unsupported| unsupported.span)
        .unwrap_or(ManifestSourceSpan {
            start_byte: 0,
            end_byte: 0,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        });
    script.unsupported.clear();

    let mut stems = Vec::new();
    if let Ok(entries) = fs::read_dir(fixture_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("bsv") {
                continue;
            }
            let is_regular_file = fs::symlink_metadata(&path)
                .map(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
                .unwrap_or(false);
            if !is_regular_file {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                stems.push(stem.to_owned());
            }
        }
    }
    stems.sort();

    for stem in stems {
        script.contracts.push(Contract::Compile(CompileContract {
            source: format!("{stem}.bsv"),
            working_directory: None,
            helper: "bsc_compile_verilog".to_owned(),
            arguments: vec![format!("{stem}.bsv"), String::new(), "-elab".to_owned()],
            guard: Guard::Capability {
                capability: Capability::Verilog,
            },
            span,
            expansion: Vec::new(),
        }));
        for (syntax, suffix) in [
            (crate::model::BluetclSyntax::Bsv, "bluetcl-out"),
            (crate::model::BluetclSyntax::Bh, "bluetcl-bh-out"),
        ] {
            let stdout = format!("ShowH.tcl_sys{stem}.{suffix}");
            script.workflow_actions.push(WorkflowAction::BluetclRun(
                crate::model::BluetclRunAction {
                    invocation: crate::model::BluetclInvocation::Script {
                        script: "ShowH.tcl".to_owned(),
                        args: vec![format!("sys{stem}")],
                        syntax,
                    },
                    working_directory: None,
                    artifact_inputs: vec![format!("sys{stem}.ba")],
                    artifact_outputs: Vec::new(),
                    expected_exit: ExpectedExit::Success,
                    stdout: stdout.clone(),
                    guard: Guard::Capability {
                        capability: Capability::Verilog,
                    },
                    span,
                    expansion: Vec::new(),
                },
            ));
            script.comparisons.push(crate::model::ComparisonContract {
                helper: "compare_bluetcl".to_owned(),
                arguments: vec![stdout],
                guard: Guard::Capability {
                    capability: Capability::Verilog,
                },
                span,
                expansion: Vec::new(),
            });
        }
    }
}

fn empty_manifest_span() -> ManifestSourceSpan {
    ManifestSourceSpan {
        start_byte: 0,
        end_byte: 0,
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 1,
    }
}

fn closed_target_bluetcl_scenarios(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<Option<Vec<ImportedScenario>>, ImportDiagnostic> {
    match script.origin.as_str() {
        "testsuite/bsc.bluetcl/packages/InstSynth/InstSynth.exp" => {
            closed_instsynth_scenario(script, fixture_root).map(|scenario| Some(vec![scenario]))
        }
        "testsuite/bsc.bluetcl/packages/expandPorts/expandPorts.exp" => {
            closed_expand_ports_scenarios(script, fixture_root).map(Some)
        }
        MAKEDEPEND_PLAN_ORIGIN => {
            closed_makedepend_scenario(script).map(|scenario| Some(vec![scenario]))
        }
        _ => Ok(None),
    }
}

const MAKEDEPEND_PLAN_ORIGIN: &str = "testsuite/bsc.bluetcl/packages/makedepend/makedepend.exp";

fn closed_compile_operation(
    contract: &CompileContract,
    fixture_root: &Path,
) -> Result<(OperationRecord, BTreeSet<String>, BscCompileMode), String> {
    let shape = compile_shape(contract)?;
    let paths = compile_artifact_paths(&shape, &contract.source, fixture_root);
    let mut operation = OperationRecord::new(
        Action::BscCompile {
            source: normalize_path(&contract.source),
            working_directory: contract.working_directory.clone(),
            mode: shape.mode,
            module: shape.module,
            args: shape.args,
            absolute_import_paths: Vec::new(),
            dependency_mode: shape.dependency_mode,
            expected_exit: shape.expected_exit,
            unexpected_success_forbidden_regex: shape.unexpected_success_forbidden_regex,
            environment: None,
            stdout: shape.stdout,
        },
        shape.expectation,
        provenance(contract.span, &contract.expansion),
    );
    attach_bluetcl_package_requirement(&mut operation, &contract.guard);
    for output in &paths {
        if !operation.artifacts.outputs.contains(output) {
            operation.artifacts.outputs.push(output.clone());
        }
    }
    Ok((operation, paths, shape.mode))
}

fn append_comparison_golden(consumption: &mut ImportConsumption, operation: &OperationRecord) {
    consumption.golden_paths.extend(
        operation
            .action
            .expected_paths()
            .into_iter()
            .map(str::to_owned),
    );
}

fn closed_instsynth_scenario(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let span = script
        .contracts
        .first()
        .map(contract_source_span)
        .or_else(|| script.workflow_actions.first().map(action_span))
        .unwrap_or_else(empty_manifest_span);
    let fail = |message: String| error_diagnostic("import.instsynth", message, span, &[]);
    if !script.unsupported.is_empty()
        || script.contracts.len() != 2
        || script.workflow_actions.len() != 2
        || script.comparisons.len() != 4
        || script.assertions.len() != 5
        || !script.bluesim_sequences.is_empty()
        || !script.bluesim_workflows.is_empty()
        || !script.systemc_workflows.is_empty()
    {
        return Err(fail(
            "InstSynth no longer matches the audited package/run/compile/check shape".to_owned(),
        ));
    }
    let package_guard = Guard::Capability {
        capability: Capability::BluetclPackage(BluetclPackage::InstSynth),
    };
    if script
        .contracts
        .iter()
        .any(|contract| contract.guard() != &package_guard)
        || script
            .workflow_actions
            .iter()
            .any(|action| action.guard() != &package_guard)
    {
        return Err(fail("InstSynth package guard changed".to_owned()));
    }

    let mut operations = Vec::new();
    let mut consumption = ImportConsumption::default();
    for index in 0..2 {
        let WorkflowAction::BluetclRun(run) = &script.workflow_actions[index] else {
            return Err(fail(
                "InstSynth requires exactly two typed Bluetcl runs".to_owned(),
            ));
        };
        operations.push(map_action(&script.workflow_actions[index]).map_err(&fail)?);
        consumption.actions.push(index);
        let comparison = &script.comparisons[index];
        if comparison.arguments.first() != Some(&run.stdout) {
            return Err(fail(
                "InstSynth Bluetcl output comparison is not bound".to_owned(),
            ));
        }
        let comparison = map_comparison(comparison).map_err(&fail)?;
        append_comparison_golden(&mut consumption, &comparison);
        operations.push(comparison);
        consumption.comparisons.push(index);
    }
    for index in 2..4 {
        let comparison = map_comparison(&script.comparisons[index]).map_err(&fail)?;
        append_comparison_golden(&mut consumption, &comparison);
        operations.push(comparison);
        consumption.comparisons.push(index);
    }

    let mut prior_compile_output = None;
    for (index, contract) in script.contracts.iter().enumerate() {
        let Contract::Compile(contract) = contract else {
            return Err(fail("InstSynth requires compile contracts only".to_owned()));
        };
        let (mut operation, paths, mode) =
            closed_compile_operation(contract, fixture_root).map_err(&fail)?;
        let expected_mode = if index == 0 {
            mode == BscCompileMode::Frontend
        } else {
            matches!(
                mode,
                BscCompileMode::Verilog | BscCompileMode::VerilogSchedule
            )
        };
        if !expected_mode {
            return Err(fail("InstSynth compile helper mode changed".to_owned()));
        }
        if index == 0 {
            for generated in ["FIFO.include.bsv", "FIFOLevel.include.bsv"] {
                if !operation.artifacts.inputs.contains(&generated.to_owned()) {
                    operation.artifacts.inputs.push(generated.to_owned());
                }
            }
            prior_compile_output = paths
                .iter()
                .find(|path| path.as_str() == "Inst_auto.bo")
                .cloned();
        } else {
            let prior = prior_compile_output.clone().ok_or_else(|| {
                fail("InstSynth first compile does not produce Inst_auto.bo".to_owned())
            })?;
            operation.artifacts.inputs.push(prior);
        }
        operations.push(operation);
    }
    for (index, assertion) in script.assertions.iter().enumerate() {
        operations.push(map_assertion(assertion).map_err(&fail)?);
        consumption.assertions.push(index);
    }

    Ok(ImportedScenario {
        scenario: Scenario {
            id: "instsynth-package-episode".to_owned(),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: vec![Requirement::Verilog, Requirement::Bluetcl],
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "ordered-package-run-compile-checks".to_owned(),
                operations,
            }],
        },
        consumption,
    })
}

fn closed_expand_ports_scenarios(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<Vec<ImportedScenario>, ImportDiagnostic> {
    let span = script
        .contracts
        .first()
        .map(contract_source_span)
        .unwrap_or_else(empty_manifest_span);
    let fail = |message: String| error_diagnostic("import.expand_ports", message, span, &[]);
    if !script.unsupported.is_empty()
        || script.contracts.len() != 13
        || script.workflow_actions.len() != 13
        || script.comparisons.len() != 26
        || !script.assertions.is_empty()
    {
        return Err(fail(
            "expandPorts no longer matches the audited finite 13-case loop".to_owned(),
        ));
    }
    let mut imported = Vec::new();
    for index in 0..13 {
        let Contract::Compile(contract) = &script.contracts[index] else {
            return Err(fail(
                "expandPorts loop requires compile contracts only".to_owned(),
            ));
        };
        let WorkflowAction::BluetclRun(run) = &script.workflow_actions[index] else {
            return Err(fail(
                "expandPorts loop requires one Bluetcl run per compile".to_owned(),
            ));
        };
        if !matches!(
            run.invocation,
            crate::model::BluetclInvocation::InstalledScript {
                script: BluetclInstalledScript::ExpandPorts,
                ..
            }
        ) {
            return Err(fail(
                "expandPorts installed-script contract changed".to_owned(),
            ));
        }
        let (mut compile, _, mode) =
            closed_compile_operation(contract, fixture_root).map_err(&fail)?;
        if mode != BscCompileMode::Frontend {
            return Err(fail("expandPorts compile helper mode changed".to_owned()));
        }
        let package = contract.source.trim_end_matches(".bsv");
        let module = format!("mk{package}");
        let expected_inputs = BTreeSet::from([
            format!("{package}.bo"),
            format!("{module}.ba"),
            format!("{module}.v"),
        ]);
        let actual_inputs = run
            .artifact_inputs
            .iter()
            .filter(|input| !input.ends_with(".rename.tcl"))
            .cloned()
            .collect::<BTreeSet<_>>();
        if actual_inputs != expected_inputs {
            return Err(fail(format!(
                "expandPorts compile outputs changed for {}",
                contract.source
            )));
        }
        for input in actual_inputs {
            if !compile.artifacts.outputs.contains(&input) {
                compile.artifacts.outputs.push(input);
            }
        }
        let mut operations = vec![
            compile,
            map_action(&script.workflow_actions[index]).map_err(&fail)?,
        ];
        let mut consumption = ImportConsumption {
            actions: vec![index],
            ..ImportConsumption::default()
        };
        for comparison_index in [index * 2, index * 2 + 1] {
            let comparison =
                map_comparison(&script.comparisons[comparison_index]).map_err(&fail)?;
            append_comparison_golden(&mut consumption, &comparison);
            operations.push(comparison);
            consumption.comparisons.push(comparison_index);
        }
        let stem = contract.source.trim_end_matches(".bsv");
        imported.push(ImportedScenario {
            scenario: Scenario {
                id: format!("expand-ports-{stem}"),
                resource: ResourceClass::Normal,
                fixtures: Vec::new(),
                requires: vec![Requirement::Verilog, Requirement::Bluetcl],
                bsc_options_append: None,
                timeouts: Timeouts::default(),
                stages: vec![Stage {
                    id: "compile-expand-compare".to_owned(),
                    operations,
                }],
            },
            consumption,
        });
    }
    Ok(imported)
}

fn closed_makedepend_scenario(
    script: &ScriptManifest,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let span = script
        .workflow_actions
        .first()
        .map(action_span)
        .unwrap_or_else(empty_manifest_span);
    let fail = |message: String| error_diagnostic("import.makedepend", message, span, &[]);
    if !script.contracts.is_empty()
        || !script.unsupported.is_empty()
        || !script.assertions.is_empty()
        || script.workflow_actions.len() != 13
        || script.comparisons.len() != 13
    {
        return Err(fail(
            "makedepend no longer matches the audited 12 invocations, mkdir, and 13 comparisons"
                .to_owned(),
        ));
    }
    #[derive(Clone, Copy)]
    enum Event {
        Action(usize),
        Comparison(usize),
    }
    let mut events = script
        .workflow_actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            (
                execution_order_key(action_span(action), action_expansion(action)),
                Event::Action(index),
            )
        })
        .chain(
            script
                .comparisons
                .iter()
                .enumerate()
                .map(|(index, comparison)| {
                    (
                        execution_order_key(comparison.span, &comparison.expansion),
                        Event::Comparison(index),
                    )
                }),
        )
        .collect::<Vec<_>>();
    events.sort_by(|left, right| left.0.cmp(&right.0));

    let mut operations = Vec::new();
    let mut consumption = ImportConsumption::default();
    let mut staged_updir = false;
    for (_, event) in events {
        match event {
            Event::Action(index) => {
                if let WorkflowAction::BluetclRun(run) = &script.workflow_actions[index] {
                    if run.working_directory.as_deref() == Some("makedepend") && !staged_updir {
                        let provenance = provenance(run.span, &run.expansion);
                        operations.push(OperationRecord::new(
                            Action::FsMkdir {
                                path: "makedepend".to_owned(),
                            },
                            OperationExpectation::Required,
                            provenance.clone(),
                        ));
                        for input in &run.artifact_inputs {
                            let source = input
                                .strip_prefix("makedepend/")
                                .ok_or_else(|| fail("unsafe makedepend staged input".to_owned()))?;
                            operations.push(OperationRecord::new(
                                Action::FsCopy {
                                    source: source.to_owned(),
                                    destination: input.clone(),
                                },
                                OperationExpectation::Required,
                                provenance.clone(),
                            ));
                        }
                        operations.push(OperationRecord::new(
                            Action::FsMkdir {
                                path: "makedepend/objs".to_owned(),
                            },
                            OperationExpectation::Required,
                            provenance,
                        ));
                        staged_updir = true;
                    }
                }
                operations.push(map_action(&script.workflow_actions[index]).map_err(&fail)?);
                consumption.actions.push(index);
            }
            Event::Comparison(index) => {
                let comparison = map_comparison(&script.comparisons[index]).map_err(&fail)?;
                append_comparison_golden(&mut consumption, &comparison);
                operations.push(comparison);
                consumption.comparisons.push(index);
            }
        }
    }
    if !staged_updir {
        return Err(fail("makedepend updir workspace was not staged".to_owned()));
    }
    Ok(ImportedScenario {
        scenario: Scenario {
            id: "makedepend-static-invocations".to_owned(),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: vec![Requirement::Bluetcl],
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "ordered-makedepend-invocations".to_owned(),
                operations,
            }],
        },
        consumption,
    })
}

fn closed_bsc_compile_bluetcl_scenario(
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<Option<ImportedScenario>, ImportDiagnostic> {
    if !matches!(
        script.origin.as_str(),
        "testsuite/bsc.bluetcl/commands/commands.exp"
            | "testsuite/bsc.bluetcl/hierarchy/hierarchy.exp"
            | "testsuite/bsc.bluetcl/hierarchy2/hierarchy2.exp"
            | "testsuite/bsc.bluetcl/targeted/port_types/port_types.exp"
            | "testsuite/bsc.bluetcl/targeted/type/type.exp"
    ) {
        return Ok(None);
    }
    let Some(first_contract) = script.contracts.first() else {
        return Ok(None);
    };
    let (first_span, first_expansion) = match first_contract {
        Contract::Compile(contract) => (contract.span, contract.expansion.as_slice()),
        _ => return Ok(None),
    };
    let fail = |message: String| {
        error_diagnostic(
            "import.bsc_compile_bluetcl",
            message,
            first_span,
            first_expansion,
        )
    };
    if !script.unsupported.is_empty()
        || !script.assertions.is_empty()
        || !script.bluesim_sequences.is_empty()
        || !script.bluesim_workflows.is_empty()
        || !script.systemc_workflows.is_empty()
        || script.contracts.iter().any(|contract| {
            !matches!(contract, Contract::Compile(contract) if matches!(contract.helper.as_str(), "bsc_compile" | "bsc_compile_verilog") && contract.guard.is_resolved())
        })
        || script.workflow_actions.iter().any(|action| {
            !matches!(
                action,
                WorkflowAction::BluetclRun(run) if run.guard.is_resolved()
            ) && !matches!(
                action,
                WorkflowAction::CreateDirectory(directory)
                    if directory.guard.is_resolved()
                        && normalize_path(&directory.path) == "BOUTDIR"
            )
        })
    {
        return Ok(None);
    }

    #[derive(Clone, Copy)]
    enum Event {
        Compile(usize),
        Action(usize),
    }

    let mut events = script
        .contracts
        .iter()
        .enumerate()
        .filter_map(|(index, contract)| match contract {
            Contract::Compile(contract) => Some((
                execution_order_key(contract.span, &contract.expansion),
                Event::Compile(index),
            )),
            _ => None,
        })
        .chain(
            script
                .workflow_actions
                .iter()
                .enumerate()
                .map(|(index, action)| {
                    (
                        execution_order_key(action_span(action), action_expansion(action)),
                        Event::Action(index),
                    )
                }),
        )
        .collect::<Vec<_>>();
    events.sort_by(|left, right| left.0.cmp(&right.0));

    let mut operations = Vec::<OperationRecord>::new();
    let mut producers = Vec::<(usize, String, BTreeSet<String>)>::new();
    let mut requirements = BTreeSet::from([Requirement::Bluetcl]);
    let mut consumption = ImportConsumption::default();
    let mut consumed_comparisons = BTreeSet::new();

    for (_, event) in events {
        match event {
            Event::Compile(contract_index) => {
                let Contract::Compile(contract) = &script.contracts[contract_index] else {
                    unreachable!("closed Bluetcl compile event references a compile contract")
                };
                let shape = compile_shape(contract).map_err(&fail)?;
                let ignored_result_helper = contract.helper == "bsc_compile";
                if ignored_result_helper && shape.expected_exit != ExpectedExit::Unchecked {
                    return Err(fail(
                        "bsc_compile must preserve its ignored Boolean result".to_owned(),
                    ));
                }
                collect_requirements(&contract.guard, &mut requirements).map_err(&fail)?;
                if shape.uses_verilog_backend() {
                    requirements.insert(Requirement::Verilog);
                }
                let operation_index = operations.len();
                operations.push(OperationRecord::new(
                    Action::BscCompile {
                        source: normalize_path(&contract.source),
                        working_directory: contract.working_directory.clone(),
                        mode: shape.mode,
                        module: shape.module.clone(),
                        args: shape.args.clone(),
                        absolute_import_paths: Vec::new(),
                        dependency_mode: shape.dependency_mode,
                        expected_exit: shape.expected_exit,
                        unexpected_success_forbidden_regex: None,
                        environment: None,
                        stdout: shape.stdout.clone(),
                    },
                    OperationExpectation::Required,
                    provenance(contract.span, &contract.expansion),
                ));
                producers.push((
                    operation_index,
                    normalize_path(&contract.source),
                    compile_artifact_paths(&shape, &contract.source, fixture_root),
                ));
            }
            Event::Action(action_index) => match &script.workflow_actions[action_index] {
                WorkflowAction::CreateDirectory(directory) => {
                    let mut operation =
                        map_action(&script.workflow_actions[action_index]).map_err(&fail)?;
                    if script.origin == "testsuite/bsc.bluetcl/commands/commands.exp"
                        && normalize_path(&directory.path) == "BOUTDIR"
                    {
                        operation.requires.push(Requirement::NonWindows);
                    }
                    operations.push(operation);
                    consumption.actions.push(action_index);
                    collect_requirements(&directory.guard, &mut requirements).map_err(&fail)?;
                }
                WorkflowAction::BluetclRun(run) => {
                    collect_requirements(&run.guard, &mut requirements).map_err(&fail)?;
                    for input in &run.artifact_inputs {
                        let input = normalize_path(input);
                        if !is_safe_relative(&input) {
                            return Err(fail(format!(
                                "bluetcl.run input must be a safe relative path: {input:?}"
                            )));
                        }
                        let matches = producers
                            .iter()
                            .filter(|(_, source, paths)| {
                                paths.contains(&input)
                                    || closed_bsc_compile_artifact(&script.origin, source, &input)
                                    || (script.origin
                                        == "testsuite/bsc.bluetcl/hierarchy2/hierarchy2.exp"
                                        && input
                                            == format!(
                                                "sys{}.ba",
                                                Path::new(source)
                                                    .file_stem()
                                                    .and_then(|stem| stem.to_str())
                                                    .unwrap_or_default()
                                            ))
                            })
                            .map(|(operation_index, _, _)| *operation_index)
                            .collect::<Vec<_>>();
                        match matches.as_slice() {
                            [producer] => {
                                if !operations[*producer].artifacts.outputs.contains(&input) {
                                    operations[*producer].artifacts.outputs.push(input);
                                }
                            }
                            [] if fixture_root.join(&input).is_file() => {}
                            [] => {
                                return Err(fail(format!(
                                "bluetcl.run input has no preceding producer or fixture: {input:?}"
                            )))
                            }
                            _ => {
                                return Err(fail(format!(
                                    "bluetcl.run input has multiple preceding producers: {input:?}"
                                )))
                            }
                        }
                    }
                    let non_windows_depend = script.origin
                        == "testsuite/bsc.bluetcl/commands/commands.exp"
                        && matches!(
                            &run.invocation,
                            crate::model::BluetclInvocation::Script { script, .. }
                                if script == "depend.tcl"
                        );
                    let mut run_operation =
                        map_action(&script.workflow_actions[action_index]).map_err(&fail)?;
                    if non_windows_depend {
                        run_operation.requires.push(Requirement::NonWindows);
                    }
                    operations.push(run_operation);
                    consumption.actions.push(action_index);
                    let comparison_matches = script
                        .comparisons
                        .iter()
                        .enumerate()
                        .filter(|(index, comparison)| {
                            !consumed_comparisons.contains(index)
                                && comparison.arguments.first() == Some(&run.stdout)
                                && comparison.guard == run.guard
                        })
                        .collect::<Vec<_>>();
                    let [(comparison_index, comparison)] = comparison_matches.as_slice() else {
                        return Err(fail(format!(
                            "bluetcl.run output {:?} requires exactly one ordered comparison",
                            run.stdout
                        )));
                    };
                    let mut comparison_operation = map_comparison(comparison).map_err(&fail)?;
                    if non_windows_depend {
                        comparison_operation.requires.push(Requirement::NonWindows);
                    }
                    consumption.golden_paths.extend(
                        comparison_operation
                            .artifacts
                            .inputs
                            .iter()
                            .filter(|path| path.ends_with(".expected"))
                            .cloned(),
                    );
                    operations.push(comparison_operation);
                    consumed_comparisons.insert(*comparison_index);
                    consumption.comparisons.push(*comparison_index);
                }
                _ => unreachable!("closed Bluetcl batch contains only mkdir and bluetcl.run"),
            },
        }
    }
    if consumed_comparisons.len() != script.comparisons.len() {
        return Err(fail(
            "closed Bluetcl batch left comparisons outside the ordered action stream".to_owned(),
        ));
    }

    Ok(Some(ImportedScenario {
        scenario: Scenario {
            id: "bsc-compile-bluetcl".to_owned(),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "ordered-bsc-compile-bluetcl".to_owned(),
                operations,
            }],
        },
        consumption,
    }))
}

fn closed_bsc_compile_artifact(origin: &str, source: &str, artifact: &str) -> bool {
    matches!(
        (origin, source, artifact),
        (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "Test.bsv",
            "Test.bo"
        ) | (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "Test.bsv",
            "mkT.ba"
        ) | (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "Test.bsv",
            "mkM.ba"
        ) | (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "Test.bsv",
            "mkS.ba"
        ) | (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "Test2.bsv",
            "mkTest.ba"
        ) | (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "TestSchedErr.bsv",
            "mkTestSchedErr.ba"
        ) | (
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "TaggedUnionPoly.bsv",
            "TaggedUnionPoly.bo"
        ) | (
            "testsuite/bsc.bluetcl/hierarchy/hierarchy.exp",
            "Design.bsv",
            "mkDesign.ba"
        ) | (
            "testsuite/bsc.bluetcl/hierarchy/hierarchy.exp",
            "Example.bsv",
            "mkExample.ba"
        ) | (
            "testsuite/bsc.bluetcl/targeted/port_types/port_types.exp",
            "InhighEnable.bsv",
            "sysInhighEnable.ba"
        ) | (
            "testsuite/bsc.bluetcl/targeted/port_types/port_types.exp",
            "ZeroSize.bsv",
            "sysZeroSize.ba"
        ) | (
            "testsuite/bsc.bluetcl/targeted/port_types/port_types.exp",
            "Prims.bsv",
            "sysPrims.ba"
        ) | (
            "testsuite/bsc.bluetcl/targeted/port_types/port_types.exp",
            "SplitPortTypes.bs",
            "mkSplitPortTypes.ba"
        ) | (
            "testsuite/bsc.bluetcl/targeted/type/type.exp",
            "PolyField.bsv",
            "PolyField.bo"
        )
    )
}

fn ovl_scenario(
    contract: &OvlContract,
    fixture_root: &Path,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let fail = |message: String| {
        error_diagnostic(
            "import.ovl_contract",
            message,
            contract.span,
            &contract.expansion,
        )
    };
    if contract.guard
        != (Guard::Capability {
            capability: Capability::Verilog,
        })
    {
        return Err(fail(
            "test_ovl requires the Verilog capability guard".to_owned(),
        ));
    }
    for path in [
        format!("{}/{}.bsv", contract.case_dir, contract.top),
        format!("{}/{}.out.expected", contract.case_dir, contract.top),
        format!("std_ovl/{}", contract.library),
    ] {
        if !is_safe_relative(&path)
            || !fs::symlink_metadata(fixture_root.join(&path))
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        {
            return Err(fail(format!(
                "test_ovl required fixture {path} is not a local regular file"
            )));
        }
    }
    let source = format!("{}/{}.bsv", contract.case_dir, contract.top);
    let golden = format!("{}/{}.out.expected", contract.case_dir, contract.top);
    let output = format!("{}.out", contract.top);
    let compile_output = format!("{}.bsv.bsc-vcomp-out", contract.top);
    let provenance = provenance(contract.span, &contract.expansion);
    let copy = |source: String, destination: String| {
        OperationRecord::new(
            Action::FsCopy {
                source,
                destination,
            },
            OperationExpectation::Required,
            provenance.clone(),
        )
    };
    let compile = OperationRecord::new(
        Action::BscCompile {
            source: format!("{}.bsv", contract.top),
            working_directory: None,
            mode: BscCompileMode::Verilog,
            module: Some(contract.top.clone()),
            args: Vec::new(),
            absolute_import_paths: Vec::new(),
            dependency_mode: DependencyMode::Update,
            expected_exit: ExpectedExit::Success,
            unexpected_success_forbidden_regex: None,
            environment: None,
            stdout: compile_output,
        },
        OperationExpectation::Required,
        provenance.clone(),
    );
    let mut link = OperationRecord::new(
        Action::BscLink {
            backend: PlanSimulationBackend::Icarus,
            mode: BscLinkMode::Standard,
            objects: Vec::new(),
            top: contract.top.clone(),
            args: vec![
                "-D".to_owned(),
                "OVL_VERILOG=1".to_owned(),
                "-D".to_owned(),
                "OVL_ASSERT_ON=1".to_owned(),
                "-vsearch".to_owned(),
                "std_ovl".to_owned(),
                "-Xv".to_owned(),
                format!("std_ovl/{}", contract.library),
            ],
            expected_exit: ExpectedExit::Success,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        },
        OperationExpectation::Required,
        provenance.clone(),
    );
    link.artifacts.inputs.extend(
        ovl_include_closure(fixture_root, &contract.library)
            .map_err(fail)?
            .into_iter()
            .filter(|path| path != &format!("std_ovl/{}", contract.library)),
    );
    link.artifacts.inputs.sort();
    link.artifacts.inputs.dedup();
    let run = OperationRecord::new(
        Action::SimulationRun {
            backend: PlanSimulationBackend::Icarus,
            executable: contract.top.clone(),
            args: Vec::new(),
            stdout: output.clone(),
            expected_exits: ExpectedExitSet::default(),
            vcd: None,
        },
        OperationExpectation::Required,
        provenance.clone(),
    );
    let comparison = OperationRecord::new(
        Action::AssertGolden {
            actual: output,
            expected: format!("{}.out.expected", contract.top),
        },
        OperationExpectation::Required,
        provenance.clone(),
    );
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("ovl-{}", contract.top),
            resource: ResourceClass::Heavy,
            fixtures: Vec::new(),
            requires: vec![Requirement::Icarus, Requirement::Verilog],
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![
                Stage {
                    id: format!("prepare-{}", contract.top),
                    operations: vec![
                        copy(source, format!("{}.bsv", contract.top)),
                        copy(golden, format!("{}.out.expected", contract.top)),
                    ],
                },
                Stage {
                    id: format!("verilog-{}", contract.top),
                    operations: vec![compile, link, run, comparison],
                },
            ],
        },
        consumption: ImportConsumption::default(),
    })
}

fn ovl_include_closure(fixture_root: &Path, library: &str) -> Result<BTreeSet<String>, String> {
    let root = "std_ovl";
    let mut pending = vec![format!("{root}/{library}")];
    let mut closure = BTreeSet::new();
    let include = Regex::new(r#"(?m)^\s*`include\s+\"([^\"]+)\""#)
        .expect("valid literal Verilog include regex");

    while let Some(path) = pending.pop() {
        if !closure.insert(path.clone()) {
            continue;
        }
        let metadata = fs::symlink_metadata(fixture_root.join(&path)).map_err(|error| {
            format!("test_ovl dependency {path} is not a local regular fixture: {error}")
        })?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!(
                "test_ovl dependency {path} is not a local regular fixture"
            ));
        }
        let contents = fs::read_to_string(fixture_root.join(&path))
            .map_err(|error| format!("could not read test_ovl dependency {path}: {error}"))?;
        let parent = Path::new(&path).parent().unwrap_or_else(|| Path::new(""));
        for capture in include.captures_iter(&contents) {
            let value = capture.get(1).expect("include capture exists").as_str();
            let joined = parent.join(value);
            let normalized = joined
                .components()
                .map(|component| match component {
                    Component::Normal(segment) => segment.to_string_lossy().to_string(),
                    Component::CurDir => String::new(),
                    _ => "..".to_owned(),
                })
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>();
            if normalized.is_empty()
                || normalized.iter().any(|segment| segment == "..")
                || normalized.first().is_none_or(|segment| segment != root)
            {
                return Err(format!(
                    "test_ovl include {value:?} in {path} escapes the closed std_ovl fixture root"
                ));
            }
            pending.push(normalized.join("/"));
        }
    }
    Ok(closure)
}

fn prepend_rendered_simulation_golden(
    render: &RenderGoldenContract,
    simulation: &SimulationContract,
    script: &ScriptManifest,
    fixture_root: &Path,
    mut imported: ImportedScenario,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let fail = |message: String| {
        error_diagnostic(
            "import.render_golden_contract",
            message,
            render.span,
            &render.expansion,
        )
    };
    let template = normalize_path(&render.template);
    let output = normalize_path(&render.output);
    if !render.guard.is_resolved() || render.guard != simulation.guard {
        return Err(fail(
            "golden derivation and simulation must have the same resolved guard".to_owned(),
        ));
    }
    if !is_safe_relative(&template) || !is_safe_relative(&output) {
        return Err(fail(
            "golden derivation paths must be safe relative paths".to_owned(),
        ));
    }
    if template.eq_ignore_ascii_case(&output) {
        return Err(fail(
            "golden derivation input and output must not collide on Windows".to_owned(),
        ));
    }
    if !fs::symlink_metadata(fixture_root.join(&template))
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
    {
        return Err(fail(format!(
            "golden derivation input {template} is not a local regular fixture"
        )));
    }
    if imported.consumption.golden_paths.as_slice() != [output.as_str()] {
        return Err(fail(format!(
            "golden derivation output {output} must exactly match the simulation golden"
        )));
    }
    let render_order = contract_order_key(&Contract::RenderGolden(render.clone()));
    let simulation_order = contract_order_key(&Contract::Simulation(simulation.clone()));
    if render_order >= simulation_order
        || has_execution_barrier_between(script, &render_order, &simulation_order)
    {
        return Err(fail(
            "golden derivation and simulation must be adjacent without an execution barrier"
                .to_owned(),
        ));
    }
    let replacement = match render.macro_value {
        GoldenMacroValue::BluespecDir => GoldenReplacement::BluespecDir,
        GoldenMacroValue::WorkDir => GoldenReplacement::WorkDir,
        GoldenMacroValue::FifoWarningLocations => GoldenReplacement::FifoWarningLocations,
    };
    imported
        .scenario
        .stages
        .first_mut()
        .ok_or_else(|| fail("simulation scenario has no stage".to_owned()))?
        .operations
        .insert(
            0,
            OperationRecord::new(
                Action::RenderGolden {
                    template: template.clone(),
                    output,
                    replacement,
                },
                OperationExpectation::Required,
                provenance(render.span, &render.expansion),
            ),
        );
    imported.consumption.golden_paths = vec![template];
    Ok(imported)
}

fn rendered_basic_options_scenario(
    renders: &[&RenderGoldenContract],
    options: &BasicOptionsContract,
    script: &ScriptManifest,
    fixture_root: &Path,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let Some(first) = renders.first() else {
        return basic_options_scenario(options, &[], fixture_root);
    };
    let fail = |message: String| {
        error_diagnostic(
            "import.render_golden_contract",
            message,
            first.span,
            &first.expansion,
        )
    };
    if renders
        .iter()
        .any(|render| !render.guard.is_resolved() || render.guard != options.guard)
    {
        return Err(fail(
            "m4_process and test_basic_options must have the same resolved guard".to_owned(),
        ));
    }
    for pair in renders.windows(2) {
        if normalize_path(&pair[0].output) != normalize_path(&pair[1].template) {
            return Err(fail(
                "chained m4_process outputs must feed the next template exactly".to_owned(),
            ));
        }
    }
    let template = normalize_path(&first.template);
    let rendered_output = normalize_path(&renders.last().expect("non-empty render chain").output);
    let expected = normalize_path(&options.expected);
    if rendered_output != expected {
        return Err(fail(
            "m4_process output must exactly match the following test_basic_options expected path"
                .to_owned(),
        ));
    }
    for render in renders {
        let input = normalize_path(&render.template);
        let output = normalize_path(&render.output);
        if !is_safe_relative(&input) || !is_safe_relative(&output) {
            return Err(fail(
                "m4_process template and output paths must be safe relative paths".to_owned(),
            ));
        }
        if input.eq_ignore_ascii_case(&output) {
            return Err(fail(
                "m4_process template and output paths must not collide on Windows".to_owned(),
            ));
        }
    }
    if !fs::symlink_metadata(fixture_root.join(&template))
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
    {
        return Err(fail(format!(
            "m4_process template {template} is not a local regular fixture"
        )));
    }
    let options_order = contract_order_key(&Contract::BasicOptions(options.clone()));
    for render in renders {
        let render_order = contract_order_key(&Contract::RenderGolden((*render).clone()));
        if render_order >= options_order
            || (!is_pinned_options_plan(script)
                && has_execution_barrier_between(script, &render_order, &options_order))
        {
            return Err(fail(
                "m4_process and test_basic_options must be adjacent without an execution barrier"
                    .to_owned(),
            ));
        }
    }
    basic_options_scenario(options, renders, fixture_root)
}

fn has_execution_barrier_between(
    script: &ScriptManifest,
    start: &ExecutionOrderKey,
    end: &ExecutionOrderKey,
) -> bool {
    let between = |order: ExecutionOrderKey| start < &order && &order < end;
    script
        .unsupported
        .iter()
        .any(|item| between(execution_order_key(item.span, &item.expansion)))
        || script.workflow_actions.iter().any(|item| {
            between(execution_order_key(
                action_span(item),
                action_expansion(item),
            ))
        })
        || script
            .assertions
            .iter()
            .any(|item| between(execution_order_key(item.span, &item.expansion)))
        || script
            .comparisons
            .iter()
            .any(|item| between(execution_order_key(item.span, &item.expansion)))
}

fn no_source_compile_scenario(
    contract: &NoSourceCompileContract,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let fail = |message: String| {
        error_diagnostic(
            "import.no_source_compile_contract",
            message,
            contract.span,
            &contract.expansion,
        )
    };
    if !contract.guard.is_resolved()
        || contract.name.is_empty()
        || contract.diagnostic.is_empty()
        || contract.count.parse::<u64>().is_err()
    {
        return Err(fail(
            "no-source compile failure contract is not fully static".to_owned(),
        ));
    }
    let args = parse_static_tcl_list(&contract.options).map_err(|error| {
        fail(format!(
            "could not parse no-source compile options: {error}"
        ))
    })?;
    let stdout = format!("{}.bsc-out", contract.name);
    let command = OperationRecord::new(
        Action::BscOptions {
            args,
            expected_exit: ExpectedExit::Failure,
            bsc_options_prepend: None,
            stdout: stdout.clone(),
        },
        OperationExpectation::Required,
        provenance(contract.span, &contract.expansion),
    );
    let diagnostic = map_assertion(&AssertionContract {
        helper: "find_n_error".to_owned(),
        arguments: vec![stdout, contract.diagnostic.clone(), contract.count.clone()],
        guard: contract.guard.clone(),
        span: contract.span,
        expansion: contract.expansion.clone(),
    })
    .map_err(&fail)?;
    let mut requirements = BTreeSet::new();
    collect_requirements(&contract.guard, &mut requirements).map_err(&fail)?;
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("no-source-options-{}", contract.name),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("reject-options-{}", contract.name),
                operations: vec![command, diagnostic],
            }],
        },
        consumption: ImportConsumption::default(),
    })
}

fn basic_options_scenario(
    contract: &BasicOptionsContract,
    renders: &[&RenderGoldenContract],
    _fixture_root: &Path,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let fail = |message: String| {
        error_diagnostic(
            "import.basic_options_contract",
            message,
            contract.span,
            &contract.expansion,
        )
    };
    if !contract.guard.is_resolved() {
        return Err(fail("test_basic_options has a dynamic guard".to_owned()));
    }
    let output = normalize_path(&contract.output);
    let expected = normalize_path(&contract.expected);
    if !is_safe_relative(&output) || !is_safe_relative(&expected) {
        return Err(fail(
            "test_basic_options output and expected paths must be safe relative paths".to_owned(),
        ));
    }
    if output.eq_ignore_ascii_case(&expected) {
        return Err(fail(
            "test_basic_options output and expected paths must not collide on Windows".to_owned(),
        ));
    }
    let args = parse_static_tcl_list(&contract.options).map_err(|error| {
        fail(format!(
            "could not parse test_basic_options options: {error}"
        ))
    })?;
    let mut requirements = BTreeSet::new();
    collect_requirements(&contract.guard, &mut requirements).map_err(&fail)?;
    if path_requires_non_windows(&output) || path_requires_non_windows(&expected) {
        requirements.insert(Requirement::NonWindows);
    }
    let stem = Path::new(&output)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| fail("test_basic_options output has no portable stem".to_owned()))?
        .to_owned();
    let operation = OperationRecord::new(
        Action::BscOptions {
            args,
            expected_exit: ExpectedExit::Success,
            bsc_options_prepend: (contract.output == "bsc.test_bsc_option.out"
                && contract.span.start_line == 311)
                .then(|| "-print-flags -vsearch foo -steps 12345678".to_owned()),
            stdout: output.clone(),
        },
        OperationExpectation::Required,
        provenance(contract.span, &contract.expansion),
    );
    let render_operations = renders.iter().map(|render| {
        OperationRecord::new(
            Action::RenderGolden {
                template: normalize_path(&render.template),
                output: normalize_path(&render.output),
                replacement: match render.macro_value {
                    GoldenMacroValue::BluespecDir => GoldenReplacement::BluespecDir,
                    GoldenMacroValue::WorkDir => GoldenReplacement::WorkDir,
                    GoldenMacroValue::FifoWarningLocations => {
                        GoldenReplacement::FifoWarningLocations
                    }
                },
            },
            OperationExpectation::Required,
            provenance(render.span, &render.expansion),
        )
    });
    let comparison = OperationRecord::new(
        Action::AssertGolden {
            actual: output,
            expected: expected.clone(),
        },
        OperationExpectation::Required,
        provenance(contract.span, &contract.expansion),
    );
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("basic-options-{stem}"),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("basic-options-{stem}"),
                operations: render_operations.chain([operation, comparison]).collect(),
            }],
        },
        consumption: ImportConsumption {
            golden_paths: vec![renders
                .first()
                .map_or(expected, |render| normalize_path(&render.template))],
            ..ImportConsumption::default()
        },
    })
}

fn compile_scenario(
    contract_index: usize,
    contract: &CompileContract,
    pinned_options: bool,
    defer_directory_option_binding: bool,
    previous_contract_order: Option<&ExecutionOrderKey>,
    workflow_actions: &[WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    next_contract_order: Option<&ExecutionOrderKey>,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    bindings: Option<&Vec<BoundCheck>>,
    bound_workflow_actions: Option<&BTreeSet<usize>>,
    static_fixture_sources: &BTreeSet<String>,
    unsupported: &[UnsupportedConstruct],
    fixture_root: &Path,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let fail = |message: String| {
        error_diagnostic(
            "import.compile_contract",
            message,
            contract.span,
            &contract.expansion,
        )
    };
    let shape = compile_shape(contract).map_err(&fail)?;
    let artifact_paths = compile_artifact_paths(&shape, &contract.source, fixture_root)
        .into_iter()
        .map(|path| compile_contract_path(contract, &path))
        .collect::<BTreeSet<_>>();
    let generated_artifacts =
        shape.generated_artifact_profile(contract.working_directory.as_deref());
    let mut requirements = BTreeSet::new();
    collect_requirements(&contract.guard, &mut requirements).map_err(&fail)?;
    match shape.mode {
        BscCompileMode::Frontend => {}
        BscCompileMode::BluesimObject => {
            requirements.insert(Requirement::Bluesim);
        }
        BscCompileMode::Verilog | BscCompileMode::VerilogSchedule | BscCompileMode::Synthesize => {
            requirements.insert(Requirement::Verilog);
        }
    }

    let mut compile_operation = OperationRecord::new(
        Action::BscCompile {
            source: normalize_path(&contract.source),
            working_directory: contract.working_directory.clone(),
            mode: shape.mode,
            module: shape.module,
            args: shape.args.clone(),
            absolute_import_paths: (pinned_options && contract.span.start_line == 201)
                .then(|| "incfiles".to_owned())
                .into_iter()
                .collect(),
            dependency_mode: shape.dependency_mode,
            expected_exit: shape.expected_exit,
            unexpected_success_forbidden_regex: shape.unexpected_success_forbidden_regex,
            environment: (contract.helper == "compile_verilog_pass_ghcrts_m1_2g")
                .then_some(BscCompileEnvironment::GhcrtsM1_2g),
            stdout: shape.stdout.clone(),
        },
        shape.expectation.clone(),
        provenance(contract.span, &contract.expansion),
    );
    attach_bluetcl_package_requirement(&mut compile_operation, &contract.guard);
    for output in &artifact_paths {
        if !compile_operation.artifacts.outputs.contains(output) {
            compile_operation.artifacts.outputs.push(output.clone());
        }
    }
    let mut operations = Vec::new();
    let mut consumption = ImportConsumption::default();
    let contract_order = execution_order_key(contract.span, &contract.expansion);
    let preceding_window = ProvenanceWindow {
        after: previous_contract_order,
        before: Some(&contract_order),
    };
    let mut preceding_actions = if defer_directory_option_binding {
        Vec::new()
    } else {
        closed_preceding_directory_actions(
            &contract.guard,
            workflow_actions,
            already_consumed_actions,
            unsupported,
            assertions,
            comparisons,
            preceding_window,
        )
    };
    if defer_directory_option_binding {
        compile_directory_options(&shape.args).map_err(&fail)?;
    } else {
        preceding_actions.extend(
            compile_preceding_directory_actions(
                &shape.args,
                &contract.guard,
                workflow_actions,
                already_consumed_actions,
                preceding_window,
            )
            .map_err(&fail)?,
        );
    }
    preceding_actions.extend(
        compile_preceding_fixture_copies(
            contract,
            workflow_actions,
            already_consumed_actions,
            previous_contract_order,
            &contract_order,
            assertions,
            comparisons,
            unsupported,
            static_fixture_sources,
        )
        .map_err(&fail)?,
    );
    if let Some(touch) = compile_preceding_fixture_touch(
        contract,
        workflow_actions,
        already_consumed_actions,
        previous_contract_order,
        &contract_order,
        assertions,
        comparisons,
        unsupported,
        static_fixture_sources,
    ) {
        preceding_actions.push(touch);
    }
    preceding_actions.sort_by_key(|(_, action)| {
        execution_order_key(action_span(action), action_expansion(action))
    });
    preceding_actions.dedup_by_key(|(index, _)| *index);
    for (index, action) in preceding_actions {
        match action {
            WorkflowAction::EnsureDirectoryAbsent(directory) => {
                let path = normalize_path(&directory.path);
                if path_requires_non_windows(&path) {
                    requirements.insert(Requirement::NonWindows);
                }
            }
            WorkflowAction::CreateDirectory(directory) => {
                let path = normalize_path(&directory.path);
                if path_requires_non_windows(&path) {
                    requirements.insert(Requirement::NonWindows);
                }
            }
            WorkflowAction::TransferArtifact(transfer) => {
                collect_requirements(&transfer.guard, &mut requirements).map_err(&fail)?;
                let source = normalize_path(&transfer.source);
                let destination = normalize_path(&transfer.destination);
                if path_requires_non_windows(&source) || path_requires_non_windows(&destination) {
                    requirements.insert(Requirement::NonWindows);
                }
            }
            WorkflowAction::TouchArtifact(touch) => {
                let path = normalize_path(&touch.path);
                if path_requires_non_windows(&path) {
                    requirements.insert(Requirement::NonWindows);
                }
            }
            _ => unreachable!(
                "compile preconditions must be directory reset, mkdir, fixture copy, or fixture touch actions"
            ),
        }
        operations.push(map_action(action).map_err(&fail)?);
        consumption.actions.push(index);
    }
    operations.push(compile_operation);
    operations.extend(shape.diagnostics.into_iter().map(|diagnostic| {
        OperationRecord::new(
            diagnostic.action,
            diagnostic.expectation,
            provenance(contract.span, &contract.expansion),
        )
    }));

    let mut artifact_flow = ArtifactFlow::new(artifact_paths);
    let action_window = workflow_actions_in_window(
        workflow_actions,
        already_consumed_actions,
        ProvenanceWindow {
            after: Some(&contract_order),
            before: next_contract_order,
        },
    );
    let mut needed_paths = bound_check_paths(bindings, assertions, comparisons);
    let mut relevant_transfers = BTreeSet::new();
    let mut relevant_touches = BTreeSet::new();
    for (index, action) in &action_window {
        let WorkflowAction::TouchArtifact(touch) = action else {
            continue;
        };
        let path = normalize_path(&touch.path);
        if guard_covers(&contract.guard, &touch.guard) && artifact_flow.contains(&path) {
            relevant_touches.insert(*index);
        }
    }
    for (index, action) in action_window.iter().rev() {
        let WorkflowAction::TransferArtifact(transfer) = action else {
            continue;
        };
        let destination = normalize_path(&transfer.destination);
        if guard_covers(&contract.guard, &transfer.guard) && needed_paths.remove(&destination) {
            needed_paths.insert(normalize_path(&transfer.source));
            relevant_transfers.insert(*index);
        }
    }

    #[derive(Clone, Copy)]
    enum CompileWorkflowEvent {
        Action(usize),
        Check(BoundCheck),
    }

    let mut events = action_window
        .into_iter()
        .filter_map(|(index, action)| {
            (bound_workflow_actions.is_some_and(|actions| actions.contains(&index))
                || relevant_transfers.contains(&index)
                || relevant_touches.contains(&index))
            .then(|| {
                (
                    execution_order_key(action_span(action), action_expansion(action)),
                    CompileWorkflowEvent::Action(index),
                )
            })
        })
        .collect::<Vec<_>>();
    events.extend(bindings.into_iter().flatten().map(|check| {
        let (span, expansion) = match *check {
            BoundCheck::Assertion(index) => {
                let assertion = &assertions[index];
                (assertion.span, assertion.expansion.as_slice())
            }
            BoundCheck::Comparison(index) => {
                let comparison = &comparisons[index];
                (comparison.span, comparison.expansion.as_slice())
            }
        };
        (
            execution_order_key(span, expansion),
            CompileWorkflowEvent::Check(*check),
        )
    }));
    events.sort_by(|left, right| left.0.cmp(&right.0));

    let mut has_verilog_workflow = false;
    for (_, event) in events {
        match event {
            CompileWorkflowEvent::Action(index) => match &workflow_actions[index] {
                WorkflowAction::LinkVerilog(link) => {
                    if !guard_covers(&contract.guard, &link.guard) {
                        return Err(fail(format!(
                            "link_verilog_pass guard is incompatible with {}",
                            contract.helper
                        )));
                    }
                    collect_requirements(&link.guard, &mut requirements).map_err(&fail)?;
                    verilog_link_extends_flow(&mut artifact_flow, &generated_artifacts, link)
                        .map_err(&fail)?;
                    let link_operation =
                        map_action(&workflow_actions[index]).map_err(|message| {
                            error_diagnostic(
                                "import.compile_contract",
                                message,
                                link.span,
                                &link.expansion,
                            )
                        })?;
                    for input in &link_operation.artifacts.inputs {
                        declare_bound_output(&mut operations, input.clone());
                    }
                    operations.push(link_operation);
                    consumption.actions.push(index);
                    requirements.insert(Requirement::Verilog);
                    requirements.insert(Requirement::Icarus);
                    has_verilog_workflow = true;
                }
                WorkflowAction::RunVerilog(run) => {
                    if !guard_covers(&contract.guard, &run.guard) {
                        return Err(fail(format!(
                            "{} guard is incompatible with {}",
                            workflow_actions[index].helper_name(),
                            contract.helper
                        )));
                    }
                    collect_requirements(&run.guard, &mut requirements).map_err(&fail)?;
                    verilog_run_extends_flow(&mut artifact_flow, run).map_err(&fail)?;
                    operations.push(map_action(&workflow_actions[index]).map_err(|message| {
                        error_diagnostic(
                            "import.compile_contract",
                            message,
                            run.span,
                            &run.expansion,
                        )
                    })?);
                    consumption.actions.push(index);
                    requirements.insert(Requirement::Verilog);
                    requirements.insert(Requirement::Icarus);
                    has_verilog_workflow = true;
                }
                WorkflowAction::ShowRules(action) => {
                    if !guard_covers(&contract.guard, &action.guard) {
                        return Err(fail(format!(
                            "showrules guard is incompatible with {}",
                            contract.helper
                        )));
                    }
                    collect_requirements(&action.guard, &mut requirements).map_err(&fail)?;
                    if !showrules_extends_flow(&mut artifact_flow, action) {
                        return Err(fail(format!(
                            "showrules is not connected to {} input {:?}",
                            contract.helper, action.input
                        )));
                    }
                    operations.push(map_action(&workflow_actions[index]).map_err(|message| {
                        error_diagnostic(
                            "import.compile_contract",
                            message,
                            action.span,
                            &action.expansion,
                        )
                    })?);
                    consumption.actions.push(index);
                    requirements.insert(Requirement::ShowRules);
                }
                WorkflowAction::BluetclRun(run) => {
                    if !guard_covers(&contract.guard, &run.guard) {
                        return Err(fail(format!(
                            "{} guard is incompatible with {}",
                            workflow_actions[index].helper_name(),
                            contract.helper
                        )));
                    }
                    let inputs = run
                        .artifact_inputs
                        .iter()
                        .map(|path| normalize_path(path))
                        .collect::<Vec<_>>();
                    if !inputs.iter().all(|input| artifact_flow.contains(input)) {
                        return Err(fail(format!(
                            "{} requires preceding generated artifacts {inputs:?}",
                            workflow_actions[index].helper_name()
                        )));
                    }
                    let operation = map_action(&workflow_actions[index]).map_err(|message| {
                        error_diagnostic(
                            "import.compile_contract",
                            message,
                            run.span,
                            &run.expansion,
                        )
                    })?;
                    for input in inputs {
                        declare_bound_output(&mut operations, input);
                    }
                    operations.push(operation);
                    artifact_flow.insert(normalize_path(&run.stdout));
                    for output in &run.artifact_outputs {
                        artifact_flow.insert(normalize_path(output));
                    }
                    consumption.actions.push(index);
                    requirements.insert(Requirement::Bluetcl);
                }
                WorkflowAction::RenderGolden(render) => {
                    if !guard_covers(&contract.guard, &render.guard) {
                        return Err(fail(format!(
                            "golden render guard is incompatible with {}",
                            contract.helper
                        )));
                    }
                    let template = normalize_path(&render.template);
                    let output = normalize_path(&render.output);
                    if !is_safe_relative(&template) || !is_safe_relative(&output) {
                        return Err(fail(
                            "golden render paths must be safe relative paths".to_owned(),
                        ));
                    }
                    operations.push(map_action(&workflow_actions[index]).map_err(&fail)?);
                    artifact_flow.insert(output);
                    consumption.actions.push(index);
                }
                WorkflowAction::RenderM4Curdir(render) => {
                    if !guard_covers(&contract.guard, &render.guard) {
                        return Err(fail(format!(
                            "M4 CURDIR render guard is incompatible with {}",
                            contract.helper
                        )));
                    }
                    let template = normalize_path(&render.template);
                    let output = normalize_path(&render.output);
                    if !is_safe_relative(&template) || !is_safe_relative(&output) {
                        return Err(fail(
                            "M4 CURDIR render paths must be safe relative paths".to_owned(),
                        ));
                    }
                    operations.push(map_action(&workflow_actions[index]).map_err(&fail)?);
                    artifact_flow.insert(output);
                    consumption.actions.push(index);
                }
                WorkflowAction::TextNormalize(action) => {
                    let source = normalize_path(&action.source);
                    let destination = normalize_path(&action.destination);
                    if !guard_covers(&contract.guard, &action.guard)
                        || !is_safe_relative(&source)
                        || !is_safe_relative(&destination)
                        || !artifact_flow.contains(&source)
                        || artifact_flow.contains(&destination)
                    {
                        return Err(fail(format!(
                            "text normalization is not connected to {} artifacts: {source:?} -> {destination:?}",
                            contract.helper
                        )));
                    }
                    operations.push(map_action(&workflow_actions[index]).map_err(&fail)?);
                    artifact_flow.insert(destination);
                    consumption.actions.push(index);
                }
                WorkflowAction::VerilogFilter(action) => {
                    let path = normalize_path(&action.path);
                    if !guard_covers(&contract.guard, &action.guard)
                        || !is_safe_relative(&path)
                        || !artifact_flow.contains(&path)
                    {
                        return Err(fail(format!(
                            "Verilog filter is not connected to {} artifact {path:?}",
                            contract.helper
                        )));
                    }
                    operations.push(map_action(&workflow_actions[index]).map_err(&fail)?);
                    consumption.actions.push(index);
                }
                WorkflowAction::DumpIntermediate(dump) => {
                    if !guard_covers(&contract.guard, &dump.guard) {
                        return Err(fail(format!(
                            "{} guard is incompatible with {}",
                            workflow_actions[index].helper_name(),
                            contract.helper
                        )));
                    }
                    let mut dump_operation =
                        map_action(&workflow_actions[index]).map_err(|message| {
                            error_diagnostic(
                                "import.compile_contract",
                                message,
                                dump.span,
                                &dump.expansion,
                            )
                        })?;
                    dump_operation.requires.push(Requirement::InternalChecks);
                    declare_bound_output(&mut operations, normalize_path(&dump.input));
                    operations.push(dump_operation);
                    consumption.actions.push(index);
                }
                WorkflowAction::TouchArtifact(touch) => {
                    if !guard_covers(&contract.guard, &touch.guard) {
                        return Err(fail(format!(
                            "touch guard is incompatible with {}",
                            contract.helper
                        )));
                    }
                    let path = normalize_path(&touch.path);
                    if !is_safe_relative(&path) || !artifact_flow.contains(&path) {
                        return Err(fail(format!(
                            "{} touch requires a preceding available artifact: {path:?}",
                            contract.helper
                        )));
                    }
                    if path_requires_non_windows(&path) {
                        requirements.insert(Requirement::NonWindows);
                    }
                    operations.push(map_action(&workflow_actions[index]).map_err(&fail)?);
                    consumption.actions.push(index);
                }
                WorkflowAction::TransferArtifact(transfer) => {
                    if !artifact_flow.apply(transfer) {
                        return Err(fail(format!(
                            "{} transfer is not connected to an available artifact: {:?} -> {:?}",
                            contract.helper, transfer.source, transfer.destination
                        )));
                    }
                    collect_requirements(&transfer.guard, &mut requirements).map_err(&fail)?;
                    let source = normalize_path(&transfer.source);
                    let destination = normalize_path(&transfer.destination);
                    if !is_safe_relative(&source) || !is_safe_relative(&destination) {
                        return Err(fail(format!(
                            "{} transfer paths must be safe relative paths: {source:?} -> {destination:?}",
                            contract.helper
                        )));
                    }
                    if path_requires_non_windows(&source) || path_requires_non_windows(&destination)
                    {
                        requirements.insert(Requirement::NonWindows);
                    }
                    operations.push(OperationRecord::new(
                        map_transfer(transfer),
                        OperationExpectation::Required,
                        provenance(transfer.span, &transfer.expansion),
                    ));
                    consumption.actions.push(index);
                }
                _ => unreachable!("compile workflow events contain only selected actions"),
            },
            CompileWorkflowEvent::Check(check) => {
                if let BoundCheck::Assertion(index) = check {
                    let assertion = &assertions[index];
                    if guard_has_capability(&assertion.guard, Capability::InternalChecks) {
                        if let Some(output) =
                            assertion.arguments.first().map(|path| normalize_path(path))
                        {
                            if let Some(input) = implicit_dumpbo_input(&output) {
                                if artifact_flow.contains(&input)
                                    && !artifact_flow.contains(&output)
                                {
                                    let mut dump = OperationRecord::new(
                                        Action::DumpIntermediate {
                                            input,
                                            output: output.clone(),
                                            view: bsc_test_plan::IntermediateDumpView::Bo,
                                        },
                                        OperationExpectation::Required,
                                        provenance(assertion.span, &assertion.expansion),
                                    );
                                    dump.requires.push(Requirement::InternalChecks);
                                    operations.push(dump);
                                    artifact_flow.insert(output);
                                }
                            }
                        }
                    }
                }
                append_bound_checks(
                    Some(&vec![check]),
                    assertions,
                    comparisons,
                    &mut requirements,
                    &mut operations,
                    &mut consumption,
                )?;
            }
        }
    }

    propagate_posix_echo_probe_requirements(&mut operations);

    let stem = Path::new(&contract.source)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| fail(format!("{} source has no portable stem", contract.helper)))?;
    let suffix = contract_index + 1;
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("compile-{suffix}-{stem}"),
            resource: if has_verilog_workflow {
                ResourceClass::Heavy
            } else {
                ResourceClass::Normal
            },
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("compile-{stem}"),
                operations,
            }],
        },
        consumption,
    })
}

fn propagate_posix_echo_probe_requirements(operations: &mut [OperationRecord]) {
    let mut probe_episode = false;
    for operation in operations {
        if let Action::BscLink { simulator, .. } = &operation.action {
            probe_episode = *simulator == bsc_test_plan::IcarusSimulatorSelector::PosixEchoProbe;
        }
        if probe_episode && !operation.requires.contains(&Requirement::NonWindows) {
            operation.requires.push(Requirement::NonWindows);
        }
    }
}

fn compile_directory_options(arguments: &[String]) -> Result<BTreeSet<String>, String> {
    let mut directories = BTreeSet::new();
    for option in ["-bdir", "-vdir", "-fdir"] {
        directories.extend(
            option_values(arguments, option)?
                .into_iter()
                .map(|directory| normalize_path(&directory)),
        );
    }
    if let Some(directory) = directories
        .iter()
        .find(|directory| !is_safe_relative(directory))
    {
        return Err(format!(
            "compile directory option must be a safe relative path: {directory:?}"
        ));
    }
    Ok(directories)
}

fn compile_preceding_directory_actions<'a>(
    arguments: &[String],
    guard: &Guard,
    workflow_actions: &'a [WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    window: ProvenanceWindow<'_>,
) -> Result<Vec<(usize, &'a WorkflowAction)>, String> {
    let mut actions = Vec::new();
    for directory in compile_directory_options(arguments)? {
        let matches = workflow_actions
            .iter()
            .enumerate()
            .filter(|(index, action)| {
                !already_consumed_actions.contains(index)
                    && window.contains(&execution_order_key(
                        action_span(action),
                        action_expansion(action),
                    ))
                    && matches!(
                        action,
                        WorkflowAction::CreateDirectory(action)
                            if normalize_path(&action.path) == directory && &action.guard == guard
                    )
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "compile directory option {directory:?} requires exactly one matching preceding mkdir, found {}",
                matches.len()
            ));
        }
        actions.push(matches[0]);
    }
    Ok(actions)
}

fn closed_preceding_directory_actions<'a>(
    guard: &Guard,
    workflow_actions: &'a [WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    unsupported: &[UnsupportedConstruct],
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    window: ProvenanceWindow<'_>,
) -> Vec<(usize, &'a WorkflowAction)> {
    let mut barriers = window
        .after
        .iter()
        .map(|order| (*order).clone())
        .collect::<Vec<_>>();
    barriers.extend(
        unsupported
            .iter()
            .map(|item| execution_order_key(item.span, &item.expansion))
            .filter(|order| window.contains(order)),
    );
    barriers.extend(
        assertions
            .iter()
            .map(|item| execution_order_key(item.span, &item.expansion))
            .filter(|order| window.contains(order)),
    );
    barriers.extend(
        comparisons
            .iter()
            .map(|item| execution_order_key(item.span, &item.expansion))
            .filter(|order| window.contains(order)),
    );
    barriers.extend(
        workflow_actions
            .iter()
            .enumerate()
            .filter(|(index, action)| {
                !already_consumed_actions.contains(index)
                    && !matches!(
                        action,
                        WorkflowAction::EnsureDirectoryAbsent(_)
                            | WorkflowAction::CreateDirectory(_)
                    )
            })
            .map(|(_, action)| execution_order_key(action_span(action), action_expansion(action)))
            .filter(|order| window.contains(order)),
    );
    let barrier = barriers.into_iter().max();
    let directory_actions = workflow_actions
        .iter()
        .enumerate()
        .filter(|(index, action)| {
            !already_consumed_actions.contains(index)
                && action.guard() == guard
                && matches!(
                    action,
                    WorkflowAction::EnsureDirectoryAbsent(_) | WorkflowAction::CreateDirectory(_)
                )
        })
        .filter(|(_, action)| {
            let order = execution_order_key(action_span(action), action_expansion(action));
            window.contains(&order) && barrier.as_ref().is_none_or(|barrier| order > *barrier)
        })
        .collect::<Vec<_>>();

    let mut result = Vec::new();
    for (ensure_index, ensure_action) in &directory_actions {
        let WorkflowAction::EnsureDirectoryAbsent(ensure) = ensure_action else {
            continue;
        };
        let path = normalize_path(&ensure.path);
        if !is_safe_relative(&path)
            || directory_actions
                .iter()
                .filter(|(_, action)| {
                    matches!(
                        action,
                        WorkflowAction::EnsureDirectoryAbsent(candidate)
                            if normalize_path(&candidate.path) == path
                    )
                })
                .count()
                != 1
        {
            continue;
        }
        let ensure_order = execution_order_key(ensure.span, &ensure.expansion);
        let creates = directory_actions
            .iter()
            .filter(|(_, action)| {
                matches!(
                    action,
                    WorkflowAction::CreateDirectory(candidate)
                        if normalize_path(&candidate.path) == path
                            && execution_order_key(candidate.span, &candidate.expansion)
                                > ensure_order
                )
            })
            .collect::<Vec<_>>();
        let [create] = creates.as_slice() else {
            continue;
        };
        result.push((*ensure_index, *ensure_action));
        result.push((create.0, create.1));
    }
    result.sort_by_key(|(_, action)| {
        execution_order_key(action_span(action), action_expansion(action))
    });
    result.dedup_by_key(|(index, _)| *index);
    result
}

fn compile_preceding_fixture_touch<'a>(
    contract: &CompileContract,
    workflow_actions: &'a [WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    previous_contract_order: Option<&ExecutionOrderKey>,
    contract_order: &ExecutionOrderKey,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    unsupported: &[UnsupportedConstruct],
    static_fixture_sources: &BTreeSet<String>,
) -> Option<(usize, &'a WorkflowAction)> {
    let source = normalize_path(&contract.source);
    if !is_safe_relative(&source)
        || !static_fixture_sources.contains(&source)
        || !contract.guard.is_resolved()
    {
        return None;
    }
    let candidates = workflow_actions
        .iter()
        .enumerate()
        .filter(|(index, action)| {
            !already_consumed_actions.contains(index)
                && ProvenanceWindow {
                    after: previous_contract_order,
                    before: Some(contract_order),
                }
                .contains(&execution_order_key(
                    action_span(action),
                    action_expansion(action),
                ))
                && matches!(
                    action,
                    WorkflowAction::TouchArtifact(touch)
                        if touch.guard.is_resolved()
                            && touch.guard == contract.guard
                            && normalize_path(&touch.path) == source
                )
        })
        .collect::<Vec<_>>();
    let [(index, action)] = candidates.as_slice() else {
        return None;
    };
    let touch_order = execution_order_key(action_span(action), action_expansion(action));
    let window = ProvenanceWindow {
        after: Some(&touch_order),
        before: Some(contract_order),
    };
    let has_barrier = workflow_actions
        .iter()
        .enumerate()
        .any(|(candidate_index, candidate)| {
            candidate_index != *index
                && !already_consumed_actions.contains(&candidate_index)
                && window.contains(&execution_order_key(
                    action_span(candidate),
                    action_expansion(candidate),
                ))
        })
        || assertions.iter().any(|assertion| {
            window.contains(&execution_order_key(assertion.span, &assertion.expansion))
        })
        || comparisons.iter().any(|comparison| {
            window.contains(&execution_order_key(comparison.span, &comparison.expansion))
        })
        || unsupported.iter().any(|construct| {
            window.contains(&execution_order_key(construct.span, &construct.expansion))
        });
    (!has_barrier).then_some((*index, *action))
}

fn compile_preceding_fixture_copies<'a>(
    contract: &CompileContract,
    workflow_actions: &'a [WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    previous_contract_order: Option<&ExecutionOrderKey>,
    contract_order: &ExecutionOrderKey,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    unsupported: &[UnsupportedConstruct],
    static_fixture_sources: &BTreeSet<String>,
) -> Result<Vec<(usize, &'a WorkflowAction)>, String> {
    let source = normalize_path(&contract.source);
    if !is_safe_relative(&source) {
        return Ok(Vec::new());
    }
    let window = ProvenanceWindow {
        after: previous_contract_order,
        before: Some(contract_order),
    };
    let actions = workflow_actions_in_window(workflow_actions, already_consumed_actions, window);
    let copies = actions
        .iter()
        .filter(|(_, action)| {
            matches!(
                action,
                WorkflowAction::TransferArtifact(transfer)
                    if transfer.operation == ArtifactTransferOperation::Copy
                        && is_safe_relative(&normalize_path(&transfer.source))
                        && is_safe_relative(&normalize_path(&transfer.destination))
                        && static_fixture_sources.contains(&normalize_path(&transfer.source))
                        && normalize_path(&transfer.source) != normalize_path(&transfer.destination)
                        && Path::new(&normalize_path(&transfer.destination)).extension().is_some_and(|extension| extension == "bsv")
                        && guard_covers(&transfer.guard, &contract.guard)
            )
        })
        .copied()
        .collect::<Vec<_>>();
    if copies.is_empty()
        || copies
            .iter()
            .filter(|(_, action)| {
                matches!(
                    action,
                    WorkflowAction::TransferArtifact(transfer)
                        if normalize_path(&transfer.destination) == source
                )
            })
            .count()
            != 1
    {
        return Ok(Vec::new());
    }
    let first_copy_order =
        execution_order_key(action_span(copies[0].1), action_expansion(copies[0].1));
    let setup_window = ProvenanceWindow {
        after: Some(&first_copy_order),
        before: Some(contract_order),
    };
    if actions.iter().any(|(index, action)| {
        setup_window.contains(&execution_order_key(
            action_span(action),
            action_expansion(action),
        )) && !copies.iter().any(|(copy_index, _)| copy_index == index)
            && !matches!(action, WorkflowAction::CreateDirectory(_))
    }) || assertions.iter().any(|assertion| {
        setup_window.contains(&execution_order_key(assertion.span, &assertion.expansion))
    }) || comparisons.iter().any(|comparison| {
        setup_window.contains(&execution_order_key(comparison.span, &comparison.expansion))
    }) || unsupported.iter().any(|construct| {
        setup_window.contains(&execution_order_key(construct.span, &construct.expansion))
    }) {
        return Ok(Vec::new());
    }
    Ok(copies)
}

fn simulation_preceding_fixture_touch<'a>(
    contract: &SimulationContract,
    workflow_actions: &'a [WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    previous_contract_order: Option<&ExecutionOrderKey>,
    contract_order: &ExecutionOrderKey,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    fixture_root: &Path,
) -> Option<(usize, &'a WorkflowAction)> {
    let source = normalize_path(&contract.source);
    let source_is_static_fixture = is_safe_relative(&source)
        && fs::symlink_metadata(fixture_root.join(&source))
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink());
    if !source_is_static_fixture || !contract.guard.is_resolved() {
        return None;
    }
    let candidates = workflow_actions_in_window(
        workflow_actions,
        already_consumed_actions,
        ProvenanceWindow {
            after: previous_contract_order,
            before: Some(contract_order),
        },
    )
    .into_iter()
    .filter(|(_, action)| {
        matches!(
            action,
            WorkflowAction::TouchArtifact(touch)
                if touch.guard.is_resolved()
                    && touch.guard == contract.guard
                    && normalize_path(&touch.path) == source
        )
    })
    .collect::<Vec<_>>();
    let [(index, action)] = candidates.as_slice() else {
        return None;
    };
    let touch_order = execution_order_key(action_span(action), action_expansion(action));
    let window = ProvenanceWindow {
        after: Some(&touch_order),
        before: Some(contract_order),
    };
    let has_barrier = workflow_actions
        .iter()
        .enumerate()
        .any(|(candidate_index, candidate)| {
            candidate_index != *index
                && !already_consumed_actions.contains(&candidate_index)
                && window.contains(&execution_order_key(
                    action_span(candidate),
                    action_expansion(candidate),
                ))
        })
        || assertions.iter().any(|assertion| {
            window.contains(&execution_order_key(assertion.span, &assertion.expansion))
        })
        || comparisons.iter().any(|comparison| {
            window.contains(&execution_order_key(comparison.span, &comparison.expansion))
        });
    (!has_barrier).then_some((*index, *action))
}

fn bound_check_paths(
    bindings: Option<&Vec<BoundCheck>>,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
) -> BTreeSet<String> {
    bindings
        .into_iter()
        .flatten()
        .filter_map(|binding| match binding {
            BoundCheck::Assertion(index) => assertions
                .get(*index)
                .and_then(|assertion| assertion.arguments.first()),
            BoundCheck::Comparison(index) => comparisons
                .get(*index)
                .and_then(|comparison| comparison.arguments.first()),
        })
        .map(|path| normalize_path(path))
        .collect()
}

fn compile_shape(contract: &CompileContract) -> Result<CompileShape, String> {
    let arguments = &contract.arguments;
    let require_arity = |minimum: usize, maximum: usize| {
        if (minimum..=maximum).contains(&arguments.len()) {
            Ok(())
        } else {
            Err(format!(
                "{} requires {minimum} to {maximum} static arguments, found {}",
                contract.helper,
                arguments.len()
            ))
        }
    };
    let count = |value: &str| {
        value.parse::<usize>().map_err(|error| {
            format!(
                "{} has invalid diagnostic count {value:?}: {error}",
                contract.helper
            )
        })
    };
    let diagnostic = |path: String,
                      kind: DiagnosticKind,
                      code: Option<String>,
                      count: usize,
                      expectation: OperationExpectation| {
        CompileDiagnostic {
            action: Action::AssertDiagnosticCount {
                path,
                kind,
                code,
                count,
            },
            expectation,
        }
    };
    let dependency_mode = |value: &str| match value {
        "0" | "" => Ok(DependencyMode::Update),
        "1" => Ok(DependencyMode::NoDeps),
        _ => Err(format!(
            "{} has invalid nodeps value {value:?}",
            contract.helper
        )),
    };
    let source = normalize_path(&contract.source);
    if source.is_empty()
        || arguments.first().map(|value| normalize_path(value)) != Some(source.clone())
    {
        return Err(format!(
            "{} recovered source {:?}, expected {:?}",
            contract.helper,
            arguments.first(),
            contract.source
        ));
    }

    let (mode, module, options, dependency_mode, expected_exit, diagnostics, stdout) =
        match contract.helper.as_str() {
            "bsc_compile" => {
                require_arity(1, 3)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 1, ""),
                    dependency_mode(argument_or_default(arguments, 2, "0"))?,
                    ExpectedExit::Unchecked,
                    Vec::new(),
                    format!("{source}.bsc-out"),
                )
            }
            "compile_pass_bug" | "compile_fail_bug" => {
                require_arity(1, 4)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 2, ""),
                    dependency_mode(argument_or_default(arguments, 3, "0"))?,
                    if contract.helper == "compile_pass_bug" {
                        ExpectedExit::Success
                    } else {
                        ExpectedExit::Failure
                    },
                    Vec::new(),
                    format!("{source}.bsc-out"),
                )
            }
            "compile_pass_bug_error" => {
                require_arity(2, 5)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-out"),
                        DiagnosticKind::Error,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 3, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-out"),
                )
            }
            "compile_backend_pass" => {
                require_arity(1, 3)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 1, ""),
                    dependency_mode(argument_or_default(arguments, 2, "0"))?,
                    ExpectedExit::Success,
                    Vec::new(),
                    format!("{source}.bsc-out"),
                )
            }
            "compile_pass" | "compile_fail" => {
                require_arity(1, 3)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 1, ""),
                    dependency_mode(argument_or_default(arguments, 2, "0"))?,
                    if contract.helper == "compile_pass" {
                        ExpectedExit::Success
                    } else {
                        ExpectedExit::Failure
                    },
                    Vec::new(),
                    format!("{source}.bsc-out"),
                )
            }
            "compile_pass_no_warning" => {
                require_arity(1, 3)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 1, ""),
                    dependency_mode(argument_or_default(arguments, 2, "0"))?,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-out"),
                        DiagnosticKind::Warning,
                        None,
                        0,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-out"),
                )
            }
            "compile_pass_warning" => {
                require_arity(2, 4)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 3, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-out"),
                        DiagnosticKind::Warning,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 2, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-out"),
                )
            }
            "compile_pass_warning_bug" => {
                require_arity(2, 5)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-out"),
                        DiagnosticKind::Warning,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 3, "1"))?,
                        known_bug_expectation(argument_or_default(arguments, 2, "")),
                    )],
                    format!("{source}.bsc-out"),
                )
            }
            "compile_fail_error" => {
                require_arity(2, 5)?;
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 3, ""),
                    dependency_mode(argument_or_default(arguments, 4, "0"))?,
                    ExpectedExit::Failure,
                    vec![diagnostic(
                        format!("{source}.bsc-out"),
                        DiagnosticKind::Error,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 2, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-out"),
                )
            }
            "compile_fail_error_bug" => {
                require_arity(2, 5)?;
                let expectation = known_bug_expectation(argument_or_default(arguments, 2, ""));
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Failure,
                    vec![diagnostic(
                        format!("{source}.bsc-out"),
                        DiagnosticKind::Error,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 3, "1"))?,
                        expectation,
                    )],
                    format!("{source}.bsc-out"),
                )
            }
            "compile_fail_error_warnings" => {
                require_arity(2, 5)?;
                let output = format!("{source}.bsc-out");
                let mut diagnostics = vec![diagnostic(
                    output.clone(),
                    DiagnosticKind::Error,
                    Some(argument_or_default(arguments, 1, "").to_owned()),
                    count(argument_or_default(arguments, 2, "1"))?,
                    OperationExpectation::Required,
                )];
                for warning in parse_arguments(
                    argument_or_default(arguments, 3, ""),
                    "warning specifications",
                )? {
                    let fields = parse_arguments(&warning, "warning specification")?;
                    let (tag, expected_count) = match fields.as_slice() {
                        [tag] => (tag.clone(), 1),
                        [tag, expected_count] => (tag.clone(), count(expected_count)?),
                        _ => {
                            return Err(format!(
                                "{} warning specification requires 1 to 2 static fields, found {}",
                                contract.helper,
                                fields.len()
                            ));
                        }
                    };
                    diagnostics.push(diagnostic(
                        output.clone(),
                        DiagnosticKind::Warning,
                        Some(tag),
                        expected_count,
                        OperationExpectation::Required,
                    ));
                }
                (
                    BscCompileMode::Frontend,
                    None,
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Failure,
                    diagnostics,
                    output,
                )
            }
            "compile_object_pass_bug" => {
                require_arity(1, 4)?;
                (
                    BscCompileMode::BluesimObject,
                    non_empty(argument_or_default(arguments, 1, "")),
                    argument_or_default(arguments, 3, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    Vec::new(),
                    format!("{source}.bsc-ccomp-out"),
                )
            }
            "compile_object_pass_warning" => {
                require_arity(2, 5)?;
                (
                    BscCompileMode::BluesimObject,
                    non_empty(argument_or_default(arguments, 3, "")),
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-ccomp-out"),
                        DiagnosticKind::Warning,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 2, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-ccomp-out"),
                )
            }
            "compile_object_fail" => (
                BscCompileMode::BluesimObject,
                non_empty(argument_or_default(arguments, 1, "")),
                argument_or_default(arguments, 2, ""),
                DependencyMode::Update,
                ExpectedExit::Failure,
                Vec::new(),
                format!("{source}.bsc-ccomp-out"),
            ),
            "compile_object_fail_error" => {
                require_arity(2, 5)?;
                (
                    BscCompileMode::BluesimObject,
                    non_empty(argument_or_default(arguments, 3, "")),
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Failure,
                    vec![diagnostic(
                        format!("{source}.bsc-ccomp-out"),
                        DiagnosticKind::Error,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 2, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-ccomp-out"),
                )
            }
            "compile_synthesize_verilog_pass_bug" => {
                require_arity(1, 4)?;
                (
                    BscCompileMode::Synthesize,
                    non_empty(argument_or_default(arguments, 1, "")),
                    argument_or_default(arguments, 3, ""),
                    DependencyMode::NoDeps,
                    ExpectedExit::Success,
                    Vec::new(),
                    format!("{source}.bsc-vcomp-syn-out"),
                )
            }
            "compile_verilog_pass_bug"
            | "compile_verilog_fail_bug"
            | "compile_verilog_schedule_pass_bug"
            | "compile_verilog_schedule_fail_bug" => {
                require_arity(1, 4)?;
                let mode = if contract.helper.starts_with("compile_verilog_schedule_") {
                    BscCompileMode::VerilogSchedule
                } else {
                    BscCompileMode::Verilog
                };
                (
                    mode,
                    non_empty(argument_or_default(arguments, 1, "")),
                    argument_or_default(arguments, 3, ""),
                    DependencyMode::Update,
                    if contract.helper.contains("_fail_bug") {
                        ExpectedExit::Failure
                    } else {
                        ExpectedExit::Success
                    },
                    Vec::new(),
                    if matches!(mode, BscCompileMode::VerilogSchedule) {
                        format!("{source}.bsc-sched-out")
                    } else {
                        format!("{source}.bsc-vcomp-out")
                    },
                )
            }
            "compile_verilog_pass_bug_error" => {
                require_arity(2, 6)?;
                (
                    BscCompileMode::Verilog,
                    non_empty(argument_or_default(arguments, 2, "")),
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-vcomp-out"),
                        DiagnosticKind::Error,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 5, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-vcomp-out"),
                )
            }
            "compile_verilog_fail_no_internal_error" => {
                require_arity(1, 1)?;
                (
                    BscCompileMode::Verilog,
                    None,
                    "",
                    DependencyMode::Update,
                    ExpectedExit::Failure,
                    Vec::new(),
                    format!("{source}.bsc-vcomp-out"),
                )
            }
            "bsc_compile_verilog"
            | "compile_verilog_pass"
            | "compile_verilog_pass_ghcrts_m1_2g"
            | "compile_verilog_fail"
            | "compile_verilog_schedule_pass"
            | "compile_verilog_schedule_fail" => {
                require_arity(1, 3)?;
                let mode = if contract.helper.starts_with("compile_verilog_schedule_") {
                    BscCompileMode::VerilogSchedule
                } else {
                    BscCompileMode::Verilog
                };
                (
                    mode,
                    non_empty(argument_or_default(arguments, 1, "")),
                    argument_or_default(arguments, 2, ""),
                    DependencyMode::Update,
                    if contract.helper.ends_with("_fail") {
                        ExpectedExit::Failure
                    } else {
                        ExpectedExit::Success
                    },
                    Vec::new(),
                    if matches!(mode, BscCompileMode::VerilogSchedule) {
                        format!("{source}.bsc-sched-out")
                    } else {
                        format!("{source}.bsc-vcomp-out")
                    },
                )
            }
            "compile_verilog_pass_warning_bug" | "compile_verilog_pass_no_warning_bug" => {
                require_arity(2, 6)?;
                let expectation = known_bug_expectation(argument_or_default(arguments, 2, ""));
                let tagged_warning = diagnostic(
                    format!("{source}.bsc-vcomp-out"),
                    DiagnosticKind::Warning,
                    Some(argument_or_default(arguments, 1, "").to_owned()),
                    count(argument_or_default(arguments, 3, "1"))?,
                    if contract.helper == "compile_verilog_pass_warning_bug" {
                        expectation.clone()
                    } else {
                        OperationExpectation::Required
                    },
                );
                let diagnostics = if contract.helper == "compile_verilog_pass_no_warning_bug" {
                    vec![
                        diagnostic(
                            format!("{source}.bsc-vcomp-out"),
                            DiagnosticKind::Warning,
                            None,
                            0,
                            expectation,
                        ),
                        tagged_warning,
                    ]
                } else {
                    vec![tagged_warning]
                };
                (
                    BscCompileMode::Verilog,
                    non_empty(argument_or_default(arguments, 4, "")),
                    argument_or_default(arguments, 5, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    diagnostics,
                    format!("{source}.bsc-vcomp-out"),
                )
            }
            "compile_verilog_pass_no_warning" => {
                require_arity(1, 3)?;
                (
                    BscCompileMode::Verilog,
                    non_empty(argument_or_default(arguments, 1, "")),
                    argument_or_default(arguments, 2, ""),
                    DependencyMode::Update,
                    ExpectedExit::Success,
                    vec![diagnostic(
                        format!("{source}.bsc-vcomp-out"),
                        DiagnosticKind::Warning,
                        None,
                        0,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-vcomp-out"),
                )
            }
            "compile_verilog_fail_error_bug" => {
                require_arity(2, 6)?;
                let expectation = known_bug_expectation(argument_or_default(arguments, 2, ""));
                (
                    BscCompileMode::Verilog,
                    non_empty(argument_or_default(arguments, 4, "")),
                    argument_or_default(arguments, 5, ""),
                    DependencyMode::Update,
                    ExpectedExit::Failure,
                    vec![diagnostic(
                        format!("{source}.bsc-vcomp-out"),
                        DiagnosticKind::Error,
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 3, "1"))?,
                        expectation,
                    )],
                    format!("{source}.bsc-vcomp-out"),
                )
            }
            "compile_verilog_fail_error" | "compile_verilog_pass_warning" => {
                require_arity(2, 5)?;
                let warning = contract.helper == "compile_verilog_pass_warning";
                (
                    BscCompileMode::Verilog,
                    non_empty(argument_or_default(arguments, 3, "")),
                    argument_or_default(arguments, 4, ""),
                    DependencyMode::Update,
                    if warning {
                        ExpectedExit::Success
                    } else {
                        ExpectedExit::Failure
                    },
                    vec![diagnostic(
                        format!("{source}.bsc-vcomp-out"),
                        if warning {
                            DiagnosticKind::Warning
                        } else {
                            DiagnosticKind::Error
                        },
                        Some(argument_or_default(arguments, 1, "").to_owned()),
                        count(argument_or_default(arguments, 2, "1"))?,
                        OperationExpectation::Required,
                    )],
                    format!("{source}.bsc-vcomp-out"),
                )
            }
            helper => return Err(format!("unsupported compile helper {helper}")),
        };
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            &diagnostic.action,
            Action::AssertDiagnosticCount { code: Some(code), .. } if code.is_empty()
        )
    }) {
        return Err(format!(
            "{} requires a non-empty diagnostic tag",
            contract.helper
        ));
    }
    let mut args = parse_arguments(options, "compile options")?;
    if contract.helper == "compile_backend_pass" {
        args.insert(0, "-verilog".to_owned());
    }
    Ok(CompileShape {
        mode,
        module,
        args,
        dependency_mode,
        expected_exit,
        unexpected_success_forbidden_regex: (contract.helper
            == "compile_verilog_fail_no_internal_error")
            .then(|| "Internal.*Error".to_owned()),
        expectation: compile_expectation(contract)?,
        stdout,
        diagnostics,
    })
}

fn compile_expectation(contract: &CompileContract) -> Result<OperationExpectation, String> {
    let bug_index = match contract.helper.as_str() {
        "compile_pass_bug" | "compile_fail_bug" => Some(1),
        "compile_pass_bug_error" => Some(2),
        "compile_verilog_pass_bug"
        | "compile_verilog_fail_bug"
        | "compile_synthesize_verilog_pass_bug"
        | "compile_verilog_schedule_pass_bug"
        | "compile_verilog_schedule_fail_bug" => Some(2),
        "compile_verilog_pass_bug_error" => Some(3),
        "compile_object_pass_bug" => Some(2),
        _ => None,
    };
    let Some(index) = bug_index else {
        return Ok(OperationExpectation::Required);
    };
    let annotation = argument_or_default(&contract.arguments, index, "").trim();
    if matches!(
        contract.helper.as_str(),
        "compile_pass_bug"
            | "compile_fail_bug"
            | "compile_verilog_pass_bug"
            | "compile_verilog_fail_bug"
            | "compile_synthesize_verilog_pass_bug"
    ) {
        // Upstream *_bug helpers unconditionally arm setup_xfail before the
        // underlying check, so an unannotated call still carries an XFAIL
        // contract. If a fixed bug now satisfies the unbugged expectation,
        // canonical execution must report XPASS rather than silently pass.
        if annotation.is_empty() {
            return Ok(OperationExpectation::Xfail {
                reason: "upstream unannotated known failure".to_owned(),
            });
        }
    }
    Ok(known_bug_expectation(annotation))
}

fn known_bug_expectation(annotation: &str) -> OperationExpectation {
    let annotation = annotation.trim();
    if annotation.is_empty() {
        return OperationExpectation::Required;
    }
    OperationExpectation::Xfail {
        reason: format!("upstream bug {annotation}"),
    }
}

fn argument_or_default<'a>(
    arguments: &'a [String],
    index: usize,
    default: &'static str,
) -> &'a str {
    arguments.get(index).map_or(default, String::as_str)
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn compile_preprocessor_dump_paths(arguments: &[String]) -> BTreeSet<String> {
    arguments
        .iter()
        .filter_map(|argument| argument.strip_prefix("-dvpp="))
        .map(normalize_path)
        .filter(|path| is_safe_relative(path))
        .collect()
}

/// Declares only the finite compiler dump options whose output paths are static
/// and documented by the corresponding upstream helper contracts.
fn compile_dump_paths(arguments: &[String], module: &str) -> BTreeSet<String> {
    arguments
        .iter()
        .filter_map(|argument| {
            let (option, path) = argument.split_once('=')?;
            let path = normalize_path(path);
            match option {
                "-dATS" | "-dATSexpand" | "-dastate" | "-dsplitIf" if !path.contains("%") => {
                    Some(path)
                }
                _ if option.starts_with("-d") && path.contains("%m") => {
                    Some(normalize_path(&path.replace("%m", module)))
                }
                _ => None,
            }
        })
        .filter(|path| is_safe_relative(path))
        .collect()
}

fn generation_artifact_paths(
    generation: &CompileObjectAction,
    workflow_top: Option<&str>,
) -> Result<BTreeSet<String>, String> {
    let source = normalize_path(&generation.source);
    let arguments = parse_arguments(&generation.options, "generation options")?;
    let mut paths = BTreeSet::from([format!("{source}.bsc-ccomp-out")]);
    paths.extend(generation_module_artifact_paths(generation, &arguments)?);
    paths.extend(generation_static_dump_artifacts(
        &arguments,
        generation.module.as_deref().or(workflow_top),
    ));
    if arguments
        .iter()
        .any(|argument| argument == "-show-schedule")
    {
        let top = workflow_top.map(str::to_owned).or_else(|| {
            generation.module.clone().or_else(|| {
                Path::new(&source)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| format!("sys{stem}"))
            })
        });
        if let Some(top) = top {
            paths.insert(format!("{top}.sched"));
        }
    }
    Ok(paths)
}

fn generation_module_artifact_paths(
    generation: &CompileObjectAction,
    arguments: &[String],
) -> Result<BTreeSet<String>, String> {
    let output_directory = generation_output_directory(arguments)?;
    let output_path = |module: &str| {
        output_directory.as_deref().map_or_else(
            || format!("{module}.ba"),
            |directory| format!("{directory}/{module}.ba"),
        )
    };
    Ok(generation_modules(generation, arguments)?
        .into_iter()
        .map(|module| output_path(&module))
        .collect())
}

fn generation_output_directory(arguments: &[String]) -> Result<Option<String>, String> {
    let directories = arguments
        .windows(2)
        .filter(|pair| pair[0] == "-bdir")
        .map(|pair| normalize_path(&pair[1]))
        .collect::<BTreeSet<_>>();
    match directories.into_iter().collect::<Vec<_>>().as_slice() {
        [] => Ok(None),
        [directory] if is_safe_relative(directory) => Ok(Some(directory.clone())),
        [directory] => Err(format!(
            "generation -bdir must be a safe relative path: {directory:?}"
        )),
        directories => Err(format!(
            "generation has ambiguous -bdir output directories: {directories:?}"
        )),
    }
}

fn generation_modules(
    generation: &CompileObjectAction,
    arguments: &[String],
) -> Result<BTreeSet<String>, String> {
    let mut modules = BTreeSet::new();
    if let Some(module) = generation.module.as_deref() {
        validate_generation_module(module)?;
        modules.insert(module.to_owned());
    }
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] != "-g" {
            index += 1;
            continue;
        }
        let module = arguments
            .get(index + 1)
            .ok_or_else(|| "generation -g requires one static module-name argument".to_owned())?;
        validate_generation_module(module)?;
        modules.insert(module.clone());
        index += 2;
    }
    Ok(modules)
}

fn validate_generation_module(module: &str) -> Result<(), String> {
    let portable = !module.is_empty()
        && module != "."
        && module != ".."
        && !module.contains(['/', '\\', '.', '<', '>', ':', '"', '|', '?', '*'])
        && !module.ends_with([' ', '.'])
        && !module.bytes().any(|byte| byte.is_ascii_control());
    if portable {
        Ok(())
    } else {
        Err(format!(
            "generation -g module must be an unambiguous portable module-name segment: {module:?}"
        ))
    }
}

fn declare_generation_module_artifacts(
    operation: &mut OperationRecord,
    generation: &CompileObjectAction,
) -> Result<(), String> {
    let arguments = parse_arguments(&generation.options, "generation options")?;
    let artifacts = generation_module_artifact_paths(generation, &arguments)?;
    if let Some(module) = generation.module.as_deref() {
        let default_artifact = format!("{module}.ba");
        operation
            .artifacts
            .outputs
            .retain(|path| path != &default_artifact);
    }
    operation.artifacts.outputs.extend(artifacts);
    let mut seen = BTreeSet::new();
    operation
        .artifacts
        .outputs
        .retain(|path| seen.insert(path.to_ascii_lowercase()));
    Ok(())
}

fn standalone_generation_scenario(
    action_index: usize,
    generation: &CompileObjectAction,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    bindings: Option<&Vec<BoundCheck>>,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let action = WorkflowAction::CompileObject(generation.clone());
    let fail = |message: String| {
        error_diagnostic(
            "import.workflow_action",
            message,
            generation.span,
            &generation.expansion,
        )
    };
    let mut requirements = BTreeSet::new();
    collect_requirements(&generation.guard, &mut requirements).map_err(&fail)?;
    let mut operations = vec![map_action(&action).map_err(&fail)?];
    let mut consumption = ImportConsumption::default();
    append_bound_checks(
        bindings,
        assertions,
        comparisons,
        &mut requirements,
        &mut operations,
        &mut consumption,
    )?;
    requirements.insert(Requirement::Bluesim);
    let stem = Path::new(&generation.source)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| fail("compile_object_pass source has no portable stem".to_owned()))?;
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("bluesim-generation-{}-{stem}", action_index + 1),
            resource: ResourceClass::Heavy,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("generate-{stem}"),
                operations,
            }],
        },
        consumption,
    })
}

fn standalone_bsc2bsv_scenario(
    action_index: usize,
    action: &crate::model::Bsc2BsvAction,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let workflow_action = WorkflowAction::Bsc2Bsv(action.clone());
    let fail = |message: String| {
        error_diagnostic(
            "import.workflow_action",
            message,
            action.span,
            &action.expansion,
        )
    };
    let mut requirements = BTreeSet::new();
    let mut operation = map_action(&workflow_action).map_err(&fail)?;
    collect_check_requirements(&action.guard, &mut requirements, &mut operation.requires)
        .map_err(&fail)?;
    let stem = Path::new(&action.source)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| fail("run_bsc2bsv source has no portable stem".to_owned()))?;
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("bsc2bsv-{}-{stem}", action_index + 1),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("bsc2bsv-{stem}"),
                operations: vec![operation],
            }],
        },
        consumption: ImportConsumption::default(),
    })
}

fn standalone_bsc_parse_pretty_scenario(
    action_index: usize,
    action: &crate::model::BscParsePrettyAction,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let workflow_action = WorkflowAction::BscParsePretty(action.clone());
    let fail = |message: String| {
        error_diagnostic(
            "import.workflow_action",
            message,
            action.span,
            &action.expansion,
        )
    };
    let mut requirements = BTreeSet::new();
    let mut operation = map_action(&workflow_action).map_err(&fail)?;
    collect_check_requirements(&action.guard, &mut requirements, &mut operation.requires)
        .map_err(&fail)?;
    requirements.insert(Requirement::Frontend);
    let stem = Path::new(&action.source)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| fail("parse-pretty source has no portable stem".to_owned()))?;
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("parse-pretty-{}-{stem}", action_index + 1),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: format!("parse-pretty-{stem}"),
                operations: vec![operation],
            }],
        },
        consumption: ImportConsumption::default(),
    })
}

fn standalone_bluetcl_scenario(
    action_index: usize,
    run: &crate::model::BluetclRunAction,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    bindings: Option<&Vec<BoundCheck>>,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let action = WorkflowAction::BluetclRun(run.clone());
    let fail = |message: String| {
        error_diagnostic("import.workflow_action", message, run.span, &run.expansion)
    };
    if !run.artifact_inputs.is_empty() {
        return Err(fail(
            "standalone bluetcl_run cannot consume generated artifacts".to_owned(),
        ));
    }
    let mut requirements = BTreeSet::new();
    collect_requirements(&run.guard, &mut requirements).map_err(&fail)?;
    requirements.insert(Requirement::Bluetcl);
    let mut operations = vec![map_action(&action).map_err(&fail)?];
    let mut consumption = ImportConsumption::default();
    append_bound_checks(
        bindings,
        assertions,
        comparisons,
        &mut requirements,
        &mut operations,
        &mut consumption,
    )?;
    let syntax = match &run.invocation {
        crate::model::BluetclInvocation::Script { syntax, .. } => match syntax {
            crate::model::BluetclSyntax::Bsv => "bsv",
            crate::model::BluetclSyntax::Bh => "bh",
        },
        crate::model::BluetclInvocation::Exec { .. } => "exec",
        crate::model::BluetclInvocation::InstalledScript { .. } => "installed-script",
        crate::model::BluetclInvocation::Makedepend { .. } => "makedepend",
    };
    Ok(ImportedScenario {
        scenario: Scenario {
            id: format!("bluetcl-{}-{syntax}", action_index + 1),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "bluetcl-run".to_owned(),
                operations,
            }],
        },
        consumption,
    })
}

fn systemc_workflow_scenario(
    workflow_index: usize,
    workflow: &crate::model::SystemcWorkflow,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let Some(_first) = workflow.operations.first() else {
        unreachable!("SystemC workflow composer never emits an empty workflow");
    };
    let fail = |message: String, action: &WorkflowAction| {
        error_diagnostic(
            "import.systemc_workflow",
            message,
            action_span(action),
            action_expansion(action),
        )
    };
    let mut requirements = BTreeSet::from([Requirement::SystemC, Requirement::Bluesim]);
    let mut operations = Vec::new();
    let mut pending_generations = BTreeMap::<String, usize>::new();
    let mut immediately_preceding_unannotated_generation = None;
    let mut has_run = false;
    let mut consumption = ImportConsumption::default();
    for action in &workflow.operations {
        collect_requirements(action.guard(), &mut requirements)
            .map_err(|message| fail(message, action))?;
        match action {
            WorkflowAction::CompileObject(generation) => {
                let operation = map_action(action).map_err(|message| fail(message, action))?;
                let generation_options =
                    parse_arguments(&generation.options, "SystemC generation options")
                        .map_err(|message| fail(message, action))?;
                let modules = generation_modules(generation, &generation_options)
                    .map_err(|message| fail(message, action))?;
                for module in modules {
                    pending_generations.insert(format!("{module}.ba"), operations.len());
                }
                immediately_preceding_unannotated_generation =
                    generation.module.is_none().then_some(operations.len());
                operations.push(operation);
            }
            WorkflowAction::LinkSystemc(link) => {
                if !parse_arguments(&link.options, "SystemC link options")
                    .map_err(|message| fail(message, action))?
                    .iter()
                    .all(|option| option == "-systemc")
                {
                    return Err(fail(
                        "create_systemc_objects only supports the fixed -systemc option".to_owned(),
                        action,
                    ));
                }
                let objects = parse_arguments(&link.objects, "SystemC object inputs")
                    .map_err(|message| fail(message, action))?
                    .into_iter()
                    .map(|path| normalize_path(&path))
                    .collect::<Vec<_>>();
                for object in &objects {
                    let index = pending_generations.get(object).copied().or_else(|| {
                        (objects.len() == 1).then_some(immediately_preceding_unannotated_generation).flatten()
                    }).ok_or_else(|| fail(
                        format!("SystemC object {object:?} is not produced by a preceding explicit generation"),
                        action,
                    ))?;
                    if !operations[index].artifacts.outputs.contains(object) {
                        operations[index].artifacts.outputs.push(object.clone());
                    }
                }
                immediately_preceding_unannotated_generation = None;
                let operation = map_action(action).map_err(|message| fail(message, action))?;
                if let Some(diagnostic) = systemc_link_error_diagnostic_operation(link)
                    .map_err(|message| fail(message, action))?
                {
                    operations.push(operation);
                    operations.push(diagnostic);
                } else {
                    operations.push(operation);
                }
            }
            WorkflowAction::BuildSystemc(_) => {
                immediately_preceding_unannotated_generation = None;
                operations.push(map_action(action).map_err(|message| fail(message, action))?);
            }
            WorkflowAction::RunSystemc(run) => {
                immediately_preceding_unannotated_generation = None;
                has_run = true;
                operations.push(map_action(action).map_err(|message| fail(message, action))?);
                let expected = normalize_path(&run.expected);
                consumption.golden_paths.push(expected.clone());
                operations.push(OperationRecord::new(
                    Action::AssertGolden {
                        actual: format!("{}.out", normalize_path(&run.executable)),
                        expected,
                    },
                    OperationExpectation::Required,
                    provenance(run.span, &run.expansion),
                ));
            }
            _ => {
                return Err(fail(
                    "SystemC workflow contains a non-SystemC action".to_owned(),
                    action,
                ))
            }
        }
    }
    let terminal = workflow
        .operations
        .last()
        .expect("nonempty SystemC workflow");
    let id = match terminal {
        WorkflowAction::RunSystemc(run) => format!("systemc-workflow-{}", run.executable),
        WorkflowAction::LinkSystemc(link) => format!("systemc-link-{}", link.top),
        _ => format!("systemc-workflow-{}", workflow_index + 1),
    };
    let stage = Stage {
        id: if has_run {
            "systemc-run".to_owned()
        } else {
            "systemc-build".to_owned()
        },
        operations,
    };
    Ok(ImportedScenario {
        scenario: Scenario {
            id: if workflow_index == 0 {
                id
            } else {
                format!("{}-{}", workflow_index + 1, id)
            },
            resource: ResourceClass::Heavy,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![stage],
        },
        consumption,
    })
}

fn workflow_scenario(
    workflow_index: usize,
    workflow: &crate::model::BluesimWorkflow,
    workflow_actions: &[WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    unsupported: &[UnsupportedConstruct],
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    bindings: &CheckBindings,
    fixture_root: &Path,
) -> Result<ImportedScenario, ImportDiagnostic> {
    let fail = |message: String, action: &WorkflowAction| {
        error_diagnostic(
            "import.bluesim_workflow",
            message,
            action_span(action),
            action_expansion(action),
        )
    };
    let mut requirements = BTreeSet::new();
    let mut consumption = ImportConsumption::default();
    let workflow_order = workflow
        .generations
        .iter()
        .map(|action| execution_order_key(action.span, &action.expansion))
        .chain(
            workflow
                .pre_link_transfers
                .iter()
                .map(|action| execution_order_key(action.span, &action.expansion)),
        )
        .chain(std::iter::once(execution_order_key(
            workflow.link.span,
            &workflow.link.expansion,
        )))
        .min()
        .expect("Bluesim workflow contains a link action");
    let mut build_operations = Vec::new();
    for (index, action) in closed_preceding_directory_actions(
        &workflow.link.guard,
        workflow_actions,
        already_consumed_actions,
        unsupported,
        assertions,
        comparisons,
        ProvenanceWindow {
            after: None,
            before: Some(&workflow_order),
        },
    ) {
        collect_requirements(action.guard(), &mut requirements)
            .map_err(|message| fail(message, action))?;
        build_operations.push(map_action(action).map_err(|message| fail(message, action))?);
        consumption.actions.push(index);
    }
    for generation in &workflow.generations {
        let action = WorkflowAction::CompileObject(generation.clone());
        collect_requirements(action.guard(), &mut requirements)
            .map_err(|message| fail(message, &action))?;
        build_operations.push(map_action(&action).map_err(|message| fail(message, &action))?);
    }
    for transfer in &workflow.pre_link_transfers {
        let action = WorkflowAction::TransferArtifact(transfer.clone());
        collect_requirements(action.guard(), &mut requirements)
            .map_err(|message| fail(message, &action))?;
        let source = normalize_path(&transfer.source);
        let destination = normalize_path(&transfer.destination);
        let source_is_static_fixture = is_safe_relative(&source)
            && fs::symlink_metadata(fixture_root.join(&source))
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink());
        if transfer.operation != ArtifactTransferOperation::Copy
            || !source_is_static_fixture
            || !is_safe_relative(&destination)
        {
            return Err(fail(
                "pre-link copy must transfer a static, safe local fixture".to_owned(),
                &action,
            ));
        }
        if path_requires_non_windows(&source) || path_requires_non_windows(&destination) {
            requirements.insert(Requirement::NonWindows);
        }
        build_operations.push(map_action(&action).map_err(|message| fail(message, &action))?);
    }
    let link = WorkflowAction::LinkObjects(workflow.link.clone());
    collect_requirements(link.guard(), &mut requirements)
        .map_err(|message| fail(message, &link))?;
    build_operations.push(map_action(&link).map_err(|message| fail(message, &link))?);
    if let Some(diagnostic) =
        link_error_diagnostic_operation(&workflow.link).map_err(|message| fail(message, &link))?
    {
        build_operations.push(diagnostic);
    }
    let mut link_flow = ArtifactFlow::new(link_initial_artifact_paths(&workflow.link));
    for transfer in &workflow.link_transfers {
        let action = WorkflowAction::TransferArtifact(transfer.clone());
        collect_requirements(action.guard(), &mut requirements)
            .map_err(|message| fail(message, &action))?;
        if !link_flow.apply(transfer) {
            return Err(fail(
                format!(
                    "transfer source {:?} is not available after the preceding link operations",
                    normalize_path(&transfer.source)
                ),
                &action,
            ));
        }
        build_operations.push(map_action(&action).map_err(|message| fail(message, &action))?);
    }

    let build_key = ProducerKey::Workflow {
        index: workflow_index,
        stage: WorkflowStageKey::Build,
    };
    append_bound_workflow_actions(
        bindings.workflow_actions(&build_key),
        workflow_actions,
        &mut requirements,
        &mut build_operations,
        &mut consumption,
    )?;
    append_bound_checks(
        bindings.get(&build_key),
        assertions,
        comparisons,
        &mut requirements,
        &mut build_operations,
        &mut consumption,
    )?;
    let mut stages = vec![Stage {
        id: format!("build-{}", workflow.top),
        operations: build_operations,
    }];
    for (run_index, run) in workflow.runs.iter().enumerate() {
        let action = WorkflowAction::RunBluesim(run.action.clone());
        collect_requirements(action.guard(), &mut requirements)
            .map_err(|message| fail(message, &action))?;
        let mut operations = vec![map_action(&action).map_err(|message| fail(message, &action))?];
        let mut run_flow = ArtifactFlow::new(run_initial_artifact_paths(run));
        for transfer in &run.transfers {
            let action = WorkflowAction::TransferArtifact(transfer.clone());
            collect_requirements(action.guard(), &mut requirements)
                .map_err(|message| fail(message, &action))?;
            if !run_flow.apply(transfer) {
                return Err(fail(
                    format!(
                        "transfer source {:?} is not available after the preceding run operations",
                        normalize_path(&transfer.source)
                    ),
                    &action,
                ));
            }
            operations.push(map_action(&action).map_err(|message| fail(message, &action))?);
        }
        let run_key = ProducerKey::Workflow {
            index: workflow_index,
            stage: WorkflowStageKey::Run(run_index),
        };
        append_bound_workflow_actions(
            bindings.workflow_actions(&run_key),
            workflow_actions,
            &mut requirements,
            &mut operations,
            &mut consumption,
        )?;
        append_bound_checks(
            bindings.get(&run_key),
            assertions,
            comparisons,
            &mut requirements,
            &mut operations,
            &mut consumption,
        )?;
        stages.push(Stage {
            id: format!("run-{}", run_index + 1),
            operations,
        });
    }
    requirements.insert(Requirement::Bluesim);
    Ok(ImportedScenario {
        scenario: Scenario {
            id: if workflow_index == 0 {
                format!("bluesim-workflow-{}", workflow.top)
            } else {
                format!("bluesim-workflow-{}-{}", workflow_index + 1, workflow.top)
            },
            resource: ResourceClass::Heavy,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages,
        },
        consumption,
    })
}

fn append_bound_workflow_actions(
    actions: Option<&BTreeSet<usize>>,
    workflow_actions: &[WorkflowAction],
    requirements: &mut BTreeSet<Requirement>,
    operations: &mut Vec<OperationRecord>,
    consumption: &mut ImportConsumption,
) -> Result<(), ImportDiagnostic> {
    let Some(actions) = actions else {
        return Ok(());
    };
    let mut actions = actions.iter().copied().collect::<Vec<_>>();
    actions.sort_by_key(|index| {
        execution_order_key(
            action_span(&workflow_actions[*index]),
            action_expansion(&workflow_actions[*index]),
        )
    });
    for index in actions {
        let action = &workflow_actions[index];
        if !matches!(
            action,
            WorkflowAction::LinkVerilog(_)
                | WorkflowAction::RunVerilog(_)
                | WorkflowAction::ShowRules(_)
                | WorkflowAction::TransferArtifact(_)
                | WorkflowAction::RenderGolden(_)
                | WorkflowAction::RenderM4Curdir(_)
                | WorkflowAction::TextNormalize(_)
                | WorkflowAction::VerilogFilter(_)
        ) {
            continue;
        }
        let mut operation = map_action(action).map_err(|message| {
            error_diagnostic(
                "import.workflow_transform",
                message,
                action_span(action),
                action_expansion(action),
            )
        })?;
        collect_requirements(action.guard(), requirements).map_err(|message| {
            error_diagnostic(
                "import.workflow_transform",
                message,
                action_span(action),
                action_expansion(action),
            )
        })?;
        if operation.action.requires_non_windows() {
            requirements.insert(Requirement::NonWindows);
            if !operation.requires.contains(&Requirement::NonWindows) {
                operation.requires.push(Requirement::NonWindows);
            }
        }
        operations.push(operation);
        consumption.actions.push(index);
    }
    Ok(())
}

fn append_bound_checks(
    checks: Option<&Vec<BoundCheck>>,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    requirements: &mut BTreeSet<Requirement>,
    operations: &mut Vec<OperationRecord>,
    consumption: &mut ImportConsumption,
) -> Result<(), ImportDiagnostic> {
    let Some(checks) = checks else {
        return Ok(());
    };
    for check in checks {
        let (guard, span, expansion, mapped) = match *check {
            BoundCheck::Assertion(index) => {
                let assertion = &assertions[index];
                (
                    &assertion.guard,
                    assertion.span,
                    assertion.expansion.as_slice(),
                    map_assertion(assertion),
                )
            }
            BoundCheck::Comparison(index) => {
                let comparison = &comparisons[index];
                (
                    &comparison.guard,
                    comparison.span,
                    comparison.expansion.as_slice(),
                    map_comparison(comparison),
                )
            }
        };
        let mut operation = mapped.map_err(|message| {
            error_diagnostic("import.bluesim_workflow", message, span, expansion)
        })?;
        collect_check_requirements(guard, requirements, &mut operation.requires).map_err(
            |message| error_diagnostic("import.bluesim_workflow", message, span, expansion),
        )?;
        if let Some(path) = operation.action.asserted_path().map(normalize_path) {
            declare_bound_output(operations, path);
        }
        if operation
            .artifacts
            .inputs
            .iter()
            .any(|path| path_requires_non_windows(path))
        {
            requirements.insert(Requirement::NonWindows);
        }
        consumption.golden_paths.extend(
            operation
                .action
                .expected_paths()
                .into_iter()
                .map(str::to_owned),
        );
        operations.push(operation);
        match *check {
            BoundCheck::Assertion(index) => consumption.assertions.push(index),
            BoundCheck::Comparison(index) => consumption.comparisons.push(index),
        }
    }
    Ok(())
}

fn collect_check_requirements(
    guard: &Guard,
    scenario: &mut BTreeSet<Requirement>,
    operation: &mut Vec<Requirement>,
) -> Result<(), String> {
    match guard {
        Guard::Capability {
            capability: Capability::InternalChecks,
        } => {
            if !operation.contains(&Requirement::InternalChecks) {
                operation.push(Requirement::InternalChecks);
            }
            Ok(())
        }
        Guard::All { guards } => {
            for guard in guards {
                collect_check_requirements(guard, scenario, operation)?;
            }
            Ok(())
        }
        guard => collect_requirements(guard, scenario),
    }
}

fn declare_bound_output(operations: &mut [OperationRecord], path: String) {
    if operations.iter().any(|operation| {
        operation.artifacts.outputs.contains(&path)
            || operation
                .artifacts
                .output_alternatives
                .iter()
                .flatten()
                .any(|alternative| alternative == &path)
    }) {
        return;
    }
    let producer = operations.iter_mut().rev().find(|operation| {
        matches!(
            operation.action,
            Action::BscCompile { .. }
                | Action::BscGenerate { .. }
                | Action::BscLink { .. }
                | Action::SimulationRun { .. }
        )
    });
    if let Some(producer) = producer {
        producer.artifacts.outputs.push(path);
    }
}

fn uniquify_scenario_id(scenario: &mut Scenario, existing: &[Scenario]) {
    if !existing.iter().any(|candidate| candidate.id == scenario.id) {
        return;
    }
    let base = scenario.id.clone();
    let mut occurrence = 2;
    loop {
        let candidate = format!("{base}-{occurrence}");
        if !existing.iter().any(|scenario| scenario.id == candidate) {
            scenario.id = candidate;
            return;
        }
        occurrence += 1;
    }
}

fn same_simulation_invocation(left: &SimulationContract, right: &SimulationContract) -> bool {
    left.source == right.source
        && left.helper == right.helper
        && left.arguments == right.arguments
        && left.generation == right.generation
        && left.guard == right.guard
        && left.span == right.span
        && left.expansion == right.expansion
}

fn known_simulation_output_xfail(
    backend: SimulationBackend,
    annotation: &str,
) -> Result<Option<String>, String> {
    if annotation.is_empty() {
        return Ok(None);
    }
    if annotation.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(Some(format!("upstream bug {annotation}")));
    }
    match backend {
        SimulationBackend::Bluesim => Ok(Some(format!("upstream bug {annotation}"))),
        SimulationBackend::Icarus => {
            let simulators = parse_arguments(annotation, "Verilog known-failure simulator list")?;
            Ok(simulators
                .iter()
                .any(|simulator| simulator == "iverilog")
                .then(|| format!("upstream simulator bug list {annotation:?}")))
        }
    }
}

fn simulation_scenario(
    contracts: &[&SimulationContract],
    previous_contract_order: Option<&ExecutionOrderKey>,
    workflow_actions: &[WorkflowAction],
    already_consumed_actions: &BTreeSet<usize>,
    bound_workflow_actions: Option<&BTreeSet<usize>>,
    assertions: &[AssertionContract],
    comparisons: &[ComparisonContract],
    bindings: Option<&Vec<BoundCheck>>,
    fixture_root: &Path,
) -> Result<Option<ImportedScenario>, ImportDiagnostic> {
    let Some(&contract) = contracts.first() else {
        return Ok(None);
    };
    if contracts
        .iter()
        .any(|candidate| !same_simulation_invocation(contract, candidate))
    {
        return Err(error_diagnostic(
            "import.simulation_contract",
            "simulation backend contracts do not describe one shared invocation".to_owned(),
            contract.span,
            &contract.expansion,
        ));
    }
    let mut backends = Vec::new();
    for contract in contracts {
        if backends.contains(&contract.backend) {
            return Err(error_diagnostic(
                "import.simulation_contract",
                "simulation invocation contains a duplicate backend contract".to_owned(),
                contract.span,
                &contract.expansion,
            ));
        }
        backends.push(contract.backend);
    }

    let fail = |message: String| {
        error_diagnostic(
            "import.simulation_contract",
            message,
            contract.span,
            &contract.expansion,
        )
    };
    let argument = |index: usize| contract.arguments.get(index).map_or("", String::as_str);
    let raw_shape = match contract.helper.as_str() {
        "test_c_veri_worker" => {
            if !(9..=11).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 9 to 11 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let enabled = |index: usize| {
                argument(index)
                    .parse::<i64>()
                    .map(|value| value != 0)
                    .map_err(|error| {
                        format!(
                            "{} has invalid backend flag {:?}: {error}",
                            contract.helper,
                            argument(index)
                        )
                    })
            };
            let expected_backends = [
                enabled(4)
                    .map_err(&fail)?
                    .then_some(SimulationBackend::Bluesim),
                enabled(5)
                    .map_err(&fail)?
                    .then_some(SimulationBackend::Icarus),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            if expected_backends != backends {
                return Err(fail(format!(
                    "{} backend flags describe {expected_backends:?}, but contracts contain {backends:?}",
                    contract.helper
                )));
            }
            RawSimulationShape {
                source: format!("{}.{}", argument(0), argument(3)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                expected_output: argument(6),
                bluesim_failure: argument(7),
                icarus_failure: argument(8),
                sort_output: argument(9),
                check_vcd: if contract.arguments.len() > 10 {
                    argument(10)
                } else {
                    "1"
                },
                ..RawSimulationShape::default()
            }
        }
        "test_c_only" | "test_c_only_bsv" => {
            if !(1..=4).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 1 to 4 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            let extension = if contract.helper == "test_c_only" {
                "bs"
            } else {
                "bsv"
            };
            RawSimulationShape {
                source: format!("{source_stem}.{extension}"),
                top: format!("sys{source_stem}"),
                expected_output: argument(1),
                bluesim_failure: argument(2),
                sort_output: argument(3),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_c_only_bsv_modules" => {
            if !(2..=5).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 2 to 5 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            RawSimulationShape {
                source: format!("{source_stem}.bsv"),
                top: format!("sys{source_stem}"),
                module_list: argument(1),
                expected_output: argument(2),
                bluesim_failure: argument(3),
                sort_output: argument(4),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_c_only_bs_modules_options" | "test_c_only_bsv_modules_options" => {
            if !(3..=8).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 8 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            let extension = if contract.helper == "test_c_only_bs_modules_options" {
                "bs"
            } else {
                "bsv"
            };
            RawSimulationShape {
                source: format!("{source_stem}.{extension}"),
                top: format!("sys{source_stem}"),
                module_list: argument(1),
                generation_options: argument(2),
                expected_output: argument(3),
                bluesim_failure: argument(4),
                link_options: argument(5),
                simulation_options: argument(6),
                sort_output: argument(7),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_c_only_bsv_multi" => {
            if !(3..=7).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 7 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            RawSimulationShape {
                source: format!("{}.bsv", argument(0)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                expected_output: argument(3),
                bluesim_failure: argument(4),
                sort_output: argument(5),
                check_vcd: argument(6),
                ..RawSimulationShape::default()
            }
        }
        "test_c_only_bsv_multi_options" => {
            if !(4..=10).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 4 to 10 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            RawSimulationShape {
                source: format!("{}.bsv", argument(0)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                generation_options: argument(3),
                expected_output: argument(4),
                bluesim_failure: argument(5),
                link_options: argument(6),
                simulation_options: argument(7),
                sort_output: argument(8),
                check_vcd: argument(9),
                ..RawSimulationShape::default()
            }
        }
        "test_veri_only" | "test_veri_only_bsv" => {
            if !(1..=4).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 1 to 4 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            let extension = if contract.helper == "test_veri_only" {
                "bs"
            } else {
                "bsv"
            };
            RawSimulationShape {
                source: format!("{source_stem}.{extension}"),
                top: format!("sys{source_stem}"),
                expected_output: argument(1),
                icarus_failure: argument(2),
                sort_output: argument(3),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_veri_only_bsv_modules" => {
            if !(2..=5).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 2 to 5 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            RawSimulationShape {
                source: format!("{source_stem}.bsv"),
                top: format!("sys{source_stem}"),
                module_list: argument(1),
                expected_output: argument(2),
                icarus_failure: argument(3),
                sort_output: argument(4),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_veri_only_bsv_modules_options" => {
            if !(3..=8).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 8 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            RawSimulationShape {
                source: format!("{source_stem}.bsv"),
                top: format!("sys{source_stem}"),
                module_list: argument(1),
                generation_options: argument(2),
                expected_output: argument(3),
                icarus_failure: argument(4),
                link_options: argument(5),
                simulation_options: argument(6),
                sort_output: argument(7),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_veri_only_bsv_multi" => {
            if !(3..=7).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 7 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            RawSimulationShape {
                source: format!("{}.bsv", argument(0)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                expected_output: argument(3),
                icarus_failure: argument(4),
                sort_output: argument(5),
                check_vcd: argument(6),
                ..RawSimulationShape::default()
            }
        }
        "test_veri_only_bsv_multi_options" => {
            if !(4..=10).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 4 to 10 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            RawSimulationShape {
                source: format!("{}.bsv", argument(0)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                generation_options: argument(3),
                expected_output: argument(4),
                icarus_failure: argument(5),
                link_options: argument(6),
                simulation_options: argument(7),
                sort_output: argument(8),
                check_vcd: argument(9),
                ..RawSimulationShape::default()
            }
        }
        "test_c_veri" | "test_c_veri_bsv" | "test_c_veri_bsv_separately" => {
            if !(1..=5).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 1 to 5 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            let extension = if contract.helper == "test_c_veri" {
                "bs"
            } else {
                "bsv"
            };
            RawSimulationShape {
                source: format!("{source_stem}.{extension}"),
                top: format!("sys{source_stem}"),
                expected_output: argument(1),
                bluesim_failure: argument(2),
                icarus_failure: argument(3),
                sort_output: argument(4),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_c_veri_bs_modules" | "test_c_veri_bsv_modules" => {
            if !(2..=6).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 2 to 6 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            let extension = if contract.helper == "test_c_veri_bs_modules" {
                "bs"
            } else {
                "bsv"
            };
            RawSimulationShape {
                source: format!("{source_stem}.{extension}"),
                top: format!("sys{source_stem}"),
                module_list: argument(1),
                expected_output: argument(2),
                bluesim_failure: argument(3),
                icarus_failure: argument(4),
                sort_output: argument(5),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_c_veri_bs_modules_options"
        | "test_c_veri_bsv_modules_options"
        | "test_c_veri_bsv_modules_options_separately" => {
            if !(3..=9).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 9 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            let source_stem = argument(0);
            let extension = if contract.helper == "test_c_veri_bs_modules_options" {
                "bs"
            } else {
                "bsv"
            };
            RawSimulationShape {
                source: format!("{source_stem}.{extension}"),
                top: format!("sys{source_stem}"),
                module_list: argument(1),
                generation_options: argument(2),
                expected_output: argument(3),
                bluesim_failure: argument(4),
                icarus_failure: argument(5),
                link_options: argument(6),
                simulation_options: argument(7),
                sort_output: argument(8),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        "test_c_veri_bsv_multi" => {
            if !(3..=8).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 8 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            RawSimulationShape {
                source: format!("{}.bsv", argument(0)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                expected_output: argument(3),
                bluesim_failure: argument(4),
                icarus_failure: argument(5),
                sort_output: argument(6),
                check_vcd: argument(7),
                ..RawSimulationShape::default()
            }
        }
        "test_c_veri_bsv_multi_options" | "test_c_veri_bsv_multi_options_separately" => {
            if !(3..=12).contains(&contract.arguments.len()) {
                return Err(fail(format!(
                    "{} requires 3 to 12 static arguments, found {}",
                    contract.helper,
                    contract.arguments.len()
                )));
            }
            RawSimulationShape {
                source: format!("{}.bsv", argument(0)),
                top: argument(1).to_owned(),
                module_list: argument(2),
                generation_options: argument(3),
                expected_output: argument(4),
                bluesim_failure: argument(5),
                icarus_failure: argument(6),
                link_options: argument(9),
                simulation_options: argument(10),
                sort_output: argument(11),
                check_vcd: "1",
                ..RawSimulationShape::default()
            }
        }
        _ => return Ok(None),
    };
    if argument(0).is_empty() || raw_shape.top.is_empty() {
        return Err(fail(format!(
            "{} requires non-empty source and top module names",
            contract.helper
        )));
    }
    let SimulationShape {
        source,
        top,
        modules,
        generation_args,
        link_args,
        simulation_args,
        expected,
        bluesim_xfail,
        icarus_xfail,
        sort_output,
        check_vcd,
    } = parse_simulation_shape(raw_shape, &backends, &contract.helper).map_err(&fail)?;
    if source != normalize_path(&contract.source) {
        return Err(fail(format!(
            "{} recovered source {source:?}, expected {:?}",
            contract.helper, contract.source
        )));
    }

    let generation_mode = match contract.generation {
        crate::model::GenerationStrategy::Shared => {
            if !(backends.contains(&SimulationBackend::Bluesim)
                && backends.contains(&SimulationBackend::Icarus))
            {
                return Err(fail(
                    "shared simulation generation requires Bluesim and Icarus contracts".to_owned(),
                ));
            }
            SimulationGenerationMode::SharedElaboration
        }
        crate::model::GenerationStrategy::Bluesim => {
            if backends != [SimulationBackend::Bluesim] {
                return Err(fail(
                    "Bluesim generation must contain only a Bluesim contract".to_owned(),
                ));
            }
            SimulationGenerationMode::Bluesim
        }
        crate::model::GenerationStrategy::Icarus => {
            if backends != [SimulationBackend::Icarus] {
                return Err(fail(
                    "Verilog generation must contain only an Icarus contract".to_owned(),
                ));
            }
            SimulationGenerationMode::Verilog
        }
    };

    let mut requirements = BTreeSet::new();
    collect_requirements(&contract.guard, &mut requirements).map_err(&fail)?;
    if backends.contains(&SimulationBackend::Bluesim) {
        requirements.insert(Requirement::Bluesim);
    }
    if backends.contains(&SimulationBackend::Icarus) {
        requirements.insert(Requirement::Verilog);
        requirements.insert(Requirement::Icarus);
    }

    let mut simulation_directories = BTreeSet::new();
    for arguments in [&generation_args, &link_args] {
        for directory in option_values(arguments, "-simdir").map_err(&fail)? {
            simulation_directories.insert(normalize_path(&directory));
        }
    }
    let mut consumed_actions = Vec::new();
    let mut operations = Vec::new();
    let contract_order = execution_order_key(contract.span, &contract.expansion);
    if let Some((index, touch)) = simulation_preceding_fixture_touch(
        contract,
        workflow_actions,
        already_consumed_actions,
        previous_contract_order,
        &contract_order,
        assertions,
        comparisons,
        fixture_root,
    ) {
        operations.push(map_action(touch).map_err(&fail)?);
        consumed_actions.push(index);
    }
    for (index, action) in workflow_actions_in_window(
        workflow_actions,
        already_consumed_actions,
        ProvenanceWindow {
            after: previous_contract_order,
            before: Some(&contract_order),
        },
    ) {
        let WorkflowAction::EraseArtifact(erase) = action else {
            continue;
        };
        if erase.guard != contract.guard {
            continue;
        }
        collect_requirements(&erase.guard, &mut requirements).map_err(&fail)?;
        let path = normalize_path(&erase.path);
        if path_requires_non_windows(&path) {
            requirements.insert(Requirement::NonWindows);
        }
        operations.push(OperationRecord::new(
            map_erase(erase, EraseMode::EnsureAbsent),
            OperationExpectation::Required,
            provenance(erase.span, &erase.expansion),
        ));
        consumed_actions.push(index);
    }
    for directory in simulation_directories {
        let matches = workflow_actions
            .iter()
            .enumerate()
            .filter(|(index, action)| {
                !already_consumed_actions.contains(index)
                    && execution_order_key(action_span(action), action_expansion(action))
                        < contract_order
                    && matches!(
                        action,
                        WorkflowAction::CreateDirectory(action)
                            if normalize_path(&action.path) == directory
                                && action.guard == contract.guard
                    )
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(fail(format!(
                "{} requires exactly one matching mkdir for {directory:?}, found {}",
                contract.helper,
                matches.len()
            )));
        }
        let (index, action) = matches[0];
        collect_requirements(action.guard(), &mut requirements).map_err(&fail)?;
        if path_requires_non_windows(&directory) {
            requirements.insert(Requirement::NonWindows);
        }
        operations.push(map_action(action).map_err(&fail)?);
        consumed_actions.push(index);
    }

    let mut objects = vec![top.to_owned()];
    objects.extend(modules.into_iter().map(|path| normalize_path(&path)));
    let mut generation = simulation_operation(
        Action::BscGenerate {
            source: source.clone(),
            mode: generation_mode,
            module: Some(top.to_owned()),
            args: generation_args,
        },
        contract,
    );
    declare_generated_module_artifacts(&mut generation, generation_mode, &objects);
    declare_proven_bdpi_generation_artifacts(&mut generation, fixture_root, &source, &objects);
    operations.push(generation);
    let has_native_link_source = objects.iter().any(|path| is_native_link_source(path));
    let native_link_requires_non_windows =
        native_link_inputs_require_non_windows(&objects, fixture_root);

    for backend in [SimulationBackend::Icarus, SimulationBackend::Bluesim] {
        if !backends.contains(&backend) {
            continue;
        }
        let backend_operation_start = operations.len();
        let backend_contract = contracts
            .iter()
            .copied()
            .find(|contract| contract.backend == backend)
            .expect("validated simulation backend");
        let (plan_backend, suffix, xfail_reason) = match backend {
            SimulationBackend::Bluesim => (
                PlanSimulationBackend::Bluesim,
                "c",
                bluesim_xfail.as_deref(),
            ),
            SimulationBackend::Icarus => {
                (PlanSimulationBackend::Icarus, "v", icarus_xfail.as_deref())
            }
        };
        operations.push(simulation_operation(
            Action::BscLink {
                backend: plan_backend,
                mode: BscLinkMode::Standard,
                objects: objects.clone(),
                top: top.to_owned(),
                args: link_args.clone(),
                expected_exit: ExpectedExit::Success,
                simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                missing_objects: Vec::new(),
            },
            backend_contract,
        ));
        let normal_output = format!("{top}.{suffix}.out");
        operations.push(simulation_operation(
            Action::SimulationRun {
                backend: plan_backend,
                executable: top.to_owned(),
                args: simulation_args.clone(),
                stdout: normal_output.clone(),
                expected_exits: ExpectedExitSet::default(),
                vcd: None,
            },
            backend_contract,
        ));
        let golden_action = if let Some(reason) = xfail_reason {
            Action::AssertGoldenXfail {
                actual: normal_output.clone(),
                expected: expected.clone(),
                reason: reason.to_owned(),
            }
        } else if sort_output {
            Action::AssertGoldenSortedLines {
                actual: normal_output.clone(),
                expected: expected.clone(),
            }
        } else if backend == SimulationBackend::Bluesim && has_native_link_source {
            Action::AssertGoldenNative {
                actual: normal_output.clone(),
                expected: expected.clone(),
            }
        } else {
            Action::AssertGolden {
                actual: normal_output.clone(),
                expected: expected.clone(),
            }
        };
        operations.push(simulation_operation(golden_action, backend_contract));
        if check_vcd {
            let vcd_path = format!("{top}.{suffix}.vcd");
            let vcd_output = format!("{top}.{suffix}-vcd.out");
            operations.push(simulation_operation(
                Action::SimulationRun {
                    backend: plan_backend,
                    executable: top.to_owned(),
                    args: simulation_args.clone(),
                    stdout: vcd_output.clone(),
                    expected_exits: ExpectedExitSet::default(),
                    vcd: Some(vcd_path.clone()),
                },
                backend_contract,
            ));
            if !(backend == SimulationBackend::Icarus && xfail_reason.is_some()) {
                let vcd_assertion = match backend {
                    SimulationBackend::Bluesim => Action::AssertVcdValid { path: vcd_path },
                    SimulationBackend::Icarus => Action::AssertVcdValidIfPresent { path: vcd_path },
                };
                operations.push(simulation_operation(vcd_assertion, backend_contract));
            }
            if backend == SimulationBackend::Bluesim && xfail_reason.is_none() {
                let vcd_output_assertion = if has_native_link_source {
                    Action::AssertGoldenNative {
                        actual: vcd_output,
                        expected: normal_output,
                    }
                } else {
                    Action::AssertGolden {
                        actual: vcd_output,
                        expected: normal_output,
                    }
                };
                operations.push(simulation_operation(vcd_output_assertion, backend_contract));
            }
        }
        if (backend == SimulationBackend::Icarus && has_native_link_source)
            || (backend == SimulationBackend::Bluesim && native_link_requires_non_windows)
        {
            for operation in &mut operations[backend_operation_start..] {
                if !operation.requires.contains(&Requirement::NonWindows) {
                    operation.requires.push(Requirement::NonWindows);
                }
            }
        }
    }

    let mut consumption = ImportConsumption {
        actions: consumed_actions,
        golden_paths: vec![expected],
        ..ImportConsumption::default()
    };
    append_bound_workflow_actions(
        bound_workflow_actions,
        workflow_actions,
        &mut requirements,
        &mut operations,
        &mut consumption,
    )?;
    let bound_check_start = operations.len();
    append_bound_checks(
        bindings,
        assertions,
        comparisons,
        &mut requirements,
        &mut operations,
        &mut consumption,
    )?;
    if backends == [SimulationBackend::Bluesim] && has_native_link_source {
        for operation in &mut operations[bound_check_start..] {
            if let Action::AssertGolden { actual, expected } = &operation.action {
                *operation = OperationRecord::new(
                    Action::AssertGoldenNative {
                        actual: actual.clone(),
                        expected: expected.clone(),
                    },
                    operation.expectation.clone(),
                    operation.provenance.clone(),
                );
            }
        }
    }
    if operations
        .iter()
        .any(|operation| operation.action.requires_non_windows())
    {
        requirements.insert(Requirement::NonWindows);
    }

    Ok(Some(ImportedScenario {
        scenario: Scenario {
            id: format!("simulation-{top}"),
            resource: ResourceClass::Heavy,
            fixtures: Vec::new(),
            requires: requirements.into_iter().collect(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: top.to_owned(),
                operations,
            }],
        },
        consumption,
    }))
}

fn declare_proven_bdpi_generation_artifacts(
    operation: &mut OperationRecord,
    fixture_root: &Path,
    source: &str,
    modules: &[String],
) {
    let resolution = resolve_local_dependency_closures(
        fixture_root,
        &[BTreeSet::from([normalize_path(source)])],
    );
    if !resolution.diagnostics.is_empty() {
        return;
    }
    let Some(paths) = resolution.paths.first() else {
        return;
    };
    let sources = paths
        .iter()
        .filter_map(|path| fs::read_to_string(fixture_root.join(path)).ok())
        .collect::<Vec<_>>();
    for module in modules {
        if Path::new(module)
            .extension()
            .and_then(|value| value.to_str())
            != Some("ba")
        {
            continue;
        }
        let Some(stem) = Path::new(module)
            .file_stem()
            .and_then(|value| value.to_str())
        else {
            continue;
        };
        let escaped = regex::escape(stem);
        let pattern = format!(
            r#"(?ms)\bimport\s+"BDPI"\s+(?:{escaped}\s*=\s*function\b|function\b[^;]*?\b{escaped}\s*\()"#
        );
        let Ok(import) = Regex::new(&pattern) else {
            continue;
        };
        if sources.iter().any(|contents| import.is_match(contents)) {
            let artifact = normalize_path(module);
            if !operation.artifacts.outputs.contains(&artifact) {
                operation.artifacts.outputs.push(artifact);
            }
        }
    }
}

fn option_values(arguments: &[String], option: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == option {
            let value = arguments
                .get(index + 1)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{option} requires a non-empty value"))?;
            values.push(value.clone());
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(values)
}

fn simulation_operation(action: Action, contract: &SimulationContract) -> OperationRecord {
    OperationRecord::new(
        action,
        OperationExpectation::Required,
        provenance(contract.span, &contract.expansion),
    )
}

fn declare_generated_module_artifacts(
    operation: &mut OperationRecord,
    mode: SimulationGenerationMode,
    modules: &[String],
) {
    for module in modules {
        let extension = Path::new(module)
            .extension()
            .and_then(|extension| extension.to_str());
        let artifacts = if extension.is_some() {
            continue;
        } else {
            match mode {
                SimulationGenerationMode::Bluesim => vec![format!("{module}.ba")],
                SimulationGenerationMode::Verilog => vec![format!("{module}.v")],
                SimulationGenerationMode::SharedElaboration => {
                    vec![format!("{module}.ba"), format!("{module}.v")]
                }
            }
        };
        for artifact in artifacts {
            if !operation.artifacts.outputs.contains(&artifact) {
                operation.artifacts.outputs.push(artifact);
            }
        }
    }
    let relocation_args = match &operation.action {
        Action::BscCompile { args, .. } | Action::BscGenerate { args, .. } => Some(args),
        _ => None,
    };
    if let Some(args) = relocation_args {
        operation.artifacts.outputs = relocate_compile_artifact_paths(
            std::mem::take(&mut operation.artifacts.outputs)
                .into_iter()
                .collect(),
            args,
        )
        .into_iter()
        .collect();
    }
    let mut seen = BTreeSet::new();
    operation
        .artifacts
        .outputs
        .retain(|path| seen.insert(path.to_ascii_lowercase()));
}

#[derive(Clone, Copy)]
enum EraseMode {
    RequirePresent,
    EnsureAbsent,
}

fn map_transfer(transfer: &crate::model::ArtifactTransferAction) -> Action {
    let source = normalize_path(&transfer.source);
    let destination = normalize_path(&transfer.destination);
    match transfer.operation {
        ArtifactTransferOperation::Copy => Action::FsCopy {
            source,
            destination,
        },
        ArtifactTransferOperation::Move => Action::FsMove {
            source,
            destination,
        },
    }
}

fn map_erase(erase: &crate::model::EraseArtifactAction, mode: EraseMode) -> Action {
    let path = normalize_path(&erase.path);
    match mode {
        EraseMode::RequirePresent => Action::FsRemove { path },
        EraseMode::EnsureAbsent => Action::FsEnsureAbsent { path },
    }
}

fn map_action(action: &WorkflowAction) -> Result<OperationRecord, String> {
    let mapped = match action {
        WorkflowAction::CompileObject(action) => Action::BscGenerate {
            source: normalize_path(&action.source),
            mode: SimulationGenerationMode::Bluesim,
            module: action.module.clone(),
            args: parse_arguments(&action.options, "generation options")?,
        },
        WorkflowAction::BuildCObject(action) => Action::CObjectBuild {
            source: normalize_path(&action.source),
            makefile: normalize_path(&action.makefile),
            output: normalize_path(&action.output),
        },
        WorkflowAction::LinkObjects(action) => Action::BscLink {
            backend: PlanSimulationBackend::Bluesim,
            mode: BscLinkMode::Standard,
            objects: parse_arguments(&action.objects, "link objects")?
                .into_iter()
                .map(|path| normalize_path(&path))
                .collect(),
            top: action.top.clone(),
            args: parse_arguments(&action.options, "link options")?,
            expected_exit: action.expected_exit,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            missing_objects: Vec::new(),
        },
        WorkflowAction::LinkSystemc(action) => Action::BscSystemcLink {
            objects: parse_arguments(&action.objects, "SystemC link objects")?
                .into_iter()
                .map(|object| normalize_path(&object))
                .collect(),
            top: action.top.clone(),
            expected_exit: action.expected_exit,
        },
        WorkflowAction::BuildSystemc(action) => Action::SystemcCxxLink {
            executable: action.executable.clone(),
            sources: parse_arguments(&action.sources, "SystemC C++ sources")?
                .into_iter()
                .map(|source| normalize_path(&source))
                .collect(),
            top_modules: parse_arguments(&action.top_modules, "SystemC top modules")?,
            other_modules: parse_arguments(&action.other_modules, "SystemC other modules")?,
            defines: parse_systemc_defines(&action.options)?,
        },
        WorkflowAction::RunSystemc(action) => {
            if !parse_arguments(&action.options, "SystemC run options")?.is_empty() {
                return Err(
                    "run_systemc_executable options are not supported by the typed executor"
                        .to_owned(),
                );
            }
            Action::SystemcRun {
                executable: action.executable.clone(),
                stdout: format!("{}.out", normalize_path(&action.executable)),
                sort_output: action.sort_output,
            }
        }
        WorkflowAction::Bsc2Bsv(action) => Action::Bsc2Bsv {
            source: normalize_path(&action.source),
            stdout: normalize_path(&action.stdout),
        },
        WorkflowAction::BscParsePretty(action) => Action::BscParsePretty {
            source: normalize_path(&action.source),
            args: parse_arguments(&action.options, "parse-pretty options")?,
            pretty_output: normalize_path(&action.pretty_output),
        },
        WorkflowAction::BluetclRun(action) => Action::BluetclRun {
            invocation: match &action.invocation {
                crate::model::BluetclInvocation::Script {
                    script,
                    args,
                    syntax,
                } => bsc_test_plan::BluetclInvocation::Script {
                    script: normalize_path(script),
                    args: args.clone(),
                    syntax: match syntax {
                        crate::model::BluetclSyntax::Bsv => bsc_test_plan::BluetclSyntax::Bsv,
                        crate::model::BluetclSyntax::Bh => bsc_test_plan::BluetclSyntax::Bh,
                    },
                },
                crate::model::BluetclInvocation::Exec { script, args } => {
                    bsc_test_plan::BluetclInvocation::Exec {
                        script: normalize_path(script),
                        args: args.clone(),
                    }
                }
                crate::model::BluetclInvocation::InstalledScript { script, args } => {
                    bsc_test_plan::BluetclInvocation::InstalledScript {
                        script: *script,
                        args: args.clone(),
                    }
                }
                crate::model::BluetclInvocation::Makedepend { command, args } => {
                    bsc_test_plan::BluetclInvocation::Makedepend {
                        command: *command,
                        args: args.clone(),
                    }
                }
            },
            working_directory: action.working_directory.clone(),
            artifact_inputs: action
                .artifact_inputs
                .iter()
                .map(|path| normalize_path(path))
                .collect(),
            artifact_outputs: action
                .artifact_outputs
                .iter()
                .map(|path| normalize_path(path))
                .collect(),
            expected_exit: action.expected_exit,
            stdout: normalize_path(&action.stdout),
        },
        WorkflowAction::LinkVerilog(action) => {
            let objects = parse_arguments(&action.objects, "Verilog link objects")?;
            if objects
                .iter()
                .any(|object| object.contains(['*', '?', '[', ']']))
            {
                return Err("link_verilog_pass object globs require shell expansion and are not statically executable".to_owned());
            }
            Action::BscLink {
                backend: PlanSimulationBackend::Icarus,
                mode: if action.no_main {
                    BscLinkMode::NoMain
                } else {
                    BscLinkMode::Standard
                },
                objects: objects
                    .into_iter()
                    .map(|path| normalize_path(&path))
                    .collect(),
                top: action.top.clone(),
                args: if action.no_main {
                    Vec::new()
                } else {
                    parse_arguments(&action.options, "Verilog link options")?
                },
                expected_exit: action.expected_exit,
                simulator: action.simulator,
                missing_objects: Vec::new(),
            }
        }
        WorkflowAction::RunBluesim(action) => Action::SimulationRun {
            backend: PlanSimulationBackend::Bluesim,
            executable: normalize_path(&action.executable),
            args: parse_arguments(&action.options, "Bluesim options")?,
            stdout: normalize_path(&action.stdout),
            expected_exits: ExpectedExitSet::new(
                action.expected_exits.clone(),
                action.aarch64_expected_exits.clone(),
                action.windows_expected_exits.clone(),
            ),
            vcd: None,
        },
        WorkflowAction::RunVerilog(action) => {
            let mut args = parse_arguments(&action.options, "Icarus simulation options")?;
            let options_request_vcd = args.iter().any(|argument| argument == "+bscvcd");
            args.retain(|argument| argument != "+bscvcd");
            Action::SimulationRun {
                backend: PlanSimulationBackend::Icarus,
                executable: normalize_path(&action.executable),
                args,
                stdout: normalize_path(&action.stdout),
                expected_exits: ExpectedExitSet::new(action.expected_exits.clone(), None, None),
                vcd: (action.vcd || options_request_vcd).then(|| "dump.vcd".to_owned()),
            }
        }
        WorkflowAction::ShowRules(action) => Action::ShowRules {
            top: action.top.clone(),
            input: normalize_path(&action.input),
            output: normalize_path(&action.output),
            design_inputs: Vec::new(),
            stdout: normalize_path(&action.stdout),
        },
        WorkflowAction::DumpIntermediate(action) => Action::DumpIntermediate {
            input: normalize_path(&action.input),
            output: normalize_path(&action.output),
            view: match action.view {
                crate::model::IntermediateDumpView::Bi => bsc_test_plan::IntermediateDumpView::Bi,
                crate::model::IntermediateDumpView::Bo => bsc_test_plan::IntermediateDumpView::Bo,
            },
        },
        WorkflowAction::TransferArtifact(action) => map_transfer(action),
        WorkflowAction::EraseArtifact(action) => map_erase(action, EraseMode::RequirePresent),
        WorkflowAction::EnsureDirectoryAbsent(action) => Action::FsEnsureDirectoryAbsent {
            path: normalize_path(&action.path),
        },
        WorkflowAction::CreateDirectory(action) => Action::FsMkdir {
            path: normalize_path(&action.path),
        },
        WorkflowAction::TouchArtifact(action) => Action::FsTouch {
            path: normalize_path(&action.path),
        },
        WorkflowAction::TouchCreateArtifact(action) => Action::FsTouchCreate {
            path: normalize_path(&action.path),
            delay_milliseconds: action.delay_milliseconds,
        },
        WorkflowAction::RemoveUserRead(action) => Action::FsRemoveUserRead {
            path: normalize_path(&action.path),
        },
        WorkflowAction::RewriteDarwinCppIncludePath(action) => {
            Action::FsRewriteDarwinCppIncludePath {
                source: normalize_path(&action.source),
                destination: normalize_path(&action.destination),
            }
        }
        WorkflowAction::RenderGolden(action) => Action::RenderGolden {
            template: normalize_path(&action.template),
            output: normalize_path(&action.output),
            replacement: match action.macro_value {
                GoldenMacroValue::BluespecDir => GoldenReplacement::BluespecDir,
                GoldenMacroValue::WorkDir => GoldenReplacement::WorkDir,
                GoldenMacroValue::FifoWarningLocations => GoldenReplacement::FifoWarningLocations,
            },
        },
        WorkflowAction::RenderM4Curdir(action) => Action::M4CurdirRender {
            template: normalize_path(&action.template),
            output: normalize_path(&action.output),
        },
        WorkflowAction::TextNormalize(action) => Action::TextNormalize {
            source: normalize_path(&action.source),
            destination: normalize_path(&action.destination),
            transform: action.transform,
        },
        WorkflowAction::VerilogFilter(action) => Action::VerilogFilter {
            path: normalize_path(&action.path),
            profiles: action.profiles.clone(),
            expected_exit: action.expected_exit,
        },
        WorkflowAction::Delay(action) => Action::Delay {
            milliseconds: action.milliseconds,
        },
    };
    let expectation = match action {
        WorkflowAction::LinkObjects(action) => action.expectation.clone(),
        WorkflowAction::LinkVerilog(action) => action.expectation.clone(),
        WorkflowAction::BscParsePretty(action) => action.expectation.clone(),
        _ => OperationExpectation::Required,
    };
    let mut operation = OperationRecord::new(
        mapped,
        expectation,
        provenance(action_span(action), action_expansion(action)),
    );
    if let WorkflowAction::CompileObject(generation) = action {
        declare_generation_module_artifacts(&mut operation, generation)?;
    }
    if matches!(action, WorkflowAction::RemoveUserRead(_)) {
        operation.requires.push(Requirement::PosixUnreadability);
    }
    if operation.action.requires_non_windows()
        && !operation.requires.contains(&Requirement::NonWindows)
    {
        operation.requires.push(Requirement::NonWindows);
    }
    attach_bluetcl_package_requirement(&mut operation, action.guard());
    Ok(operation)
}

fn map_assertion(assertion: &AssertionContract) -> Result<OperationRecord, String> {
    let argument = |index: usize| {
        assertion
            .arguments
            .get(index)
            .cloned()
            .ok_or_else(|| format!("{} is missing argument {index}", assertion.helper))
    };
    let path = || argument(0).map(|path| normalize_path(&path));
    let count = |index: usize| {
        argument(index)?
            .parse::<usize>()
            .map_err(|error| format!("{} has an invalid count: {error}", assertion.helper))
    };
    let action = match assertion.helper.as_str() {
        "files_exist" => {
            if assertion.arguments.len() != 1 {
                return Err("files_exist requires exactly one file path".to_owned());
            }
            Action::AssertExists { path: path()? }
        }
        "find_n_strings" | "find_n_strings_bug" => Action::AssertTextCount {
            path: path()?,
            text: argument(1)?,
            count: count(2)?,
        },
        "find_n_error" | "find_n_warning" => {
            if assertion.arguments.len() != 3 {
                return Err(format!(
                    "{} requires filename, diagnostic code, and count",
                    assertion.helper
                ));
            }
            let code = argument(1)?;
            let bytes = code.as_bytes();
            if bytes.len() != 5
                || !bytes[0].is_ascii_uppercase()
                || !bytes[1..].iter().all(u8::is_ascii_digit)
            {
                return Err(format!(
                    "{} diagnostic tag {code:?} is not a literal BSC diagnostic code",
                    assertion.helper
                ));
            }
            Action::AssertDiagnosticCount {
                path: path()?,
                kind: if assertion.helper == "find_n_error" {
                    DiagnosticKind::Error
                } else {
                    DiagnosticKind::Warning
                },
                code: Some(code),
                count: count(2)?,
            }
        }
        "no_warnings" => {
            if assertion.arguments.len() != 1 {
                return Err("no_warnings requires exactly one output path".to_owned());
            }
            Action::AssertDiagnosticCount {
                path: path()?,
                kind: DiagnosticKind::Warning,
                code: None,
                count: 0,
            }
        }
        "string_occurs" => Action::AssertTextContains {
            path: path()?,
            text: argument(1)?,
        },
        "string_does_not_occur" => Action::AssertTextAbsent {
            path: path()?,
            text: argument(1)?,
        },
        "find_regexp" | "find_regexp_bug" => Action::AssertRegex {
            path: path()?,
            pattern: canonical_tcl_regex(&argument(1)?),
        },
        "find_regexp_fail" | "find_regexp_fail_bug" => Action::AssertRegexAbsent {
            path: path()?,
            pattern: canonical_tcl_regex(&argument(1)?),
        },
        "find_n_regexp" => Action::AssertRegexCount {
            path: path()?,
            pattern: canonical_tcl_regex(&argument(1)?),
            count: count(2)?,
        },
        "find_n_emsg" => Action::AssertDiagnosticCount {
            path: path()?,
            kind: match argument(1)?.trim_matches('"') {
                "Error" | "error" => DiagnosticKind::Error,
                "Warning" | "warning" => DiagnosticKind::Warning,
                kind => {
                    return Err(format!(
                        "find_n_emsg has an unknown diagnostic kind {kind:?}"
                    ))
                }
            },
            code: Some(argument(2)?),
            count: count(3)?,
        },
        "vcdcheck_pass" | "vcdcheck_fail" => {
            if assertion.arguments.len() != 2 {
                return Err(format!(
                    "{} requires exactly a VCD path and a static option list",
                    assertion.helper
                ));
            }
            let options = parse_static_tcl_list(&argument(1)?).map_err(|error| {
                format!(
                    "{} options are not a static Tcl list: {error}",
                    assertion.helper
                )
            })?;
            let mut checks = Vec::new();
            let mut options = options.into_iter();
            while let Some(option) = options.next() {
                if option != "-c" {
                    return Err(format!(
                        "{} only supports repeated -c CHECK options, found {option:?}",
                        assertion.helper
                    ));
                }
                let check = options
                    .next()
                    .ok_or_else(|| format!("{} has a dangling -c option", assertion.helper))?;
                if check.is_empty() {
                    return Err(format!("{} checks must not be empty", assertion.helper));
                }
                checks.push(check);
            }
            if checks.is_empty() {
                return Err(format!(
                    "{} requires at least one -c CHECK option",
                    assertion.helper
                ));
            }
            Action::VcdCheck {
                path: path()?,
                checks,
                expected_exit: if assertion.helper == "vcdcheck_pass" {
                    ExpectedExit::Success
                } else {
                    ExpectedExit::Failure
                },
            }
        }
        helper => return Err(format!("unsupported assertion helper {helper}")),
    };
    let expectation = match assertion.helper.as_str() {
        "find_n_strings_bug" => assertion
            .arguments
            .get(3)
            .map_or(OperationExpectation::Required, |bug| {
                known_bug_expectation(bug)
            }),
        "find_regexp_bug" | "find_regexp_fail_bug" => assertion
            .arguments
            .get(2)
            .map_or(OperationExpectation::Required, |bug| {
                known_bug_expectation(bug)
            }),
        _ => OperationExpectation::Required,
    };
    let mut operation = OperationRecord::new(
        action,
        expectation,
        provenance(assertion.span, &assertion.expansion),
    );
    attach_bluetcl_package_requirement(&mut operation, &assertion.guard);
    Ok(operation)
}

fn map_comparison(comparison: &ComparisonContract) -> Result<OperationRecord, String> {
    if comparison.arguments.is_empty() {
        return Err(format!("{} requires an actual path", comparison.helper));
    }
    let actual = normalize_path(&comparison.arguments[0]);
    if comparison.helper == "compare_file_list" {
        if !(2..=3).contains(&comparison.arguments.len()) {
            return Err(
                "compare_file_list requires a filename, expected list, and optional status label"
                    .to_owned(),
            );
        }
        let expected = parse_static_tcl_list(&comparison.arguments[1])
            .map_err(|error| format!("compare_file_list expected list is not static: {error}"))?
            .into_iter()
            .map(|path| normalize_path(&path))
            .collect::<Vec<_>>();
        if expected.is_empty() {
            return Err("compare_file_list expected list must not be empty".to_owned());
        }
        let mut operation = OperationRecord::new(
            Action::AssertGoldenAny { actual, expected },
            OperationExpectation::Required,
            provenance(comparison.span, &comparison.expansion),
        );
        attach_bluetcl_package_requirement(&mut operation, &comparison.guard);
        return Ok(operation);
    }
    let (expected, expectation) =
        match comparison.helper.as_str() {
            "compare_file_bug" => match comparison.arguments.as_slice() {
                [_] => (format!("{actual}.expected"), OperationExpectation::Required),
                [_, bug] if is_numeric_bug_id(bug) => {
                    (format!("{actual}.expected"), known_bug_expectation(bug))
                }
                [_, expected] => (normalize_path(expected), OperationExpectation::Required),
                [_, expected, bug] => (
                    if expected.is_empty() {
                        format!("{actual}.expected")
                    } else {
                        normalize_path(expected)
                    },
                    known_bug_expectation(bug),
                ),
                _ => return Err(
                    "compare_file_bug accepts filename, optional expected path, and optional bug"
                        .to_owned(),
                ),
            },
            "compare_verilog_bug" => {
                if comparison.arguments.len() > 3 {
                    return Err(
                        "compare_verilog_bug accepts filename, bug, and optional expected path"
                            .to_owned(),
                    );
                }
                let expected = comparison
                    .arguments
                    .get(2)
                    .filter(|path| !path.is_empty())
                    .map(|path| normalize_path(path))
                    .unwrap_or_else(|| format!("{actual}.expected"));
                let expectation = comparison
                    .arguments
                    .get(1)
                    .map_or(OperationExpectation::Required, |bug| {
                        known_bug_expectation(bug)
                    });
                (expected, expectation)
            }
            "compare_file_filter_prelude" => {
                if comparison.arguments.len() > 2 {
                    return Err(
                    "compare_file_filter_prelude accepts an actual path and optional expected path"
                        .to_owned(),
                );
                }
                let expected = comparison
                    .arguments
                    .get(1)
                    .filter(|path| !path.is_empty())
                    .map(|path| normalize_path(path))
                    .unwrap_or_else(|| format!("{actual}.expected"));
                (expected, OperationExpectation::Required)
            }
            "compare_file_filtered" => {
                if comparison.arguments.len() > 4 {
                    return Err("compare_file_filtered accepts at most four arguments".to_owned());
                }
                let expected = comparison
                    .arguments
                    .get(1)
                    .filter(|path| !path.is_empty())
                    .map(|path| normalize_path(path))
                    .unwrap_or_else(|| format!("{actual}.expected"));
                (expected, OperationExpectation::Required)
            }
            "compare_file_filter_ids" => {
                if comparison.arguments.len() > 4 {
                    return Err("compare_file_filter_ids accepts at most four arguments".to_owned());
                }
                let expected = comparison
                    .arguments
                    .get(1)
                    .filter(|path| !path.is_empty())
                    .map(|path| normalize_path(path))
                    .unwrap_or_else(|| format!("{actual}.expected"));
                (expected, OperationExpectation::Required)
            }
            _ => {
                if comparison.arguments.len() > 2 {
                    return Err(format!(
                        "{} requires one actual path and at most one expected path",
                        comparison.helper
                    ));
                }
                let expected = comparison
                    .arguments
                    .get(1)
                    .filter(|path| !path.is_empty())
                    .map(|path| normalize_path(path))
                    .unwrap_or_else(|| format!("{actual}.expected"));
                (expected, OperationExpectation::Required)
            }
        };
    let action = match comparison.helper.as_str() {
        "compare_file" | "compare_file_bug" => Action::AssertGolden { actual, expected },
        "compare_bluetcl"
        | "compare_bluetcl_position_digits"
        | "compare_bluetcl_creg_positions"
        | "compare_bluetcl_libraries"
        | "compare_bluetcl_prelude_library" => {
            let mut normalizations = vec![GoldenNormalization::BluetclOutput];
            normalizations.extend(match comparison.helper.as_str() {
                "compare_bluetcl_position_digits" => {
                    [GoldenNormalization::BluetclPositionDigits].as_slice()
                }
                "compare_bluetcl_creg_positions" => {
                    [GoldenNormalization::BluetclCregPositions].as_slice()
                }
                "compare_bluetcl_libraries" => [GoldenNormalization::BluetclLibraries].as_slice(),
                "compare_bluetcl_prelude_library" => {
                    [GoldenNormalization::BluetclPreludeLibrary].as_slice()
                }
                _ => [].as_slice(),
            });
            Action::AssertGoldenNormalized {
                actual,
                expected,
                normalizations,
            }
        }
        "compare_file_filtered_times" => Action::AssertGoldenNormalized {
            actual,
            expected,
            normalizations: vec![GoldenNormalization::BracketedTimes],
        },
        "compare_file_split_if_rules" => Action::AssertGoldenNormalized {
            actual,
            expected,
            normalizations: vec![GoldenNormalization::SplitIfRules],
        },
        "compare_file_filter_prelude" => Action::AssertGoldenNormalized {
            actual,
            expected,
            normalizations: vec![GoldenNormalization::PreludePositions],
        },
        "compare_file_filtered" => {
            let bre_options = comparison.arguments.get(2).map_or("", |value| value.trim());
            let ere_options = comparison.arguments.get(3).map_or("", |value| value.trim());
            let normalizations = match (bre_options, ere_options) {
                ("s+HERE+HERE+g", "") => vec![GoldenNormalization::WorkspaceRoot],
                ("", "s+\\`line\\(.\\*\\)+\\`line\\(POS\\)+g") => {
                    vec![GoldenNormalization::LineDirectivePositions]
                }
                ("/Bluespec\\ Compiler.*/d", "") => {
                    vec![GoldenNormalization::CompilerBannerLines]
                }
                ("", "-e \"/^ *Time:/d\" -e \"/^ *Scope:/d\" -e \"s/^(ERROR|FATAL):.*: //\"") => {
                    vec![GoldenNormalization::SystemVerilogTaskDiagnostics]
                }
                (
                    "",
                    r#"-e s/\"PreludeBSV\.bsv\",\ line\ \[0-9\]\+,/\"PreludeBSV\.bsv\"\,\ line\ NNNN,/g"#,
                ) => {
                    vec![GoldenNormalization::PreludeBsvLineNumbers]
                }
                _ => {
                    return Err(
                        "compare_file_filtered only supports an audited normalization filter"
                            .to_owned(),
                    )
                }
            };
            Action::AssertGoldenNormalized {
                actual,
                expected,
                normalizations,
            }
        }
        "compare_file_filter_ids" => {
            if comparison.arguments.len() > 4 {
                return Err("compare_file_filter_ids accepts at most four arguments".to_owned());
            }
            let bre_options = comparison.arguments.get(2).map_or("", |value| value.trim());
            let ere_options = comparison.arguments.get(3).map_or("", |value| value.trim());
            let mut normalizations = vec![GoldenNormalization::GeneratedIds];
            match (bre_options, ere_options) {
                ("", "") => {}
                ("", "-e s/VRWire\\[0-9\\]\\+/VRWireNNNN/g") => {
                    normalizations.push(GoldenNormalization::VrWireIds);
                }
                _ => {
                    return Err(
                        "compare_file_filter_ids custom sed filters are not representable yet"
                            .to_owned(),
                    );
                }
            }
            Action::AssertGoldenNormalized {
                actual,
                expected,
                normalizations,
            }
        }
        "compare_verilog" | "compare_verilog_bug" => Action::AssertVerilog { actual, expected },
        helper => return Err(format!("unsupported comparison helper {helper}")),
    };
    let mut operation = OperationRecord::new(
        action,
        expectation,
        provenance(comparison.span, &comparison.expansion),
    );
    attach_bluetcl_package_requirement(&mut operation, &comparison.guard);
    Ok(operation)
}

fn attach_bluetcl_package_requirement(operation: &mut OperationRecord, guard: &Guard) {
    match guard {
        Guard::Capability {
            capability: Capability::BluetclPackage(package),
        } => {
            let requirement = Requirement::BluetclPackage(*package);
            if !operation.requires.contains(&requirement) {
                operation.requires.push(requirement);
                operation.requires.sort();
            }
        }
        Guard::All { guards } => {
            for guard in guards {
                attach_bluetcl_package_requirement(operation, guard);
            }
        }
        Guard::Always
        | Guard::Capability { .. }
        | Guard::Not { .. }
        | Guard::UnsupportedExpression { .. } => {}
    }
}

fn collect_requirements(
    guard: &Guard,
    requirements: &mut BTreeSet<Requirement>,
) -> Result<(), String> {
    match guard {
        Guard::Always => Ok(()),
        Guard::Capability { capability } => {
            requirements.insert(match capability {
                Capability::Bluesim => Requirement::Bluesim,
                Capability::Verilog => Requirement::Verilog,
                Capability::SystemC => Requirement::SystemC,
                Capability::ShowRules => Requirement::ShowRules,
                Capability::InternalChecks => Requirement::InternalChecks,
                Capability::Darwin => Requirement::Darwin,
                Capability::BluetclPackage(_) => return Ok(()),
            });
            Ok(())
        }
        Guard::All { guards } => {
            for guard in guards {
                collect_requirements(guard, requirements)?;
            }
            Ok(())
        }
        Guard::Not { guard } => match guard.as_ref() {
            Guard::Capability {
                capability: Capability::Verilog,
            } => {
                requirements.insert(Requirement::Frontend);
                Ok(())
            }
            _ => Err(format!(
                "negative capability guard {guard:?} is not representable yet"
            )),
        },
        Guard::UnsupportedExpression { source, .. } => {
            Err(format!("dynamic guard {source:?} is not representable"))
        }
    }
}

fn scenario_dependency_roots(scenario: &Scenario) -> BTreeSet<String> {
    let mut roots = BTreeSet::new();
    let mut produced = BTreeSet::new();
    for operation in scenario.stages.iter().flat_map(|stage| &stage.operations) {
        match &operation.action {
            Action::BscCompile {
                source,
                working_directory,
                ..
            } => {
                let source = working_directory.as_ref().map_or_else(
                    || source.clone(),
                    |directory| format!("{directory}/{source}"),
                );
                if !produced.contains(&source) {
                    roots.insert(source);
                }
                roots.extend(operation.artifacts.inputs.iter().filter_map(|input| {
                    (!produced.contains(input)
                        && Path::new(input)
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| matches!(extension, "bsv" | "bs")))
                    .then(|| input.clone())
                }));
            }
            Action::BscGenerate { source, .. } => {
                if !produced.contains(source) {
                    roots.insert(source.clone());
                }
            }
            _ => {}
        }
        for removed in &operation.artifacts.removes {
            produced.remove(removed);
        }
        produced.extend(operation.artifacts.outputs.iter().cloned());
        produced.extend(
            operation
                .artifacts
                .output_alternatives
                .iter()
                .flatten()
                .cloned(),
        );
    }
    roots
}

fn verilog_search_directories(arguments: &[String]) -> BTreeSet<String> {
    option_values(arguments, "-vsearch")
        .unwrap_or_default()
        .into_iter()
        .flat_map(|value| {
            value
                .split([':', ';'])
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .map(|path| normalize_path(&path))
        .filter(|path| path != "+" && is_safe_relative(path))
        .collect()
}

fn collect_local_verilog_files(fixture_root: &Path, relative: &Path, files: &mut BTreeSet<String>) {
    let directory = fixture_root.join(relative);
    let Ok(metadata) = fs::symlink_metadata(&directory) else {
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(&directory) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let relative = relative.join(entry.file_name());
        let path = fixture_root.join(&relative);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_local_verilog_files(fixture_root, &relative, files);
        } else if metadata.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(extension.to_ascii_lowercase().as_str(), "v" | "sv" | "vh")
                })
        {
            files.insert(unix_path(&relative));
        }
    }
}

fn append_local_verilog_search_dependencies(scenarios: &mut [Scenario], fixture_root: &Path) {
    for operation in scenarios
        .iter_mut()
        .flat_map(|scenario| &mut scenario.stages)
        .flat_map(|stage| &mut stage.operations)
    {
        let Action::BscLink {
            backend: PlanSimulationBackend::Icarus,
            args,
            ..
        } = &operation.action
        else {
            continue;
        };
        let mut files = BTreeSet::new();
        for directory in verilog_search_directories(args) {
            collect_local_verilog_files(fixture_root, Path::new(&directory), &mut files);
        }
        for path in files {
            if !operation.artifacts.inputs.contains(&path) {
                operation.artifacts.inputs.push(path);
            }
        }
    }
}

fn append_foreign_link_dependencies(
    scenarios: &mut [Scenario],
    foreign_link_paths: &[BTreeSet<String>],
) {
    for (scenario, foreign_paths) in scenarios.iter_mut().zip(foreign_link_paths) {
        for operation in scenario
            .stages
            .iter_mut()
            .flat_map(|stage| &mut stage.operations)
        {
            let Action::BscLink {
                backend: PlanSimulationBackend::Icarus,
                objects,
                ..
            } = &mut operation.action
            else {
                continue;
            };
            for path in foreign_paths {
                if !objects
                    .iter()
                    .any(|object| link_object_path(PlanSimulationBackend::Icarus, object) == *path)
                {
                    objects.push(path.clone());
                }
                if !operation.artifacts.inputs.contains(path) {
                    operation.artifacts.inputs.push(path.clone());
                }
            }
        }
    }
}

fn declare_dependency_generation_artifacts(scenarios: &mut [Scenario], fixture_root: &Path) {
    for scenario in scenarios {
        // SystemC links name their required `.ba` inputs explicitly. Its workflow
        // mapper assigns those outputs to their individual producers.
        if scenario.requires.contains(&Requirement::SystemC) {
            continue;
        }
        for operation in scenario
            .stages
            .iter_mut()
            .flat_map(|stage| &mut stage.operations)
        {
            let (source, mode) = match &operation.action {
                Action::BscGenerate { source, mode, .. } => (source.clone(), *mode),
                Action::BscCompile {
                    source,
                    mode: BscCompileMode::Verilog | BscCompileMode::VerilogSchedule,
                    expected_exit,
                    args,
                    ..
                } if *expected_exit == ExpectedExit::Success
                    || args
                        .iter()
                        .any(|argument| argument == "-continue-after-errors") =>
                {
                    (source.clone(), SimulationGenerationMode::Verilog)
                }
                _ => continue,
            };
            let resolution = resolve_local_dependency_closures(
                fixture_root,
                &[BTreeSet::from([normalize_path(&source)])],
            );
            if !resolution.diagnostics.is_empty() {
                continue;
            }
            let Some(paths) = resolution.paths.first() else {
                continue;
            };
            let modules = synthesized_modules(fixture_root, paths);
            declare_generated_module_artifacts(operation, mode, &modules);
        }
    }
}

fn declare_showrules_design_inputs(scenarios: &mut [Scenario], fixture_root: &Path) {
    for scenario in scenarios {
        let mut bindings = Vec::new();
        for (stage_index, stage) in scenario.stages.iter().enumerate() {
            for (operation_index, operation) in stage.operations.iter().enumerate() {
                let Action::ShowRules { top, .. } = &operation.action else {
                    continue;
                };
                let preceding = scenario
                    .stages
                    .iter()
                    .enumerate()
                    .flat_map(|(candidate_stage, stage)| {
                        stage.operations.iter().enumerate().map(
                            move |(candidate_operation, operation)| {
                                (candidate_stage, candidate_operation, operation)
                            },
                        )
                    })
                    .take_while(|(candidate_stage, candidate_operation, _)| {
                        (*candidate_stage, *candidate_operation) < (stage_index, operation_index)
                    })
                    .filter_map(|(candidate_stage, candidate_operation, operation)| {
                        let source = match &operation.action {
                            Action::BscGenerate {
                                source,
                                module: Some(module),
                                ..
                            } if module == top => source,
                            Action::BscCompile {
                                source,
                                module: Some(module),
                                args,
                                expected_exit: ExpectedExit::Success,
                                ..
                            } if module == top
                                && args.iter().any(|argument| argument == "-elab") =>
                            {
                                source
                            }
                            _ => return None,
                        };
                        Some((candidate_stage, candidate_operation, source.clone()))
                    })
                    .last();
                let Some((producer_stage, producer_operation, source)) = preceding else {
                    continue;
                };
                let resolution = resolve_local_dependency_closures(
                    fixture_root,
                    &[BTreeSet::from([normalize_path(&source)])],
                );
                if !resolution.diagnostics.is_empty() {
                    continue;
                }
                let Some(paths) = resolution.paths.first() else {
                    continue;
                };
                let design_inputs = synthesized_modules(fixture_root, paths)
                    .into_iter()
                    .map(|module| format!("{module}.ba"))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                if design_inputs.is_empty()
                    || !design_inputs
                        .iter()
                        .any(|path| path == &format!("{top}.ba"))
                {
                    continue;
                }
                bindings.push((
                    stage_index,
                    operation_index,
                    producer_stage,
                    producer_operation,
                    design_inputs,
                ));
            }
        }
        for (stage_index, operation_index, producer_stage, producer_operation, design_inputs) in
            bindings
        {
            let producer = &mut scenario.stages[producer_stage].operations[producer_operation];
            for design_input in &design_inputs {
                if !producer.artifacts.outputs.contains(design_input) {
                    producer.artifacts.outputs.push(design_input.clone());
                }
            }
            let operation = &mut scenario.stages[stage_index].operations[operation_index];
            let Action::ShowRules {
                design_inputs: declared,
                ..
            } = &mut operation.action
            else {
                unreachable!("showrules binding must reference a showrules operation");
            };
            *declared = design_inputs;
            operation.artifacts = ArtifactContract::for_action(&operation.action);
        }
    }
}

fn compose_persistent_generated_artifact_producers(
    scenarios: &mut [Scenario],
    fixture_root: &Path,
) {
    let producers = scenarios
        .iter()
        .flat_map(|scenario| &scenario.stages)
        .flat_map(|stage| &stage.operations)
        .filter(|operation| matches!(operation.action, Action::BscGenerate { .. }))
        .filter(|operation| {
            operation.artifacts.inputs.iter().all(|input| {
                is_safe_relative(input)
                    && fs::symlink_metadata(fixture_root.join(input)).is_ok_and(|metadata| {
                        metadata.is_file() && !metadata.file_type().is_symlink()
                    })
            })
        })
        .map(|operation| (operation_order(operation), operation.clone()))
        .collect::<Vec<_>>();

    for scenario in scenarios {
        let Some(start) = scenario_start_order(scenario) else {
            continue;
        };
        let required_inputs = scenario_external_inputs(scenario);
        let mut selected = Vec::<(ExecutionOrderKey, OperationRecord)>::new();
        for input in required_inputs {
            if Path::new(&input)
                .extension()
                .and_then(|value| value.to_str())
                != Some("ba")
                || fs::symlink_metadata(fixture_root.join(&input)).is_ok()
            {
                continue;
            }
            let mut matches = producers
                .iter()
                .filter(|(order, operation)| {
                    *order < start && operation.artifacts.outputs.contains(&input)
                })
                .collect::<Vec<_>>();
            matches.sort_by(|left, right| left.0.cmp(&right.0));
            let Some((order, operation)) = matches.last() else {
                continue;
            };
            if matches
                .iter()
                .rev()
                .skip(1)
                .next()
                .is_some_and(|(previous, _)| previous == order)
            {
                continue;
            }
            if !selected
                .iter()
                .any(|(_, selected_operation)| *selected_operation == *operation)
            {
                selected.push(((*order).clone(), (*operation).clone()));
            }
        }
        selected.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some(stage) = scenario.stages.first_mut() {
            for (_, operation) in selected.into_iter().rev() {
                stage.operations.insert(0, operation);
            }
        }
    }
}

fn synthesize_module_regex() -> Regex {
    Regex::new(
        r"(?ms)^(?:[ \t]*\(\*\s*synthesize\s*\*\)(?:\s*\(\*.*?\*\))*\s*module(?:\s*\[\s*Module\s*\])?\s+|[ \t]*\{-#\s*verilog\s+)([A-Za-z_][A-Za-z0-9_$]*)",
    )
    .expect("valid synthesize module regex")
}

fn synthesized_modules(fixture_root: &Path, sources: &BTreeSet<String>) -> Vec<String> {
    let synthesize = synthesize_module_regex();
    sources
        .iter()
        .filter_map(|source| fs::read_to_string(fixture_root.join(source)).ok())
        .flat_map(|contents| {
            synthesize
                .captures_iter(&contents)
                .filter_map(|capture| capture.get(1))
                .map(|module| module.as_str().to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn prepend_prior_compile_prerequisites(
    scenarios: &mut [Scenario],
    dependency_paths: &[BTreeSet<String>],
) {
    let mut prior_compiles = Vec::<(String, OperationRecord)>::new();
    for (scenario, dependencies) in scenarios.iter_mut().zip(dependency_paths) {
        let primary = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .find_map(|operation| match &operation.action {
                Action::BscCompile {
                    source,
                    expected_exit: ExpectedExit::Success,
                    ..
                } => Some((normalize_path(source), operation.clone())),
                _ => None,
            });
        let Some((source, operation)) = primary else {
            continue;
        };
        let prerequisites = prior_compiles
            .iter()
            .filter(|(candidate, _)| candidate != &source && dependencies.contains(candidate))
            .map(|(_, operation)| operation.clone())
            .collect::<Vec<_>>();
        if !prerequisites.is_empty() {
            scenario.stages[0].operations.splice(0..0, prerequisites);
        }
        prior_compiles.retain(|(candidate, _)| candidate != &source);
        prior_compiles.push((source, operation));
    }
}

fn reconcile_expected_failure_link_inputs(scenarios: &mut [Scenario], fixture_root: &Path) {
    // Upstream runs each test directory in one shared work directory, so a
    // Bluesim link may reference objects produced by an earlier test's
    // compile. For links whose asserted failure is a missing module (G0084),
    // pull the producing operation forward into the dependent scenario; an
    // object that nothing produces is the asserted failure itself and stays
    // in the link arguments without being a declared input.
    let mut prior_producers = Vec::<(String, OperationRecord)>::new();
    for scenario in scenarios.iter_mut() {
        let asserts_missing_module = scenario.stages.iter().any(|stage| {
            stage.operations.iter().any(|candidate| {
                matches!(&candidate.action,
                    Action::AssertDiagnosticCount { code: Some(code), .. }
                        if code == "G0084")
            })
        });
        let mut pending_removals = Vec::<(usize, usize, String)>::new();
        let mut prepend = Vec::<OperationRecord>::new();
        for (stage_index, stage) in scenario.stages.iter().enumerate() {
            for (operation_index, operation) in stage.operations.iter().enumerate() {
                let Action::BscLink {
                    backend: PlanSimulationBackend::Bluesim,
                    expected_exit: ExpectedExit::Failure,
                    objects,
                    top,
                    ..
                } = &operation.action
                else {
                    continue;
                };
                if !asserts_missing_module {
                    continue;
                }
                let locally_produced = scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .flat_map(|operation| operation.artifacts.outputs.iter().cloned())
                    .chain(
                        prepend
                            .iter()
                            .flat_map(|operation| operation.artifacts.outputs.iter().cloned()),
                    )
                    .collect::<BTreeSet<_>>();
                let mut required = objects
                    .iter()
                    .map(|object| link_object_path(PlanSimulationBackend::Bluesim, object))
                    .collect::<Vec<_>>();
                required.push(link_object_path(PlanSimulationBackend::Bluesim, top));
                for input in required {
                    if locally_produced.contains(&input) || fixture_root.join(&input).is_file() {
                        continue;
                    }
                    if let Some((_, producer)) = prior_producers
                        .iter()
                        .find(|(artifact, _)| *artifact == input)
                    {
                        prepend.push(producer.clone());
                    } else if objects.contains(&input) {
                        pending_removals.push((stage_index, operation_index, input));
                    }
                }
            }
        }
        for (stage_index, operation_index, object) in pending_removals {
            let operation = &mut scenario.stages[stage_index].operations[operation_index];
            let input_path = link_object_path(PlanSimulationBackend::Bluesim, &object);
            operation
                .artifacts
                .inputs
                .retain(|candidate| *candidate != input_path);
            if let Action::BscLink {
                missing_objects, ..
            } = &mut operation.action
            {
                if !missing_objects.contains(&object) {
                    missing_objects.push(object);
                }
            }
        }
        if !prepend.is_empty() {
            scenario.stages[0].operations.splice(0..0, prepend);
        }
        for stage in &scenario.stages {
            for operation in &stage.operations {
                for output in &operation.artifacts.outputs {
                    prior_producers.retain(|(artifact, _)| artifact != output);
                    prior_producers.push((output.clone(), operation.clone()));
                }
            }
        }
    }
}

fn native_link_inputs_require_non_windows(objects: &[String], fixture_root: &Path) -> bool {
    objects
        .iter()
        .filter(|path| is_native_link_source(path))
        .filter_map(|path| {
            [
                normalize_path(path),
                format!("{}.keep", normalize_path(path)),
            ]
            .into_iter()
            .find_map(|candidate| fs::read_to_string(fixture_root.join(candidate)).ok())
        })
        .any(|contents| {
            Regex::new(r"\b(?:random|srandom)\s*\(")
                .expect("valid POSIX random function regex")
                .is_match(&contents)
        })
}

fn is_native_link_source(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "c" | "cc" | "cpp" | "cxx" | "o" | "obj" | "a" | "lib"
            )
        })
}

fn link_object_path(backend: PlanSimulationBackend, path: &str) -> String {
    let path = normalize_path(path);
    if Path::new(&path).extension().is_some() {
        path
    } else {
        let extension = match backend {
            PlanSimulationBackend::Bluesim => "ba",
            PlanSimulationBackend::Icarus => "v",
        };
        format!("{path}.{extension}")
    }
}

fn scenario_declared_fixture_inputs(scenario: &Scenario) -> impl Iterator<Item = String> + '_ {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .flat_map(|operation| operation.artifacts.inputs.iter().cloned())
        .filter(|path| is_safe_relative(path))
}

fn local_operation_data_paths(fixture_root: &Path, scenario: &Scenario) -> BTreeSet<String> {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .filter(|operation| {
            matches!(
                operation.action,
                Action::SimulationRun { .. }
                    | Action::SystemcCxxLink { .. }
                    | Action::VcdCheck { .. }
            )
        })
        .flat_map(|operation| operation.artifacts.inputs.iter().cloned())
        .filter(|path| fixture_root.join(path).is_file())
        .collect()
}

fn local_transfer_fixture_paths(fixture_root: &Path, scenario: &Scenario) -> BTreeSet<String> {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .filter_map(|operation| match &operation.action {
            Action::FsCopy { source, .. } | Action::FsCopyReplace { source, .. } => {
                Some(source.clone())
            }
            _ => None,
        })
        .filter(|path| fixture_root.join(path).is_file())
        .collect()
}

fn local_link_fixture_paths(fixture_root: &Path, scenario: &Scenario) -> BTreeSet<String> {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .filter(|operation| {
            matches!(
                operation.action,
                Action::BscLink { .. } | Action::BscSystemcLink { .. }
            )
        })
        .flat_map(|operation| operation.artifacts.inputs.iter().cloned())
        .filter(|path| fixture_root.join(path).is_file())
        .collect()
}

fn resolve_extensionless_contract_sources(script: &mut ScriptManifest, fixture_root: &Path) {
    for contract in &mut script.contracts {
        let (source, arguments) = match contract {
            Contract::Compile(contract) => (&mut contract.source, &mut contract.arguments),
            Contract::Simulation(contract) => (&mut contract.source, &mut contract.arguments),
            Contract::BasicOptions(_)
            | Contract::NoSourceCompile(_)
            | Contract::Ovl(_)
            | Contract::RenderGolden(_)
            | Contract::ExternalSet(_) => continue,
        };
        let Some(resolved) = resolve_extensionless_source(source, fixture_root) else {
            continue;
        };
        *source = resolved.clone();
        if let Some(argument) = arguments.first_mut() {
            *argument = resolved;
        }
    }
}

fn resolve_extensionless_source(source: &str, fixture_root: &Path) -> Option<String> {
    let source = normalize_path(source);
    if !is_safe_relative(&source)
        || Path::new(&source).extension().is_some()
        || fixture_root.join(&source).is_file()
    {
        return None;
    }
    let candidates = ["bsv", "bs"]
        .into_iter()
        .map(|extension| format!("{source}.{extension}"))
        .filter(|candidate| fixture_root.join(candidate).is_file())
        .collect::<Vec<_>>();
    (candidates.len() == 1).then(|| candidates.into_iter().next().unwrap())
}

fn workflow_generated_destinations(script: &ScriptManifest) -> BTreeSet<String> {
    let mut directories = BTreeSet::new();
    let mut generated = BTreeSet::new();
    let mut actions = script.workflow_actions.iter().collect::<Vec<_>>();
    actions
        .sort_by_key(|action| execution_order_key(action_span(action), action_expansion(action)));
    for action in actions {
        match action {
            WorkflowAction::CreateDirectory(directory) => {
                directories.insert(
                    normalize_path(&directory.path)
                        .trim_end_matches('/')
                        .to_owned(),
                );
            }
            WorkflowAction::TransferArtifact(transfer) => {
                let destination = normalize_path(&transfer.destination);
                let directory = destination.trim_end_matches('/');
                if directories.contains(directory) {
                    if let Some(name) = Path::new(&transfer.source)
                        .file_name()
                        .and_then(|name| name.to_str())
                    {
                        generated.insert(format!("{directory}/{name}"));
                    }
                } else {
                    generated.insert(destination);
                }
            }
            WorkflowAction::TouchCreateArtifact(touch) => {
                generated.insert(normalize_path(&touch.path));
            }
            WorkflowAction::RenderM4Curdir(render) => {
                generated.insert(normalize_path(&render.output));
            }
            WorkflowAction::RenderGolden(render) => {
                generated.insert(normalize_path(&render.output));
            }
            WorkflowAction::TextNormalize(normalize) => {
                generated.insert(normalize_path(&normalize.destination));
            }
            _ => {}
        }
    }
    generated
}

fn static_fixture_sources(script: &ScriptManifest, fixture_root: &Path) -> BTreeSet<String> {
    let generated = workflow_generated_destinations(script);
    collect_source_paths(script)
        .into_iter()
        .chain(
            script
                .workflow_actions
                .iter()
                .filter_map(|action| match action {
                    WorkflowAction::TransferArtifact(transfer) => {
                        Some(normalize_path(&transfer.source))
                    }
                    _ => None,
                }),
        )
        .filter(|path| {
            is_safe_relative(path)
                && !generated.contains(path)
                && fs::symlink_metadata(fixture_root.join(path))
                    .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        })
        .collect()
}

fn collect_source_paths(script: &ScriptManifest) -> BTreeSet<String> {
    let mut sources = BTreeSet::new();
    for contract in &script.contracts {
        match contract {
            Contract::BasicOptions(_)
            | Contract::NoSourceCompile(_)
            | Contract::Ovl(_)
            | Contract::RenderGolden(_) => {}

            Contract::Compile(contract) => {
                sources.insert(compile_contract_path(contract, &contract.source));
            }
            Contract::Simulation(contract) => {
                sources.insert(normalize_path(&contract.source));
            }
            Contract::ExternalSet(contract) => {
                if contract.external_kind == ExternalContractKind::SchedulerSat {
                    sources.extend(contract.cases.iter().map(|case| format!("{case}.bsv")));
                }
            }
        }
    }
    for sequence in &script.bluesim_sequences {
        for action in sequence
            .contracts
            .iter()
            .flat_map(|contract| contract.actions())
        {
            if let WorkflowAction::CompileObject(action) = action {
                sources.insert(normalize_path(&action.source));
            }
        }
    }
    for workflow in &script.bluesim_workflows {
        sources.extend(
            workflow
                .generations
                .iter()
                .map(|generation| normalize_path(&generation.source)),
        );
    }
    for action in &script.workflow_actions {
        match action {
            WorkflowAction::CompileObject(action) => {
                sources.insert(normalize_path(&action.source));
            }
            WorkflowAction::Bsc2Bsv(action) => {
                sources.insert(normalize_path(&action.source));
            }
            WorkflowAction::BscParsePretty(action) => {
                sources.insert(normalize_path(&action.source));
            }
            WorkflowAction::RenderM4Curdir(action) => {
                sources.insert(normalize_path(&action.template));
            }
            WorkflowAction::RenderGolden(action) => {
                sources.insert(normalize_path(&action.template));
            }

            WorkflowAction::BluetclRun(action) => {
                sources.extend(action.artifact_inputs.iter().filter_map(|path| {
                    let path = normalize_path(path);
                    Path::new(&path)
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| matches!(extension, "bs" | "bsv"))
                        .then_some(path)
                }));
            }
            _ => {}
        }
    }
    sources
}

fn collect_fixtures(
    project_root: &Path,
    fixture_dir: &str,
    source_paths: BTreeSet<String>,
    golden_paths: BTreeSet<String>,
    data_paths: BTreeSet<String>,
    build_input_paths: BTreeSet<String>,
    script_paths: BTreeSet<String>,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Vec<Fixture> {
    let mut paths = source_paths
        .into_iter()
        .map(|path| (path, FixtureRole::Source))
        .collect::<BTreeMap<_, _>>();
    for path in golden_paths {
        paths.entry(path).or_insert(FixtureRole::Golden);
    }
    for path in data_paths {
        paths.entry(path).or_insert(FixtureRole::Data);
    }
    for path in build_input_paths {
        paths.insert(path, FixtureRole::BuildInput);
    }
    for path in script_paths {
        paths.insert(path, FixtureRole::Script);
    }

    let project_root = match fs::canonicalize(project_root) {
        Ok(path) => path,
        Err(error) => {
            diagnostics.push(global_error(
                "fixture.root",
                format!("could not canonicalize project root: {error}"),
            ));
            return Vec::new();
        }
    };
    let fixture_root = project_root.join(fixture_dir);
    let fixture_root = match fs::symlink_metadata(&fixture_root).and_then(|metadata| {
        if metadata.file_type().is_symlink() {
            Err(std::io::Error::other(
                "fixture directory is a symbolic link",
            ))
        } else {
            fs::canonicalize(&fixture_root)
        }
    }) {
        Ok(path) if path.starts_with(&project_root) => path,
        Ok(path) => {
            diagnostics.push(global_error(
                "fixture.unsafe_root",
                format!(
                    "fixture directory escapes the project root: {}",
                    path.display()
                ),
            ));
            return Vec::new();
        }
        Err(error) => {
            diagnostics.push(global_error(
                "fixture.root",
                format!("could not validate fixture directory: {error}"),
            ));
            return Vec::new();
        }
    };

    let mut fixtures = Vec::new();
    for (path, role) in paths {
        if !is_safe_relative(&path) {
            diagnostics.push(global_error(
                "fixture.unsafe_path",
                format!("fixture path {path:?} is not a safe relative path"),
            ));
            continue;
        }
        let (absolute, source) = match validate_fixture_input(&fixture_root, &path) {
            Ok(validated) => validated,
            Err(error) => {
                diagnostics.push(global_error(
                    "fixture.missing",
                    format!(
                        "could not validate fixture {}: {error}",
                        fixture_root.join(&path).display()
                    ),
                ));
                continue;
            }
        };
        if source.is_some() && role != FixtureRole::Source {
            diagnostics.push(global_error(
                "fixture.unsafe_alias",
                format!("fixture alias {path:?} must have the source role"),
            ));
            continue;
        }
        match fs::read(&absolute) {
            Ok(contents) => fixtures.push(Fixture {
                path,
                source,
                sha256: sha256(&contents),
                role,
            }),
            Err(error) => diagnostics.push(global_error(
                "fixture.missing",
                format!("could not read fixture {}: {error}", absolute.display()),
            )),
        }
    }
    fixtures
}

fn validate_fixture_input(
    fixture_root: &Path,
    logical: &str,
) -> Result<(PathBuf, Option<String>), String> {
    let logical_path = Path::new(logical);
    let mut parent = fixture_root.to_owned();
    if let Some(relative_parent) = logical_path.parent() {
        for component in relative_parent.components() {
            let Component::Normal(component) = component else {
                return Err("fixture parent is not a normalized relative path".to_owned());
            };
            parent.push(component);
            let metadata = fs::symlink_metadata(&parent)
                .map_err(|error| format!("inspect fixture parent {}: {error}", parent.display()))?;
            if metadata.file_type().is_symlink()
                || metadata_is_reparse_point(&metadata)
                || !metadata.is_dir()
            {
                return Err(format!(
                    "fixture parent must be a regular non-link directory: {}",
                    parent.display()
                ));
            }
        }
    }

    let absolute = fixture_root.join(logical_path);
    let metadata = fs::symlink_metadata(&absolute)
        .map_err(|error| format!("inspect fixture {}: {error}", absolute.display()))?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(&absolute)
            .map_err(|error| format!("read fixture symlink {}: {error}", absolute.display()))?;
        let source = resolve_fixture_alias_target(logical_path, &target)?;
        let target = fixture_root.join(&source);
        let target_metadata = fs::symlink_metadata(&target).map_err(|error| {
            format!("inspect fixture alias target {}: {error}", target.display())
        })?;
        if target_metadata.file_type().is_symlink()
            || metadata_is_reparse_point(&target_metadata)
            || !target_metadata.is_file()
        {
            return Err(format!(
                "fixture alias target must be a regular non-link file: {}",
                target.display()
            ));
        }
        let canonical = fs::canonicalize(&target).map_err(|error| {
            format!(
                "canonicalize fixture alias target {}: {error}",
                target.display()
            )
        })?;
        if !canonical.starts_with(fixture_root) {
            return Err(format!(
                "fixture alias target escapes its fixture root: {}",
                target.display()
            ));
        }
        return Ok((canonical, Some(source)));
    }
    if metadata_is_reparse_point(&metadata) || !metadata.is_file() {
        return Err("fixture must be a regular non-symbolic-link, non-reparse file".to_owned());
    }
    let canonical = fs::canonicalize(&absolute)
        .map_err(|error| format!("canonicalize fixture {}: {error}", absolute.display()))?;
    if !canonical.starts_with(fixture_root) {
        return Err(format!(
            "fixture path escapes its fixture root: {}",
            absolute.display()
        ));
    }
    Ok((canonical, None))
}

fn resolve_fixture_alias_target(logical: &Path, target: &Path) -> Result<String, String> {
    if target.is_absolute() {
        return Err("fixture alias target must be relative".to_owned());
    }
    let mut resolved = logical
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_path_buf();
    for component in target.components() {
        match component {
            Component::Normal(component) => resolved.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err("fixture alias target escapes its fixture root".to_owned());
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("fixture alias target must be relative".to_owned())
            }
        }
    }
    let source = resolved.to_string_lossy().replace('\\', "/");
    if source.is_empty() || !is_safe_relative(&source) || resolved == logical {
        return Err("fixture alias target is not a distinct safe fixture path".to_owned());
    }
    Ok(source)
}

fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

fn unconverted_contract(contract: &Contract) -> ImportDiagnostic {
    match contract {
        Contract::NoSourceCompile(contract) => error_diagnostic(
            "import.no_source_compile_contract",
            format!(
                "no-source compile options {} are typed but not yet executable from Test Plan",
                contract.name
            ),
            contract.span,
            &contract.expansion,
        ),
        Contract::BasicOptions(contract) => error_diagnostic(
            "import.basic_options_contract",
            format!(
                "test_basic_options output {} is typed but not yet executable from Test Plan",
                contract.output
            ),
            contract.span,
            &contract.expansion,
        ),
        Contract::Ovl(contract) => error_diagnostic(
            "import.ovl_contract",
            format!(
                "test_ovl {} with {} is typed but not executable from Test Plan",
                contract.top, contract.library
            ),
            contract.span,
            &contract.expansion,
        ),
        Contract::RenderGolden(contract) => error_diagnostic(
            "import.render_golden_contract",
            format!(
                "m4_process rendering {} to {} is typed but not paired with test_basic_options",
                contract.template, contract.output
            ),
            contract.span,
            &contract.expansion,
        ),
        Contract::Compile(contract) => error_diagnostic(
            "import.compile_contract",
            format!(
                "compile helper {} for {} is typed but not yet executable from Test Plan",
                contract.helper, contract.source
            ),
            contract.span,
            &contract.expansion,
        ),
        Contract::Simulation(contract) => error_diagnostic(
            "import.simulation_contract",
            format!(
                "simulation helper {} for {} is typed but not yet executable from Test Plan",
                contract.helper, contract.source
            ),
            contract.span,
            &contract.expansion,
        ),
        Contract::ExternalSet(contract) => error_diagnostic(
            "import.external_contract",
            format!(
                "external contract set {:?} with {} cases requires a dedicated plan operation",
                contract.external_kind,
                contract.cases.len()
            ),
            contract.span,
            &contract.expansion,
        ),
    }
}

fn unsupported_diagnostic(unsupported: &UnsupportedConstruct) -> ImportDiagnostic {
    error_diagnostic(
        "import.unsupported_tcl",
        format!(
            "{}: {}",
            unsupported
                .command
                .as_deref()
                .unwrap_or("unrecognized Tcl construct"),
            unsupported_reason_label(unsupported.reason)
        ),
        unsupported.span,
        &unsupported.expansion,
    )
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

fn contract_source_span(contract: &Contract) -> ManifestSourceSpan {
    match contract {
        Contract::BasicOptions(contract) => contract.span,
        Contract::NoSourceCompile(contract) => contract.span,
        Contract::Ovl(contract) => contract.span,
        Contract::RenderGolden(contract) => contract.span,
        Contract::Compile(contract) => contract.span,
        Contract::Simulation(contract) => contract.span,
        Contract::ExternalSet(contract) => contract.span,
    }
}

fn contract_order_key(contract: &Contract) -> ExecutionOrderKey {
    match contract {
        Contract::BasicOptions(contract) => execution_order_key(contract.span, &contract.expansion),
        Contract::NoSourceCompile(contract) => {
            execution_order_key(contract.span, &contract.expansion)
        }
        Contract::Ovl(contract) => execution_order_key(contract.span, &contract.expansion),
        Contract::RenderGolden(contract) => execution_order_key(contract.span, &contract.expansion),
        Contract::Compile(contract) => execution_order_key(contract.span, &contract.expansion),
        Contract::Simulation(contract) => execution_order_key(contract.span, &contract.expansion),
        Contract::ExternalSet(contract) => execution_order_key(contract.span, &contract.expansion),
    }
}

fn workflow_actions_in_window<'a>(
    actions: &'a [WorkflowAction],
    consumed: &BTreeSet<usize>,
    window: ProvenanceWindow<'_>,
) -> Vec<(usize, &'a WorkflowAction)> {
    actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            let order = execution_order_key(action_span(action), action_expansion(action));
            (!consumed.contains(&index) && window.contains(&order)).then_some((index, action))
        })
        .collect()
}

fn execution_order_key(
    span: ManifestSourceSpan,
    expansion: &[ManifestSourceSpan],
) -> ExecutionOrderKey {
    ExecutionOrderKey(
        expansion
            .iter()
            .map(|span| span.start_byte)
            .chain(std::iter::once(span.start_byte))
            .collect(),
    )
}

fn action_span(action: &WorkflowAction) -> ManifestSourceSpan {
    match action {
        WorkflowAction::CompileObject(action) => action.span,
        WorkflowAction::BuildCObject(action) => action.span,
        WorkflowAction::LinkObjects(action) => action.span,
        WorkflowAction::LinkVerilog(action) => action.span,
        WorkflowAction::RunBluesim(action) => action.span,
        WorkflowAction::RunVerilog(action) => action.span,
        WorkflowAction::ShowRules(action) => action.span,
        WorkflowAction::LinkSystemc(action) => action.span,
        WorkflowAction::BuildSystemc(action) => action.span,
        WorkflowAction::RunSystemc(action) => action.span,
        WorkflowAction::BluetclRun(action) => action.span,
        WorkflowAction::Bsc2Bsv(action) => action.span,
        WorkflowAction::BscParsePretty(action) => action.span,
        WorkflowAction::TransferArtifact(action) => action.span,
        WorkflowAction::EraseArtifact(action) => action.span,
        WorkflowAction::EnsureDirectoryAbsent(action) => action.span,
        WorkflowAction::CreateDirectory(action) => action.span,
        WorkflowAction::TouchArtifact(action) => action.span,
        WorkflowAction::TouchCreateArtifact(action) => action.span,
        WorkflowAction::RemoveUserRead(action) => action.span,
        WorkflowAction::RewriteDarwinCppIncludePath(action) => action.span,
        WorkflowAction::RenderGolden(action) => action.span,
        WorkflowAction::RenderM4Curdir(action) => action.span,
        WorkflowAction::TextNormalize(action) => action.span,
        WorkflowAction::VerilogFilter(action) => action.span,
        WorkflowAction::Delay(action) => action.span,
        WorkflowAction::DumpIntermediate(action) => action.span,
    }
}

fn action_expansion(action: &WorkflowAction) -> &[ManifestSourceSpan] {
    match action {
        WorkflowAction::CompileObject(action) => &action.expansion,
        WorkflowAction::BuildCObject(action) => &action.expansion,
        WorkflowAction::LinkObjects(action) => &action.expansion,
        WorkflowAction::LinkVerilog(action) => &action.expansion,
        WorkflowAction::RunBluesim(action) => &action.expansion,
        WorkflowAction::RunVerilog(action) => &action.expansion,
        WorkflowAction::ShowRules(action) => &action.expansion,
        WorkflowAction::LinkSystemc(action) => &action.expansion,
        WorkflowAction::BuildSystemc(action) => &action.expansion,
        WorkflowAction::RunSystemc(action) => &action.expansion,
        WorkflowAction::BluetclRun(action) => &action.expansion,
        WorkflowAction::Bsc2Bsv(action) => &action.expansion,
        WorkflowAction::BscParsePretty(action) => &action.expansion,
        WorkflowAction::TransferArtifact(action) => &action.expansion,
        WorkflowAction::EraseArtifact(action) => &action.expansion,
        WorkflowAction::EnsureDirectoryAbsent(action) => &action.expansion,
        WorkflowAction::CreateDirectory(action) => &action.expansion,
        WorkflowAction::TouchArtifact(action) => &action.expansion,
        WorkflowAction::TouchCreateArtifact(action) => &action.expansion,
        WorkflowAction::RemoveUserRead(action) => &action.expansion,
        WorkflowAction::RewriteDarwinCppIncludePath(action) => &action.expansion,
        WorkflowAction::RenderGolden(action) => &action.expansion,
        WorkflowAction::RenderM4Curdir(action) => &action.expansion,
        WorkflowAction::TextNormalize(action) => &action.expansion,
        WorkflowAction::VerilogFilter(action) => &action.expansion,
        WorkflowAction::Delay(action) => &action.expansion,
        WorkflowAction::DumpIntermediate(action) => &action.expansion,
    }
}

fn operation_span(operation: &WorkflowOperation) -> ManifestSourceSpan {
    match operation {
        WorkflowOperation::Action(action) => action_span(action),
        WorkflowOperation::Assertion(assertion) => assertion.span,
    }
}

fn operation_expansion(operation: &WorkflowOperation) -> &[ManifestSourceSpan] {
    match operation {
        WorkflowOperation::Action(action) => action_expansion(action),
        WorkflowOperation::Assertion(assertion) => &assertion.expansion,
    }
}

fn error_diagnostic(
    code: &str,
    message: String,
    span: ManifestSourceSpan,
    expansion: &[ManifestSourceSpan],
) -> ImportDiagnostic {
    ImportDiagnostic {
        severity: DiagnosticSeverity::Error,
        code: code.to_owned(),
        message,
        provenance: provenance(span, expansion),
    }
}

fn global_error(code: &str, message: String) -> ImportDiagnostic {
    global_diagnostic(DiagnosticSeverity::Error, code, message)
}

fn global_warning(code: &str, message: String) -> ImportDiagnostic {
    global_diagnostic(DiagnosticSeverity::Warning, code, message)
}

fn global_diagnostic(
    severity: DiagnosticSeverity,
    code: &str,
    message: String,
) -> ImportDiagnostic {
    ImportDiagnostic {
        severity,
        code: code.to_owned(),
        message,
        provenance: provenance(
            ManifestSourceSpan {
                start_byte: 0,
                end_byte: 0,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            &[],
        ),
    }
}

fn provenance(span: ManifestSourceSpan, expansion: &[ManifestSourceSpan]) -> Provenance {
    Provenance {
        span: source_span(span),
        expansion: expansion.iter().copied().map(source_span).collect(),
    }
}

fn source_span(span: ManifestSourceSpan) -> SourceSpan {
    SourceSpan {
        start_byte: span.start_byte,
        end_byte: span.end_byte,
        start_line: span.start_line,
        start_column: span.start_column,
        end_line: span.end_line,
        end_column: span.end_column,
    }
}

fn parse_arguments(source: &str, label: &str) -> Result<Vec<String>, String> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    parse_static_tcl_list(source).map_err(|error| format!("could not parse {label}: {error}"))
}

/// Translate the narrowly allowed upstream SystemC C++ macro form into one
/// direct-process argv entry.  Tcl's backslash-escaped quotes existed solely
/// for its shell command string and never cross the typed boundary.
fn parse_systemc_defines(source: &str) -> Result<Vec<String>, String> {
    let defines = parse_arguments(source, "SystemC C++ options")?;
    if defines.iter().any(|define| !define.starts_with("-D")) {
        return Err("build_systemc_executable only supports static -D definitions".to_owned());
    }
    Ok(defines
        .into_iter()
        .map(|define| define.replace(r#"\""#, "\""))
        .collect())
}

fn canonical_tcl_regex(pattern: &str) -> String {
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
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
        )
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn is_numeric_bug_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok()
}

fn compile_contract_path(contract: &CompileContract, path: &str) -> String {
    let path = normalize_path(path);
    contract
        .working_directory
        .as_ref()
        .map_or(path.clone(), |directory| {
            format!("{}/{}", normalize_path(directory), path)
        })
}

fn is_safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn sha256(contents: &[u8]) -> String {
    format!("{:x}", Sha256::digest(contents))
}

fn unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsc_test_plan::{BluetclInvocation, BluetclMakedependCommand};

    fn project_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|candidate| {
                candidate.join("Cargo.toml").is_file() && candidate.join("testsuite").is_dir()
            })
            .expect("workspace root containing testsuite")
    }

    #[test]
    fn imports_showrules_as_twelve_closed_source_ordered_transform_episodes() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let script = manifest
            .scripts
            .iter()
            .find(|script| script.origin == "testsuite/bsc.showrules/showrules.exp")
            .expect("showrules script exists");
        let plan = plan_from_script(root, script).plan;
        assert_eq!(plan.status, PlanStatus::Complete);
        assert!(plan.diagnostics.is_empty());
        assert_eq!(plan.scenarios.len(), 12);

        let mut showrules_count = 0;
        let mut gcd_hierarchies = Vec::new();
        for scenario in &plan.scenarios {
            assert!(scenario.requires.contains(&Requirement::ShowRules));
            let operations = scenario
                .stages
                .iter()
                .flat_map(|stage| &stage.operations)
                .collect::<Vec<_>>();
            let showrules_index = operations
                .iter()
                .position(|operation| matches!(operation.action, Action::ShowRules { .. }))
                .expect("scenario has showrules transform");
            showrules_count += 1;
            assert!(showrules_index > 0);
            assert!(matches!(
                operations[showrules_index + 1].action,
                Action::VcdCheck { .. }
            ));
            match &operations[showrules_index].action {
                Action::ShowRules {
                    top,
                    output,
                    design_inputs,
                    ..
                } => {
                    assert!(!top.contains("MCD"));
                    assert!(matches!(
                        &operations[showrules_index + 1].action,
                        Action::VcdCheck { path, .. } if path == output
                    ));
                    if top == "mkTbGCD" {
                        gcd_hierarchies.push(design_inputs.clone());
                    }
                }
                _ => unreachable!(),
            }
            if scenario.requires.contains(&Requirement::Verilog) {
                assert!(matches!(
                    operations[showrules_index - 1].action,
                    Action::FsMove { .. }
                ));
            } else {
                assert!(matches!(
                    operations[showrules_index - 1].action,
                    Action::SimulationRun {
                        backend: PlanSimulationBackend::Bluesim,
                        ..
                    }
                ));
            }
        }
        assert_eq!(showrules_count, 12);
        assert_eq!(gcd_hierarchies.len(), 2);
        assert!(gcd_hierarchies
            .iter()
            .all(|inputs| { inputs == &["mkGCD.ba".to_owned(), "mkTbGCD.ba".to_owned()] }));
    }

    #[cfg(unix)]
    fn create_fixture_symlink(target: &Path, link: &Path, _directory: bool) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_fixture_symlink(target: &Path, link: &Path, directory: bool) -> std::io::Result<()> {
        if directory {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            std::os::windows::fs::symlink_file(target, link)
        }
    }

    #[test]
    fn imports_requested_workspace_episodes_with_typed_producer_order() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let plan = |origin: &str| {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .unwrap_or_else(|| panic!("missing fixture script {origin}"));
            plan_from_script(root, script).plan
        };

        let depend = plan("testsuite/bsc.driver/depend/depend.exp");
        assert_eq!(
            depend.status,
            PlanStatus::Complete,
            "{:#?}",
            depend.diagnostics
        );
        assert!(depend.diagnostics.is_empty());
        assert!(!depend.fixtures.iter().any(|fixture| {
            matches!(
                fixture.path.as_str(),
                "BdirVsSame_BdirNew.bsv" | "BdirVsSame_BdirOld.bsv"
            )
        }));
        let depend_operations = depend
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let create_bdir = depend_operations
            .iter()
            .position(|operation| matches!(&operation.action, Action::FsCreateDirAll { path } if path == "bdir"))
            .expect("typed bdir creation");
        let first_touch = depend_operations
            .iter()
            .position(|operation| matches!(&operation.action, Action::FsTouchCreate { path, delay_milliseconds: 1000 } if path == "BdirVsSame_BdirNew.bsv"))
            .expect("typed touch-create");
        let bdir_compile = depend_operations
            .iter()
            .position(|operation| matches!(&operation.action, Action::BscCompile { args, .. } if args.windows(2).any(|pair| pair == ["-bdir", "bdir"])))
            .expect("compile using proven bdir");
        assert!(create_bdir < first_touch && first_touch < bdir_compile);

        let imports = plan("testsuite/bsc.driver/imports/imports.exp");
        assert_eq!(
            imports.status,
            PlanStatus::Complete,
            "{:#?}",
            imports.diagnostics
        );
        assert!(!imports
            .fixtures
            .iter()
            .any(|fixture| fixture.path == "CircTop.bsv"));
        assert!(imports
            .scenarios
            .iter()
            .all(|scenario| { !scenario.requires.contains(&Requirement::PosixUnreadability) }));
        let import_operations = imports
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        assert!(import_operations.iter().any(|operation| matches!(
            &operation.action,
            Action::FsCopy { source, destination }
                if source == "DupPkg.bsv" && destination == "libdir1/DupPkg.bsv"
        )));
        assert!(import_operations
            .iter()
            .filter(|operation| match &operation.action {
                Action::FsRemoveUserRead { .. } => true,
                Action::BscCompile { source, .. } => source == "UnreadableTop.bsv",
                Action::AssertGolden { actual, .. } => {
                    actual == "UnreadableTop.bsv.bsc-out"
                }
                _ => false,
            })
            .all(|operation| operation
                .requires
                .contains(&Requirement::PosixUnreadability)));

        let include = plan("testsuite/bsc.preprocessor/include/include.exp");
        assert_eq!(
            include.status,
            PlanStatus::Complete,
            "{:#?}",
            include.diagnostics
        );
        let fixture_paths = include
            .fixtures
            .iter()
            .map(|fixture| fixture.path.as_str())
            .collect::<BTreeSet<_>>();
        assert!(fixture_paths.contains("IncludeAbsolute.bsv.pre-m4"));
        assert!(fixture_paths.contains("IncludeAbsolute.bsv.bsc-vcomp-out.expected.pre-m4"));
        assert!(!fixture_paths.contains("IncludeAbsolute.bsv"));
        assert!(!fixture_paths.contains("IncludeAbsolute.bsv.bsc-vcomp-out.expected"));
        let operations = include
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let source_render = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::M4CurdirRender { output, .. } if output == "IncludeAbsolute.bsv"
                )
            })
            .expect("source renderer");
        let compile = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::BscCompile { source, .. } if source == "IncludeAbsolute.bsv"
                )
            })
            .expect("rendered-source compile");
        let golden_render = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::M4CurdirRender { output, .. }
                        if output == "IncludeAbsolute.bsv.bsc-vcomp-out.expected"
                )
            })
            .expect("golden renderer");
        let compare = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::AssertGolden { actual, expected }
                        if actual == "IncludeAbsolute.bsv.bsc-vcomp-out"
                            && expected == "IncludeAbsolute.bsv.bsc-vcomp-out.expected"
                )
            })
            .expect("rendered golden comparison");
        assert!(source_render < compile && compile < golden_render && golden_render < compare);
    }

    #[test]
    fn completes_foreign_plans_with_typed_alias_build_and_dump_composition() {
        let root = project_root();
        let manifest = build_manifest(root).unwrap();
        let plan = |origin: &str| {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .unwrap_or_else(|| panic!("missing fixture script {origin}"));
            plan_from_script(root, script).plan
        };

        let foreign = plan("testsuite/bsc.codegen/foreign/foreign.exp");
        assert_eq!(
            foreign.status,
            PlanStatus::Complete,
            "foreign diagnostics: {:#?}",
            foreign.diagnostics
        );
        assert!(foreign.diagnostics.is_empty());
        for fixture in ["convert.c.keep", "convert.mk"] {
            assert!(
                foreign
                    .fixtures
                    .iter()
                    .any(|candidate| candidate.path == fixture),
                "missing foreign fixture {fixture}"
            );
        }
        let convert_scenarios = foreign
            .scenarios
            .iter()
            .filter(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::CObjectBuild { output, .. } if output == "convert.o"
                        )
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(convert_scenarios.len(), 2);
        for scenario in convert_scenarios {
            assert!(matches!(
                scenario.stages[0].operations.as_slice(),
                [
                    OperationRecord {
                        action: Action::FsCopy { source, destination },
                        ..
                    },
                    OperationRecord {
                        action: Action::CObjectBuild {
                            source: build_source,
                            makefile,
                            output,
                        },
                        ..
                    },
                    ..
                ] if source == "convert.c.keep"
                    && destination == "convert.c"
                    && build_source == "convert.c"
                    && makefile == "convert.mk"
                    && output == "convert.o"
            ));
        }
        let capital = foreign
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "simulation-mkBDPI_CapitalLinkName")
            .expect("capital-link-name scenario");
        let capital_operations = &capital.stages[0].operations;
        let generations = capital_operations
            .iter()
            .filter_map(|operation| match &operation.action {
                Action::BscGenerate { source, .. } => Some((source, operation)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(generations.len(), 2);
        assert_eq!(generations[0].0, "BDPIBit8.bsv");
        assert!(generations[0]
            .1
            .artifacts
            .outputs
            .contains(&"my_C_or.ba".to_owned()));
        assert_eq!(generations[1].0, "BDPI_CapitalLinkName.bsv");
        assert!(!generations[1]
            .1
            .artifacts
            .outputs
            .contains(&"my_C_or.ba".to_owned()));
        for operation in capital_operations {
            match &operation.action {
                Action::BscLink {
                    backend: PlanSimulationBackend::Icarus,
                    ..
                }
                | Action::SimulationRun {
                    backend: PlanSimulationBackend::Icarus,
                    ..
                } => assert!(operation.requires.contains(&Requirement::NonWindows)),
                Action::BscLink {
                    backend: PlanSimulationBackend::Bluesim,
                    ..
                }
                | Action::SimulationRun {
                    backend: PlanSimulationBackend::Bluesim,
                    ..
                } => assert!(!operation.requires.contains(&Requirement::NonWindows)),
                _ => {}
            }
        }

        let battery = plan("testsuite/bsc.codegen/foreign/battery/battery.exp");
        assert_eq!(
            battery.status,
            PlanStatus::Complete,
            "battery diagnostics: {:#?}",
            battery.diagnostics
        );
        assert!(battery.diagnostics.is_empty());
        for fixture in [
            "common.h.keep",
            "common.c.keep",
            "values.c.keep",
            "actions.c.keep",
            "actionvalues.c.keep",
        ] {
            assert!(
                battery
                    .fixtures
                    .iter()
                    .any(|candidate| candidate.path == fixture),
                "missing battery fixture {fixture}"
            );
        }
        let aggressive = battery
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "simulation-mkTestAggressiveConditions")
            .expect("aggressive-conditions scenario");
        assert!(aggressive.requires.contains(&Requirement::Bluesim));
        assert!(aggressive.requires.contains(&Requirement::Verilog));
        let operations = &aggressive.stages[0].operations;
        assert!(matches!(
            operations.first().map(|operation| &operation.action),
            Some(Action::FsCopy { source, destination })
                if source == "common.h.keep" && destination == "common.h"
        ));
        let generation = operations
            .iter()
            .find(|operation| {
                matches!(
                    &operation.action,
                    Action::BscGenerate { module, .. }
                        if module.as_deref() == Some("mkTestAggressiveConditions")
                )
            })
            .expect("aggressive-conditions generation");
        assert!(generation
            .artifacts
            .outputs
            .contains(&"mkTestAggressiveConditions-ats.txt".to_owned()));
        assert!(operations.iter().any(|operation| {
            matches!(
                &operation.action,
                Action::BscLink {
                    backend: PlanSimulationBackend::Bluesim,
                    ..
                }
            ) && operation.requires.contains(&Requirement::NonWindows)
        }));
        let ats_assertion = operations
            .iter()
            .find(|operation| {
                matches!(
                    &operation.action,
                    Action::AssertRegex { path, .. }
                        if path == "mkTestAggressiveConditions-ats.txt"
                )
            })
            .expect("ATS assertion");
        assert!(!ats_assertion.requires.contains(&Requirement::NonWindows));
    }

    #[test]
    fn fixture_aliases_accept_only_one_hop_relative_regular_targets() {
        let cpp_root = fs::canonicalize(project_root().join("testsuite/bsc.driver/cpp"))
            .expect("canonical cpp fixture root");
        let (physical, source) = validate_fixture_input(&cpp_root, "Cpreprocess1.bsv")
            .expect("checked-in cpp alias is valid");
        assert_eq!(source.as_deref(), Some("Cpreprocess.bsv"));
        assert_eq!(
            physical,
            fs::canonicalize(cpp_root.join("Cpreprocess.bsv")).unwrap()
        );

        assert!(resolve_fixture_alias_target(
            Path::new("Alias.bsv"),
            &project_root().join("outside.bsv")
        )
        .is_err());
        assert!(
            resolve_fixture_alias_target(Path::new("Alias.bsv"), Path::new("../outside.bsv"))
                .is_err()
        );

        let root = project_root().join(".pixi/tmp").join(format!(
            "manifest-fixture-alias-security-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("directory")).expect("create alias test root");
        fs::write(root.join("target.bsv"), "package Target; endpackage\n")
            .expect("write regular target");
        if create_fixture_symlink(Path::new("target.bsv"), &root.join("first.bsv"), false).is_ok() {
            create_fixture_symlink(Path::new("first.bsv"), &root.join("chain.bsv"), false)
                .expect("create symlink chain");
            assert!(validate_fixture_input(&root, "chain.bsv").is_err());

            create_fixture_symlink(Path::new("directory"), &root.join("directory.bsv"), true)
                .expect("create directory symlink");
            assert!(validate_fixture_input(&root, "directory.bsv").is_err());
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn maps_only_numeric_two_argument_compare_file_bug_as_an_implicit_golden() {
        let comparison = |arguments: &[&str]| ComparisonContract {
            helper: "compare_file_bug".to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        let mapped = |arguments: &[&str]| map_comparison(&comparison(arguments)).unwrap();

        let implicit = mapped(&["actual.out", "770"]);
        assert!(matches!(
            implicit.action,
            Action::AssertGolden { ref actual, ref expected }
                if actual == "actual.out" && expected == "actual.out.expected"
        ));
        assert_eq!(
            implicit.expectation,
            OperationExpectation::Xfail {
                reason: "upstream bug 770".to_owned()
            }
        );

        let explicit = mapped(&["actual.out", "expected.out"]);
        assert!(matches!(
            explicit.action,
            Action::AssertGolden { ref expected, .. } if expected == "expected.out"
        ));
        assert_eq!(explicit.expectation, OperationExpectation::Required);

        let explicit_bug = mapped(&["actual.out", "expected.out", "771"]);
        assert!(matches!(
            explicit_bug.action,
            Action::AssertGolden { ref expected, .. } if expected == "expected.out"
        ));
        assert!(matches!(
            explicit_bug.expectation,
            OperationExpectation::Xfail { ref reason } if reason == "upstream bug 771"
        ));

        let arbitrary = mapped(&["actual.out", "Rob_Brown"]);
        assert!(matches!(
            arbitrary.action,
            Action::AssertGolden { ref expected, .. } if expected == "Rob_Brown"
        ));
        assert_eq!(arbitrary.expectation, OperationExpectation::Required);
    }

    #[test]
    fn persistent_fixture_aliases_stop_on_mutation_and_resume_on_replacement() {
        fn span(start_byte: usize) -> ManifestSourceSpan {
            ManifestSourceSpan {
                start_byte,
                end_byte: start_byte + 1,
                start_line: start_byte + 1,
                start_column: 1,
                end_line: start_byte + 1,
                end_column: 2,
            }
        }

        fn copy(start_byte: usize) -> WorkflowAction {
            WorkflowAction::TransferArtifact(crate::model::ArtifactTransferAction {
                operation: ArtifactTransferOperation::Copy,
                source: "convert.c.keep".to_owned(),
                destination: "convert.c".to_owned(),
                guard: Guard::Always,
                span: span(start_byte),
                expansion: Vec::new(),
            })
        }

        fn touch(start_byte: usize) -> WorkflowAction {
            WorkflowAction::TouchArtifact(crate::model::TouchArtifactAction {
                path: "convert.c".to_owned(),
                guard: Guard::Always,
                span: span(start_byte),
                expansion: Vec::new(),
            })
        }

        fn build(start_byte: usize) -> WorkflowAction {
            WorkflowAction::BuildCObject(crate::model::CObjectBuildAction {
                source: "convert.c".to_owned(),
                makefile: "convert.mk".to_owned(),
                output: "convert.o".to_owned(),
                guard: Guard::Always,
                span: span(start_byte),
                expansion: Vec::new(),
            })
        }

        fn assembly() -> PlanAssembly {
            PlanAssembly {
                scenarios: vec![Scenario {
                    id: "alias-consumer".to_owned(),
                    resource: ResourceClass::Normal,
                    fixtures: Vec::new(),
                    requires: Vec::new(),
                    bsc_options_append: None,
                    timeouts: Timeouts::default(),
                    stages: vec![Stage {
                        id: "consume-alias".to_owned(),
                        operations: vec![map_action(&build(40)).unwrap()],
                    }],
                }],
                ..PlanAssembly::default()
            }
        }

        fn script(workflow_actions: Vec<WorkflowAction>) -> ScriptManifest {
            ScriptManifest {
                origin: "testsuite/bsc.codegen/foreign/foreign.exp".to_owned(),
                source_sha256: String::new(),
                contracts: Vec::new(),
                assertions: Vec::new(),
                comparisons: Vec::new(),
                bluesim_sequences: Vec::new(),
                bluesim_workflows: Vec::new(),
                systemc_workflows: Vec::new(),
                workflow_actions,
                make_test_data_actions: Vec::new(),
                bsc_options_overlays: Vec::new(),
                unsupported: Vec::new(),
            }
        }

        let fixture_root = project_root().join("testsuite/bsc.codegen/foreign");
        let stopped = script(vec![copy(10), touch(20)]);
        let mut stopped_assembly = assembly();
        compose_persistent_fixture_aliases(&stopped, &fixture_root, &mut stopped_assembly);
        assert!(matches!(
            stopped_assembly.scenarios[0].stages[0]
                .operations
                .as_slice(),
            [OperationRecord {
                action: Action::CObjectBuild { .. },
                ..
            }]
        ));
        assert!(stopped_assembly.consumed_actions.is_empty());

        let replaced = script(vec![copy(10), touch(20), copy(30)]);
        let mut replaced_assembly = assembly();
        compose_persistent_fixture_aliases(&replaced, &fixture_root, &mut replaced_assembly);
        assert!(matches!(
            replaced_assembly.scenarios[0].stages[0].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::FsCopy { source, destination },
                    ..
                },
                OperationRecord {
                    action: Action::CObjectBuild { .. },
                    ..
                }
            ] if source == "convert.c.keep" && destination == "convert.c"
        ));
        assert_eq!(replaced_assembly.consumed_actions, BTreeSet::from([2]));
    }

    #[test]
    fn imports_scheduler_sat_external_set_as_canonical_plan_scenarios() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = &generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.scheduler/sat/sat")
            .expect("scheduler SAT plan")
            .plan;
        assert_eq!(
            plan.status,
            PlanStatus::Complete,
            "scheduler SAT diagnostics: {:#?}",
            plan.diagnostics
        );
        assert!(plan.diagnostics.is_empty());
        assert_eq!(plan.scenarios.len(), 24);
        assert!(plan.scenarios.iter().all(|scenario| {
            scenario.resource == ResourceClass::Heavy
                && scenario.requires == [Requirement::Verilog]
                && matches!(
                    scenario.stages[0].operations.as_slice(),
                    [
                        OperationRecord {
                            action: Action::FsCopy { .. },
                            ..
                        },
                        OperationRecord {
                            action: Action::BscCompile {
                                mode: BscCompileMode::VerilogSchedule,
                                args,
                                ..
                            },
                            ..
                        },
                        OperationRecord {
                            action: Action::AssertGoldenNormalized { normalizations, .. },
                            ..
                        }
                    ] if args == &["-sat-z3"]
                        && normalizations == &[
                            GoldenNormalization::GeneratedIds,
                            GoldenNormalization::SatSolverNames,
                        ]
                )
        }));
    }

    #[test]
    fn imports_closed_bluetcl_targets_with_strict_fixture_and_producer_contracts() {
        let root = project_root();
        let manifest = build_manifest(root).unwrap();
        let plan = |origin: &str| {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .unwrap_or_else(|| panic!("missing fixture script {origin}"));
            plan_from_script(root, script).plan
        };

        let utils = plan("testsuite/bsc.bluetcl/packages/utils/utils.exp");
        assert_eq!(utils.status, PlanStatus::Complete);
        assert!(utils.diagnostics.is_empty());
        assert_eq!(utils.scenarios.len(), 2);
        assert!(utils.fixtures.iter().any(|fixture| {
            fixture.path == "utils_test.tcl" && fixture.role == FixtureRole::Script
        }));
        for scenario in &utils.scenarios {
            assert_eq!(scenario.requires, [Requirement::Bluetcl]);
            assert!(matches!(
                scenario.stages[0].operations.as_slice(),
                [
                    OperationRecord {
                        action: Action::BluetclRun {
                            artifact_inputs,
                            ..
                        },
                        ..
                    },
                    OperationRecord {
                        action: Action::AssertGoldenNormalized {
                            normalizations,
                            ..
                        },
                        ..
                    }
                ] if artifact_inputs.is_empty()
                    && normalizations == &[GoldenNormalization::BluetclOutput]
            ));
        }

        let methods = plan("testsuite/bsc.misc/method_conditions/method_conditions.exp");
        assert_eq!(methods.status, PlanStatus::Complete);
        assert!(methods.diagnostics.is_empty());
        assert!(methods.fixtures.iter().any(|fixture| {
            fixture.path == "dump_poss.tcl" && fixture.role == FixtureRole::Script
        }));
        let bluetcl_scenarios = methods
            .scenarios
            .iter()
            .filter(|scenario| scenario.requires.contains(&Requirement::Bluetcl))
            .collect::<Vec<_>>();
        assert_eq!(bluetcl_scenarios.len(), 17);
        for scenario in bluetcl_scenarios {
            let operations = &scenario.stages[0].operations;
            let run_index = operations
                .iter()
                .position(|operation| matches!(operation.action, Action::BluetclRun { .. }))
                .unwrap();
            let Action::BluetclRun {
                artifact_inputs,
                stdout,
                ..
            } = &operations[run_index].action
            else {
                unreachable!()
            };
            assert_eq!(artifact_inputs.len(), 1);
            assert!(artifact_inputs[0].ends_with(".ba"));
            assert!(matches!(
                operations.first().map(|operation| &operation.action),
                Some(Action::BscCompile { .. })
            ));
            assert!(operations[..run_index]
                .iter()
                .any(|operation| operation.artifacts.outputs.contains(&artifact_inputs[0])));
            assert!(matches!(
                operations.get(run_index + 1).map(|operation| &operation.action),
                Some(Action::AssertGoldenNormalized {
                    actual,
                    normalizations,
                    ..
                }) if actual == stdout
                    && normalizations == &[GoldenNormalization::BluetclOutput]
            ));
            assert!(!scenario.fixtures.contains(&artifact_inputs[0]));
            assert!(scenario.fixtures.contains(&"dump_poss.tcl".to_owned()));
        }
    }

    #[test]
    fn imports_binary_ghcrts_as_a_single_child_environment_overlay() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let script = manifest
            .scripts
            .iter()
            .find(|script| script.origin == "testsuite/bsc.binary/binary.exp")
            .expect("binary script exists");
        let plan = plan_from_script(root, script).plan;
        assert_eq!(plan.status, PlanStatus::Complete);
        assert!(plan.diagnostics.is_empty());
        assert!(matches!(
            plan.scenarios.as_slice(),
            [Scenario { stages, .. }]
                if matches!(
                    stages[0].operations.as_slice(),
                    [OperationRecord {
                        action: Action::BscCompile {
                            source,
                            environment: Some(BscCompileEnvironment::GhcrtsM1_2g),
                            ..
                        },
                        ..
                    }] if source == "ManyMeths.bsv"
                )
        ));
    }

    #[test]
    fn imports_ordered_bsc_compile_bluetcl_batches_without_open_shapes() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let plan = |origin: &str| {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .unwrap_or_else(|| panic!("missing fixture script {origin}"));
            plan_from_script(root, script).plan
        };

        for (origin, compile_count) in [
            ("testsuite/bsc.bluetcl/commands/commands.exp", 4),
            ("testsuite/bsc.bluetcl/hierarchy/hierarchy.exp", 2),
            (
                "testsuite/bsc.bluetcl/targeted/port_types/port_types.exp",
                4,
            ),
            ("testsuite/bsc.bluetcl/targeted/type/type.exp", 1),
        ] {
            let imported = plan(origin);
            assert_eq!(imported.status, PlanStatus::Complete, "{origin}");
            assert!(imported.diagnostics.is_empty(), "{origin}");
            assert_eq!(imported.scenarios.len(), 1, "{origin}");
            let scenario = &imported.scenarios[0];
            assert!(scenario.requires.contains(&Requirement::Bluetcl));
            assert!(scenario.requires.contains(&Requirement::Verilog));
            let operations = &scenario.stages[0].operations;
            assert_eq!(
                operations
                    .iter()
                    .filter(|operation| matches!(operation.action, Action::BscCompile { .. }))
                    .count(),
                compile_count,
                "{origin}"
            );
            assert!(operations.windows(2).all(|pair| {
                pair[0].provenance.span.start_byte <= pair[1].provenance.span.start_byte
            }));
            for (index, operation) in operations.iter().enumerate() {
                match &operation.action {
                    Action::BscCompile { expected_exit, .. } => {
                        assert_eq!(*expected_exit, ExpectedExit::Unchecked);
                    }
                    Action::BluetclRun {
                        artifact_inputs,
                        stdout,
                        ..
                    } => {
                        assert!(matches!(
                            operations.get(index + 1).map(|operation| &operation.action),
                            Some(Action::AssertGoldenNormalized { actual, .. }) if actual == stdout
                        ));
                        for input in artifact_inputs {
                            if input.ends_with(".ba") || input.ends_with(".bo") {
                                assert!(operations[..index].iter().any(|operation| {
                                    operation.artifacts.outputs.contains(input)
                                }));
                                assert!(!scenario.fixtures.contains(input));
                            } else {
                                assert!(scenario.fixtures.contains(input));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let commands = plan("testsuite/bsc.bluetcl/commands/commands.exp");
        let operations = &commands.scenarios[0].stages[0].operations;
        let mkdir = operations
            .iter()
            .enumerate()
            .filter_map(|(index, operation)| match &operation.action {
                Action::FsMkdir { path } if path == "BOUTDIR" => Some(index),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(mkdir.len(), 1);
        assert_eq!(operations[mkdir[0]].requires, [Requirement::NonWindows]);
        let depend = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::BluetclRun {
                        invocation: bsc_test_plan::BluetclInvocation::Script { script, .. },
                        ..
                    } if script == "depend.tcl"
                )
            })
            .expect("depend Bluetcl run exists");
        assert!(mkdir[0] < depend);
        assert_eq!(operations[depend].requires, [Requirement::NonWindows]);
        assert_eq!(operations[depend + 1].requires, [Requirement::NonWindows]);
        assert!(commands.fixtures.iter().any(|fixture| {
            fixture.path == "pprint.tcl" && fixture.role == FixtureRole::Script
        }));
        let profiles = operations
            .iter()
            .filter_map(|operation| match &operation.action {
                Action::AssertGoldenNormalized { normalizations, .. } => Some(normalizations),
                _ => None,
            })
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        assert!(profiles.contains(&GoldenNormalization::BluetclPositionDigits));
        assert!(profiles.contains(&GoldenNormalization::BluetclCregPositions));
        assert!(profiles.contains(&GoldenNormalization::BluetclPreludeLibrary));

        let hierarchy = plan("testsuite/bsc.bluetcl/hierarchy/hierarchy.exp");
        assert!(hierarchy.scenarios[0].stages[0]
            .operations
            .iter()
            .any(|operation| matches!(
                &operation.action,
                Action::AssertGoldenNormalized { normalizations, .. }
                    if normalizations.contains(&GoldenNormalization::BluetclLibraries)
            )));

        let hierarchy2 = plan("testsuite/bsc.bluetcl/hierarchy2/hierarchy2.exp");
        assert_eq!(hierarchy2.status, PlanStatus::Complete);
        assert!(hierarchy2.diagnostics.is_empty());
        let operations = &hierarchy2.scenarios[0].stages[0].operations;
        assert_eq!(
            operations
                .iter()
                .filter(|operation| matches!(operation.action, Action::BscCompile { .. }))
                .count(),
            19
        );
        assert_eq!(
            operations
                .iter()
                .filter(|operation| matches!(operation.action, Action::BluetclRun { .. }))
                .count(),
            38
        );
        for stem in ["Design", "TestV", "MultFor"] {
            assert!(
                hierarchy2
                    .fixtures
                    .iter()
                    .any(|fixture| fixture.path
                        == format!("ShowH.tcl_sys{stem}.bluetcl-out.expected"))
            );
        }
        // The stale sysMultiFor golden has no matching source and must not
        // be imported even though the file exists on disk.
        assert!(!hierarchy2
            .fixtures
            .iter()
            .any(|fixture| fixture.path == "ShowH.tcl_sysMultiFor.bluetcl-out.expected"));

        let expand_ports = plan("testsuite/bsc.bluetcl/packages/expandPorts/expandPorts.exp");
        assert_eq!(
            expand_ports.status,
            PlanStatus::Complete,
            "{:?}",
            expand_ports.diagnostics
        );
        assert!(expand_ports.diagnostics.is_empty());

        let instsynth = plan("testsuite/bsc.bluetcl/packages/InstSynth/InstSynth.exp");
        assert_eq!(instsynth.status, PlanStatus::Blocked);
        assert!(!instsynth.diagnostics.is_empty());
    }

    #[test]
    fn composes_only_closed_directory_prefixes_for_remaining_nukedir_origins() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let plan = |origin: &str| {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .expect("manifest contains requested origin");
            plan_from_script(root, script).plan
        };

        let no_filenames = plan("testsuite/bsc.driver/no_filenames/no_filenames.exp");
        assert_eq!(no_filenames.status, PlanStatus::Complete);
        assert!(no_filenames.diagnostics.is_empty());
        let verilog = no_filenames
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-1-Top")
            .expect("Verilog scenario is imported");
        assert!(matches!(
            verilog.stages[0].operations.as_slice(),
            [
                OperationRecord { action: Action::FsEnsureDirectoryAbsent { path: remove_bd }, .. },
                OperationRecord { action: Action::FsEnsureDirectoryAbsent { path: remove_vd }, .. },
                OperationRecord { action: Action::FsMkdir { path: mkdir_bd }, .. },
                OperationRecord { action: Action::FsMkdir { path: mkdir_vd }, .. },
                OperationRecord { action: Action::BscCompile { .. }, .. },
                ..
            ] if remove_bd == "bd"
                && remove_vd == "vd"
                && mkdir_bd == "bd"
                && mkdir_vd == "vd"
        ));
        assert!(no_filenames
            .fixtures
            .iter()
            .any(|fixture| fixture.path == "vlib/Banner.v"));
        let verilog_link = verilog.stages[0]
            .operations
            .iter()
            .find(|operation| matches!(operation.action, Action::BscLink { .. }))
            .expect("Verilog link is imported");
        assert!(verilog_link
            .artifacts
            .inputs
            .contains(&"vlib/Banner.v".to_owned()));
        let bluesim = no_filenames
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "bluesim-workflow-mkTop")
            .expect("Bluesim scenario is imported");
        assert!(matches!(
            bluesim.stages[0].operations.as_slice(),
            [
                OperationRecord { action: Action::FsEnsureDirectoryAbsent { path: remove_bd }, .. },
                OperationRecord { action: Action::FsEnsureDirectoryAbsent { path: remove_sd }, .. },
                OperationRecord { action: Action::FsMkdir { path: mkdir_bd }, .. },
                OperationRecord { action: Action::FsMkdir { path: mkdir_sd }, .. },
                OperationRecord { action: Action::BscGenerate { .. }, .. },
                ..
            ] if remove_bd == "bd"
                && remove_sd == "sd"
                && mkdir_bd == "bd"
                && mkdir_sd == "sd"
        ));
        let generation = &bluesim.stages[0].operations[4];
        assert!(generation
            .artifacts
            .outputs
            .contains(&"bd/mkMid1.ba".to_owned()));
        assert!(!generation
            .artifacts
            .outputs
            .contains(&"mkMid1.ba".to_owned()));

        let incremental = plan("testsuite/bsc.driver/bluesim/bluesim.exp");
        assert_eq!(incremental.status, PlanStatus::Complete);
        assert!(incremental.diagnostics.is_empty());
        assert_eq!(incremental.scenarios.len(), 1);
        let resets = incremental
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .filter(|operation| matches!(operation.action, Action::FsEnsureDirectoryAbsent { .. }))
            .count();
        assert_eq!(
            resets, 2,
            "the one reset prefix must not be copied to later workflows"
        );
        assert!(!incremental.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("nukedir")
                || diagnostic.message.contains("workflow action mkdir")
        }));

        let options = plan("testsuite/bsc.options/options.exp");
        assert_eq!(options.status, PlanStatus::Complete);
        assert!(options.diagnostics.is_empty());
        assert!(!options.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "import.unsupported_tcl"
                && diagnostic.message.starts_with("nukedir:")
        }));
        assert!(!options.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "import.uncomposed_action" && diagnostic.message.contains("nukedir")
        }));
    }

    #[test]
    fn maps_only_the_closed_vcdcheck_option_subset() {
        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let assertion = |helper: &str, options: &str| AssertionContract {
            helper: helper.to_owned(),
            arguments: vec!["dump.vcd".to_owned(), options.to_owned()],
            guard: Guard::Capability {
                capability: Capability::InternalChecks,
            },
            span,
            expansion: Vec::new(),
        };

        let operation = map_assertion(&assertion(
            "vcdcheck_pass",
            "-c {main.top.signal exists} -c {main.top.signal toggles}",
        ))
        .expect("closed vcdcheck options map");
        assert!(matches!(
            operation.action,
            Action::VcdCheck { path, checks, expected_exit: ExpectedExit::Success }
                if path == "dump.vcd"
                    && checks == ["main.top.signal exists", "main.top.signal toggles"]
        ));

        assert!(map_assertion(&assertion("vcdcheck_fail", "-x nope")).is_err());
        assert!(map_assertion(&assertion("vcdcheck_fail", "-c")).is_err());
        assert!(map_assertion(&assertion("vcdcheck_fail", "")).is_err());
    }

    #[test]
    fn recognizes_only_local_dynamic_compile_outputs() {
        assert!(is_dynamic_generated_output("compile-time.log"));
        assert!(is_dynamic_generated_output("compile-time.out"));
        assert!(!is_dynamic_generated_output("nested/compile-time.log"));
        assert!(!is_dynamic_generated_output("simulation.c.out"));
        assert!(!is_dynamic_generated_output("simulation.v.out"));
        assert!(!is_dynamic_generated_output("compile-time.txt"));
    }

    #[test]
    fn declares_only_audited_static_compile_dump_outputs() {
        let arguments = [
            "-dATS=result.ats".to_owned(),
            "-dATSexpand=%m.atsexpand".to_owned(),
            "-dsplitIf=split.dump".to_owned(),
            "-dUnknown=unknown.dump".to_owned(),
        ];

        assert_eq!(
            compile_dump_paths(&arguments, "mkDemo"),
            BTreeSet::from([
                "mkDemo.atsexpand".to_owned(),
                "result.ats".to_owned(),
                "split.dump".to_owned(),
            ])
        );
    }

    #[test]
    fn synthesized_modules_ignore_commented_attributes() {
        let fixture_root = project_root().join("testsuite/bsc.bsv_examples/mesa/spiless-tx-bsv");
        let modules =
            synthesized_modules(&fixture_root, &BTreeSet::from(["TestMesa.bsv".to_owned()]));

        assert!(modules.contains(&"sysTestMesa".to_owned()));
        assert!(!modules.contains(&"mkTestRecv".to_owned()));
        assert!(!modules.contains(&"mkTestTrans".to_owned()));

        assert_eq!(
            synthesized_modules(
                &project_root().join("testsuite/bsc.evaluator"),
                &BTreeSet::from(["Bug45a.bs".to_owned()]),
            ),
            ["sysBug45a"]
        );
    }

    #[test]
    fn verilog_elaboration_declares_synthesized_ba_in_bdir() {
        let fixture_root = project_root().join("testsuite/bsc.bugs/bluespec_inc/b1480");
        let shape = CompileShape {
            mode: BscCompileMode::Verilog,
            module: None,
            args: vec!["-elab".to_owned(), "-bdir".to_owned(), "build".to_owned()],
            dependency_mode: DependencyMode::Update,
            expected_exit: ExpectedExit::Success,
            unexpected_success_forbidden_regex: None,
            expectation: OperationExpectation::Required,
            stdout: "If1.bsv.bsc-vcomp-out".to_owned(),
            diagnostics: Vec::new(),
        };

        let outputs = compile_artifact_paths(&shape, "If1.bsv", &fixture_root);
        assert!(outputs.contains("build/sysIf1.ba"));
        assert!(outputs.contains("sysIf1.v"));

        let without_elab = CompileShape {
            args: vec!["-bdir".to_owned(), "build".to_owned()],
            ..shape
        };
        assert!(
            !compile_artifact_paths(&without_elab, "If1.bsv", &fixture_root)
                .contains("build/sysIf1.ba")
        );

        let backend_dump = CompileShape {
            mode: BscCompileMode::Frontend,
            module: Some("sysDump".to_owned()),
            args: vec!["-verilog".to_owned(), "-dvschedinfo=%m.sched".to_owned()],
            stdout: "If1.bsv.bsc-out".to_owned(),
            ..without_elab
        };
        let outputs = compile_artifact_paths(&backend_dump, "If1.bsv", &fixture_root);
        assert!(outputs.contains("sysDump.v"));
        assert!(outputs.contains("sysDump.sched"));

        let killed_dump = CompileShape {
            args: vec![
                "-verilog".to_owned(),
                "-dATSexpand=%m.atsexpand".to_owned(),
                "-KILLATSexpand".to_owned(),
            ],
            ..backend_dump
        };
        let outputs = compile_artifact_paths(&killed_dump, "If1.bsv", &fixture_root);
        assert!(outputs.contains("sysDump.atsexpand"));
        assert!(!outputs.contains("sysDump.v"));
        assert!(!outputs.contains("If1.bo"));
    }

    #[test]
    fn applies_a_closed_bsc_options_overlay_only_to_contained_scenarios() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let operation = |start_byte| {
            OperationRecord::new(
                Action::BscCompile {
                    source: "Demo.bsv".to_owned(),
                    working_directory: None,
                    mode: BscCompileMode::Frontend,
                    module: None,
                    args: Vec::new(),
                    absolute_import_paths: Vec::new(),
                    dependency_mode: DependencyMode::Update,
                    expected_exit: ExpectedExit::Success,
                    unexpected_success_forbidden_regex: None,
                    environment: None,
                    stdout: "Demo.out".to_owned(),
                },
                OperationExpectation::Required,
                provenance(span(start_byte), &[]),
            )
        };
        let mut assembly = PlanAssembly {
            scenarios: vec![Scenario {
                id: "inside".to_owned(),
                resource: ResourceClass::Normal,
                fixtures: Vec::new(),
                requires: Vec::new(),
                bsc_options_append: None,
                timeouts: Timeouts::default(),
                stages: vec![Stage {
                    id: "compile".to_owned(),
                    operations: vec![operation(10)],
                }],
            }],
            ..PlanAssembly::default()
        };
        let script = ScriptManifest {
            origin: "testsuite/example/example.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: Vec::new(),
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: vec![crate::model::BscOptionsOverlay {
                append: "-D FOO".to_owned(),
                start: span(0),
                end: span(20),
            }],
            unsupported: Vec::new(),
        };
        let mut diagnostics = Vec::new();

        apply_bsc_options_overlays(&script, &mut assembly, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(
            assembly.scenarios[0].bsc_options_append.as_deref(),
            Some("-D FOO")
        );
    }

    #[test]
    fn injects_make_test_data_before_each_downstream_scenario() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let scenario = |id: &str, start_byte| Scenario {
            id: id.to_owned(),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: Vec::new(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "compile".to_owned(),
                operations: vec![OperationRecord::new(
                    Action::BscCompile {
                        source: "Demo.bsv".to_owned(),
                        working_directory: None,
                        mode: BscCompileMode::Frontend,
                        module: None,
                        args: Vec::new(),
                        absolute_import_paths: Vec::new(),
                        dependency_mode: DependencyMode::Update,
                        expected_exit: ExpectedExit::Success,
                        unexpected_success_forbidden_regex: None,
                        environment: None,
                        stdout: format!("{id}.out"),
                    },
                    OperationExpectation::Required,
                    provenance(span(start_byte), &[]),
                )],
            }],
        };
        let script = ScriptManifest {
            origin: "testsuite/example/example.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: Vec::new(),
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: Vec::new(),
            make_test_data_actions: vec![MakeTestDataAction {
                guard: Guard::Always,
                span: span(10),
                expansion: Vec::new(),
            }],
            bsc_options_overlays: Vec::new(),
            unsupported: Vec::new(),
        };
        let mut assembly = PlanAssembly {
            scenarios: vec![
                scenario("before", 5),
                scenario("first", 20),
                scenario("second", 30),
            ],
            ..PlanAssembly::default()
        };
        let mut diagnostics = Vec::new();

        inject_make_test_data_actions(&script, &mut assembly, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert!(!matches!(
            assembly.scenarios[0].stages[0].operations[0].action,
            Action::MakeTestData
        ));
        for scenario in &assembly.scenarios[1..] {
            assert!(matches!(
                scenario.stages[0].operations[0].action,
                Action::MakeTestData
            ));
            assert_eq!(
                scenario.stages[0].operations[0].artifacts.inputs,
                ["Makefile.data", "dumper.c"]
            );
        }
    }

    #[test]
    fn imports_upstream_make_test_data_for_each_example_scenario() {
        let manifest = build_manifest(project_root()).unwrap();
        for origin in [
            "testsuite/bsc.bsv_examples/AssertionsDemo/assert_demo.exp",
            "testsuite/bsc.bsv_examples/FloatingPoint/floating_point.exp",
        ] {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .unwrap_or_else(|| panic!("missing fixture script {origin}"));
            let plan = plan_from_script(project_root(), script).plan;

            assert_eq!(
                plan.status,
                PlanStatus::Complete,
                "{origin}: {:?}",
                plan.diagnostics
            );
            for scenario in &plan.scenarios {
                assert!(matches!(
                    scenario.stages[0].operations[0].action,
                    Action::MakeTestData
                ));
                assert!(scenario.fixtures.contains(&"Makefile.data".to_owned()));
                assert!(scenario.fixtures.contains(&"dumper.c".to_owned()));
            }
            for path in ["Makefile.data", "dumper.c"] {
                assert!(matches!(
                    plan.fixtures.iter().find(|fixture| fixture.path == path),
                    Some(Fixture {
                        role: FixtureRole::BuildInput,
                        ..
                    })
                ));
            }
        }
    }

    #[test]
    fn empty_object_bluesim_link_declares_the_unique_default_module_artifact() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let script = ScriptManifest {
            origin: "testsuite/example/example.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: Vec::new(),
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            workflow_actions: vec![
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: "Top.bs".to_owned(),
                    module: None,
                    options: String::new(),
                    guard: Guard::Always,
                    span: span(0),
                    expansion: Vec::new(),
                }),
                WorkflowAction::LinkObjects(crate::model::LinkObjectsAction {
                    objects: String::new(),
                    top: "sysTop".to_owned(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Success,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: None,
                    guard: Guard::Always,
                    span: span(10),
                    expansion: Vec::new(),
                }),
            ],
            unsupported: Vec::new(),
        };
        let imported = standalone_generation_scenario(
            0,
            match &script.workflow_actions[0] {
                WorkflowAction::CompileObject(generation) => generation,
                _ => unreachable!("test fixture starts with generation"),
            },
            &[],
            &[],
            None,
        )
        .unwrap();
        let mut assembly = PlanAssembly::default();
        assembly.push(imported);
        compose_ordered_bluesim_links(Path::new("missing-fixtures"), &script, &mut assembly);

        assert_eq!(assembly.scenarios.len(), 1);
        assert!(assembly.consumed_actions.contains(&1));
        let operations = &assembly.scenarios[0].stages;
        assert!(operations
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                operation
                    .artifacts
                    .outputs
                    .contains(&"sysTop.ba".to_owned())
            }));
        assert!(operations
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    operation.action,
                    Action::BscLink { ref objects, ref top, .. }
                        if objects.is_empty() && top == "sysTop"
                )
            }));
    }

    #[test]
    fn failed_verilog_compile_declares_only_explicitly_continued_outputs() {
        let contract = |options: &str| CompileContract {
            source: "Broken.bsv".to_owned(),
            working_directory: None,
            helper: "compile_verilog_fail_error".to_owned(),
            arguments: vec![
                "Broken.bsv".to_owned(),
                "G0001".to_owned(),
                "1".to_owned(),
                String::new(),
                options.to_owned(),
            ],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        let root = std::env::temp_dir().join(format!(
            "bsc-failed-verilog-artifacts-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Broken.bsv"),
            "(* synthesize *) module sysBroken(); endmodule\n",
        )
        .unwrap();

        let failed = compile_shape(&contract("")).unwrap();
        assert!(!compile_artifact_paths(&failed, "Broken.bsv", &root).contains("sysBroken.v"));
        let continued = compile_shape(&contract("-continue-after-errors")).unwrap();
        assert!(compile_artifact_paths(&continued, "Broken.bsv", &root).contains("sysBroken.v"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn compile_object_declares_static_explicit_generation_modules() {
        let generation = CompileObjectAction {
            source: "Source.bs".to_owned(),
            module: Some("sysTop".to_owned()),
            options: "-g sysChild -bdir generated -g sysLeaf".to_owned(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        let action = WorkflowAction::CompileObject(generation.clone());
        let operation = map_action(&action).unwrap();
        assert_eq!(
            operation
                .artifacts
                .outputs
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "Source.bs.bsc-ccomp-out".to_owned(),
                "generated/Source.bo".to_owned(),
                "generated/sysChild.ba".to_owned(),
                "generated/sysLeaf.ba".to_owned(),
                "generated/sysTop.ba".to_owned(),
            ])
        );
        assert_eq!(
            generation_artifact_paths(&generation, None).unwrap(),
            BTreeSet::from([
                "Source.bs.bsc-ccomp-out".to_owned(),
                "generated/sysChild.ba".to_owned(),
                "generated/sysLeaf.ba".to_owned(),
                "generated/sysTop.ba".to_owned(),
            ])
        );
    }

    #[test]
    fn compile_object_rejects_ambiguous_or_missing_generation_module_arguments() {
        let generation = |options: &str| CompileObjectAction {
            source: "Source.bs".to_owned(),
            module: None,
            options: options.to_owned(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        assert!(generation_artifact_paths(&generation("-g"), None)
            .unwrap_err()
            .contains("-g requires"));
        assert!(
            generation_artifact_paths(&generation("-bdir one -bdir two"), None)
                .unwrap_err()
                .contains("ambiguous -bdir")
        );
        assert!(
            generation_artifact_paths(&generation("-g ../sysChild"), None)
                .unwrap_err()
                .contains("portable module-name")
        );
    }

    #[test]
    fn filesystem_composition_binds_to_the_last_producer_of_its_source() {
        let source_span = |start_byte| SourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let operation = |span, action| {
            OperationRecord::new(
                action,
                OperationExpectation::Required,
                Provenance {
                    span,
                    expansion: Vec::new(),
                },
            )
        };
        let scenario = Scenario {
            id: "producer-boundary".to_owned(),
            resource: ResourceClass::Normal,
            fixtures: Vec::new(),
            requires: Vec::new(),
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "stage".to_owned(),
                operations: vec![
                    operation(
                        source_span(10),
                        Action::BscCompile {
                            source: "Design.bsv".to_owned(),
                            working_directory: None,
                            mode: BscCompileMode::Frontend,
                            module: None,
                            args: Vec::new(),
                            absolute_import_paths: Vec::new(),
                            dependency_mode: DependencyMode::Update,
                            expected_exit: ExpectedExit::Success,
                            unexpected_success_forbidden_regex: None,
                            environment: None,
                            stdout: "generated-output".to_owned(),
                        },
                    ),
                    operation(
                        source_span(100),
                        Action::AssertExists {
                            path: "generated-output".to_owned(),
                        },
                    ),
                ],
            }],
        };

        assert_eq!(
            artifact_producer_order(&scenario, "generated-output"),
            Some(execution_order_key(
                ManifestSourceSpan {
                    start_byte: 10,
                    end_byte: 11,
                    start_line: 1,
                    start_column: 11,
                    end_line: 1,
                    end_column: 12,
                },
                &[],
            ))
        );
    }

    #[test]
    fn maps_only_the_audited_prelude_bsv_line_filter() {
        let comparison = ComparisonContract {
            helper: "compare_file_filtered".to_owned(),
            arguments: vec![
                "output.log".to_owned(),
                String::new(),
                String::new(),
                r#"-e s/\"PreludeBSV\.bsv\",\ line\ \[0-9\]\+,/\"PreludeBSV\.bsv\"\,\ line\ NNNN,/g"#.to_owned(),
            ],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 1,
                end_byte: 2,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        assert!(matches!(
            map_comparison(&comparison).unwrap().action,
            Action::AssertGoldenNormalized {
                normalizations,
                ..
            } if normalizations == vec![GoldenNormalization::PreludeBsvLineNumbers]
        ));

        let mut unsupported = comparison.clone();
        unsupported.arguments[3] = "-e s/other/other/g".to_owned();
        assert!(map_comparison(&unsupported).is_err());
    }

    #[test]
    fn imports_adjacent_rendered_basic_options_without_promoting_output_to_fixture() {
        let template = project_root()
            .join("testsuite")
            .join("rendered-basic-options-template.expected");
        fs::write(&template, "bsc -i BLUESPECDIR -print-flags\n").unwrap();
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let script = ScriptManifest {
            origin: "testsuite/rendered-basic-options.exp".to_owned(),
            contracts: vec![
                Contract::RenderGolden(RenderGoldenContract {
                    template: "rendered-basic-options-template.expected".to_owned(),
                    output: "rendered-basic-options.expected".to_owned(),
                    macro_value: GoldenMacroValue::BluespecDir,
                    guard: Guard::Always,
                    span: span(10),
                    expansion: Vec::new(),
                }),
                Contract::BasicOptions(BasicOptionsContract {
                    options: "-print-flags".to_owned(),
                    output: "rendered-basic-options.actual".to_owned(),
                    expected: "rendered-basic-options.expected".to_owned(),
                    guard: Guard::Always,
                    span: span(20),
                    expansion: Vec::new(),
                }),
            ],
            source_sha256: String::new(),
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            unsupported: Vec::new(),
        };
        let generated = plan_from_script(project_root(), &script);
        fs::remove_file(&template).unwrap();

        assert_eq!(generated.plan.status, PlanStatus::Complete);
        assert!(generated.plan.diagnostics.is_empty());
        assert_eq!(generated.plan.scenarios.len(), 1);
        assert!(matches!(
            generated.plan.scenarios[0].stages[0].operations.as_slice(),
            [
                OperationRecord { action: Action::RenderGolden { template, output, replacement: GoldenReplacement::BluespecDir }, .. },
                OperationRecord { action: Action::BscOptions { .. }, .. },
                OperationRecord { action: Action::AssertGolden { expected, .. }, .. },
            ] if template == "rendered-basic-options-template.expected"
                && output == "rendered-basic-options.expected"
                && expected == "rendered-basic-options.expected"
        ));
        assert!(generated.plan.fixtures.iter().any(|fixture| {
            fixture.path == "rendered-basic-options-template.expected"
                && fixture.role == FixtureRole::Golden
        }));
        assert!(!generated
            .plan
            .fixtures
            .iter()
            .any(|fixture| { fixture.path == "rendered-basic-options.expected" }));
    }

    #[test]
    fn rejects_rendered_basic_options_when_output_does_not_match_expected() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let script = ScriptManifest {
            origin: "testsuite/rendered-basic-options-mismatch.exp".to_owned(),
            contracts: vec![
                Contract::RenderGolden(RenderGoldenContract {
                    template: "template.expected".to_owned(),
                    output: "rendered.expected".to_owned(),
                    macro_value: GoldenMacroValue::BluespecDir,
                    guard: Guard::Always,
                    span: span(10),
                    expansion: Vec::new(),
                }),
                Contract::BasicOptions(BasicOptionsContract {
                    options: "-print-flags".to_owned(),
                    output: "actual.out".to_owned(),
                    expected: "other.expected".to_owned(),
                    guard: Guard::Always,
                    span: span(20),
                    expansion: Vec::new(),
                }),
            ],
            source_sha256: String::new(),
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            unsupported: Vec::new(),
        };
        let generated = plan_from_script(project_root(), &script);

        assert_eq!(generated.plan.status, PlanStatus::Blocked);
        assert!(generated.plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "import.render_golden_contract"
                && diagnostic.message.contains("exactly match")
        }));
    }

    #[test]
    fn compile_fixture_copy_preconditions_require_a_closed_static_phase() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let contract = CompileContract {
            source: "Derived.bsv".to_owned(),
            working_directory: None,
            helper: "compile_verilog_pass".to_owned(),
            arguments: vec!["Derived.bsv".to_owned()],
            guard: Guard::Always,
            span: span(20),
            expansion: Vec::new(),
        };
        let copy = |destination: &str| {
            WorkflowAction::TransferArtifact(crate::model::ArtifactTransferAction {
                operation: ArtifactTransferOperation::Copy,
                source: "Source.bsv".to_owned(),
                destination: destination.to_owned(),
                guard: Guard::Always,
                span: span(10),
                expansion: Vec::new(),
            })
        };
        let source_paths = BTreeSet::from(["Source.bsv".to_owned()]);

        let consumed = BTreeSet::new();
        let contract_order = execution_order_key(contract.span, &contract.expansion);
        let matching_copy = vec![copy("Derived.bsv")];
        assert!(matches!(
            compile_preceding_fixture_copies(
                &contract,
                &matching_copy,
                &consumed,
                None,
                &contract_order,
                &[],
                &[],
                &[],
                &source_paths,
            )
            .unwrap()
            .as_slice(),
            [(0, WorkflowAction::TransferArtifact(_))]
        ));
        let mismatched_copy = vec![copy("Other.bsv")];
        assert!(compile_preceding_fixture_copies(
            &contract,
            &mismatched_copy,
            &consumed,
            None,
            &contract_order,
            &[],
            &[],
            &[],
            &source_paths,
        )
        .unwrap()
        .is_empty());
        let barrier = vec![AssertionContract {
            helper: "files_exist".to_owned(),
            arguments: vec!["Source.bsv".to_owned()],
            guard: Guard::Always,
            span: span(15),
            expansion: Vec::new(),
        }];
        assert!(compile_preceding_fixture_copies(
            &contract,
            &matching_copy,
            &consumed,
            None,
            &contract_order,
            &barrier,
            &[],
            &[],
            &source_paths,
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn compile_fixture_touch_precondition_requires_a_unique_adjacent_matching_touch() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let contract = CompileContract {
            source: "Source.bsv".to_owned(),
            working_directory: None,
            helper: "compile_verilog_pass".to_owned(),
            arguments: vec!["Source.bsv".to_owned()],
            guard: Guard::Always,
            span: span(20),
            expansion: Vec::new(),
        };
        let touch = |path: &str, guard: Guard| {
            WorkflowAction::TouchArtifact(crate::model::TouchArtifactAction {
                path: path.to_owned(),
                guard,
                span: span(10),
                expansion: Vec::new(),
            })
        };
        let source_paths = BTreeSet::from(["Source.bsv".to_owned()]);
        let consumed = BTreeSet::new();
        let contract_order = execution_order_key(contract.span, &contract.expansion);
        let matching_touch = vec![touch("Source.bsv", Guard::Always)];
        assert!(matches!(
            compile_preceding_fixture_touch(
                &contract,
                &matching_touch,
                &consumed,
                None,
                &contract_order,
                &[],
                &[],
                &[],
                &source_paths,
            ),
            Some((0, WorkflowAction::TouchArtifact(_)))
        ));
        let mismatched_touch = vec![touch("Other.bsv", Guard::Always)];
        assert!(compile_preceding_fixture_touch(
            &contract,
            &mismatched_touch,
            &consumed,
            None,
            &contract_order,
            &[],
            &[],
            &[],
            &source_paths,
        )
        .is_none());
        let barrier = vec![AssertionContract {
            helper: "files_exist".to_owned(),
            arguments: vec!["Source.bsv".to_owned()],
            guard: Guard::Always,
            span: span(15),
            expansion: Vec::new(),
        }];
        assert!(compile_preceding_fixture_touch(
            &contract,
            &matching_touch,
            &consumed,
            None,
            &contract_order,
            &barrier,
            &[],
            &[],
            &source_paths,
        )
        .is_none());
        let guarded_touch = vec![touch(
            "Source.bsv",
            Guard::Capability {
                capability: Capability::Verilog,
            },
        )];
        assert!(compile_preceding_fixture_touch(
            &contract,
            &guarded_touch,
            &consumed,
            None,
            &contract_order,
            &[],
            &[],
            &[],
            &source_paths,
        )
        .is_none());
    }

    #[test]
    fn maps_manual_verilog_workflow_actions_and_rejects_unsafe_contracts() {
        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let link = WorkflowAction::LinkVerilog(crate::model::LinkVerilogAction {
            objects: "mkDemo.v helper.v".to_owned(),
            top: "mkDemo".to_owned(),
            options: "-L lib".to_owned(),
            no_main: false,
            expected_exit: ExpectedExit::Success,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            expectation: OperationExpectation::Required,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        });
        assert!(matches!(
            map_action(&link).unwrap().action,
            Action::BscLink {
                backend: PlanSimulationBackend::Icarus,
                objects,
                top,
                args,
                mode: BscLinkMode::Standard,
                expected_exit: ExpectedExit::Success,
                ..
            } if objects == ["mkDemo.v", "helper.v"]
                && top == "mkDemo"
                && args == ["-L", "lib"]
        ));

        let no_main = WorkflowAction::LinkVerilog(crate::model::LinkVerilogAction {
            objects: "Tb.v mkDemo.v".to_owned(),
            top: "Tb".to_owned(),
            options: "-ignored-by-tcl".to_owned(),
            no_main: true,
            expected_exit: ExpectedExit::Success,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            expectation: OperationExpectation::Required,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        });
        assert!(matches!(
            map_action(&no_main).unwrap().action,
            Action::BscLink {
                backend: PlanSimulationBackend::Icarus,
                mode: BscLinkMode::NoMain,
                objects,
                top,
                args,
                expected_exit: ExpectedExit::Success,
                ..
            } if objects == ["Tb.v", "mkDemo.v"] && top == "Tb" && args.is_empty()
        ));

        let no_main_binding = crate::model::LinkVerilogAction {
            objects: "Tb.v mkDemo.v".to_owned(),
            top: "Tb".to_owned(),
            options: String::new(),
            no_main: true,
            expected_exit: ExpectedExit::Success,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            expectation: OperationExpectation::Required,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        };
        assert!(verilog_link_extends_flow(
            &mut ArtifactFlow::new(BTreeSet::from(["mkDemo.v".to_owned()])),
            &GeneratedArtifactProfile::default(),
            &no_main_binding,
        )
        .is_ok());
        assert!(verilog_link_extends_flow(
            &mut ArtifactFlow::new(BTreeSet::from(["mkDemo.v".to_owned()])),
            &GeneratedArtifactProfile::default(),
            &crate::model::LinkVerilogAction {
                no_main: false,
                ..no_main_binding
            },
        )
        .is_err());

        for (expected_exits, vcd, expected_exit) in [
            (Vec::new(), false, 0),
            (vec![3], false, 3),
            (Vec::new(), true, 0),
        ] {
            let run = WorkflowAction::RunVerilog(crate::model::RunVerilogAction {
                executable: "mkDemo".to_owned(),
                options: "+arg".to_owned(),
                stdout: "mkDemo.out".to_owned(),
                expected_exits,
                vcd,
                guard: Guard::Always,
                span,
                expansion: Vec::new(),
            });
            assert!(matches!(
                map_action(&run).unwrap().action,
                Action::SimulationRun {
                    backend: PlanSimulationBackend::Icarus,
                    expected_exits: mapped_exits,
                    vcd: mapped_vcd,
                    ..
                } if mapped_exits.codes == [expected_exit]
                    && mapped_vcd == vcd.then(|| "dump.vcd".to_owned())
            ));
        }

        let options_vcd = WorkflowAction::RunVerilog(crate::model::RunVerilogAction {
            executable: "mkDemo".to_owned(),
            options: "+bscvcd +foo".to_owned(),
            stdout: "mkDemo.out".to_owned(),
            expected_exits: Vec::new(),
            vcd: false,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        });
        assert!(matches!(
            map_action(&options_vcd).unwrap().action,
            Action::SimulationRun {
                args,
                vcd: Some(vcd),
                ..
            } if args == ["+foo"] && vcd == "dump.vcd"
        ));

        let multiple_statuses = WorkflowAction::RunVerilog(crate::model::RunVerilogAction {
            executable: "mkDemo".to_owned(),
            options: String::new(),
            stdout: "mkDemo.out".to_owned(),
            expected_exits: vec![0, 3],
            vcd: false,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        });
        assert!(matches!(
            map_action(&multiple_statuses).unwrap().action,
            Action::SimulationRun { expected_exits, .. }
                if expected_exits.codes == [0, 3]
        ));

        let wildcard = WorkflowAction::LinkVerilog(crate::model::LinkVerilogAction {
            objects: "*.v".to_owned(),
            top: "mkDemo".to_owned(),
            options: String::new(),
            no_main: false,
            expected_exit: ExpectedExit::Success,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            expectation: OperationExpectation::Required,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        });
        assert!(map_action(&wildcard)
            .unwrap_err()
            .contains("require shell expansion"));
    }

    #[test]
    fn maps_link_failure_diagnostic_to_its_link_output() {
        let link = crate::model::LinkObjectsAction {
            objects: "mkDesign.ba".to_owned(),
            top: "mkDesign".to_owned(),
            options: String::new(),
            expected_exit: ExpectedExit::Failure,
            expectation: OperationExpectation::Required,
            error_diagnostic: Some(crate::model::LinkErrorDiagnostic {
                code: "G0099".to_owned(),
                count: "2".to_owned(),
            }),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        assert_eq!(
            link_initial_artifact_paths(&link),
            BTreeSet::from(["mkDesign.bsc-ccomp-out".to_owned()])
        );
        assert!(matches!(
            link_error_diagnostic_operation(&link).unwrap(),
            Some(OperationRecord {
                action: Action::AssertDiagnosticCount {
                    path,
                    kind: DiagnosticKind::Error,
                    code: Some(code),
                    count: 2,
                },
                ..
            }) if path == "mkDesign.bsc-ccomp-out" && code == "G0099"
        ));
    }

    #[test]
    fn resolves_only_unique_extensionless_local_sources() {
        let root = std::env::temp_dir().join(format!(
            "bsc-extensionless-source-resolution-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Only.bsv"), "package Only; endpackage\n").unwrap();
        fs::write(root.join("Both.bsv"), "package Both; endpackage\n").unwrap();
        fs::write(root.join("Both.bs"), "package Both; endpackage\n").unwrap();
        fs::write(root.join("Explicit.bs"), "package Explicit; endpackage\n").unwrap();

        assert_eq!(
            resolve_extensionless_source("Only", &root).as_deref(),
            Some("Only.bsv")
        );
        assert_eq!(resolve_extensionless_source("Both", &root), None);
        assert_eq!(resolve_extensionless_source("Missing", &root), None);
        assert_eq!(resolve_extensionless_source("Explicit.bs", &root), None);

        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let mut script = ScriptManifest {
            origin: "testsuite/example/example.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: vec![Contract::Compile(CompileContract {
                source: "Only".to_owned(),
                working_directory: None,
                helper: "compile_verilog_pass".to_owned(),
                arguments: vec!["Only".to_owned()],
                guard: Guard::Always,
                span,
                expansion: Vec::new(),
            })],
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            unsupported: Vec::new(),
        };
        resolve_extensionless_contract_sources(&mut script, &root);
        let Contract::Compile(contract) = &script.contracts[0] else {
            unreachable!("test fixture has one compile contract");
        };
        assert_eq!(contract.source, "Only.bsv");
        assert_eq!(contract.arguments, ["Only.bsv"]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn attributes_synthesized_verilog_artifacts_to_no_main_links() {
        let root = std::env::temp_dir().join(format!(
            "bsc-synthesize-artifact-attribution-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("Design.bsv"),
            "(* synthesize *)\nmodule mkDesign_02();\nendmodule\n",
        )
        .unwrap();
        fs::write(root.join("Plain.bsv"), "module mkPlain();\nendmodule\n").unwrap();

        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let compile = |source: &str| CompileContract {
            source: source.to_owned(),
            working_directory: None,
            helper: "compile_verilog_pass".to_owned(),
            arguments: vec![source.to_owned()],
            guard: Guard::Always,
            span: span(0),
            expansion: Vec::new(),
        };
        let no_main = crate::model::LinkVerilogAction {
            objects: "Tb02.v mkDesign_02.v".to_owned(),
            top: "Tb".to_owned(),
            options: String::new(),
            no_main: true,
            expected_exit: ExpectedExit::Success,
            simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
            expectation: OperationExpectation::Required,
            guard: Guard::Always,
            span: span(10),
            expansion: Vec::new(),
        };
        let shape = compile_shape(&compile("Design.bsv")).unwrap();
        let artifacts = compile_artifact_paths(&shape, "Design.bsv", &root);
        assert!(artifacts.contains("Design.bo"));
        assert!(!artifacts.contains("Design.bi"));
        assert!(artifacts.contains("mkDesign_02.v"));
        assert!(!artifacts.contains("sysDesign.v"));
        assert!(!compile_artifact_paths(&shape, "Plain.bsv", &root).contains("mkDesign_02.v"));
        assert!(!compile_artifact_paths(&shape, "Missing.bsv", &root).contains("mkDesign_02.v"));

        let mut dump_shape = compile_shape(&compile("Design.bsv")).unwrap();
        dump_shape.args.push("-dwrappergen=%m.wrap".to_owned());
        let dump_artifacts = compile_artifact_paths(&dump_shape, "Design.bsv", &root);
        assert!(dump_artifacts.contains("mkDesign_02.wrap"));
        assert!(!dump_artifacts.contains("sysDesign.wrap"));

        let mut explicit_shape = compile_shape(&compile("Design.bsv")).unwrap();
        explicit_shape.module = Some("mkExplicit".to_owned());
        assert!(explicit_shape
            .artifact_paths("Design.bsv")
            .contains("mkExplicit.v"));

        let script = |source: &str| ScriptManifest {
            origin: "testsuite/example/example.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: vec![Contract::Compile(compile(source))],
            assertions: Vec::new(),
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: vec![WorkflowAction::LinkVerilog(no_main.clone())],
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            unsupported: Vec::new(),
        };
        assert_eq!(
            check_bindings(&script("Design.bsv"), &root).workflow_actions(&ProducerKey::Compile(0)),
            Some(&BTreeSet::from([0]))
        );
        assert_eq!(
            check_bindings(&script("Plain.bsv"), &root).workflow_actions(&ProducerKey::Compile(0)),
            None
        );

        let standard = crate::model::LinkVerilogAction {
            objects: "helper.v".to_owned(),
            top: "mkDemo".to_owned(),
            no_main: false,
            ..no_main
        };
        assert!(verilog_link_extends_flow(
            &mut ArtifactFlow::new(BTreeSet::new()),
            &GeneratedArtifactProfile {
                verilog: true,
                ..GeneratedArtifactProfile::default()
            },
            &standard,
        )
        .is_ok());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn maps_backend_specific_simulation_bug_annotations() {
        assert_eq!(
            known_simulation_output_xfail(SimulationBackend::Bluesim, "FIFO_sim_issue").unwrap(),
            Some("upstream bug FIFO_sim_issue".to_owned())
        );
        assert_eq!(
            known_simulation_output_xfail(SimulationBackend::Icarus, "verilator").unwrap(),
            None
        );
        assert_eq!(
            known_simulation_output_xfail(SimulationBackend::Icarus, "modelsim iverilog verilator")
                .unwrap(),
            Some("upstream simulator bug list \"modelsim iverilog verilator\"".to_owned())
        );
        assert_eq!(
            known_simulation_output_xfail(SimulationBackend::Icarus, " modelsim").unwrap(),
            None
        );
        assert!(known_simulation_output_xfail(SimulationBackend::Icarus, "{unterminated").is_err());
    }

    #[test]
    fn imports_divmod_architecture_specific_exit_sets() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.misc/divmod/divmod")
            .expect("divmod plan is generated");

        assert_eq!(
            plan.plan.status,
            PlanStatus::Complete,
            "{:#?}",
            plan.plan.diagnostics
        );
        assert!(plan.plan.diagnostics.is_empty());
        let find_exits = |executable: &str| {
            plan.plan
                .scenarios
                .iter()
                .flat_map(|scenario| &scenario.stages)
                .flat_map(|stage| &stage.operations)
                .find_map(|operation| match &operation.action {
                    Action::SimulationRun {
                        backend: PlanSimulationBackend::Bluesim,
                        executable: candidate,
                        expected_exits,
                        ..
                    } if candidate == executable && !expected_exits.is_success() => {
                        Some(expected_exits)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{executable} status run is composed"))
        };
        let narrow = find_exits("sysDivideByZero");
        assert_eq!(narrow.codes, [8, 136]);
        assert_eq!(narrow.aarch64_codes.as_deref(), Some(&[0][..]));
        assert_eq!(narrow.windows_codes.as_deref(), Some(&[127][..]));
        let wide = find_exits("sysDivideByZeroWide");
        assert_eq!(wide.codes, [8, 136]);
        assert_eq!(wide.aarch64_codes, None);
        assert_eq!(wide.windows_codes.as_deref(), Some(&[3][..]));
    }

    #[test]
    fn imports_explicit_bluesim_vcd_output_with_proven_optional_move() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| {
                generated.plan.id == "bsc.verilog/positivereset/simulation/simulation"
            })
            .expect("positive-reset simulation plan is generated");

        assert_eq!(
            plan.plan.status,
            PlanStatus::Complete,
            "{:#?}",
            plan.plan.diagnostics
        );
        assert!(plan.plan.diagnostics.is_empty());
        let operations = plan
            .plan
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        assert!(operations.iter().any(|operation| {
            matches!(
                &operation.action,
                Action::SimulationRun {
                    backend: PlanSimulationBackend::Bluesim,
                    args,
                    ..
                } if args == &["-V", "sysNoReset_sim.vcd"]
                    && operation
                        .artifacts
                        .outputs
                        .contains(&"sysNoReset_sim.vcd".to_owned())
                    && !operation.artifacts.outputs.contains(&"dump.vcd".to_owned())
            )
        }));
        assert!(operations.iter().any(|operation| {
            matches!(
                &operation.action,
                Action::VcdCheck { path, .. } if path == "sysNoReset_sim.vcd"
            )
        }));
        assert!(operations.iter().any(|operation| {
            matches!(
                &operation.action,
                Action::FsMove { source, destination }
                    if source == "dump.vcd" && destination == "sysNoReset_veri.vcd"
            )
        }));
        assert!(!operations.iter().any(|operation| {
            matches!(
                &operation.action,
                Action::FsMove { source, destination }
                    if source == "dump.vcd" && destination == "sysNoReset.vcd"
            )
        }));
    }

    #[test]
    fn imports_generated_verilog_dependencies_before_composing_links() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.verilog/parameters/parameters")
            .expect("parameters plan is generated");

        assert_eq!(plan.plan.status, PlanStatus::Complete);
        assert!(plan.plan.diagnostics.is_empty());
        let scenario = plan
            .plan
            .scenarios
            .iter()
            .find(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::BscLink { top, .. } if top == "tbParamSize"
                        )
                    })
            })
            .expect("tbParamSize Verilog workflow is composed");
        let operations = scenario
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let tail = operations
            .get(operations.len().saturating_sub(4)..)
            .expect("tbParamSize workflow has four terminal operations");
        assert_eq!(tail.len(), 4);
        assert!(matches!(
            &tail[0].action,
            Action::BscLink { top, objects, .. }
                if top == "tbParamSize"
                    && objects == &["tbParamSize.v", "mkParamSize_Sub.v"]
        ));
        assert!(matches!(
            &tail[1].action,
            Action::SimulationRun { stdout, .. } if stdout == "tbParamSize.out"
        ));
        assert!(matches!(
            &tail[2].action,
            Action::FsMove { source, destination }
                if source == "tbParamSize.out" && destination == "tbParamSize.v.out"
        ));
        assert!(matches!(
            &tail[3].action,
            Action::AssertGolden { actual, expected }
                if actual == "tbParamSize.v.out" && expected == "sysParamSize.out.expected"
        ));
    }

    #[test]
    fn imports_synthesize_compile_without_dependency_update_and_preserves_xfail() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.synthesize/synthesize")
            .expect("synthesize plan is generated");

        assert_eq!(plan.plan.status, PlanStatus::Complete);
        assert!(plan.plan.diagnostics.is_empty());
        assert_eq!(plan.plan.scenarios.len(), 1);
        let operations = &plan.plan.scenarios[0].stages[0].operations;
        assert!(matches!(
            operations.as_slice(),
            [OperationRecord {
                action: Action::BscCompile {
                    source,
                    mode: BscCompileMode::Synthesize,
                    module: Some(module),
                    args,
                    dependency_mode: DependencyMode::NoDeps,
                    expected_exit: ExpectedExit::Success,
                    stdout,
                    ..
                },
                expectation: OperationExpectation::Xfail { reason },
                ..
            }] if source == "FACT.bs"
                && module == "sysFACT"
                && args.is_empty()
                && stdout == "FACT.bs.bsc-vcomp-syn-out"
                && reason == "upstream unannotated known failure"
        ));
        assert_eq!(
            operations[0].artifacts.outputs,
            ["FACT.bs.bsc-vcomp-syn-out", "FACT.bo", "sysFACT.v"]
        );
    }

    #[test]
    fn imports_bsc2bsv_as_a_closed_internal_operation() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.bugs/bluespec_inc/b611/b611")
            .expect("b611 plan is generated");

        assert_eq!(plan.plan.status, PlanStatus::Complete);
        assert!(plan.plan.diagnostics.is_empty());
        assert_eq!(plan.plan.scenarios.len(), 1);
        assert_eq!(plan.plan.scenarios[0].fixtures, ["Bug611.bs"]);
        assert!(plan.plan.scenarios[0].requires.is_empty());
        let operations = &plan.plan.scenarios[0].stages[0].operations;
        assert!(matches!(
            operations.as_slice(),
            [OperationRecord {
                action: Action::Bsc2Bsv { source, stdout },
                requires,
                ..
            }] if source == "Bug611.bs"
                && stdout == "Bug611.bs.bsc2bsv-out"
                && requires == &[Requirement::InternalChecks]
        ));
    }

    #[test]
    fn imports_final_state_split_if_and_parse_pretty_plans() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = |id: &str| {
            generated
                .plans
                .iter()
                .find(|generated| generated.plan.id == id)
                .unwrap_or_else(|| panic!("{id} plan is generated"))
        };

        let negativeshift = plan("bsc.bugs/bluespec_inc/b530/negativeshift");
        assert_eq!(negativeshift.plan.status, PlanStatus::Complete);
        assert!(negativeshift.plan.diagnostics.is_empty());
        assert!(negativeshift.plan.scenarios.iter().any(|scenario| {
            scenario.stages.iter().any(|stage| {
                stage.operations.iter().any(|operation| {
                    matches!(
                        &operation.action,
                        Action::SimulationRun {
                            backend: PlanSimulationBackend::Bluesim,
                            executable,
                            args,
                            stdout,
                            ..
                        } if executable == "sysDesignReg"
                            && args == &["-m", "999", "-s"]
                            && stdout == "sysDesignReg.final-state"
                    )
                })
            })
        }));

        let split_if = plan("bsc.if/split/splitIf");
        assert_eq!(split_if.plan.status, PlanStatus::Complete);
        assert_eq!(split_if.plan.scenarios.len(), 68);
        assert!(split_if.plan.scenarios.iter().any(|scenario| {
            scenario.stages.iter().any(|stage| {
                stage.operations.iter().any(|operation| {
                    matches!(
                        &operation.action,
                        Action::AssertGoldenNormalized {
                            actual,
                            expected,
                            normalizations,
                        } if actual == "manyVariations.bs.splitIf.dump"
                            && expected == "manyVariations.bs.canon.dump.expected"
                            && normalizations == &[GoldenNormalization::SplitIfRules]
                    )
                })
            })
        }));

        let bh = plan("bsc.syntax/bh_parse_pretty/bh-parse-pretty");
        assert_eq!(bh.plan.status, PlanStatus::Complete);
        assert_eq!(bh.plan.scenarios.len(), 4);
        assert!(matches!(
            &bh.plan.scenarios[1].stages[0].operations[0],
            OperationRecord {
                action: Action::BscParsePretty {
                    source,
                    args,
                    pretty_output,
                },
                expectation: OperationExpectation::Xfail { reason },
                ..
            } if source == "DollarColonEqualsPrecedencePretty1.bs"
                && args.is_empty()
                && pretty_output == "DollarColonEqualsPrecedencePretty1.bs-pretty-out.bs"
                && reason == "upstream bug github#568"
        ));

        let bsv = plan("bsc.syntax/bsv05_parse_pretty/bsv05-parse-pretty");
        assert_eq!(bsv.plan.status, PlanStatus::Complete);
        assert_eq!(bsv.plan.scenarios.len(), 11);
        assert!(bsv.plan.scenarios.iter().all(|scenario| {
            scenario.requires == [Requirement::Frontend]
                && matches!(
                    scenario.stages[0].operations.as_slice(),
                    [OperationRecord {
                        action: Action::BscParsePretty { .. },
                        expectation: OperationExpectation::Required,
                        ..
                    }]
                )
        }));

        let compile = plan("bsc.compile/compile");
        assert_eq!(compile.plan.status, PlanStatus::Complete);
        let compile_episode = compile
            .plan
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "fixture-replacement-compile-Five")
            .expect("fixture replacement compile episode");
        let compile_actions = compile_episode.stages[0]
            .operations
            .iter()
            .map(|operation| &operation.action)
            .collect::<Vec<_>>();
        assert!(matches!(compile_actions[0], Action::FsCopy { .. }));
        assert!(matches!(
            compile_actions[3],
            Action::Delay { milliseconds: 1500 }
        ));
        assert!(matches!(
            compile_actions[4],
            Action::FsCopyReplace { source, destination }
                if source == "FiveB.bs" && destination == "Five.bs"
        ));
        assert!(matches!(
            compile_actions[7],
            Action::AssertDiagnosticCount { .. }
        ));

        let bluesim = plan("bsc.driver/bluesim/bluesim");
        assert_eq!(bluesim.plan.status, PlanStatus::Complete);
        assert_eq!(bluesim.plan.scenarios.len(), 1);
        let bluesim_actions = &bluesim.plan.scenarios[0].stages[0].operations;
        assert_eq!(
            bluesim_actions
                .iter()
                .filter(|operation| matches!(
                    operation.action,
                    Action::Delay { milliseconds: 2000 }
                ))
                .count(),
            3
        );
        assert_eq!(
            bluesim_actions
                .iter()
                .filter(|operation| matches!(&operation.action, Action::FsTouch { path } if path == "Sub1.bsv"))
                .count(),
            1
        );
        assert_eq!(
            bluesim_actions
                .iter()
                .filter(|operation| matches!(operation.action, Action::BscGenerate { .. }))
                .count(),
            3
        );
    }

    #[test]
    fn imports_static_fixture_vcdcheck_helpers_as_executable_operations() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.vcdcheck/vcdcheck")
            .expect("vcdcheck plan is generated");

        assert_eq!(plan.plan.status, PlanStatus::Complete);
        assert!(plan.plan.diagnostics.is_empty());
        let scenario = plan
            .plan
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "vcd-check-fixtures")
            .expect("static VCD checks form a standalone scenario");
        assert!(scenario.requires.is_empty());
        assert!(scenario.stages[0].operations.iter().all(|operation| {
            matches!(operation.action, Action::VcdCheck { .. })
                && operation.requires == [Requirement::InternalChecks]
        }));
    }

    #[test]
    fn imports_touch_before_matching_simulation_recompile() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.evaluator/opt/opt")
            .expect("opt plan is generated");

        assert_eq!(plan.plan.status, PlanStatus::Complete);
        assert!(plan
            .plan
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| {
                matches!(
                    &operation.action,
                    Action::FsTouch { path } if path == "ConcatOpt3.bsv"
                )
            }));
    }

    #[test]
    fn imports_all_contract_origins_and_preserves_ordered_sequence_assertions() {
        let generated = build_test_plans(project_root()).unwrap();
        assert_eq!(generated.plans.len(), 860);

        let b1894 = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.bugs/bluespec_inc/b1894/b1894")
            .unwrap();
        assert_eq!(b1894.plan.status, PlanStatus::Complete);
        let stages = &b1894.plan.scenarios[0].stages;
        assert_eq!(stages.len(), 3);
        assert!(matches!(
            stages[1].operations[0].action,
            Action::FsMove { .. }
        ));
        assert!(matches!(
            stages[1].operations[1].action,
            Action::AssertRegex { .. }
        ));
        assert!(matches!(
            stages[1].operations[2].action,
            Action::FsEnsureAbsent { .. }
        ));

        let plan = |id: &str| {
            &generated
                .plans
                .iter()
                .find(|generated| generated.plan.id == id)
                .unwrap()
                .plan
        };

        let ovl = plan("bsc.interra/OVL/assertAlways1/assertAlways1");
        assert_eq!(ovl.status, PlanStatus::Complete);
        assert!(ovl.diagnostics.is_empty());
        assert_eq!(ovl.fixture_dir, "testsuite/bsc.interra/OVL");
        assert!(ovl.fixtures.iter().any(|fixture| {
            fixture.path == "std_ovl/assert_always.vlib" && fixture.role == FixtureRole::Source
        }));
        assert!(ovl.fixtures.iter().any(|fixture| {
            fixture.path == "std_ovl/std_ovl_defines.h" && fixture.role == FixtureRole::Source
        }));
        assert!(ovl.fixtures.iter().any(|fixture| {
            fixture.path == "std_ovl/vlog95/assert_always_logic.v"
                && fixture.role == FixtureRole::Source
        }));
        assert!(matches!(
            ovl.scenarios[0].stages[1].operations.as_slice(),
            [
                OperationRecord { action: Action::BscCompile { source, .. }, .. },
                OperationRecord { action: Action::BscLink { backend: PlanSimulationBackend::Icarus, args, .. }, .. },
                OperationRecord { action: Action::SimulationRun { backend: PlanSimulationBackend::Icarus, .. }, .. },
                OperationRecord { action: Action::AssertGolden { .. }, .. },
            ] if source == "assertAlways1.bsv"
                && args.windows(2).any(|pair| pair == ["-vsearch", "std_ovl"])
                && args.windows(2).any(|pair| pair == ["-Xv", "std_ovl/assert_always.vlib"])
        ));

        let creg = plan("bsc.lib/CReg/CReg");
        let stateful_episodes = creg
            .scenarios
            .iter()
            .filter(|scenario| scenario.id.starts_with("stateful-simulation-"))
            .collect::<Vec<_>>();
        assert_eq!(stateful_episodes.len(), 5);
        assert!(!creg.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "import.uncomposed_action"
                && diagnostic.message == "uncomposed workflow action move requires importer support"
        }));
        let size_five = stateful_episodes
            .iter()
            .find(|scenario| scenario.id.ends_with("sysTestCReg5"))
            .unwrap();
        let operations = size_five
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let link = operations
            .iter()
            .position(|operation| {
                matches!(
                    operation.action,
                    Action::BscLink {
                        backend: PlanSimulationBackend::Icarus,
                        ref top,
                        ..
                    } if top == "sysTestCReg5"
                )
            })
            .unwrap();
        let vexe_move = operations
            .iter()
            .position(|operation| {
                matches!(
                    operation.action,
                    Action::FsMove {
                        ref source,
                        ref destination,
                    } if source == "sysTestCReg5.vexe"
                        && destination == "sysTestCReg5_NoInline.vexe"
                )
            })
            .unwrap();
        let inline_generation = operations
            .iter()
            .rposition(|operation| {
                matches!(
                    operation.action,
                    Action::BscGenerate {
                        ref source,
                        ref args,
                        ..
                    } if source == "TestCReg5.bsv"
                        && args.iter().any(|argument| argument == "-inline-creg")
                )
            })
            .unwrap();
        assert!(link < vexe_move && vexe_move < inline_generation);

        let amba_dmac = plan("bsc.bsv_examples/Amba_dmac/amba_dmac");
        assert_eq!(amba_dmac.status, PlanStatus::Complete);
        assert!(amba_dmac.diagnostics.is_empty());
        assert_eq!(amba_dmac.scenarios.len(), 1);
        let amba_stages = &amba_dmac.scenarios[0].stages;
        assert_eq!(amba_stages.len(), 3);
        assert!(matches!(
            amba_stages[2].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::BscLink {
                        mode: BscLinkMode::NoMain,
                        ..
                    },
                    ..
                },
                OperationRecord {
                    action: Action::SimulationRun {
                        backend: PlanSimulationBackend::Icarus,
                        ..
                    },
                    ..
                },
                OperationRecord {
                    action: Action::AssertGolden { .. },
                    expectation: OperationExpectation::Xfail { .. },
                    ..
                },
            ]
        ));

        let file_io = plan("bsc.evaluator/fileIO/fileIO");
        let file_paths = file_io
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-16-FilePaths")
            .expect("FilePaths compile scenario is imported");
        assert!(matches!(
            file_paths.stages[0].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::FsMkdir { path },
                    artifacts,
                    ..
                },
                OperationRecord {
                    action: Action::BscCompile { source, args, .. },
                    ..
                },
                OperationRecord {
                    action: Action::AssertExists { path: asserted },
                    ..
                },
            ] if path == "ffiles"
                && artifacts.directories == ["ffiles"]
                && source == "FilePaths.bsv"
                && args.as_slice() == ["-fdir", "ffiles"]
                && asserted == "ffiles/relative.log"
        ));
        let options = plan("bsc.options/options");
        let bdir = options
            .scenarios
            .iter()
            .find(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::BscCompile { source, args, .. }
                                if source == "DummyModule.bsv" && args.as_slice() == ["-bdir", "bfiles"]
                        )
                    })
            })
            .expect("-bdir compile scenario is imported");
        assert!(matches!(
            bdir.stages[0].operations.as_slice(),
            [
                OperationRecord { action: Action::FsCreateDirAll { path }, .. },
                OperationRecord {
                    action: Action::BscCompile { args, .. },
                    artifacts,
                    ..
                },
                OperationRecord {
                    action: Action::AssertExists { path: asserted },
                    ..
                },
            ] if path == "bfiles"
                && args.as_slice() == ["-bdir", "bfiles"]
                && artifacts.outputs.contains(&"bfiles/DummyModule.bo".to_owned())
                && !artifacts.outputs.contains(&"DummyModule.bo".to_owned())
                && asserted == "bfiles/DummyModule.bo"
        ));

        let vdir = options
            .scenarios
            .iter()
            .find(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::BscCompile { source, args, .. }
                                if source == "DummyModule.bsv" && args.as_slice() == ["-vdir", "vfiles"]
                        )
                    })
            })
            .expect("-vdir compile scenario is imported");
        assert!(matches!(
            vdir.stages[0].operations.as_slice(),
            [
                OperationRecord { action: Action::FsCreateDirAll { path }, .. },
                OperationRecord {
                    action: Action::BscCompile { args, .. },
                    artifacts,
                    ..
                },
                OperationRecord {
                    action: Action::AssertTextCount { .. },
                    ..
                },
                OperationRecord {
                    action: Action::AssertExists { path: asserted },
                    ..
                },
            ] if path == "vfiles"
                && args.as_slice() == ["-vdir", "vfiles"]
                && artifacts.outputs.contains(&"vfiles/mkDummyModule.v".to_owned())
                && !artifacts.outputs.contains(&"vfiles/sysDummyModule.v".to_owned())
                && !artifacts.outputs.contains(&"mkDummyModule.v".to_owned())
                && !artifacts.outputs.contains(&"vfiles/vfiles/mkDummyModule.v".to_owned())
                && asserted == "vfiles/mkDummyModule.v"
        ));
        assert_eq!(options.status, PlanStatus::Complete);
        assert!(options.diagnostics.is_empty());
        let preflights = options
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .filter(|operation| matches!(operation.action, Action::BscFlagPreflight { .. }))
            .collect::<Vec<_>>();
        assert_eq!(preflights.len(), 4);
        assert!(preflights.iter().all(|operation| {
            operation.artifacts.inputs.is_empty()
                && operation.artifacts.outputs.len() == 1
                && (matches!(
                    &operation.action,
                    Action::BscFlagPreflight {
                        mode: BscFlagPreflightMode::VerilogNoOptUndetermined,
                        input,
                        top: None,
                        unspecified_to: UndeterminedValue::X | UndeterminedValue::Z,
                        stdout,
                    } if matches!(input.as_str(), "NoOptUndet_UnspecToX.bsv" | "NoOptUndet_UnspecToZ.bsv")
                        && stdout == &format!("{input}.bsc-out")
                )
                    || matches!(
                        &operation.action,
                        Action::BscFlagPreflight {
                            mode: BscFlagPreflightMode::BluesimLink,
                            input,
                            top: Some(top),
                            unspecified_to: UndeterminedValue::X | UndeterminedValue::Z,
                            stdout,
                        } if input == "m.ba" && stdout == &format!("{top}.bsc-ccomp-out")
                    ))
        }));
        for non_fixture in [
            "NoOptUndet_UnspecToX.bsv",
            "NoOptUndet_UnspecToZ.bsv",
            "m.ba",
        ] {
            assert!(!options
                .fixtures
                .iter()
                .any(|fixture| fixture.path == non_fixture));
        }
        assert!(options.scenarios.iter().any(|scenario| {
            scenario
                .stages
                .iter()
                .flat_map(|stage| &stage.operations)
                .any(|operation| {
                    matches!(
                        &operation.action,
                        Action::BscCompile {
                            source,
                            absolute_import_paths,
                            ..
                        } if source == "IncludeTest.bsv"
                            && absolute_import_paths == &["incfiles"]
                    )
                })
        }));
        assert!(options.scenarios.iter().any(|scenario| {
            scenario
                .stages
                .iter()
                .flat_map(|stage| &stage.operations)
                .any(|operation| {
                    matches!(
                        &operation.action,
                        Action::BscOptions {
                            bsc_options_prepend: Some(prepend),
                            ..
                        } if prepend == "-print-flags -vsearch foo -steps 12345678"
                    )
                })
        }));

        assert!(!file_io.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("files_exist") || diagnostic.message.contains("mkdir")
        }));

        let verilog_e = plan("bsc.options/verilog-e/verilog-e");
        assert!(verilog_e.scenarios.iter().flat_map(|scenario| &scenario.stages).any(
            |stage| matches!(
                stage.operations.as_slice(),
                [
                    OperationRecord { action: Action::BscCompile { .. }, .. },
                    OperationRecord { action: Action::BscLink { backend: PlanSimulationBackend::Icarus, .. }, .. },
                    OperationRecord { action: Action::SimulationRun { backend: PlanSimulationBackend::Icarus, .. }, .. },
                    OperationRecord { action: Action::AssertGolden { actual, expected }, .. },
                    ..
                ] if actual == "sysHello.out" && expected == "sysHello.out.expected"
            )
        ));
        let verilog_e_operations = &verilog_e.scenarios[0].stages[0].operations;
        let verilog_e_links = verilog_e_operations
            .iter()
            .filter(|operation| matches!(operation.action, Action::BscLink { .. }))
            .collect::<Vec<_>>();
        assert_eq!(verilog_e_links.len(), 6);
        assert_eq!(
            verilog_e_links
                .iter()
                .map(|operation| match &operation.action {
                    Action::BscLink { simulator, .. } => *simulator,
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>(),
            [
                bsc_test_plan::IcarusSimulatorSelector::Default,
                bsc_test_plan::IcarusSimulatorSelector::BluespecDirInstalledBuilder,
                bsc_test_plan::IcarusSimulatorSelector::PosixEchoProbe,
                bsc_test_plan::IcarusSimulatorSelector::LiteralBogus,
                bsc_test_plan::IcarusSimulatorSelector::BluespecDirBogus,
                bsc_test_plan::IcarusSimulatorSelector::PosixEchoProbe,
            ]
        );
        assert_eq!(
            verilog_e_links[5].action,
            Action::BscLink {
                backend: PlanSimulationBackend::Icarus,
                mode: BscLinkMode::Standard,
                objects: vec!["sysHello.v".to_owned()],
                top: "sysHello".to_owned(),
                args: vec![
                    "-D".to_owned(),
                    "foo".to_owned(),
                    "-D".to_owned(),
                    "bar=128".to_owned(),
                ],
                expected_exit: ExpectedExit::Success,
                simulator: bsc_test_plan::IcarusSimulatorSelector::PosixEchoProbe,
                missing_objects: Vec::new(),
            }
        );
        for link in [
            &verilog_e_links[2],
            &verilog_e_links[3],
            &verilog_e_links[4],
            &verilog_e_links[5],
        ] {
            assert_eq!(link.artifacts.outputs, ["sysHello.bsc-vcomp-out"]);
        }
        for index in [7, 8, 9, 10, 15, 16, 17, 18] {
            assert!(verilog_e_operations[index]
                .requires
                .contains(&Requirement::NonWindows));
        }
        assert!(!verilog_e_operations[11]
            .requires
            .contains(&Requirement::NonWindows));
        assert!(matches!(
            &verilog_e_operations[9..11],
            [
                OperationRecord {
                    action: Action::RenderGolden { template, output, .. },
                    artifacts,
                    ..
                },
                OperationRecord {
                    action: Action::AssertGolden { actual, expected },
                    ..
                },
            ] if template == "bsc-sim-echo.expected"
                && output == "bsc-sim-echo.expected.post-m4"
                && artifacts.inputs == ["bsc-sim-echo.expected"]
                && artifacts.outputs == ["bsc-sim-echo.expected.post-m4"]
                && actual == "sysHello.sim-echo.bsc-vcomp-out"
                && expected == "bsc-sim-echo.expected.post-m4"
        ));

        let filter = plan("bsc.verilog/filter/filter");
        let filter_operations = &filter.scenarios[0].stages[0].operations;
        let filters = filter_operations
            .iter()
            .filter_map(|operation| match &operation.action {
                Action::VerilogFilter {
                    profiles,
                    expected_exit,
                    ..
                } => Some((profiles.as_slice(), *expected_exit, &operation.artifacts)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(filters.len(), 5);
        assert_eq!(
            filters[0].0,
            [bsc_test_plan::VerilogFilterProfile::RenameFire]
        );
        assert_eq!(
            filters[1].0,
            [
                bsc_test_plan::VerilogFilterProfile::RenameFire,
                bsc_test_plan::VerilogFilterProfile::RenameFire,
            ]
        );
        assert_eq!(
            filters[4].0,
            [
                bsc_test_plan::VerilogFilterProfile::RenameFire,
                bsc_test_plan::VerilogFilterProfile::MissingSed,
            ]
        );
        assert_eq!(filters[4].1, ExpectedExit::Failure);
        assert!(filters.iter().all(|(_, _, artifacts)| {
            artifacts.inputs.first().map(String::as_str) == Some("mkRenameTest.v")
                && artifacts.outputs == ["mkRenameTest.v"]
        }));
        assert!(filter_operations
            .iter()
            .all(|operation| match &operation.action {
                Action::BscCompile { args, .. } => !args.iter().any(|arg| arg == "-verilog-filter"),
                _ => true,
            }));

        let tasks = plan("bsc.verilog/tasks/tasks");
        let terminal_verilog_link = tasks
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "verilog-workflow-2-sysStopFinishV")
            .expect("terminal Verilog link workflow is imported");
        assert_eq!(terminal_verilog_link.resource, ResourceClass::Normal);
        assert_eq!(
            terminal_verilog_link.requires,
            vec![Requirement::Verilog, Requirement::Icarus]
        );
        assert!(matches!(
            terminal_verilog_link
                .stages
                .last()
                .unwrap()
                .operations
                .as_slice(),
            [OperationRecord {
                action: Action::BscLink {
                    backend: PlanSimulationBackend::Icarus,
                    ..
                },
                ..
            }]
        ));
        let logic_between_tasks = tasks
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "simulation-sysLogicBetweenTasks3-2")
            .expect("LogicBetweenTasks3 Verilog episode is imported");
        assert!(matches!(
            logic_between_tasks.stages[0].operations.as_slice(),
            [..,
                OperationRecord {
                    action: Action::FsCopy { source, destination },
                    artifacts,
                    ..
                },
                OperationRecord {
                    action: Action::AssertGolden { actual, .. },
                    ..
                }
            ] if source == "sysLogicBetweenTasks3.v.out"
                && destination == "sysLogicBetweenTasks3.v-bug.out"
                && artifacts.inputs == ["sysLogicBetweenTasks3.v.out"]
                && artifacts.outputs == ["sysLogicBetweenTasks3.v-bug.out"]
                && actual == "sysLogicBetweenTasks3.v-bug.out"
        ));
        let error_test = tasks
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-60-ErrorTest")
            .expect("ErrorTest workflow is imported");
        assert!(matches!(
            error_test.stages[0].operations.as_slice(),
            [
                OperationRecord { action: Action::BscCompile { .. }, .. },
                OperationRecord { action: Action::BscLink { backend: PlanSimulationBackend::Icarus, .. }, .. },
                OperationRecord {
                    action: Action::SimulationRun { expected_exits, .. },
                    ..
                },
                OperationRecord {
                    action: Action::FsMove { source, destination },
                    ..
                },
                OperationRecord {
                    action: Action::AssertGoldenNormalized { actual, normalizations, .. },
                    ..
                },
            ] if expected_exits.codes == [1]
                && source == "sysErrorTest.out"
                && destination == "sysErrorTest.v.out"
                && actual == "sysErrorTest.v.out"
                && normalizations == &[GoldenNormalization::SystemVerilogTaskDiagnostics]
        ));
        let task_transforms = tasks
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::TextNormalize {
                    source,
                    destination,
                    transform,
                } => Some((source.as_str(), destination.as_str(), *transform)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            task_transforms,
            [
                (
                    "sysModuleDisplay.v.out",
                    "sysModuleDisplay.sorted.v.out",
                    bsc_test_plan::TextNormalization::SortNumericField1ThenField2
                ),
                (
                    "sysModuleDisplay.sorted.v.out",
                    "sysModuleDisplay.trimmed.v.out",
                    bsc_test_plan::TextNormalization::VerilogTaskProjection
                ),
                (
                    "sysModuleDisplay.c.out",
                    "sysModuleDisplay.sorted.c.out",
                    bsc_test_plan::TextNormalization::SortNumericField1ThenField2
                ),
                (
                    "sysModuleDisplay.sorted.c.out",
                    "sysModuleDisplay.trimmed.c.out",
                    bsc_test_plan::TextNormalization::BluesimTaskProjection
                ),
            ]
        );

        let b264 = plan("bsc.bugs/bluespec_inc/b264/b264");
        assert_eq!(b264.status, PlanStatus::Complete);
        assert!(b264.scenarios[1].requires.contains(&Requirement::Verilog));
        assert!(b264.scenarios[1].requires.contains(&Requirement::Icarus));
        assert_eq!(b264.scenarios[1].resource, ResourceClass::Heavy);
        let b264_operations = &b264.scenarios[1].stages[0].operations;
        let kinds = b264_operations
            .iter()
            .map(|operation| match operation.action {
                Action::BscCompile { .. } => "compile",
                Action::AssertVerilog { .. } => "compare",
                Action::BscLink {
                    backend: PlanSimulationBackend::Icarus,
                    ..
                } => "link",
                _ => "other",
            })
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            ["compile", "compile", "compare", "compare", "link", "link", "link"]
        );

        let codegen = plan("bsc.codegen/codegen");
        assert_eq!(codegen.status, PlanStatus::Complete);
        assert!(codegen.scenarios.iter().any(|scenario| {
            matches!(
                scenario.stages[0].operations.as_slice(),
                [
                    OperationRecord {
                        action: Action::BscCompile { .. },
                        ..
                    },
                    OperationRecord {
                        action: Action::BscLink {
                            backend: PlanSimulationBackend::Icarus,
                            ..
                        },
                        ..
                    },
                    OperationRecord {
                        action: Action::SimulationRun {
                            backend: PlanSimulationBackend::Icarus,
                            ..
                        },
                        ..
                    },
                    OperationRecord {
                        action: Action::FsCopy { .. },
                        ..
                    },
                    OperationRecord {
                        action: Action::AssertGolden { .. },
                        ..
                    }
                ]
            )
        }));

        let frontend_failure = plan("bsc.bugs/bluespec_inc/b1040/b1040");
        assert_eq!(frontend_failure.status, PlanStatus::Complete);
        assert!(matches!(
            frontend_failure.scenarios[0].stages[0]
                .operations
                .as_slice(),
            [
                OperationRecord {
                    action: Action::BscCompile {
                        mode: BscCompileMode::Frontend,
                        expected_exit: ExpectedExit::Failure,
                        ..
                    },
                    ..
                },
                OperationRecord {
                    action: Action::AssertDiagnosticCount { .. },
                    ..
                }
            ]
        ));

        let b1243 = plan("bsc.bugs/bluespec_inc/b1243/b1243");
        assert_eq!(b1243.status, PlanStatus::Complete);
        assert_eq!(b1243.scenarios.len(), 1);
        assert_eq!(b1243.scenarios[0].stages.len(), 1);
        assert!(b1243.scenarios[0].requires.contains(&Requirement::Bluesim));
        assert!(matches!(
            b1243.scenarios[0].stages[0].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::BscGenerate { .. },
                    ..
                },
                OperationRecord {
                    action: Action::BscLink { .. },
                    ..
                }
            ]
        ));

        let array = plan("bsc.interra/bluesim/commandline_options/array/array");
        assert_eq!(array.status, PlanStatus::Complete);
        assert_eq!(array.scenarios.len(), 1);
        assert_eq!(array.scenarios[0].stages.len(), 3);
        assert!(matches!(
            &array.scenarios[0].stages[1].operations[0].action,
            Action::SimulationRun { args, .. } if args == &["-V", "dump.vcd"]
        ));
        assert!(matches!(
            array.scenarios[0].stages[1].operations[1].action,
            Action::FsCopy { .. }
        ));
        assert!(matches!(
            &array.scenarios[0].stages[2].operations[0].action,
            Action::SimulationRun { args, .. } if args == &["-V", "dump.vcd", "-m", "5"]
        ));
        assert!(matches!(
            array.scenarios[0].stages[2].operations[1].action,
            Action::FsCopy { .. }
        ));

        let debugging = plan("bsc.bluesim/debugging/debugging");
        assert_eq!(debugging.status, PlanStatus::Complete);
        assert_eq!(debugging.scenarios.len(), 3);
        assert!(debugging
            .fixtures
            .iter()
            .any(|fixture| { fixture.path == "GCD.bsv" && fixture.role == FixtureRole::Source }));

        let vector = plan("bsc.interra/libraries/Vector/Vector");
        assert_eq!(vector.status, PlanStatus::Complete);
        let all = vector
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "simulation-mkTestbench_All")
            .unwrap();
        let any = vector
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "simulation-mkTestbench_Any")
            .unwrap();
        assert_eq!(all.fixtures, ["All.bsv", "mkTestbench_All.out.expected"]);
        assert_eq!(any.fixtures, ["Any.bsv", "mkTestbench_Any.out.expected"]);
        assert!(!all.fixtures.iter().any(|fixture| fixture == "Any.bsv"));
        assert!(!any.fixtures.iter().any(|fixture| fixture == "All.bsv"));

        let hierarchical = plan(
            "bsc.interra/bluesim/interactive/traffic_light_controller_hierar/traffic_light_controller_hier",
        );
        assert_eq!(hierarchical.status, PlanStatus::Complete);
        assert!(hierarchical.fixtures.iter().any(|fixture| {
            fixture.path == "Design.bsv" && fixture.role == FixtureRole::Source
        }));

        let sync_ram = plan("bsc.interra/Library_latency/SyncRAM/SyncRAM");
        assert_eq!(sync_ram.status, PlanStatus::Complete);
        let run_operations = &sync_ram.scenarios[0].stages[1].operations;
        assert!(matches!(
            run_operations.as_slice(),
            [
                OperationRecord {
                    action: Action::SimulationRun { .. },
                    ..
                },
                OperationRecord {
                    action: Action::AssertGolden { .. },
                    ..
                }
            ]
        ));
        assert!(sync_ram.fixtures.iter().any(|fixture| {
            fixture.path == "mkTestbench_SPSRam.out.expected" && fixture.role == FixtureRole::Golden
        }));

        let b1489 = plan("bsc.bugs/bluespec_inc/b1489/b1489");
        assert_eq!(b1489.status, PlanStatus::Complete);
        assert!(matches!(
            b1489.scenarios[0].stages[1].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::SimulationRun { .. },
                    ..
                },
                OperationRecord {
                    action: Action::AssertTextCount { .. },
                    ..
                },
                OperationRecord {
                    action: Action::AssertTextCount { .. },
                    ..
                }
            ]
        ));

        let schedule = plan("bsc.bluesim/schedule/schedule");
        assert_eq!(schedule.status, PlanStatus::Complete);
        assert_eq!(schedule.scenarios.len(), 4);
        assert!(matches!(
            schedule.scenarios[0].stages[0]
                .operations
                .last()
                .unwrap()
                .action,
            Action::AssertRegex { .. }
        ));
        assert_eq!(schedule.scenarios[2].stages[0].operations.len(), 2);
        assert!(matches!(
            schedule.scenarios[3].stages[0]
                .operations
                .last()
                .unwrap()
                .action,
            Action::AssertTextCount { count: 0, .. }
        ));

        let eq3 = plan("bsc.misc/eq3/eq3");
        assert_eq!(eq3.status, PlanStatus::Complete);
        assert!(eq3
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(operation.action, Action::AssertTextCount { .. })));

        let b568 = plan("bsc.bugs/bluespec_inc/b568/b568");
        assert_eq!(b568.status, PlanStatus::Complete);
        assert_eq!(
            b568.scenarios
                .iter()
                .map(|scenario| {
                    scenario.stages[0]
                        .operations
                        .last()
                        .and_then(|operation| operation.action.asserted_path())
                        .unwrap()
                })
                .collect::<Vec<_>>(),
            ["mkDesign.v", "mkDesign_def.v", "mkDesign_full.v"]
        );

        let dft = plan("bsc.lib/PAClib/dft64/bsv/paclib_dft");
        assert!(dft.fixtures.iter().any(|fixture| {
            fixture.path == "FixedPointIO.c" && fixture.role == FixtureRole::Source
        }));
        let real_parameters = plan("bsc.verilog/parameters/real/real_param");
        assert!(real_parameters.fixtures.iter().any(|fixture| {
            fixture.path == "DisplayReal.v" && fixture.role == FixtureRole::Source
        }));

        let pop_count = plan("bsc.interra/libraries/PopCount/PopCount");
        let table = pop_count
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "simulation-mkTestbench_PopCountTable")
            .unwrap();
        assert!(table
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::AssertRegex { path, .. } if path == "mkTestbench_PopCountTable.v"
            )));
        assert!(!pop_count
            .scenarios
            .iter()
            .filter(|scenario| scenario.id != "simulation-mkTestbench_PopCountTable")
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::AssertRegex { path, .. } if path == "mkTestbench_PopCountTable.v"
            )));

        let always_enabled = plan("bsc.names/portRenaming/alwaysEnabled/alwaysEnabled");
        assert_eq!(always_enabled.status, PlanStatus::Complete);
        assert!(always_enabled.diagnostics.is_empty());
        let compile_operations =
            |source: &str| {
                &always_enabled
                    .scenarios
                    .iter()
                    .find(|scenario| {
                        scenario.stages.iter().any(|stage| {
                            stage.operations.iter().any(|operation| matches!(
                            &operation.action,
                            Action::BscCompile { source: candidate, .. } if candidate == source
                        ))
                        })
                    })
                    .unwrap()
                    .stages[0]
                    .operations
            };
        assert!(compile_operations("Test04.bsv")
            .iter()
            .any(|operation| matches!(
                &operation.action,
                Action::AssertTextAbsent { path, .. } if path == "mkDesign_04.v"
            )));
        assert!(!compile_operations("IFC1.bsv")
            .iter()
            .any(|operation| matches!(&operation.action, Action::AssertTextAbsent { .. })));

        for id in [
            "bsc.bugs/bluespec_inc/b1390/b1390",
            "bsc.syntax/bsv05/statename/statename",
            "bsc.lib/SShow/SShow",
        ] {
            let generated_artifacts = plan(id);
            assert_eq!(generated_artifacts.status, PlanStatus::Complete, "{id}");
            assert!(generated_artifacts.diagnostics.is_empty(), "{id}");
        }

        let config_reg = plan("bsc.interra/libraries/ConfigReg/ConfigReg");
        assert_eq!(config_reg.status, PlanStatus::Complete);
        assert!(config_reg.diagnostics.is_empty());
        assert_eq!(config_reg.scenarios.len(), 2);
        let standard = config_reg
            .scenarios
            .iter()
            .find(|scenario| scenario.id.ends_with("mkTestbench_MkConfigReg"))
            .unwrap();
        let operations = standard
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let no_inline_copy = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::FsCopy { source, destination }
                        if source == "mkTestbench_MkConfigReg.vexe"
                            && destination == "mkTestbench_MkConfigReg.vexe.no-inline-reg"
                )
            })
            .unwrap();
        let erase = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::FsRemove { path } if path == "MkConfigReg.bo"
                )
            })
            .unwrap();
        let inline_generation = operations
            .iter()
            .rposition(|operation| {
                matches!(
                    &operation.action,
                    Action::BscGenerate { source, args, .. }
                        if source == "MkConfigReg.bsv"
                            && args.iter().any(|argument| argument == "-inline-reg")
                )
            })
            .unwrap();
        assert!(no_inline_copy < erase && erase < inline_generation);

        let messages = plan("bsc.options/messages/messages");
        assert_eq!(
            messages
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "import.uncomposed_action")
                .count(),
            0
        );
        let suppress = messages
            .scenarios
            .iter()
            .find(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::BscCompile { source, .. } if source == "SuppressTest1.bsv"
                        )
                    })
            })
            .unwrap();
        assert!(matches!(
            suppress.stages[0].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::FsCopy { source, destination },
                    ..
                },
                OperationRecord {
                    action: Action::BscCompile { source: compile_source, .. },
                    ..
                },
                ..
            ] if source == "Warnings.bsv"
                && destination == "SuppressTest1.bsv"
                && compile_source == "SuppressTest1.bsv"
        ));
        assert!(messages.fixtures.iter().any(|fixture| {
            fixture.path == "Warnings.bsv" && fixture.role == FixtureRole::Source
        }));
        assert!(!messages
            .fixtures
            .iter()
            .any(|fixture| fixture.path == "SuppressTest1.bsv"));

        let fifo_sync = plan("bsc.interra/MCD_library/FIFOSync/SyncFIFO");
        assert_eq!(fifo_sync.status, PlanStatus::Complete);
        assert!(fifo_sync.diagnostics.is_empty());
        assert_eq!(fifo_sync.scenarios.len(), 6);

        for id in [
            "bsc.evaluator/errors/errors",
            "bsc.interra/messages/EStringNF/EStringNF",
            "bsc.interra/messages/ETooGeneral/ETooGeneral",
            "bsc.interra/messages/ETooManySteps/ETooManySteps",
            "bsc.interra/messages/EUnify/EUnify",
            "bsc.lib/FixedPoint/FixedPoint",
            "bsc.mcd/MultErrors/mult_errors_mcd",
            "bsc.misc/deprecate/deprecate",
            "bsc.typechecker/bound-type-vars/bound-type-vars",
        ] {
            let diagnostic_counts = plan(id);
            assert_eq!(diagnostic_counts.status, PlanStatus::Complete, "{id}");
            assert!(diagnostic_counts.diagnostics.is_empty(), "{id}");
            assert!(
                diagnostic_counts
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| matches!(
                        operation.action,
                        Action::AssertDiagnosticCount { .. }
                    )),
                "{id}"
            );
        }

        for id in [
            "bsc.mcd/Gating/portprop/portprop",
            "bsc.mcd/Reset/Reset",
            "bsc.syntax/bsv05/method-args/method-args",
        ] {
            let no_warnings = plan(id);
            assert_eq!(no_warnings.status, PlanStatus::Complete, "{id}");
            assert!(no_warnings.diagnostics.is_empty(), "{id}");
            assert!(
                no_warnings
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| matches!(
                        &operation.action,
                        Action::AssertDiagnosticCount {
                            kind: DiagnosticKind::Warning,
                            code: None,
                            count: 0,
                            ..
                        }
                    )),
                "{id}"
            );
        }

        let noinline = plan("bsc.verilog/noinline/noinline");
        assert_eq!(noinline.status, PlanStatus::Complete);
        assert!(noinline.diagnostics.is_empty());
        let mul_size = noinline
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-5-MulSize")
            .expect("MulSize compile scenario is imported");
        assert!(mul_size.requires.contains(&Requirement::Verilog));
        assert!(!mul_size.requires.contains(&Requirement::InternalChecks));
        let operations = &mul_size.stages[0].operations;
        let dump_index = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::DumpIntermediate { input, output, view }
                        if input == "MulSize.bo"
                            && output == "MulSize.bo.dumpbo-out"
                            && *view == bsc_test_plan::IntermediateDumpView::Bo
                )
            })
            .expect("compile helper's implicit dumpbo operation is materialized");
        assert_eq!(
            operations[dump_index].requires,
            [Requirement::InternalChecks]
        );
        assert!(matches!(
            &operations[dump_index + 1],
            OperationRecord {
                action: Action::AssertRegex { path, .. },
                requires,
                ..
            } if path == "MulSize.bo.dumpbo-out"
                && requires.as_slice() == [Requirement::InternalChecks]
        ));
        assert!(!operations.iter().any(|operation| {
            matches!(operation.action, Action::BscCompile { .. })
                && operation
                    .artifacts
                    .outputs
                    .contains(&"MulSize.bo.dumpbo-out".to_owned())
        }));

        let file_io = plan("bsc.evaluator/fileIO/fileIO");
        assert_eq!(file_io.status, PlanStatus::Complete);
        assert!(file_io.diagnostics.is_empty());
        for expected in ["sysBasicWrite.log", "sysBuffering.log", "sysEnvNames.log"] {
            assert!(file_io
                .scenarios
                .iter()
                .flat_map(|scenario| &scenario.stages)
                .flat_map(|stage| &stage.operations)
                .any(|operation| {
                    matches!(operation.action, Action::BscCompile { .. })
                        && operation.artifacts.outputs.contains(&expected.to_owned())
                }));
        }

        for (id, expected_cleanups) in [
            ("bsc.mcd/NullCrossing/nullcrossing", 3),
            ("bsc.typechecker/typeclasses/coherence/coherence", 1),
            ("bsc.verilog/v95/v95", 6),
        ] {
            let stateful_setup = plan(id);
            assert_eq!(stateful_setup.status, PlanStatus::Complete, "{id}");
            assert!(stateful_setup.diagnostics.is_empty(), "{id}");
            assert_eq!(
                stateful_setup
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .flat_map(|stage| &stage.operations)
                    .filter(|operation| matches!(operation.action, Action::FsEnsureAbsent { .. }))
                    .count(),
                expected_cleanups,
                "{id}"
            );
        }

        let v95 = plan("bsc.verilog/v95/v95");
        assert!(v95
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::BscLink {
                    backend: PlanSimulationBackend::Icarus,
                    objects,
                    ..
                } if objects.iter().any(|object| object == "Param.v")
            )));

        let real_parameters = plan("bsc.verilog/parameters/real/real_param");
        assert!(real_parameters
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::BscLink {
                    backend: PlanSimulationBackend::Icarus,
                    objects,
                    ..
                } if objects.iter().any(|object| {
                    link_object_path(PlanSimulationBackend::Icarus, object) == "DisplayReal.v"
                }) =>
                    Some(objects),
                _ => None,
            })
            .all(|objects| objects
                .iter()
                .filter(|object| {
                    link_object_path(PlanSimulationBackend::Icarus, object) == "DisplayReal.v"
                })
                .count()
                == 1));

        let undetermined = plan("bsc.codegen/undet/undet");
        assert_eq!(undetermined.status, PlanStatus::Complete);
        assert!(undetermined.diagnostics.is_empty());
        assert_eq!(undetermined.scenarios.len(), 8);
        assert!(undetermined.scenarios.iter().all(|scenario| {
            scenario.stages.len() == 1
                && scenario.stages[0].operations.len() == 3
                && matches!(
                    scenario.stages[0].operations[0].action,
                    Action::BscCompile { .. }
                )
                && matches!(
                    scenario.stages[0].operations[1].action,
                    Action::FsMove { .. }
                )
                && matches!(
                    scenario.stages[0].operations[2].action,
                    Action::AssertTextCount { count: 1, .. }
                )
        }));

        let aggressive_conditions =
            plan("bsc.evaluator/aggressive-conditions/aggressive-conditions");
        assert_eq!(aggressive_conditions.status, PlanStatus::Complete);
        assert!(aggressive_conditions.diagnostics.is_empty());
        let schedule_workflows = aggressive_conditions
            .scenarios
            .iter()
            .filter(|scenario| scenario.id.starts_with("bluesim-workflow-"))
            .collect::<Vec<_>>();
        assert_eq!(schedule_workflows.len(), 3);
        assert!(schedule_workflows.iter().all(|scenario| {
            matches!(
                &scenario.stages[0]
                    .operations
                    .last()
                    .expect("workflow schedule comparison")
                    .action,
                Action::AssertGolden { actual, expected }
                    if actual.ends_with(".sched") && expected == &format!("{actual}.expected")
            )
        }));

        let multiple_errors = plan("bsc.driver/mult_errors/mult_errors");
        assert_eq!(multiple_errors.status, PlanStatus::Complete);
        assert!(multiple_errors.diagnostics.is_empty());
        assert_eq!(multiple_errors.scenarios.len(), 4);
        let poisoned_chain = multiple_errors
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-chain-2-MultErrors1")
            .expect("poisoned dependency compile chain");
        assert_eq!(poisoned_chain.stages.len(), 2);
        assert!(matches!(
            &poisoned_chain.stages[0].operations[0].action,
            Action::BscCompile {
                source,
                expected_exit: ExpectedExit::Failure,
                ..
            } if source == "MultErrors1.bsv"
        ));
        assert!(matches!(
            &poisoned_chain.stages[1].operations[0].action,
            Action::BscCompile {
                source,
                expected_exit: ExpectedExit::Failure,
                ..
            } if source == "PoisonWarning.bsv"
        ));
        assert!(matches!(
            &poisoned_chain.stages[1]
                .operations
                .last()
                .expect("preserved failed output")
                .action,
            Action::FsCopy {
                source,
                destination,
            } if source == "PoisonWarning.bsv.bsc-out"
                && destination == "PoisonWarning.bsv.error.bsc-out"
        ));
        let successful_poison_warning = multiple_errors
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-4-PoisonWarning")
            .expect("later successful PoisonWarning compile remains a separate producer");
        assert!(matches!(
            &successful_poison_warning.stages[0].operations[0].action,
            Action::BscCompile {
                source,
                expected_exit: ExpectedExit::Success,
                ..
            } if source == "PoisonWarning.bsv"
        ));

        for id in [
            "bsc.evaluator/prims/type_of/type_of",
            "bsc.evaluator/primtcons/primtcons",
            "bsc.names/rtl_names/names",
            "bsc.names/state_names/state_names",
        ] {
            let static_global = plan(id);
            assert_eq!(static_global.status, PlanStatus::Complete, "{id}");
            assert!(static_global.diagnostics.is_empty(), "{id}");
        }

        for id in [
            "bsc.evaluator/reginit/reginit",
            "bsc.lib/BuildList/BuildList",
            "bsc.typechecker/class_defaults/class_defaults",
            "bsc.typechecker/higherrank/higherrank",
            "bsc.verilog/verilog",
        ] {
            let classic_backend = plan(id);
            assert_eq!(classic_backend.status, PlanStatus::Complete, "{id}");
            assert!(classic_backend.diagnostics.is_empty(), "{id}");
            assert!(
                classic_backend
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| matches!(
                        &operation.action,
                        Action::BscGenerate { source, .. } if source.ends_with(".bs")
                    )),
                "{id}"
            );
        }

        let class_defaults = plan("bsc.typechecker/class_defaults/class_defaults");
        let imported_default = class_defaults
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-2-ImportClassWithDefault")
            .unwrap();
        assert_eq!(
            imported_default.stages[0]
                .operations
                .iter()
                .filter_map(|operation| match &operation.action {
                    Action::BscCompile { source, .. } => Some(source.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            ["ClassWithDefault.bs", "ImportClassWithDefault.bsv"]
        );

        let classic_modules = plan("bsc.verilog/verilog");
        assert!(classic_modules
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::BscGenerate {
                    source,
                    mode: SimulationGenerationMode::Bluesim,
                    ..
                } if source == "Mips.bs"
            )));

        for id in [
            "bsc.bugs/bluespec_inc/b1589/b1589",
            "bsc.bugs/bluespec_inc/b262/b262",
            "bsc.bugs/bluespec_inc/b405/b405",
            "bsc.bugs/bluespec_inc/b508/b508",
            "bsc.evaluator/cache/def_cache",
            "bsc.evaluator/curry/curry",
            "bsc.interra/Urgency_Annotation/Negative_Testing/Negative_Testing",
            "bsc.interra/Urgency_Annotation/Semantics/Semantics",
            "bsc.mcd/Examples/Example",
            "bsc.names/hierarchy/hierarchy",
        ] {
            let standalone_generation = plan(id);
            assert_eq!(standalone_generation.status, PlanStatus::Complete, "{id}");
            assert!(standalone_generation.diagnostics.is_empty(), "{id}");
            assert!(
                standalone_generation
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| matches!(
                        operation.action,
                        Action::BscGenerate {
                            mode: SimulationGenerationMode::Bluesim,
                            ..
                        }
                    )),
                "{id}"
            );
        }

        let b262 = plan("bsc.bugs/bluespec_inc/b262/b262");
        assert!(b262
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::BscGenerate {
                    source,
                    module: Some(module),
                    args,
                    ..
                } if source == "Bug262.bs"
                    && module == "sysBug262"
                    && args == &["-opt-undetermined-vals"]
            )));

        for id in [
            "bsc.arrays/dynamic/arrays_dynamic",
            "bsc.interra/messages/EResources/EResources",
            "bsc.misc/lambda_calculus/lambda_calculus",
            "bsc.scheduler/avmeth/avmeth",
            "bsc.scheduler/sbr/sbr",
        ] {
            let generated_ids = plan(id);
            assert_eq!(generated_ids.status, PlanStatus::Complete, "{id}");
            assert!(generated_ids.diagnostics.is_empty(), "{id}");
            assert!(
                generated_ids
                    .scenarios
                    .iter()
                    .flat_map(|scenario| &scenario.stages)
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| matches!(
                        &operation.action,
                        Action::AssertGoldenNormalized { normalizations, .. }
                            if normalizations == &[GoldenNormalization::GeneratedIds]
                    )),
                "{id}"
            );
        }

        let avmeth = plan("bsc.scheduler/avmeth/avmeth");
        assert!(avmeth
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::AssertGoldenNormalized {
                    actual,
                    normalizations,
                    ..
                } if actual == "AVArgUse_C.bsv.bsc-sched-out"
                    && normalizations == &[GoldenNormalization::GeneratedIds]
            )));

        let signal_names = plan("bsc.names/signal_names/signal_names");
        assert_eq!(signal_names.status, PlanStatus::Complete);
        assert!(signal_names.diagnostics.is_empty());
        assert_eq!(
            signal_names
                .scenarios
                .iter()
                .flat_map(|scenario| &scenario.stages)
                .flat_map(|stage| &stage.operations)
                .filter(|operation| matches!(
                    &operation.action,
                    Action::AssertGoldenNormalized { normalizations, .. }
                        if normalizations
                            == &[
                                GoldenNormalization::GeneratedIds,
                                GoldenNormalization::VrWireIds,
                            ]
                ))
                .count(),
            7
        );

        let b925 = plan("bsc.bugs/bluespec_inc/b925/b925");
        assert_eq!(b925.status, PlanStatus::Complete);
        assert!(b925.diagnostics.is_empty());
        assert_eq!(
            b925.scenarios
                .iter()
                .flat_map(|scenario| &scenario.stages)
                .flat_map(|stage| &stage.operations)
                .filter(|operation| matches!(
                    &operation.action,
                    Action::AssertGoldenXfail { reason, .. }
                        if reason == "upstream bug FIFO_sim_issue"
                ))
                .count(),
            4
        );
        assert!(!b925
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::AssertGoldenXfail { actual, .. } if actual.ends_with(".v.out")
            )));

        let undet = plan("bsc.verilog/undet/undet");
        assert_eq!(undet.status, PlanStatus::Complete);
        assert!(undet.diagnostics.is_empty());
        let undet1 = undet
            .scenarios
            .iter()
            .find(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::BscGenerate { source, .. } if source == "Undet1.bs"
                        )
                    })
            })
            .expect("Undet1 simulation scenario");
        let undet1_operations = undet1
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        for actual in ["sysUndet1.c.out", "sysUndet1.v.out"] {
            assert!(undet1_operations.iter().any(|operation| matches!(
                &operation.action,
                Action::AssertGoldenXfail { actual: candidate, reason, .. }
                    if candidate == actual && reason == "upstream bug 138"
            )));
        }
        assert!(undet1_operations.iter().any(|operation| matches!(
            &operation.action,
            Action::SimulationRun {
                backend: PlanSimulationBackend::Bluesim,
                stdout,
                expected_exits,
                ..
            } if stdout == "sysUndet1.c.out" && expected_exits.is_success()
        )));
        assert!(undet1_operations.iter().any(|operation| matches!(
            &operation.action,
            Action::SimulationRun {
                backend: PlanSimulationBackend::Icarus,
                stdout,
                expected_exits,
                ..
            } if stdout == "sysUndet1.v.out" && expected_exits.is_success()
        )));
        assert!(!undet1_operations.iter().any(|operation| matches!(
            &operation.action,
            Action::AssertGolden { actual, expected }
                | Action::AssertGoldenNative { actual, expected }
                if actual == "sysUndet1.c-vcd.out" && expected == "sysUndet1.c.out"
        )));

        let error_recovery = plan("bsc.typechecker/error_recovery/error_recovery");
        assert_eq!(error_recovery.status, PlanStatus::Complete);
        assert!(error_recovery.diagnostics.is_empty());
        assert!(error_recovery
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::AssertVerilog { actual, expected }
                    if actual == "sysDefErrorRecovery.v"
                        && expected == "sysDefErrorRecovery.v.expected"
            )));

        let inferred_default_modules = plan("bsc.bugs/bluespec_inc/b1753/b1753");
        assert_eq!(inferred_default_modules.status, PlanStatus::Complete);
        assert!(inferred_default_modules.diagnostics.is_empty());
        assert_eq!(
            inferred_default_modules
                .scenarios
                .iter()
                .filter(|scenario| scenario.stages[0].operations.len() == 2)
                .count(),
            3
        );

        let pull = plan("bsc.interra/libraries/Pull/Pull");
        assert_eq!(pull.status, PlanStatus::Complete);
        assert!(pull.diagnostics.is_empty());
        assert!(pull.scenarios.iter().any(|scenario| matches!(
            scenario.stages[0].operations.as_slice(),
            [
                OperationRecord {
                    action: Action::BscCompile {
                        mode: BscCompileMode::BluesimObject,
                        expected_exit: ExpectedExit::Failure,
                        ..
                    },
                    ..
                },
                OperationRecord {
                    action: Action::AssertGolden { actual, .. },
                    ..
                }
            ] if actual == "Bind.bsv.bsc-ccomp-out"
        )));
        assert!(pull
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .flat_map(|stage| &stage.operations)
            .any(|operation| matches!(
                &operation.action,
                Action::BscGenerate {
                    mode: SimulationGenerationMode::SharedElaboration,
                    ..
                }
            )));

        let showrules = plan("bsc.showrules/showrules");
        assert_eq!(showrules.status, PlanStatus::Complete);
        assert!(showrules.diagnostics.is_empty());
        assert_eq!(showrules.scenarios.len(), 12);
    }

    #[test]
    fn maps_compile_bug_helpers_to_xfail_exit_contracts() {
        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let contract = |helper: &str, arguments: &[&str]| CompileContract {
            source: arguments[0].to_owned(),
            working_directory: None,
            helper: helper.to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        };
        let cases = [
            (
                "compile_pass_bug",
                vec!["Demo.bsv", "B-frontend-pass"],
                BscCompileMode::Frontend,
                None,
                Vec::<String>::new(),
                DependencyMode::Update,
                ExpectedExit::Success,
            ),
            (
                "compile_fail_bug",
                vec!["Demo.bsv", "B-frontend-fail", "-p +:lib", "1"],
                BscCompileMode::Frontend,
                None,
                vec!["-p".to_owned(), "+:lib".to_owned()],
                DependencyMode::NoDeps,
                ExpectedExit::Failure,
            ),
            (
                "compile_verilog_pass_bug",
                vec!["Demo.bsv", "mkDemo", "B-verilog-pass", "-keep-fires"],
                BscCompileMode::Verilog,
                Some("mkDemo"),
                vec!["-keep-fires".to_owned()],
                DependencyMode::Update,
                ExpectedExit::Success,
            ),
            (
                "compile_verilog_fail_bug",
                vec!["Demo.bsv", "mkDemo", "B-verilog-fail"],
                BscCompileMode::Verilog,
                Some("mkDemo"),
                Vec::new(),
                DependencyMode::Update,
                ExpectedExit::Failure,
            ),
            (
                "compile_verilog_schedule_pass_bug",
                vec!["Demo.bsv", "mkDemo", "B-schedule-pass"],
                BscCompileMode::VerilogSchedule,
                Some("mkDemo"),
                Vec::new(),
                DependencyMode::Update,
                ExpectedExit::Success,
            ),
            (
                "compile_verilog_schedule_fail_bug",
                vec!["Demo.bsv", "mkDemo", "B-schedule-fail"],
                BscCompileMode::VerilogSchedule,
                Some("mkDemo"),
                Vec::new(),
                DependencyMode::Update,
                ExpectedExit::Failure,
            ),
            (
                "compile_object_pass_bug",
                vec!["Demo.bsv", "mkDemo", "B-object-pass", "-p +:lib"],
                BscCompileMode::BluesimObject,
                Some("mkDemo"),
                vec!["-p".to_owned(), "+:lib".to_owned()],
                DependencyMode::Update,
                ExpectedExit::Success,
            ),
        ];

        for (helper, arguments, mode, module, args, dependency_mode, expected_exit) in cases {
            let contract = contract(helper, &arguments);
            let shape = compile_shape(&contract).unwrap();
            assert_eq!(shape.mode, mode, "{helper}");
            assert_eq!(shape.module.as_deref(), module, "{helper}");
            assert_eq!(shape.args, args, "{helper}");
            assert_eq!(shape.dependency_mode, dependency_mode, "{helper}");
            assert_eq!(shape.expected_exit, expected_exit, "{helper}");
            assert_eq!(
                shape.expectation,
                OperationExpectation::Xfail {
                    reason: format!(
                        "upstream bug {}",
                        if helper.starts_with("compile_verilog")
                            || helper == "compile_object_pass_bug"
                        {
                            arguments[2]
                        } else {
                            arguments[1]
                        }
                    ),
                },
                "{helper}"
            );

            let consumed_actions = BTreeSet::new();
            let imported = compile_scenario(
                0,
                &contract,
                false,
                false,
                None,
                &[],
                &consumed_actions,
                None,
                &[],
                &[],
                None,
                None,
                &BTreeSet::new(),
                &[],
                Path::new(""),
            )
            .unwrap();
            assert_eq!(
                imported.scenario.stages[0].operations[0].expectation, shape.expectation,
                "{helper}"
            );
        }
    }

    #[test]
    fn compile_bug_helpers_allow_empty_optional_annotations_and_validate_arity() {
        let contract = |helper: &str, arguments: &[&str]| CompileContract {
            source: arguments.first().copied().unwrap_or("Demo.bsv").to_owned(),
            working_directory: None,
            helper: helper.to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };

        for (helper, arguments, expectation) in [
            (
                "compile_pass_bug",
                vec!["Demo.bsv"],
                OperationExpectation::Xfail {
                    reason: "upstream unannotated known failure".to_owned(),
                },
            ),
            (
                "compile_fail_bug",
                vec!["Demo.bsv", "  "],
                OperationExpectation::Xfail {
                    reason: "upstream unannotated known failure".to_owned(),
                },
            ),
            (
                "compile_verilog_pass_bug",
                vec!["Demo.bsv", "mkDemo"],
                OperationExpectation::Xfail {
                    reason: "upstream unannotated known failure".to_owned(),
                },
            ),
            (
                "compile_verilog_schedule_fail_bug",
                vec!["Demo.bsv", "mkDemo", ""],
                OperationExpectation::Required,
            ),
            (
                "compile_object_pass_bug",
                vec!["Demo.bsv", "mkDemo"],
                OperationExpectation::Required,
            ),
        ] {
            assert_eq!(
                compile_shape(&contract(helper, &arguments))
                    .unwrap()
                    .expectation,
                expectation,
                "{helper}"
            );
        }

        let error = compile_shape(&contract(
            "compile_verilog_fail_bug",
            &["Demo.bsv", "mkDemo", "B123", "", "extra"],
        ))
        .unwrap_err();
        assert!(
            error.contains("requires 1 to 4 static arguments"),
            "{error}"
        );

        let error = compile_shape(&contract(
            "compile_object_pass_bug",
            &["Demo.bsv", "mkDemo", "B123", "", "extra"],
        ))
        .unwrap_err();
        assert!(
            error.contains("requires 1 to 4 static arguments"),
            "{error}"
        );
    }

    #[test]
    fn maps_bug_diagnostic_helpers_to_independent_operation_expectations() {
        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let contract = |helper: &str, arguments: &[&str]| CompileContract {
            source: arguments[0].to_owned(),
            working_directory: None,
            helper: helper.to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        };

        for (helper, arguments, mode, module, args, reason) in [
            (
                "compile_pass_bug_error",
                vec!["Demo.bsv", "P0017", "1391", "2", "-continue-after-errors"],
                BscCompileMode::Frontend,
                None,
                vec!["-continue-after-errors".to_owned()],
                "upstream bug 1391",
            ),
            (
                "compile_verilog_pass_bug_error",
                vec!["Demo.bsv", "G0028", "mkDemo", "598", "-keep-fires", "2"],
                BscCompileMode::Verilog,
                Some("mkDemo"),
                vec!["-keep-fires".to_owned()],
                "upstream bug 598",
            ),
            (
                "compile_fail_error_bug",
                vec!["Demo.bsv", "P0017", "1392", "2", "-continue-after-errors"],
                BscCompileMode::Frontend,
                None,
                vec!["-continue-after-errors".to_owned()],
                "upstream bug 1392",
            ),
            (
                "compile_verilog_fail_error_bug",
                vec!["Demo.bsv", "G0028", "598", "2", "mkDemo", "-keep-fires"],
                BscCompileMode::Verilog,
                Some("mkDemo"),
                vec!["-keep-fires".to_owned()],
                "upstream bug 598",
            ),
        ] {
            let shape = compile_shape(&contract(helper, &arguments)).unwrap();
            assert_eq!(shape.mode, mode, "{helper}");
            assert_eq!(shape.module.as_deref(), module, "{helper}");
            assert_eq!(shape.args, args, "{helper}");
            let diagnostic_bug = helper.ends_with("fail_error_bug");
            assert_eq!(
                shape.expected_exit,
                if diagnostic_bug {
                    ExpectedExit::Failure
                } else {
                    ExpectedExit::Success
                },
                "{helper}"
            );
            assert_eq!(
                shape.expectation,
                if diagnostic_bug {
                    OperationExpectation::Required
                } else {
                    OperationExpectation::Xfail {
                        reason: reason.to_owned(),
                    }
                },
                "{helper}"
            );
            let diagnostic_expectation = if diagnostic_bug {
                OperationExpectation::Xfail {
                    reason: reason.to_owned(),
                }
            } else {
                OperationExpectation::Required
            };
            assert!(matches!(
                shape.diagnostics.as_slice(),
                [CompileDiagnostic {
                    action: Action::AssertDiagnosticCount {
                        kind: DiagnosticKind::Error,
                        count: 2,
                        ..
                    },
                    expectation,
                }] if expectation == &diagnostic_expectation
            ));
        }

        let warning = compile_shape(&contract(
            "compile_verilog_pass_warning_bug",
            &["Demo.bsv", "G0036", "1082", "0", "mkDemo", "-keep-fires"],
        ))
        .unwrap();
        assert_eq!(warning.expectation, OperationExpectation::Required);
        assert!(matches!(
            warning.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Warning,
                    code: Some(code),
                    count: 0,
                    ..
                },
                expectation: OperationExpectation::Xfail { reason },
            }] if code == "G0036" && reason == "upstream bug 1082"
        ));

        let no_warning = compile_shape(&contract(
            "compile_verilog_pass_no_warning_bug",
            &["Demo.bsv", "G0010", "1268", "1", "mkDemo", "-keep-fires"],
        ))
        .unwrap();
        assert_eq!(no_warning.expectation, OperationExpectation::Required);
        assert!(matches!(
            no_warning.diagnostics.as_slice(),
            [
                CompileDiagnostic {
                    action: Action::AssertDiagnosticCount {
                        kind: DiagnosticKind::Warning,
                        code: None,
                        count: 0,
                        ..
                    },
                    expectation: OperationExpectation::Xfail { reason },
                },
                CompileDiagnostic {
                    action: Action::AssertDiagnosticCount {
                        kind: DiagnosticKind::Warning,
                        code: Some(code),
                        count: 1,
                        ..
                    },
                    expectation: OperationExpectation::Required,
                }
            ] if reason == "upstream bug 1268" && code == "G0010"
        ));

        for (helper, arguments) in [
            ("compile_pass_bug_error", vec!["Demo.bsv", "P0017"]),
            (
                "compile_verilog_pass_bug_error",
                vec!["Demo.bsv", "G0028", "mkDemo"],
            ),
            (
                "compile_verilog_pass_warning_bug",
                vec!["Demo.bsv", "G0036"],
            ),
            (
                "compile_verilog_pass_no_warning_bug",
                vec!["Demo.bsv", "G0010", ""],
            ),
            ("compile_fail_error_bug", vec!["Demo.bsv", "P0017"]),
            (
                "compile_verilog_fail_error_bug",
                vec!["Demo.bsv", "G0028", ""],
            ),
        ] {
            let shape = compile_shape(&contract(helper, &arguments)).unwrap();
            assert_eq!(
                shape.expectation,
                OperationExpectation::Required,
                "{helper}"
            );
            assert!(
                shape
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.expectation == OperationExpectation::Required),
                "{helper}"
            );
        }
    }

    #[test]
    fn maps_backend_compile_helper_to_frontend_with_explicit_verilog_argument() {
        let contract = |arguments: &[&str]| CompileContract {
            source: arguments[0].to_owned(),
            working_directory: None,
            helper: "compile_backend_pass".to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };

        let default = compile_shape(&contract(&["Demo.bsv"])).unwrap();
        assert_eq!(default.mode, BscCompileMode::Frontend);
        assert_eq!(default.args, ["-verilog"]);
        assert_eq!(default.dependency_mode, DependencyMode::Update);
        assert_eq!(default.expectation, OperationExpectation::Required);

        let explicit = compile_shape(&contract(&[
            "Demo.bsv",
            "-show-range-conflict -p +:lib",
            "1",
        ]))
        .unwrap();
        assert_eq!(
            explicit.args,
            ["-verilog", "-show-range-conflict", "-p", "+:lib"]
        );
        assert_eq!(explicit.dependency_mode, DependencyMode::NoDeps);

        assert!(compile_shape(&contract(&["Demo.bsv", "", "0", "extra"]))
            .unwrap_err()
            .contains("requires 1 to 3 static arguments"));
    }

    #[test]
    fn maps_conditional_no_internal_error_compile_contract() {
        let contract = CompileContract {
            source: "Demo.bsv".to_owned(),
            working_directory: None,
            helper: "compile_verilog_fail_no_internal_error".to_owned(),
            arguments: vec!["Demo.bsv".to_owned()],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        let shape = compile_shape(&contract).unwrap();
        assert_eq!(shape.mode, BscCompileMode::Verilog);
        assert_eq!(shape.module, None);
        assert_eq!(shape.expected_exit, ExpectedExit::Failure);
        assert_eq!(
            shape.unexpected_success_forbidden_regex.as_deref(),
            Some("Internal.*Error")
        );
        assert!(shape.diagnostics.is_empty());
        assert_eq!(shape.expectation, OperationExpectation::Required);

        let consumed_actions = BTreeSet::new();
        let imported = compile_scenario(
            0,
            &contract,
            false,
            false,
            None,
            &[],
            &consumed_actions,
            None,
            &[],
            &[],
            None,
            None,
            &BTreeSet::new(),
            &[],
            Path::new(""),
        )
        .unwrap();
        assert!(matches!(
            &imported.scenario.stages[0].operations[0].action,
            Action::BscCompile {
                mode: BscCompileMode::Verilog,
                expected_exit: ExpectedExit::Failure,
                unexpected_success_forbidden_regex: Some(pattern),
                ..
            } if pattern == "Internal.*Error"
        ));

        let mut extra = contract;
        extra.arguments.push("extra".to_owned());
        assert!(compile_shape(&extra)
            .unwrap_err()
            .contains("requires 1 to 1 static arguments"));
    }

    #[test]
    fn later_compile_replaces_an_earlier_output_binding() {
        let span = |line| ManifestSourceSpan {
            start_byte: line,
            end_byte: line + 1,
            start_line: line + 1,
            start_column: 1,
            end_line: line + 1,
            end_column: 2,
        };
        let compile = |line, options: &str| {
            Contract::Compile(CompileContract {
                source: "Demo.bsv".to_owned(),
                working_directory: None,
                helper: "compile_pass_warning".to_owned(),
                arguments: vec![
                    "Demo.bsv".to_owned(),
                    "T0127".to_owned(),
                    "1".to_owned(),
                    options.to_owned(),
                ],
                guard: Guard::Always,
                span: span(line),
                expansion: Vec::new(),
            })
        };
        let script = ScriptManifest {
            origin: "testsuite/sample.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: vec![compile(0, ""), compile(2, "-incoherent-instance-matches")],
            assertions: Vec::new(),
            comparisons: vec![ComparisonContract {
                helper: "compare_file".to_owned(),
                arguments: vec!["Demo.bsv.bsc-out".to_owned()],
                guard: Guard::Always,
                span: span(4),
                expansion: Vec::new(),
            }],
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            workflow_actions: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            unsupported: Vec::new(),
        };

        let bindings = check_bindings(&script, Path::new(""));
        assert_eq!(
            bindings.get(&ProducerKey::Compile(1)),
            Some(&vec![BoundCheck::Comparison(0)])
        );
        assert!(bindings.get(&ProducerKey::Compile(0)).is_none());
    }

    #[test]
    fn declares_static_preprocessor_dump_outputs() {
        let shape = compile_shape(&CompileContract {
            source: "Demo.bsv".to_owned(),
            working_directory: None,
            helper: "compile_pass".to_owned(),
            arguments: vec!["Demo.bsv".to_owned(), "-dvpp=Demo.vpp-out".to_owned()],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        })
        .unwrap();

        assert!(shape.artifact_paths("Demo.bsv").contains("Demo.vpp-out"));
    }

    #[test]
    fn maps_warning_and_no_warning_compile_helpers() {
        let contract = |helper: &str, arguments: &[&str]| CompileContract {
            source: arguments[0].to_owned(),
            working_directory: None,
            helper: helper.to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };

        let warning = compile_shape(&contract(
            "compile_pass_warning",
            &["Demo.bsv", "T0127", "2", "-keep-fires"],
        ))
        .unwrap();
        assert_eq!(warning.mode, BscCompileMode::Frontend);
        assert_eq!(warning.args, ["-keep-fires"]);
        assert_eq!(warning.expected_exit, ExpectedExit::Success);
        assert!(matches!(
            warning.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Warning,
                    code: Some(code),
                    count: 2,
                    ..
                },
                expectation: OperationExpectation::Required,
            }] if code == "T0127"
        ));

        let no_warning =
            compile_shape(&contract("compile_pass_no_warning", &["Demo.bsv", "", "1"])).unwrap();
        assert_eq!(no_warning.mode, BscCompileMode::Frontend);
        assert_eq!(no_warning.dependency_mode, DependencyMode::NoDeps);
        assert!(matches!(
            no_warning.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Warning,
                    code: None,
                    count: 0,
                    ..
                },
                expectation: OperationExpectation::Required,
            }]
        ));

        let worker = compile_shape(&contract(
            "bsc_compile_verilog",
            &[
                "Worker.bsv",
                "mkWorker",
                "-dATSexpand=%m.atsexpand -KILLATSexpand",
            ],
        ))
        .unwrap();
        assert_eq!(worker.mode, BscCompileMode::Verilog);
        assert_eq!(worker.module.as_deref(), Some("mkWorker"));
        assert_eq!(worker.args, ["-dATSexpand=%m.atsexpand", "-KILLATSexpand"]);
        assert_eq!(worker.expected_exit, ExpectedExit::Success);
        assert_eq!(worker.expectation, OperationExpectation::Required);
        assert_eq!(worker.stdout, "Worker.bsv.bsc-vcomp-out");

        let verilog_no_warning = compile_shape(&contract(
            "compile_verilog_pass_no_warning",
            &["Demo.bsv", "mkDemo", "-aggressive-conditions"],
        ))
        .unwrap();
        assert_eq!(verilog_no_warning.mode, BscCompileMode::Verilog);
        assert_eq!(verilog_no_warning.module.as_deref(), Some("mkDemo"));
        assert_eq!(verilog_no_warning.args, ["-aggressive-conditions"]);
        assert!(matches!(
            verilog_no_warning.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Warning,
                    code: None,
                    count: 0,
                    ..
                },
                expectation: OperationExpectation::Required,
            }]
        ));

        let object_warning = compile_shape(&contract(
            "compile_object_pass_warning",
            &["Demo.bsv", "G0023", "2", "mkDemo", "-keep-fires"],
        ))
        .unwrap();
        assert_eq!(object_warning.mode, BscCompileMode::BluesimObject);
        assert_eq!(object_warning.module.as_deref(), Some("mkDemo"));
        assert_eq!(object_warning.args, ["-keep-fires"]);
        assert_eq!(object_warning.expected_exit, ExpectedExit::Success);
        assert_eq!(object_warning.stdout, "Demo.bsv.bsc-ccomp-out");
        assert!(matches!(
            object_warning.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Warning,
                    code: Some(code),
                    count: 2,
                    ..
                },
                expectation: OperationExpectation::Required,
            }] if code == "G0023"
        ));

        let object_warning_defaults = compile_shape(&contract(
            "compile_object_pass_warning",
            &["Demo.bsv", "G0023"],
        ))
        .unwrap();
        assert_eq!(object_warning_defaults.module, None);
        assert!(object_warning_defaults.args.is_empty());
        assert!(matches!(
            object_warning_defaults.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount { count: 1, .. },
                ..
            }]
        ));

        let object_failure = compile_shape(&contract(
            "compile_object_fail_error",
            &["Demo.bsv", "G0028", "2", "mkDemo", "-continue-after-errors"],
        ))
        .unwrap();
        assert_eq!(object_failure.mode, BscCompileMode::BluesimObject);
        assert_eq!(object_failure.module.as_deref(), Some("mkDemo"));
        assert_eq!(object_failure.args, ["-continue-after-errors"]);
        assert_eq!(object_failure.expected_exit, ExpectedExit::Failure);
        assert_eq!(object_failure.stdout, "Demo.bsv.bsc-ccomp-out");
        assert!(matches!(
            object_failure.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Error,
                    code: Some(code),
                    count: 2,
                    ..
                },
                expectation: OperationExpectation::Required,
            }] if code == "G0028"
        ));

        let schedule_failure = compile_shape(&contract(
            "compile_verilog_schedule_fail",
            &["Demo.bsv", "mkDemo", "-aggressive-conditions"],
        ))
        .unwrap();
        assert_eq!(schedule_failure.mode, BscCompileMode::VerilogSchedule);
        assert_eq!(schedule_failure.module.as_deref(), Some("mkDemo"));
        assert_eq!(schedule_failure.args, ["-aggressive-conditions"]);
        assert_eq!(schedule_failure.expected_exit, ExpectedExit::Failure);
        assert_eq!(schedule_failure.stdout, "Demo.bsv.bsc-sched-out");

        assert!(compile_shape(&contract(
            "bsc_compile_verilog",
            &["Demo.bsv", "mkDemo", "", "extra"],
        ))
        .unwrap_err()
        .contains("requires 1 to 3 static arguments"));

        assert!(compile_shape(&contract(
            "compile_object_pass_warning",
            &["Demo.bsv", "G0023", "1", "mkDemo", "", "extra"],
        ))
        .unwrap_err()
        .contains("requires 2 to 5 static arguments"));
    }

    #[test]
    fn simulation_artifact_paths_include_named_verilog_and_bluesim_outputs() {
        let contract = SimulationContract {
            source: "Demo.bsv".to_owned(),
            helper: "test_c_veri_bsv".to_owned(),
            arguments: vec!["Demo".to_owned()],
            backend: SimulationBackend::Icarus,
            generation: crate::model::GenerationStrategy::Shared,
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        let paths = simulation_artifact_paths(&contract);
        assert!(paths.contains("sysDemo.v"));
        assert!(paths.contains("sysDemo.cxx"));
    }

    #[test]
    fn represents_not_verilog_as_the_frontend_profile() {
        let mut requirements = BTreeSet::new();
        collect_requirements(
            &Guard::Not {
                guard: Box::new(Guard::Capability {
                    capability: Capability::Verilog,
                }),
            },
            &mut requirements,
        )
        .unwrap();
        assert_eq!(requirements, [Requirement::Frontend].into());
        assert!(collect_requirements(
            &Guard::Not {
                guard: Box::new(Guard::Capability {
                    capability: Capability::Bluesim,
                }),
            },
            &mut requirements,
        )
        .unwrap_err()
        .contains("not representable"));
    }

    #[test]
    fn maps_closed_line_directive_filter_to_a_declarative_normalization() {
        let comparison = ComparisonContract {
            helper: "compare_file_filtered".to_owned(),
            arguments: vec![
                "Demo.out".to_owned(),
                String::new(),
                String::new(),
                "s+\\`line\\(.\\*\\)+\\`line\\(POS\\)+g".to_owned(),
            ],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };

        let operation = map_comparison(&comparison).unwrap();
        assert!(matches!(
            operation.action,
            Action::AssertGoldenNormalized { normalizations, .. }
                if normalizations == [GoldenNormalization::LineDirectivePositions]
        ));
    }

    #[test]
    fn maps_closed_workspace_root_filter_to_a_declarative_normalization() {
        let comparison = ComparisonContract {
            helper: "compare_file_filtered".to_owned(),
            arguments: vec![
                "Demo.out".to_owned(),
                String::new(),
                "s+HERE+HERE+g".to_owned(),
            ],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };

        let operation = map_comparison(&comparison).unwrap();
        assert!(matches!(
            operation.action,
            Action::AssertGoldenNormalized { actual, expected, normalizations }
                if actual == "Demo.out"
                    && expected == "Demo.out.expected"
                    && normalizations == [GoldenNormalization::WorkspaceRoot]
        ));
    }

    #[test]
    fn maps_compare_file_list_to_any_golden_assertion() {
        let comparison = ComparisonContract {
            helper: "compare_file_list".to_owned(),
            arguments: vec![
                "Demo.out".to_owned(),
                "Demo.out.0.expected {Demo output 1.expected}".to_owned(),
                "status label".to_owned(),
            ],
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };
        let operation = map_comparison(&comparison).unwrap();
        assert!(matches!(
            operation.action,
            Action::AssertGoldenAny { actual, expected }
                if actual == "Demo.out"
                    && expected == ["Demo.out.0.expected", "Demo output 1.expected"]
        ));
        assert!(map_comparison(&ComparisonContract {
            arguments: vec!["Demo.out".to_owned(), String::new()],
            ..comparison
        })
        .unwrap_err()
        .contains("must not be empty"));
    }

    #[test]
    fn maps_compile_failure_with_required_error_and_warning_counts() {
        let contract = |arguments: &[&str]| CompileContract {
            source: arguments[0].to_owned(),
            working_directory: None,
            helper: "compile_fail_error_warnings".to_owned(),
            arguments: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            guard: Guard::Always,
            span: ManifestSourceSpan {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 2,
            },
            expansion: Vec::new(),
        };

        let defaults = compile_shape(&contract(&["Demo.bsv", "T0066"])).unwrap();
        assert_eq!(defaults.mode, BscCompileMode::Frontend);
        assert_eq!(defaults.expected_exit, ExpectedExit::Failure);
        assert_eq!(defaults.expectation, OperationExpectation::Required);
        assert!(defaults.args.is_empty());
        assert!(matches!(
            defaults.diagnostics.as_slice(),
            [CompileDiagnostic {
                action: Action::AssertDiagnosticCount {
                    kind: DiagnosticKind::Error,
                    code: Some(code),
                    count: 1,
                    ..
                },
                expectation: OperationExpectation::Required,
            }] if code == "T0066"
        ));

        let explicit = compile_shape(&contract(&[
            "Demo.bsv",
            "T0066",
            "2",
            "{P0102 3} P0103",
            "-continue-after-errors -p +:lib",
        ]))
        .unwrap();
        assert_eq!(explicit.args, ["-continue-after-errors", "-p", "+:lib"]);
        assert!(matches!(
            explicit.diagnostics.as_slice(),
            [
                CompileDiagnostic {
                    action: Action::AssertDiagnosticCount {
                        kind: DiagnosticKind::Error,
                        code: Some(error_code),
                        count: 2,
                        ..
                    },
                    expectation: OperationExpectation::Required,
                },
                CompileDiagnostic {
                    action: Action::AssertDiagnosticCount {
                        kind: DiagnosticKind::Warning,
                        code: Some(first_warning),
                        count: 3,
                        ..
                    },
                    expectation: OperationExpectation::Required,
                },
                CompileDiagnostic {
                    action: Action::AssertDiagnosticCount {
                        kind: DiagnosticKind::Warning,
                        code: Some(second_warning),
                        count: 1,
                        ..
                    },
                    expectation: OperationExpectation::Required,
                }
            ] if error_code == "T0066"
                && first_warning == "P0102"
                && second_warning == "P0103"
        ));

        for (arguments, expected) in [
            (vec!["Demo.bsv"], "requires 2 to 5 static arguments"),
            (
                vec!["Demo.bsv", "T0066", "1", "{P0102 2 extra}"],
                "warning specification requires 1 to 2 static fields",
            ),
            (
                vec!["Demo.bsv", "T0066", "1", "{P0102 nope}"],
                "invalid diagnostic count",
            ),
            (
                vec!["Demo.bsv", "T0066", "1", "{}"],
                "warning specification requires 1 to 2 static fields",
            ),
        ] {
            let error = compile_shape(&contract(&arguments)).unwrap_err();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn simulation_import_does_not_reorder_or_reuse_mkdir() {
        let span = |start_byte, end_byte| ManifestSourceSpan {
            start_byte,
            end_byte,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: end_byte + 1,
        };
        let contract = SimulationContract {
            source: "Demo.bsv".to_owned(),
            helper: "test_c_only_bsv_multi_options".to_owned(),
            arguments: vec![
                "Demo".to_owned(),
                "mkDemo".to_owned(),
                String::new(),
                "-simdir work".to_owned(),
            ],
            backend: SimulationBackend::Bluesim,
            generation: crate::model::GenerationStrategy::Bluesim,
            guard: Guard::Always,
            span: span(10, 20),
            expansion: Vec::new(),
        };
        let future_mkdir = WorkflowAction::CreateDirectory(crate::model::CreateDirectoryAction {
            path: "work".to_owned(),
            guard: Guard::Always,
            span: span(30, 40),
            expansion: Vec::new(),
        });
        assert!(simulation_scenario(
            &[&contract],
            None,
            &[future_mkdir],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .is_err());

        let future_expanded_mkdir =
            WorkflowAction::CreateDirectory(crate::model::CreateDirectoryAction {
                path: "work".to_owned(),
                guard: Guard::Always,
                span: span(0, 5),
                expansion: vec![span(30, 40)],
            });
        assert!(simulation_scenario(
            &[&contract],
            None,
            &[future_expanded_mkdir],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .is_err());

        let preceding_mkdir =
            WorkflowAction::CreateDirectory(crate::model::CreateDirectoryAction {
                path: "work".to_owned(),
                guard: Guard::Always,
                span: span(0, 5),
                expansion: Vec::new(),
            });
        assert!(simulation_scenario(
            &[&contract],
            None,
            &[preceding_mkdir],
            &BTreeSet::from([0]),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .is_err());
    }

    #[test]
    fn imports_shared_classic_and_module_options_simulations() {
        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let contracts = |helper: &str, source: &str, arguments: &[&str]| {
            let contract = SimulationContract {
                source: source.to_owned(),
                helper: helper.to_owned(),
                arguments: arguments
                    .iter()
                    .map(|argument| (*argument).to_owned())
                    .collect(),
                backend: SimulationBackend::Bluesim,
                generation: crate::model::GenerationStrategy::Shared,
                guard: Guard::Always,
                span,
                expansion: Vec::new(),
            };
            let icarus = SimulationContract {
                backend: SimulationBackend::Icarus,
                ..contract.clone()
            };
            (contract, icarus)
        };

        let (worker, worker_icarus) = contracts(
            "test_c_veri_worker",
            "Worker.bsv",
            &[
                "Worker",
                "mkWorker",
                "Helper",
                "bsv",
                "1",
                "1",
                "worker.expected",
                "",
                "",
                "0",
                "0",
            ],
        );
        let imported = simulation_scenario(
            &[&worker, &worker_icarus],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .unwrap()
        .unwrap();
        let operations = &imported.scenario.stages[0].operations;
        assert!(matches!(
            &operations[0].action,
            Action::BscGenerate {
                source,
                mode: SimulationGenerationMode::SharedElaboration,
                module: Some(module),
                args,
            } if source == "Worker.bsv" && module == "mkWorker" && args.is_empty()
        ));
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::BscLink { objects, .. }
                if objects == &["mkWorker", "Helper"]
        )));
        assert_eq!(imported.consumption.golden_paths, ["worker.expected"]);
        assert!(!operations
            .iter()
            .any(|operation| matches!(operation.action, Action::AssertVcd { .. })));

        let (classic, classic_icarus) = contracts(
            "test_c_veri",
            "FundepSelect.bs",
            &["FundepSelect", "sysFundepSelect.out.expected"],
        );
        let imported = simulation_scenario(
            &[&classic, &classic_icarus],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            &imported.scenario.stages[0].operations[0].action,
            Action::BscGenerate {
                source,
                mode: SimulationGenerationMode::SharedElaboration,
                module: Some(module),
                args,
            } if source == "FundepSelect.bs" && module == "sysFundepSelect" && args.is_empty()
        ));
        assert_eq!(
            imported.consumption.golden_paths,
            ["sysFundepSelect.out.expected"]
        );

        let (options, options_icarus) = contracts(
            "test_c_veri_bsv_modules_options",
            "Demo.bsv",
            &[
                "Demo",
                "Helper.ba",
                "-keep-fires",
                "custom.expected",
                "",
                "",
                "-L link",
                "+sim",
            ],
        );
        let imported = simulation_scenario(
            &[&options, &options_icarus],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .unwrap()
        .unwrap();
        let operations = &imported.scenario.stages[0].operations;
        assert!(matches!(
            &operations[0].action,
            Action::BscGenerate { source, args, .. }
                if source == "Demo.bsv" && args == &["-keep-fires"]
        ));
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::BscLink { objects, args, .. }
                if objects == &["sysDemo", "Helper.ba"] && args == &["-L", "link"]
        )));
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::SimulationRun { args, .. } if args == &["+sim"]
        )));
        assert_eq!(imported.consumption.golden_paths, ["custom.expected"]);

        let separate = SimulationContract {
            helper: "test_c_veri_bsv_modules_options_separately".to_owned(),
            generation: crate::model::GenerationStrategy::Bluesim,
            ..options.clone()
        };
        let imported = simulation_scenario(
            &[&separate],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            &imported.scenario.stages[0].operations[0].action,
            Action::BscGenerate {
                mode: SimulationGenerationMode::Bluesim,
                source,
                args,
                ..
            } if source == "Demo.bsv" && args == &["-keep-fires"]
        ));
        assert!(imported.scenario.stages[0]
            .operations
            .iter()
            .any(|operation| matches!(
                &operation.action,
                Action::BscLink {
                    backend: PlanSimulationBackend::Bluesim,
                    ..
                }
            )));

        let (mut verilog_multi, _) = contracts(
            "test_veri_only_bsv_multi",
            "Top.bsv",
            &["Top", "mkTop", "Helper", "custom.expected", "", "0", "0"],
        );
        verilog_multi.backend = SimulationBackend::Icarus;
        verilog_multi.generation = crate::model::GenerationStrategy::Icarus;
        let imported = simulation_scenario(
            &[&verilog_multi],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .unwrap()
        .unwrap();
        let operations = &imported.scenario.stages[0].operations;
        assert!(matches!(
            &operations[0].action,
            Action::BscGenerate {
                mode: SimulationGenerationMode::Verilog,
                source,
                module: Some(module),
                ..
            } if source == "Top.bsv" && module == "mkTop"
        ));
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::BscLink {
                backend: PlanSimulationBackend::Icarus,
                objects,
                ..
            } if objects == &["mkTop", "Helper"]
        )));

        let mut mismatched_worker = worker;
        mismatched_worker.generation = crate::model::GenerationStrategy::Bluesim;
        let error = simulation_scenario(
            &[&mismatched_worker],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .err()
        .expect("backend mismatch should fail");
        assert!(error.message.contains("backend flags describe"));

        let mut bad_arity = worker_icarus;
        bad_arity.arguments.truncate(8);
        let error = simulation_scenario(
            &[&bad_arity],
            None,
            &[],
            &BTreeSet::new(),
            None,
            &[],
            &[],
            None,
            project_root(),
        )
        .err()
        .expect("bad arity should fail");
        assert!(error.message.contains("requires 9 to 11 static arguments"));
    }

    #[test]
    fn artifact_flow_enforces_ordered_copy_and_move_preconditions() {
        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let transfer =
            |operation, source: &str, destination: &str| crate::model::ArtifactTransferAction {
                operation,
                source: source.to_owned(),
                destination: destination.to_owned(),
                guard: Guard::Always,
                span,
                expansion: Vec::new(),
            };
        let mut flow = ArtifactFlow::new(BTreeSet::from(["source".to_owned()]));

        assert!(flow.apply(&transfer(
            ArtifactTransferOperation::Move,
            "source",
            "moved"
        )));
        assert!(!flow.apply(&transfer(
            ArtifactTransferOperation::Move,
            "source",
            "missing-source"
        )));
        assert!(flow.apply(&transfer(
            ArtifactTransferOperation::Copy,
            "moved",
            "copied"
        )));
        assert!(!flow.apply(&transfer(
            ArtifactTransferOperation::Copy,
            "moved",
            "copied"
        )));
        assert!(flow.contains("moved"));
        assert!(flow.contains("copied"));
        assert!(!flow.remove("unknown"));
        assert!(flow.remove("moved"));
        assert!(!flow.contains("moved"));
    }

    #[test]
    fn simulation_artifacts_are_backend_specific() {
        assert_eq!(
            bluesim_vcd_paths(&["-V".to_owned()]),
            BTreeSet::from(["dump.vcd".to_owned()])
        );
        assert_eq!(
            bluesim_vcd_paths(&["-V".to_owned(), "custom.vcd".to_owned()]),
            BTreeSet::from(["custom.vcd".to_owned()])
        );

        let span = ManifestSourceSpan {
            start_byte: 0,
            end_byte: 1,
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 2,
        };
        let mut contract = SimulationContract {
            source: "Demo.bsv".to_owned(),
            helper: "test_c_only".to_owned(),
            arguments: vec!["Demo".to_owned()],
            backend: SimulationBackend::Bluesim,
            generation: crate::model::GenerationStrategy::Bluesim,
            guard: Guard::Always,
            span,
            expansion: Vec::new(),
        };

        let bluesim = simulation_artifact_paths(&contract);
        assert!(bluesim.contains("sysDemo.c.out"));
        assert!(!bluesim.contains("sysDemo.v.out"));

        contract.backend = SimulationBackend::Icarus;
        contract.generation = crate::model::GenerationStrategy::Icarus;
        let icarus = simulation_artifact_paths(&contract);
        assert!(!icarus.contains("sysDemo.c.out"));
        assert!(icarus.contains("sysDemo.v.out"));
    }

    #[test]
    fn transfer_after_a_bound_check_preserves_artifact_flow() {
        let span = |start_byte| ManifestSourceSpan {
            start_byte,
            end_byte: start_byte + 1,
            start_line: 1,
            start_column: start_byte + 1,
            end_line: 1,
            end_column: start_byte + 2,
        };
        let assertion = |path: &str, start_byte| AssertionContract {
            helper: "find_n_strings".to_owned(),
            arguments: vec![path.to_owned(), "value".to_owned(), "1".to_owned()],
            guard: Guard::Always,
            span: span(start_byte),
            expansion: Vec::new(),
        };
        let script = ScriptManifest {
            origin: "testsuite/example/example.exp".to_owned(),
            source_sha256: "0".repeat(64),
            contracts: vec![Contract::Compile(CompileContract {
                source: "Demo.bsv".to_owned(),
                working_directory: None,
                helper: "compile_verilog_pass".to_owned(),
                arguments: vec!["Demo.bsv".to_owned()],
                guard: Guard::Always,
                span: span(0),
                expansion: Vec::new(),
            })],
            assertions: vec![
                assertion("Demo.bsv.bsc-vcomp-out", 10),
                assertion("moved.out", 30),
            ],
            comparisons: Vec::new(),
            bluesim_sequences: Vec::new(),
            bluesim_workflows: Vec::new(),
            systemc_workflows: Vec::new(),
            make_test_data_actions: Vec::new(),
            bsc_options_overlays: Vec::new(),
            workflow_actions: vec![WorkflowAction::TransferArtifact(
                crate::model::ArtifactTransferAction {
                    operation: ArtifactTransferOperation::Move,
                    source: "Demo.bsv.bsc-vcomp-out".to_owned(),
                    destination: "moved.out".to_owned(),
                    guard: Guard::Always,
                    span: span(20),
                    expansion: Vec::new(),
                },
            )],
            unsupported: Vec::new(),
        };

        assert_eq!(
            check_bindings(&script, Path::new("")).get(&ProducerKey::Compile(0)),
            Some(&vec![BoundCheck::Assertion(0), BoundCheck::Assertion(1)])
        );
    }

    #[test]
    fn composes_paired_declared_dump_comparisons_from_one_expansion() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.if/if")
            .expect("if plan is generated");
        assert_eq!(plan.plan.status, PlanStatus::Complete);
        assert!(plan.plan.diagnostics.is_empty());
        let paired = plan
            .plan
            .scenarios
            .iter()
            .filter(|scenario| scenario.id.starts_with("compile-dump-comparison-"))
            .collect::<Vec<_>>();
        assert_eq!(paired.len(), 5);
        let operations = paired[0]
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::BscCompile { source, .. } if source == "IfLifting.bs"
        )));
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::BscCompile { source, .. } if source == "IfLifted.bs"
        )));
        assert!(operations.iter().any(|operation| matches!(
            &operation.action,
            Action::AssertGolden { actual, expected }
                if actual == "IfLifting.bs.atsexpand" && expected == "IfLifted.bs.atsexpand"
        )));
    }

    #[test]
    fn bluetcl_package_batch_preserves_typed_order_and_operation_guards() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = |id: &str| {
            &generated
                .plans
                .iter()
                .find(|generated| generated.plan.id == id)
                .unwrap_or_else(|| panic!("missing generated plan {id}"))
                .plan
        };

        let expand_ports = plan("bsc.bluetcl/packages/expandPorts/expandPorts");
        assert_eq!(
            expand_ports.status,
            PlanStatus::Complete,
            "{:?}",
            expand_ports.diagnostics
        );
        assert!(expand_ports.diagnostics.is_empty());
        assert_eq!(expand_ports.scenarios.len(), 13);
        for scenario in &expand_ports.scenarios {
            assert!(!scenario
                .requires
                .iter()
                .any(|requirement| matches!(requirement, Requirement::BluetclPackage(_))));
            let operations = &scenario.stages[0].operations;
            assert_eq!(operations.len(), 4);
            assert!(operations.iter().all(|operation| operation
                .requires
                .contains(&Requirement::BluetclPackage(BluetclPackage::ExpandPorts))));
            assert!(matches!(
                operations.as_slice(),
                [
                    OperationRecord {
                        action: Action::BscCompile { .. },
                        ..
                    },
                    OperationRecord {
                        action: Action::BluetclRun {
                            invocation: BluetclInvocation::InstalledScript {
                                script: BluetclInstalledScript::ExpandPorts,
                                ..
                            },
                            ..
                        },
                        ..
                    },
                    OperationRecord {
                        action: Action::AssertGoldenNormalized { .. },
                        ..
                    },
                    OperationRecord {
                        action: Action::AssertGoldenNormalized { .. },
                        ..
                    },
                ]
            ));
        }

        let makedepend = plan("bsc.bluetcl/packages/makedepend/makedepend");
        assert_eq!(
            makedepend.status,
            PlanStatus::Complete,
            "{:?}",
            makedepend.diagnostics
        );
        assert!(makedepend.diagnostics.is_empty());
        let makedepend_operations = &makedepend.scenarios[0].stages[0].operations;
        assert_eq!(
            makedepend_operations
                .iter()
                .filter(|operation| matches!(
                    operation.action,
                    Action::BluetclRun {
                        invocation: BluetclInvocation::Makedepend { .. },
                        ..
                    }
                ))
                .count(),
            12
        );
        assert!(makedepend_operations.iter().any(|operation| matches!(
            &operation.action,
            Action::BluetclRun {
                invocation: BluetclInvocation::Makedepend {
                    command: BluetclMakedependCommand::Makedepend,
                    args,
                },
                working_directory: Some(directory),
                stdout,
                ..
            } if directory == "makedepend"
                && stdout == "makedepend/updir.bluetcl-out"
                && args.contains(&"../makedepend/:%/Libraries".to_owned())
        )));

        let instsynth = plan("bsc.bluetcl/packages/InstSynth/InstSynth");
        assert_eq!(instsynth.status, PlanStatus::Blocked);
        assert_eq!(
            instsynth.scenarios.len(),
            1,
            "{:?}",
            instsynth
                .scenarios
                .iter()
                .map(|scenario| scenario.id.as_str())
                .collect::<Vec<_>>()
        );
        assert!(instsynth.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("instsynth.tcl.bluetcl-bh-out.expected")));
        let operations = &instsynth.scenarios[0].stages[0].operations;
        assert_eq!(operations.len(), 13);
        assert!(operations.iter().all(|operation| operation
            .requires
            .contains(&Requirement::BluetclPackage(BluetclPackage::InstSynth))));
        assert!(matches!(
            operations.as_slice(),
            [
                OperationRecord { action: Action::BluetclRun { .. }, .. },
                OperationRecord { action: Action::AssertGoldenNormalized { .. }, .. },
                OperationRecord { action: Action::BluetclRun { .. }, .. },
                OperationRecord { action: Action::AssertGoldenNormalized { .. }, .. },
                OperationRecord { action: Action::AssertGolden { .. }, .. },
                OperationRecord { action: Action::AssertGolden { .. }, .. },
                OperationRecord { action: Action::BscCompile { source: first_source, .. }, .. },
                OperationRecord { action: Action::BscCompile { source: second_source, .. }, .. },
                OperationRecord { action: Action::AssertTextCount { .. }, .. },
                OperationRecord { action: Action::AssertTextCount { .. }, .. },
                OperationRecord { action: Action::AssertTextCount { .. }, .. },
                OperationRecord { action: Action::AssertTextCount { .. }, .. },
                OperationRecord { action: Action::AssertTextCount { .. }, .. },
            ] if first_source == "Inst_auto.bsv" && second_source == "Inst.bsv"
        ));
    }

    #[test]
    fn requested_batch_plans_are_complete_closed_and_source_ordered() {
        let generated = build_test_plans(project_root()).unwrap();
        let plan = |id: &str| {
            &generated
                .plans
                .iter()
                .find(|generated| generated.plan.id == id)
                .unwrap_or_else(|| panic!("missing generated plan {id}"))
                .plan
        };

        let imported = plan("bsc.interra/Path_Analysis/Imported_Modules/Imported_Modules");
        assert_eq!(imported.status, PlanStatus::Complete);
        assert!(imported.diagnostics.is_empty());
        assert!(!imported.fixtures.iter().any(|fixture| {
            fixture.path == "770" || fixture.path.ends_with("bsc-vcomp-out.expected")
        }));
        let missing_goldens = imported
            .scenarios
            .iter()
            .flat_map(|scenario| &scenario.stages)
            .filter_map(|stage| {
                let [producer, assertion] = stage.operations.as_slice() else {
                    return None;
                };
                match (&producer.action, &assertion.action) {
                    (
                        Action::BscCompile { .. },
                        Action::AssertGoldenMissingXfail {
                            actual,
                            expected,
                            reason,
                        },
                    ) if producer.artifacts.outputs.contains(actual) => {
                        Some((actual, expected, reason))
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(missing_goldens.len(), 2);
        assert!(missing_goldens.iter().all(|(actual, expected, reason)| {
            expected == &&format!("{actual}.expected") && *reason == "upstream bug 770"
        }));

        let b1595 = plan("bsc.bugs/bluespec_inc/b1595/b1595");
        assert_eq!(b1595.status, PlanStatus::Complete);
        assert!(b1595.diagnostics.is_empty());
        assert_eq!(b1595.scenarios.len(), 2);
        let unreadable = &b1595.scenarios[0].stages[0].operations;
        assert!(matches!(
            unreadable.as_slice(),
            [
                OperationRecord { action: Action::BscGenerate { source, .. }, .. },
                OperationRecord { action: Action::FsMkdir { path: first }, .. },
                OperationRecord { action: Action::FsMkdir { path: second }, .. },
                OperationRecord { action: Action::FsCopy { source: copy_source_one, destination: copy_one }, .. },
                OperationRecord { action: Action::FsCopy { source: copy_source_two, destination: copy_two }, .. },
                OperationRecord { action: Action::FsRemoveUserRead { path: unreadable_path }, .. },
                OperationRecord { action: Action::BscLink { top, .. }, .. },
                OperationRecord { action: Action::AssertGolden { actual, expected }, .. },
            ] if source == "TbGCD.bsv"
                && first == "libdir1"
                && second == "libdir2"
                && copy_source_one == "mkGCD.ba"
                && copy_one == "libdir1/mkGCD.ba"
                && copy_source_two == "mkGCD.ba"
                && copy_two == "libdir2/mkGCD.ba"
                && unreadable_path == "libdir1/mkGCD.ba"
                && top == "mkTbGCD"
                && actual == "mkTbGCD.bsc-ccomp-out"
                && expected == "mkTbGCD.bsc-ccomp-out.expected"
        ));
        assert!(unreadable[..5].iter().all(|operation| {
            !operation
                .requires
                .contains(&Requirement::PosixUnreadability)
        }));
        assert!(unreadable[5..].iter().all(|operation| {
            operation
                .requires
                .contains(&Requirement::PosixUnreadability)
        }));
        let wrong_module = &b1595.scenarios[1].stages[0].operations;
        assert!(matches!(
            wrong_module.as_slice(),
            [
                OperationRecord { action: Action::BscGenerate { source: wrong_mod, .. }, .. },
                OperationRecord { action: Action::BscGenerate { source: wrong_top, .. }, .. },
                OperationRecord { action: Action::FsMoveReplace { source, destination }, .. },
                OperationRecord { action: Action::BscLink { top, expected_exit: ExpectedExit::Failure, .. }, .. },
                OperationRecord { action: Action::AssertGolden { actual, expected }, .. },
            ] if wrong_mod == "WrongMod.bsv"
                && wrong_top == "WrongTop.bsv"
                && source == "mkWrongMod.ba"
                && destination == "mkRightMod.ba"
                && top == "mkWrongTop"
                && actual == "mkWrongTop.bsc-ccomp-out"
                && expected == "mkWrongTop.bsc-ccomp-out.expected"
        ));
        assert!(wrong_module.iter().all(|operation| {
            !operation
                .requires
                .contains(&Requirement::PosixUnreadability)
        }));

        let cpp = plan("bsc.driver/cpp/cpp");
        assert_eq!(cpp.status, PlanStatus::Complete);
        assert!(cpp.diagnostics.is_empty());
        assert!(cpp.fixtures.iter().any(|fixture| {
            fixture.path == "Cpreprocess1.bsv"
                && fixture.source.as_deref() == Some("Cpreprocess.bsv")
                && fixture.role == FixtureRole::Source
        }));
        let line = cpp
            .scenarios
            .iter()
            .find(|scenario| scenario.id == "compile-4-Cpreprocess_line")
            .expect("cpp line-directive scenario");
        assert!(line.fixtures.contains(&"more.bsv".to_owned()));
        let operations = &line.stages[0].operations;
        assert_eq!(
            operations[0].artifacts.inputs,
            ["Cpreprocess_line.bsv", "more.bsv"]
        );
        assert!(matches!(
            operations.as_slice(),
            [
                OperationRecord { action: Action::BscCompile { source, stdout, .. }, .. },
                OperationRecord { action: Action::FsRewriteDarwinCppIncludePath { source: rewrite_source, destination: filtered }, .. },
                OperationRecord { action: Action::FsMoveReplace { source: move_source, destination: move_destination }, .. },
                OperationRecord { action: Action::AssertGolden { actual, expected }, .. },
            ] if source == "Cpreprocess_line.bsv"
                && stdout == "Cpreprocess_line.bsv.bsc-out"
                && rewrite_source == stdout
                && filtered == "Cpreprocess_line.bsv.bsc-out.filtered"
                && move_source == filtered
                && move_destination == stdout
                && actual == stdout
                && expected == "Cpreprocess_line.bsv.bsc-out.expected"
        ));
        assert!(!operations[0].requires.contains(&Requirement::Darwin));
        assert!(operations[1..3]
            .iter()
            .all(|operation| operation.requires.contains(&Requirement::Darwin)));
        assert!(!operations[3].requires.contains(&Requirement::Darwin));
        assert_eq!(
            operations
                .iter()
                .map(|operation| operation.provenance.span.start_line)
                .collect::<Vec<_>>(),
            [11, 15, 18, 21]
        );
    }

    #[test]
    fn parallel_plan_contains_the_sequence_and_special_path_simulation() {
        let generated = build_test_plans(project_root()).unwrap();
        let parallel = generated
            .plans
            .iter()
            .find(|generated| generated.plan.id == "bsc.bluesim/parallel/parallel")
            .unwrap();
        assert_eq!(parallel.plan.status, PlanStatus::Complete);
        assert!(parallel.plan.diagnostics.is_empty());
        assert_eq!(parallel.plan.scenarios.len(), 2);
        assert_eq!(parallel.plan.scenarios[0].stages.len(), 3);

        let simulation = &parallel.plan.scenarios[1];
        assert_eq!(simulation.id, "simulation-mkTbGCD");
        assert!(simulation.requires.contains(&Requirement::Bluesim));
        assert!(simulation.requires.contains(&Requirement::NonWindows));
        assert_eq!(simulation.stages.len(), 1);
        let operations = &simulation.stages[0].operations;
        assert_eq!(operations.len(), 8);
        assert!(matches!(operations[0].action, Action::FsMkdir { .. }));
        assert!(matches!(operations[1].action, Action::BscGenerate { .. }));
        assert!(matches!(operations[2].action, Action::BscLink { .. }));
        assert!(matches!(operations[3].action, Action::SimulationRun { .. }));
        assert!(matches!(operations[4].action, Action::AssertGolden { .. }));
        assert!(matches!(operations[5].action, Action::SimulationRun { .. }));
        assert!(matches!(
            operations[6].action,
            Action::AssertVcdValid { .. }
        ));
        assert!(matches!(operations[7].action, Action::AssertGolden { .. }));
        assert!(parallel.plan.fixtures.iter().any(|fixture| {
            fixture.path == "mkTbGCD.out.expected" && fixture.role == FixtureRole::Golden
        }));
    }

    #[test]
    fn imports_the_three_pinned_closed_batches_with_exact_plan_shapes() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let plan = |origin: &str| {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .expect("manifest contains pinned origin");
            plan_from_script(root, script).plan
        };

        let course = plan(COURSE_LAB_PLAN_ORIGIN);
        assert_eq!(course.status, PlanStatus::Complete);
        assert!(course.diagnostics.is_empty());
        assert_eq!(course.scenarios.len(), 10);
        let mesa = course
            .scenarios
            .iter()
            .filter(|scenario| {
                scenario
                    .stages
                    .iter()
                    .flat_map(|stage| &stage.operations)
                    .any(|operation| {
                        matches!(
                            &operation.action,
                            Action::BscCompile { source, .. }
                                | Action::BscGenerate { source, .. }
                                if source == "TestMesa.bsv"
                        )
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(mesa.len(), 8);
        for (scenario, expected) in mesa.iter().zip([
            "MesaTx.bsv",
            "MesaTx.bsv",
            "MesaStatic.bsv",
            "MesaStatic.bsv",
            "MesaFlex.bsv",
            "MesaFlex.bsv",
            "MesaCirc.bsv",
            "MesaCirc.bsv",
        ]) {
            assert!(matches!(
                &scenario.stages[0].operations[0].action,
                Action::FsCopy { source, destination }
                    if source == expected && destination == "Mesa.bsv"
            ));
            assert!(!scenario
                .stages
                .iter()
                .flat_map(|stage| &stage.operations)
                .any(|operation| matches!(
                    &operation.action,
                    Action::FsCopy { source, .. } if source.starts_with("sysTestMesa.")
                ) || matches!(
                    &operation.action,
                    Action::FsMove { source, .. } if source == "sysTestMesa.out.bak"
                )));
            assert!(COURSE_LAB_COMMON_CLOSURE
                .iter()
                .all(|path| scenario.fixtures.iter().any(|fixture| fixture == path)));
            let (_, expected_closure) = COURSE_LAB_VARIANT_CLOSURES
                .iter()
                .find(|(variant, _)| variant == &expected)
                .unwrap();
            assert!(expected_closure
                .iter()
                .all(|path| scenario.fixtures.iter().any(|fixture| fixture == path)));
            assert!(COURSE_LAB_VARIANT_CLOSURES
                .iter()
                .filter(|(variant, _)| variant != &expected)
                .flat_map(|(_, closure)| closure.iter())
                .filter(|path| path.ends_with("Lpm.bsv"))
                .all(|path| !scenario.fixtures.iter().any(|fixture| fixture == path)));
        }

        let sal = plan(SAL_PLAN_ORIGIN);
        assert_eq!(sal.status, PlanStatus::Complete);
        assert!(sal.diagnostics.is_empty());
        assert_eq!(sal.fixture_dir, "testsuite/bsc.misc");
        assert_eq!(sal.scenarios.len(), 1);
        let sal_operations = sal.scenarios[0]
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        assert_eq!(sal_operations.len(), 80);
        assert_eq!(
            sal_operations
                .iter()
                .filter(|operation| matches!(operation.action, Action::FsCopy { .. }))
                .count(),
            18
        );
        assert_eq!(
            sal_operations
                .iter()
                .filter(|operation| matches!(operation.action, Action::FsEnsureAbsent { .. }))
                .count(),
            18
        );
        assert_eq!(
            sal_operations
                .iter()
                .filter(|operation| matches!(
                    &operation.action,
                    Action::BscCompile { working_directory: Some(directory), .. }
                        if directory == "sal"
                ))
                .count(),
            16
        );
        assert_eq!(
            sal_operations
                .iter()
                .filter(|operation| matches!(
                    &operation.action,
                    Action::AssertGoldenNormalized {
                        actual,
                        expected,
                        normalizations,
                    } if actual.starts_with("sal/")
                        && expected.starts_with("sal/")
                        && normalizations == &[GoldenNormalization::GeneratedIds]
                ))
                .count(),
            20
        );
        assert_eq!(
            sal_operations
                .iter()
                .filter(|operation| matches!(
                    &operation.action,
                    Action::AssertRegex { path, .. } if path == "sal/CTX_sysPrimMods.sal"
                ))
                .count(),
            8
        );

        let inout = plan(INOUT_PLAN_ORIGIN);
        assert_eq!(inout.status, PlanStatus::Complete);
        assert!(inout.diagnostics.is_empty());
        assert_eq!(inout.scenarios.len(), 21);
        assert_eq!(inout.scenarios[0].id, "inout-no-inline-episode");
        assert_eq!(inout.scenarios[1].id, "inout-inline-episode");
        for scenario in &inout.scenarios[..2] {
            assert_eq!(
                scenario.stages[0]
                    .operations
                    .iter()
                    .filter(|operation| matches!(operation.action, Action::FsEnsureAbsent { .. }))
                    .count(),
                24
            );
            assert!(scenario.stages[0].operations.iter().all(|operation| {
                matches!(&operation.action, Action::FsEnsureAbsent { path }
                    if path.ends_with(".bo") && !path.contains('*'))
            }));
        }
        let first_bsc_args = inout.scenarios[0]
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .filter_map(|operation| match &operation.action {
                Action::BscCompile { args, .. }
                | Action::BscGenerate { args, .. }
                | Action::BscLink { args, .. } => Some(args),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(first_bsc_args.len(), 34);
        assert!(first_bsc_args.iter().all(|args| {
            args.first().map(String::as_str) == Some("-no-inline-inout-connect")
                && args
                    .iter()
                    .filter(|arg| arg.as_str() == "-no-inline-inout-connect")
                    .count()
                    == 1
        }));
        assert!(inout.scenarios[1]
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .all(|operation| match &operation.action {
                Action::BscCompile { args, .. }
                | Action::BscGenerate { args, .. }
                | Action::BscLink { args, .. } => {
                    !args.iter().any(|arg| arg == "-no-inline-inout-connect")
                }
                _ => true,
            }));
    }

    #[test]
    fn pinned_batches_refuse_changed_hashes_and_near_match_origins() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        for origin in [COURSE_LAB_PLAN_ORIGIN, SAL_PLAN_ORIGIN, INOUT_PLAN_ORIGIN] {
            let mut changed = manifest
                .scripts
                .iter()
                .find(|script| script.origin == origin)
                .expect("manifest contains pinned origin")
                .clone();
            changed.source_sha256 = "0".repeat(64);
            let changed = plan_from_script(root, &changed).plan;
            assert_eq!(changed.status, PlanStatus::Blocked, "{origin}");
            assert!(changed.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "import.pinned_batch"
                    && diagnostic.message.contains("closed expansion refused")
            }));
        }

        let mut near_match = manifest
            .scripts
            .iter()
            .find(|script| script.origin == COURSE_LAB_PLAN_ORIGIN)
            .unwrap()
            .clone();
        near_match.origin.push_str(".near");
        let near_match = plan_from_script(root, &near_match).plan;
        assert_eq!(near_match.status, PlanStatus::Blocked);
        assert!(!near_match
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "import.pinned_batch"));
    }

    #[test]
    fn sal_membership_pin_rejects_changes_and_path_escape() {
        let root = project_root().join("testsuite/bsc.misc");
        audit_pinned_regular_membership(&root, "lambda_calculus", SAL_LAMBDA_MEMBERS)
            .expect("checked-in SAL membership matches the pin");
        assert!(audit_pinned_regular_membership(
            &root,
            "lambda_calculus",
            &SAL_LAMBDA_MEMBERS[..SAL_LAMBDA_MEMBERS.len() - 1],
        )
        .unwrap_err()
        .contains("pinned membership changed"));
        let mut added = SAL_LAMBDA_MEMBERS.to_vec();
        added.push("Unexpected.bsv");
        assert!(
            audit_pinned_regular_membership(&root, "lambda_calculus", &added)
                .unwrap_err()
                .contains("pinned membership changed")
        );
        assert!(
            audit_pinned_regular_membership(&root, "../lambda_calculus", SAL_LAMBDA_MEMBERS,)
                .unwrap_err()
                .contains("not a safe relative path")
        );
    }

    #[cfg(unix)]
    #[test]
    fn sal_membership_pin_rejects_symlinks_and_case_collisions() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let root = std::env::temp_dir().join(format!(
            "bsc-sal-membership-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let directory = root.join("lambda_calculus");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("Real.bsv"), "package Real; endpackage\n").unwrap();
        symlink("Real.bsv", directory.join("Link.bsv")).unwrap();
        assert!(audit_pinned_regular_membership(
            &root,
            "lambda_calculus",
            &["Link.bsv", "Real.bsv"],
        )
        .unwrap_err()
        .contains("regular non-link"));
        fs::remove_file(directory.join("Link.bsv")).unwrap();
        fs::write(directory.join("real.bsv"), "package real; endpackage\n").unwrap();
        assert!(audit_pinned_regular_membership(
            &root,
            "lambda_calculus",
            &["Real.bsv", "real.bsv"],
        )
        .unwrap_err()
        .contains("case-colliding"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inout_replaces_only_the_second_simple_connect_vcd_in_each_episode() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let script = manifest
            .scripts
            .iter()
            .find(|script| script.origin == INOUT_PLAN_ORIGIN)
            .unwrap();
        let plan = plan_from_script(root, script).plan;

        for scenario in &plan.scenarios[..2] {
            let operations = scenario
                .stages
                .iter()
                .flat_map(|stage| &stage.operations)
                .collect::<Vec<_>>();
            let first = operations
                .iter()
                .position(|operation| {
                    matches!(
                        &operation.action,
                        Action::SimulationRun {
                            backend: PlanSimulationBackend::Icarus,
                            executable,
                            vcd: Some(vcd),
                            ..
                        } if executable == "sysSimpleConnect1"
                            && vcd == "sysSimpleConnect1.v.vcd"
                    )
                })
                .expect("first SimpleConnect1 VCD producer");
            let second = operations
                .iter()
                .position(|operation| {
                    matches!(
                        &operation.action,
                        Action::SimulationRun {
                            backend: PlanSimulationBackend::Icarus,
                            executable,
                            vcd: Some(vcd),
                            ..
                        } if executable == "sysSimpleConnect1" && vcd == "dump.vcd"
                    )
                })
                .expect("second SimpleConnect1 raw VCD producer");
            let replace = operations
                .iter()
                .position(|operation| {
                    matches!(
                        &operation.action,
                        Action::FsMoveReplace { source, destination }
                            if source == "dump.vcd" && destination == "sysSimpleConnect1.v.vcd"
                    )
                })
                .expect("typed SimpleConnect1 VCD replacement");
            assert!(first < second && second + 1 == replace);
            assert_eq!(
                operations
                    .iter()
                    .filter(|operation| matches!(
                        &operation.action,
                        Action::FsMoveReplace { destination, .. }
                            if destination == "sysSimpleConnect1.v.vcd"
                    ))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn inout_binds_the_late_arg_to_ifc_assertion_to_its_final_real_producer() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let script = manifest
            .scripts
            .iter()
            .find(|script| script.origin == INOUT_PLAN_ORIGIN)
            .unwrap();
        let plan = plan_from_script(root, script).plan;
        let scenario = |id: &str| {
            plan.scenarios
                .iter()
                .find(|scenario| scenario.id == id)
                .unwrap_or_else(|| panic!("missing inout scenario {id}"))
        };

        let producer = scenario("compile-45-CheckResets_ArgToIfc_DiffReset");
        let operations = producer
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let compile = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::BscCompile { source, .. }
                        if source == "CheckResets_ArgToIfc_DiffReset.bsv"
                ) && operation
                    .artifacts
                    .outputs
                    .contains(&"sysArgToIfc.v".to_owned())
            })
            .expect("final real sysArgToIfc.v producer");
        let assertion = operations
            .iter()
            .position(|operation| {
                matches!(
                    &operation.action,
                    Action::AssertTextContains { path, text }
                        if path == "sysArgToIfc.v" && text == "inout  [31 : 0]"
                )
            })
            .expect("late sysArgToIfc.v assertion");
        assert!(compile < assertion);

        let stale = scenario("compile-52-FourInoutBuses");
        assert!(stale
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .all(|operation| {
                !operation
                    .artifacts
                    .outputs
                    .contains(&"sysArgToIfc.v".to_owned())
                    && !matches!(
                        &operation.action,
                        Action::AssertTextContains { path, .. } if path == "sysArgToIfc.v"
                    )
            }));
    }

    #[test]
    fn inout_archives_the_final_unique_output_producers() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let script = manifest
            .scripts
            .iter()
            .find(|script| script.origin == INOUT_PLAN_ORIGIN)
            .unwrap();
        let plan = plan_from_script(root, script).plan;
        let operations = plan.scenarios[0]
            .stages
            .iter()
            .flat_map(|stage| &stage.operations)
            .collect::<Vec<_>>();
        let archives = operations
            .iter()
            .enumerate()
            .filter_map(|(index, operation)| match &operation.action {
                Action::FsMove {
                    source,
                    destination,
                } if destination.ends_with(".no-inline-inout") => {
                    Some((index, source.clone(), destination.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(archives.len(), 16);
        assert_eq!(
            archives
                .iter()
                .map(|(_, source, _)| source)
                .collect::<BTreeSet<_>>()
                .len(),
            16
        );
        for (archive_index, source, destination) in archives {
            assert_eq!(destination, format!("{source}.no-inline-inout"));
            let producers = operations
                .iter()
                .enumerate()
                .filter(|(_, operation)| operation.artifacts.outputs.contains(&source))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            assert!(!producers.is_empty(), "{source}");
            assert!(producers.last().unwrap() < &archive_index, "{source}");
            if source == "sysSimpleConnect1.v.out" {
                assert_eq!(producers.len(), 2);
            } else {
                assert_eq!(producers.len(), 1, "{source}");
            }
        }
    }

    #[test]
    fn marks_only_pinned_empty_upstream_scripts_as_disabled() {
        let root = project_root();
        let manifest = build_manifest(root).expect("build testsuite manifest");
        let generated = build_test_plans_from_manifest(root, &manifest).expect("build Test Plans");
        let summary = generated.summary();
        assert_eq!(summary.plans, 860);
        assert_eq!(summary.complete, 854);
        assert_eq!(summary.disabled, 3);
        assert_eq!(summary.blocked, 3);

        for (origin, sha256) in DISABLED_UPSTREAM_SCRIPTS {
            let script = manifest
                .scripts
                .iter()
                .find(|script| script.origin == *origin)
                .unwrap_or_else(|| panic!("missing disabled upstream script {origin}"));
            assert_eq!(
                &script.source_sha256, sha256,
                "unexpected source hash: {origin}"
            );

            let plan = plan_from_script(root, script).plan;
            assert_eq!(plan.status, PlanStatus::Disabled, "{origin}");
            assert!(plan.scenarios.is_empty(), "{origin}");
            assert!(matches!(
                plan.diagnostics.as_slice(),
                [ImportDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code,
                    ..
                }] if code == "import.disabled"
            ));

            let mut changed = script.clone();
            changed.source_sha256 = "0".repeat(64);
            let changed_plan = plan_from_script(root, &changed).plan;
            assert_eq!(changed_plan.status, PlanStatus::Blocked, "{origin}");
            assert!(matches!(
                changed_plan.diagnostics.as_slice(),
                [ImportDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    code,
                    ..
                }] if code == "import.empty"
            ));
        }
    }
}
