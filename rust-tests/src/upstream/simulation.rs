mod outcome;
mod vcd;

#[cfg(test)]
pub(super) use self::outcome::clean_iverilog_output;
use self::outcome::normalize_backend_output;
pub(super) use self::outcome::{
    evaluate_contract_outcome, normalize_contract_output, ContractRunOutcome, PhaseFailure,
};
use self::vcd::run_vcd_contract;
#[cfg(test)]
pub(super) use self::vcd::validate_vcd;
use super::artifact::{
    check_artifact_assertions, compare_golden_output, validate_artifact_assertions,
};
use super::{
    is_safe_relative, reset_directory, ExpectedOutcome, GenerationStrategy, Requirement, RunPaths,
    SimulationBackend, SimulationContract, SimulationPhase, SimulationScenario,
};
use crate::cache::{hard_link_or_copy_directory_contents, CacheLookup, GenerationCache};
use crate::{run_bsc, run_command, Toolchain};
use std::fs;
use std::path::Path;

fn simulation_compile_arguments(scenario: &SimulationScenario) -> Vec<&str> {
    let mut arguments = Vec::with_capacity(scenario.compile_options.len() + 9);
    arguments.extend_from_slice(scenario.compile_options);
    arguments.extend_from_slice(&["-no-show-timestamps", "-no-show-version", "-u"]);
    match scenario.generation {
        GenerationStrategy::BackendSpecific(SimulationBackend::Bluesim) => arguments.push("-sim"),
        GenerationStrategy::BackendSpecific(SimulationBackend::Icarus) => {
            arguments.push("-verilog")
        }
        GenerationStrategy::SharedElaboration => {
            arguments.push("-verilog");
            if !scenario.compile_options.contains(&"-elab") {
                arguments.push("-elab");
            }
        }
    }
    arguments.extend_from_slice(&["-g", scenario.top, scenario.source]);
    arguments
}

fn generated_model_files(scenario: &SimulationScenario, backend: SimulationBackend) -> Vec<String> {
    let extension = match backend {
        SimulationBackend::Bluesim => "ba",
        SimulationBackend::Icarus => "v",
    };
    std::iter::once(scenario.top)
        .chain(scenario.generated_modules.iter().copied())
        .map(|module| format!("{module}.{extension}"))
        .collect()
}

pub(super) fn ensure_simulation_generation(
    toolchain: &Toolchain,
    generation_cache: &GenerationCache,
    scenario: &SimulationScenario,
    backends: &[SimulationBackend],
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), PhaseFailure> {
    let compile_arguments = simulation_compile_arguments(scenario);
    let compile_log = artifact_dir.join("compile.log");
    let fixture_root = toolchain.project_root.join(scenario.fixture_dir);
    let cache_allowed = scenario.contracts.iter().all(|contract| {
        contract.expectation.expected_failure_phase() != Some(SimulationPhase::Generation)
    });
    let (generation_needed, cache_key) = if cache_allowed {
        match generation_cache.lookup(
            &fixture_root,
            scenario.fixtures,
            &compile_arguments,
            work_dir,
            &compile_log,
        ) {
            Ok(CacheLookup::Hit) => (false, None),
            Ok(CacheLookup::Miss(key)) => (true, Some(key)),
            Ok(CacheLookup::Disabled) => (true, None),
            Err(error) => {
                eprintln!(
                    "warning: generation cache lookup failed for {}: {error}",
                    scenario.name
                );
                (true, None)
            }
        }
    } else {
        (true, None)
    };
    if generation_needed {
        let result = run_bsc(
            toolchain,
            &compile_arguments,
            work_dir,
            &compile_log,
            scenario.timeouts.generation,
        )
        .map_err(|error| PhaseFailure::new(SimulationPhase::Generation, error))?;
        if !result.success {
            return Err(PhaseFailure::command(
                SimulationPhase::Generation,
                "simulation model generation",
                result,
                &compile_log,
            ));
        }
    }

    for backend in backends {
        for generated_file in generated_model_files(scenario, *backend) {
            if !work_dir.join(&generated_file).is_file() {
                return Err(PhaseFailure::new(
                    SimulationPhase::Generation,
                    format!(
                        "BSC did not generate {} for {}; see {}",
                        generated_file,
                        scenario.name,
                        compile_log.display()
                    ),
                ));
            }
        }
    }
    if let Some(key) = cache_key {
        if let Err(error) = generation_cache.store(&key, work_dir) {
            eprintln!(
                "warning: generation cache store failed for {}: {error}",
                scenario.name
            );
        }
    }
    Ok(())
}

pub(super) fn run_simulation_contract(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    scenario: &SimulationScenario,
    contract: &SimulationContract,
    generation_dir: &Path,
) -> Result<ContractRunOutcome, String> {
    let (work_dir, artifact_dir) = run_paths.for_name(contract.name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;
    hard_link_or_copy_directory_contents(generation_dir, &work_dir)?;

    let execution =
        execute_simulation_contract(toolchain, scenario, contract, &work_dir, &artifact_dir);
    evaluate_contract_outcome(contract, execution, &work_dir, &artifact_dir)
}

pub(super) fn evaluate_generation_outcome(
    contract: &SimulationContract,
    execution: Result<(), PhaseFailure>,
    generation_dir: &Path,
    artifact_dir: &Path,
) -> Result<ContractRunOutcome, String> {
    reset_directory(artifact_dir)?;
    evaluate_contract_outcome(contract, execution, generation_dir, artifact_dir)
}

fn execute_simulation_contract(
    toolchain: &Toolchain,
    scenario: &SimulationScenario,
    contract: &SimulationContract,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), PhaseFailure> {
    let generated = generated_model_files(scenario, contract.backend);
    let mut link_arguments = Vec::with_capacity(contract.link_options.len() + 10);
    link_arguments.extend_from_slice(&["-no-show-timestamps", "-no-show-version"]);
    match contract.backend {
        SimulationBackend::Bluesim => link_arguments.push("-sim"),
        SimulationBackend::Icarus => {
            link_arguments.extend_from_slice(&["-verilog", "-vsim", "iverilog"]);
        }
    }
    link_arguments.extend_from_slice(&["-e", scenario.top, "-o", scenario.top]);
    link_arguments.extend_from_slice(contract.link_options);
    link_arguments.extend(generated.iter().map(String::as_str));
    let link_log = artifact_dir.join("link.log");
    let result = run_bsc(
        toolchain,
        &link_arguments,
        work_dir,
        &link_log,
        scenario.timeouts.link,
    )
    .map_err(|error| PhaseFailure::new(SimulationPhase::Link, error))?;
    if !result.success {
        return Err(PhaseFailure::command(
            SimulationPhase::Link,
            "simulation executable link",
            result,
            &link_log,
        ));
    }
    if contract.expectation.expected_failure_phase() == Some(SimulationPhase::Link) {
        return Ok(());
    }

    let mut executable = work_dir.join(scenario.top);
    if !executable.is_file() && cfg!(windows) {
        let windows_executable = executable.with_extension("exe");
        if windows_executable.is_file() {
            executable = windows_executable;
        }
    }
    if !executable.is_file() {
        return Err(PhaseFailure::new(
            SimulationPhase::Link,
            format!(
                "BSC did not link simulation executable {}; see {}",
                executable.display(),
                link_log.display()
            ),
        ));
    }

    let launcher = if cfg!(windows) && executable.extension().is_none() {
        Some(match contract.backend {
            SimulationBackend::Bluesim => "sh",
            SimulationBackend::Icarus => "vvp",
        })
    } else {
        None
    };
    let mut simulation_arguments =
        Vec::with_capacity(contract.simulation_options.len() + usize::from(launcher.is_some()) + 1);
    if launcher.is_some() {
        simulation_arguments.push(scenario.top);
    }
    if contract.backend == SimulationBackend::Icarus {
        simulation_arguments.push("-vcd-none");
    }
    simulation_arguments.extend_from_slice(contract.simulation_options);
    let simulation_log = artifact_dir.join("simulation.log");
    let program = launcher.map_or(executable.as_path(), Path::new);
    let result = run_command(
        toolchain,
        program,
        &simulation_arguments,
        work_dir,
        &simulation_log,
        scenario.timeouts.simulation,
    )
    .map_err(|error| PhaseFailure::new(SimulationPhase::Simulation, error))?;
    if !result.success {
        return Err(PhaseFailure::command(
            SimulationPhase::Simulation,
            "simulation",
            result,
            &simulation_log,
        ));
    }
    if contract.expectation.expected_failure_phase() == Some(SimulationPhase::Simulation) {
        return Ok(());
    }

    let normal_output = normalize_backend_output(contract.backend, &result.output);
    let output = normalize_contract_output(contract.output, &normal_output);
    let output_path = work_dir.join("simulation.out");
    fs::write(&output_path, &output).map_err(|error| {
        PhaseFailure::new(
            SimulationPhase::Simulation,
            format!("write simulation output {}: {error}", output_path.display()),
        )
    })?;
    if let ExpectedOutcome::Pass { output: expected } = contract.expectation {
        compare_golden_output(
            &output,
            &work_dir.join(expected),
            &output_path,
            &artifact_dir.join("golden.diff"),
        )
        .map_err(|message| PhaseFailure {
            phase: SimulationPhase::Simulation,
            message,
            output: Some(output.clone()),
        })?;
    }

    if contract.vcd.is_some() {
        run_vcd_contract(
            toolchain,
            scenario,
            contract,
            work_dir,
            artifact_dir,
            &executable,
            launcher,
            &normal_output,
        )?;
    }
    if contract.expectation.expected_failure_phase() == Some(SimulationPhase::Vcd) {
        return Ok(());
    }

    check_artifact_assertions(contract.assertions, work_dir, artifact_dir, contract.name).map_err(
        |message| PhaseFailure {
            phase: SimulationPhase::Simulation,
            message,
            output: Some(output),
        },
    )
}

pub(crate) fn validate_simulation_scenario(scenario: &SimulationScenario) -> Result<(), String> {
    if scenario.name.is_empty()
        || !is_safe_relative(scenario.fixture_dir)
        || !is_safe_relative(scenario.source)
        || scenario.top.is_empty()
        || scenario
            .generated_modules
            .iter()
            .any(|module| module.is_empty())
    {
        return Err(format!(
            "simulation scenario {} contains an empty name or unsafe path",
            scenario.name
        ));
    }
    let canonical_prefix = scenario
        .fixture_dir
        .strip_prefix("testsuite/")
        .map(|origin| format!("{origin}::"))
        .ok_or_else(|| {
            format!(
                "simulation scenario {} has a non-testsuite fixture root",
                scenario.name
            )
        })?;
    if !scenario.name.starts_with(&canonical_prefix) {
        return Err(format!(
            "simulation scenario {} must use canonical prefix {canonical_prefix}",
            scenario.name
        ));
    }
    if scenario
        .generated_modules
        .iter()
        .enumerate()
        .any(|(index, module)| {
            *module == scenario.top || scenario.generated_modules[..index].contains(module)
        })
    {
        return Err(format!(
            "simulation scenario {} contains duplicate generated modules",
            scenario.name
        ));
    }
    if [
        scenario.timeouts.generation,
        scenario.timeouts.link,
        scenario.timeouts.simulation,
        scenario.timeouts.vcd,
    ]
    .iter()
    .any(|timeout| timeout.is_zero())
    {
        return Err(format!(
            "simulation scenario {} has a zero phase timeout",
            scenario.name
        ));
    }
    if scenario.contracts.is_empty() {
        return Err(format!(
            "simulation scenario {} has no contracts",
            scenario.name
        ));
    }
    if !scenario.fixtures.contains(&scenario.source)
        || scenario
            .fixtures
            .iter()
            .any(|fixture| !is_safe_relative(fixture))
    {
        return Err(format!(
            "simulation scenario {} must declare a safe source and fixtures",
            scenario.name
        ));
    }

    let mut contract_names = std::collections::BTreeSet::new();
    let mut generation_failure_contracts = 0;
    for contract in scenario.contracts {
        let expected_output = contract.expectation.expected_output();
        if contract.name.is_empty()
            || !contract.name.starts_with(&canonical_prefix)
            || !contract_names.insert(contract.name)
            || expected_output.is_some_and(|output| {
                !is_safe_relative(output) || !scenario.fixtures.contains(&output)
            })
        {
            return Err(format!(
                "simulation scenario {} has an invalid contract {}",
                scenario.name, contract.name
            ));
        }
        if matches!(
            contract.expectation,
            ExpectedOutcome::XFail { reason: "", .. }
        ) {
            return Err(format!(
                "simulation contract {} has an empty XFAIL reason",
                contract.name
            ));
        }
        let expected_failure_phase = contract.expectation.expected_failure_phase();
        if expected_failure_phase == Some(SimulationPhase::Generation) {
            generation_failure_contracts += 1;
        }
        if expected_failure_phase == Some(SimulationPhase::Vcd) && contract.vcd.is_none() {
            return Err(format!(
                "simulation contract {} expects a VCD failure without enabling VCD",
                contract.name
            ));
        }
        validate_artifact_assertions(contract.assertions, scenario.fixtures, contract.name)?;
        let requirement_matches_backend = match contract.backend {
            SimulationBackend::Bluesim => contract.requirement == Requirement::BluesimEnabled,
            SimulationBackend::Icarus => matches!(
                contract.requirement,
                Requirement::VerilogEnabled | Requirement::IcarusAtLeast(_)
            ),
        };
        if !requirement_matches_backend {
            return Err(format!(
                "simulation contract {} has a backend/requirement mismatch",
                contract.name
            ));
        }
    }
    if generation_failure_contracts != 0 && generation_failure_contracts != scenario.contracts.len()
    {
        return Err(format!(
            "simulation scenario {} mixes generation-failure and generation-success contracts",
            scenario.name
        ));
    }

    match scenario.generation {
        GenerationStrategy::BackendSpecific(backend) => {
            if scenario.contracts.len() != 1 || scenario.contracts[0].backend != backend {
                return Err(format!(
                    "backend-specific scenario {} must contain exactly one matching contract",
                    scenario.name
                ));
            }
        }
        GenerationStrategy::SharedElaboration => {
            let bluesim = scenario
                .contracts
                .iter()
                .filter(|contract| contract.backend == SimulationBackend::Bluesim)
                .count();
            let icarus = scenario
                .contracts
                .iter()
                .filter(|contract| contract.backend == SimulationBackend::Icarus)
                .count();
            if bluesim != 1 || icarus != 1 || scenario.contracts.len() != 2 {
                return Err(format!(
                    "shared elaboration scenario {} must contain one Bluesim and one Icarus contract",
                    scenario.name
                ));
            }
        }
    }
    Ok(())
}
