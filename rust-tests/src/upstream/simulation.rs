use super::{
    compare_legacy_golden, describe_exit, is_safe_relative, normalize_legacy_golden,
    reset_directory, GenerationStrategy, Requirement, RunPaths, SimulationBackend,
    SimulationContract, SimulationScenario, VcdExpectation,
};
use crate::cache::{hard_link_or_copy_directory_contents, CacheLookup, GenerationCache};
use crate::{readable_diff, run_bsc, run_command, Toolchain};
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
) -> Result<(), String> {
    let compile_arguments = simulation_compile_arguments(scenario);
    let compile_log = artifact_dir.join("compile.log");
    let fixture_root = toolchain.project_root.join(scenario.fixture_dir);
    let (generation_needed, cache_key) = match generation_cache.lookup(
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
    };
    if generation_needed {
        run_required_bsc_step(
            toolchain,
            &compile_arguments,
            work_dir,
            &compile_log,
            "generate simulation model",
            scenario.timeout,
        )?;
    }

    for backend in backends {
        for generated_file in generated_model_files(scenario, *backend) {
            if !work_dir.join(&generated_file).is_file() {
                return Err(format!(
                    "BSC did not generate {} for {}; see {}",
                    generated_file,
                    scenario.name,
                    compile_log.display()
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
) -> Result<(), String> {
    let (work_dir, artifact_dir) = run_paths.for_name(contract.name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;
    hard_link_or_copy_directory_contents(generation_dir, &work_dir)?;

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
    run_required_bsc_step(
        toolchain,
        &link_arguments,
        &work_dir,
        &artifact_dir.join("link.log"),
        "link simulation executable",
        scenario.timeout,
    )?;

    let mut executable = work_dir.join(scenario.top);
    if !executable.is_file() && cfg!(windows) {
        let windows_executable = executable.with_extension("exe");
        if windows_executable.is_file() {
            executable = windows_executable;
        }
    }
    if !executable.is_file() {
        return Err(format!(
            "BSC did not link simulation executable {}; see {}",
            executable.display(),
            artifact_dir.join("link.log").display()
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
        &work_dir,
        &simulation_log,
        scenario.timeout,
    )?;
    if !result.success {
        return Err(format!(
            "simulation for {} exited {}; see {}",
            contract.name,
            describe_exit(result.exit_code),
            simulation_log.display()
        ));
    }

    let normal_output = match contract.backend {
        SimulationBackend::Bluesim => result.output,
        SimulationBackend::Icarus => clean_iverilog_output(&result.output),
    };
    let mut output = normal_output.clone();
    if contract.sort_output {
        let mut lines: Vec<_> = output.lines().collect();
        lines.sort_unstable();
        output = lines.join("\n");
        if !output.is_empty() {
            output.push('\n');
        }
    }
    let output_path = work_dir.join("simulation.out");
    fs::write(&output_path, &output)
        .map_err(|error| format!("write simulation output {}: {error}", output_path.display()))?;
    compare_legacy_golden(
        &output,
        &work_dir.join(contract.expected),
        &output_path,
        &artifact_dir.join("golden.diff"),
    )?;
    run_vcd_contract(
        toolchain,
        scenario,
        contract,
        &work_dir,
        &artifact_dir,
        &executable,
        launcher,
        &normal_output,
    )
}

fn run_vcd_contract(
    toolchain: &Toolchain,
    scenario: &SimulationScenario,
    contract: &SimulationContract,
    work_dir: &Path,
    artifact_dir: &Path,
    executable: &Path,
    launcher: Option<&str>,
    normal_output: &str,
) -> Result<(), String> {
    if contract.vcd == VcdExpectation::None {
        return Ok(());
    }

    let vcd_name = format!("{}.vcd", scenario.top);
    let mut arguments =
        Vec::with_capacity(contract.simulation_options.len() + usize::from(launcher.is_some()) + 2);
    if launcher.is_some() {
        arguments.push(scenario.top);
    }
    match contract.vcd {
        VcdExpectation::None => unreachable!(),
        VcdExpectation::BluesimOutputMatchesNormal => {
            arguments.extend_from_slice(&["-V", &vcd_name]);
        }
        VcdExpectation::IcarusSmoke => arguments.push("+bscvcd"),
    }
    arguments.extend_from_slice(contract.simulation_options);

    let program = launcher.map_or(executable, Path::new);
    let log_path = artifact_dir.join("vcd-simulation.log");
    let result = run_command(
        toolchain,
        program,
        &arguments,
        work_dir,
        &log_path,
        scenario.timeout,
    )?;
    if !result.success {
        return Err(format!(
            "VCD simulation for {} exited {}; see {}",
            contract.name,
            describe_exit(result.exit_code),
            log_path.display()
        ));
    }

    let vcd_output = match contract.backend {
        SimulationBackend::Bluesim => result.output,
        SimulationBackend::Icarus => clean_iverilog_output(&result.output),
    };
    let output_path = artifact_dir.join("vcd-simulation.out");
    fs::write(&output_path, &vcd_output).map_err(|error| {
        format!(
            "write VCD simulation output {}: {error}",
            output_path.display()
        )
    })?;

    let generated_vcd = match contract.vcd {
        VcdExpectation::None => unreachable!(),
        VcdExpectation::BluesimOutputMatchesNormal => work_dir.join(&vcd_name),
        VcdExpectation::IcarusSmoke => work_dir.join("dump.vcd"),
    };
    let metadata = fs::metadata(&generated_vcd)
        .map_err(|error| format!("read generated VCD {}: {error}", generated_vcd.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(format!(
            "VCD simulation for {} did not generate a non-empty {}",
            contract.name,
            generated_vcd.display()
        ));
    }
    fs::copy(&generated_vcd, artifact_dir.join("simulation.vcd")).map_err(|error| {
        format!(
            "copy generated VCD {} into artifacts: {error}",
            generated_vcd.display()
        )
    })?;

    if contract.vcd == VcdExpectation::BluesimOutputMatchesNormal {
        let expected = normalize_legacy_golden(normal_output);
        let actual = normalize_legacy_golden(&vcd_output);
        if expected != actual {
            let diff_path = artifact_dir.join("vcd-output.diff");
            let diff = readable_diff(
                &expected,
                &actual,
                "normal Bluesim output",
                "VCD Bluesim output",
            );
            fs::write(&diff_path, diff).map_err(|error| {
                format!("write VCD output diff {}: {error}", diff_path.display())
            })?;
            return Err(format!(
                "VCD simulation changed output for {}; see {}",
                contract.name,
                diff_path.display()
            ));
        }
    }

    Ok(())
}

fn run_required_bsc_step(
    toolchain: &Toolchain,
    arguments: &[&str],
    work_dir: &Path,
    log_path: &Path,
    action: &str,
    timeout: std::time::Duration,
) -> Result<(), String> {
    let result = run_bsc(toolchain, arguments, work_dir, log_path, timeout)?;
    if result.success {
        Ok(())
    } else {
        Err(format!(
            "BSC failed to {action} {}; see {}",
            describe_exit(result.exit_code),
            log_path.display()
        ))
    }
}

pub(super) fn clean_iverilog_output(output: &str) -> String {
    output
        .lines()
        .filter(|line| {
            !line.starts_with("$readmem")
                && !(line.starts_with("WARNING:") && line.contains("$readmem"))
                && !line.contains("$finish")
                && !line.starts_with("VCD info")
        })
        .map(|line| format!("{line}\n"))
        .collect()
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
    if scenario.timeout.is_zero() {
        return Err(format!(
            "simulation scenario {} has a zero timeout",
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
    for contract in scenario.contracts {
        if contract.name.is_empty()
            || !contract_names.insert(contract.name)
            || !is_safe_relative(contract.expected)
            || !scenario.fixtures.contains(&contract.expected)
        {
            return Err(format!(
                "simulation scenario {} has an invalid contract {}",
                scenario.name, contract.name
            ));
        }
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
        let vcd_matches_backend = matches!(
            (contract.backend, contract.vcd),
            (_, VcdExpectation::None)
                | (
                    SimulationBackend::Bluesim,
                    VcdExpectation::BluesimOutputMatchesNormal
                )
                | (SimulationBackend::Icarus, VcdExpectation::IcarusSmoke)
        );
        if !vcd_matches_backend {
            return Err(format!(
                "simulation contract {} has a backend/VCD expectation mismatch",
                contract.name
            ));
        }
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
