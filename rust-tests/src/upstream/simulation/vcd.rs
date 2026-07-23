use super::super::{
    normalize_golden_output, SimulationBackend, SimulationContract, SimulationPhase,
    SimulationScenario, VcdOutputExpectation,
};
use super::outcome::{normalize_backend_output, PhaseFailure};
use crate::{readable_diff, run_command, Toolchain};
use std::fs;
use std::io::BufReader;
use std::path::Path;

pub(super) fn run_vcd_contract(
    toolchain: &Toolchain,
    scenario: &SimulationScenario,
    contract: &SimulationContract,
    work_dir: &Path,
    artifact_dir: &Path,
    executable: &Path,
    launcher: Option<&str>,
    normal_output: &str,
) -> Result<(), PhaseFailure> {
    let vcd_contract = contract.vcd.expect("validated VCD contract");
    let vcd_name = format!("{}.vcd", scenario.top);
    let mut arguments =
        Vec::with_capacity(contract.simulation_options.len() + usize::from(launcher.is_some()) + 2);
    if launcher.is_some() {
        arguments.push(scenario.top);
    }
    match contract.backend {
        SimulationBackend::Bluesim => arguments.extend_from_slice(&["-V", &vcd_name]),
        SimulationBackend::Icarus => arguments.push("+bscvcd"),
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
        scenario.timeouts.vcd,
    )
    .map_err(|error| PhaseFailure::new(SimulationPhase::Vcd, error))?;
    if !result.success {
        return Err(PhaseFailure::command(
            SimulationPhase::Vcd,
            "VCD simulation",
            result,
            &log_path,
        ));
    }

    let vcd_output = normalize_backend_output(contract.backend, &result.output);
    let output_path = artifact_dir.join("vcd-simulation.out");
    fs::write(&output_path, &vcd_output).map_err(|error| {
        PhaseFailure::new(
            SimulationPhase::Vcd,
            format!(
                "write VCD simulation output {}: {error}",
                output_path.display()
            ),
        )
    })?;

    let generated_vcd = match contract.backend {
        SimulationBackend::Bluesim => work_dir.join(&vcd_name),
        SimulationBackend::Icarus => work_dir.join("dump.vcd"),
    };
    validate_vcd(&generated_vcd).map_err(|message| PhaseFailure {
        phase: SimulationPhase::Vcd,
        message,
        output: Some(vcd_output.clone()),
    })?;
    fs::copy(&generated_vcd, artifact_dir.join("simulation.vcd")).map_err(|error| {
        PhaseFailure::new(
            SimulationPhase::Vcd,
            format!(
                "copy generated VCD {} into artifacts: {error}",
                generated_vcd.display()
            ),
        )
    })?;

    if vcd_contract.output == VcdOutputExpectation::MatchesNormal {
        let expected = normalize_golden_output(normal_output);
        let actual = normalize_golden_output(&vcd_output);
        if expected != actual {
            let diff_path = artifact_dir.join("vcd-output.diff");
            let diff = readable_diff(
                &expected,
                &actual,
                "normal simulation output",
                "VCD simulation output",
            );
            fs::write(&diff_path, diff).map_err(|error| {
                PhaseFailure::new(
                    SimulationPhase::Vcd,
                    format!("write VCD output diff {}: {error}", diff_path.display()),
                )
            })?;
            return Err(PhaseFailure {
                phase: SimulationPhase::Vcd,
                message: format!(
                    "VCD simulation changed output for {}; see {}",
                    contract.name,
                    diff_path.display()
                ),
                output: Some(vcd_output),
            });
        }
    }

    Ok(())
}

pub(crate) fn validate_vcd(path: &Path) -> Result<(), String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("open generated VCD {}: {error}", path.display()))?;
    let mut parser = vcd::Parser::new(BufReader::new(file));
    let header = parser
        .parse_header()
        .map_err(|error| format!("parse VCD header {}: {error}", path.display()))?;
    if count_vcd_variables(&header.items) == 0 {
        return Err(format!(
            "generated VCD {} declares no signals",
            path.display()
        ));
    }
    for command in parser {
        command.map_err(|error| format!("parse VCD body {}: {error}", path.display()))?;
    }
    Ok(())
}

fn count_vcd_variables(items: &[vcd::ScopeItem]) -> usize {
    items
        .iter()
        .map(|item| match item {
            vcd::ScopeItem::Scope(scope) => count_vcd_variables(&scope.items),
            vcd::ScopeItem::Var(_) => 1,
            _ => 0,
        })
        .sum()
}
