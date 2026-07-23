use super::super::artifact::{check_artifact_assertions, compare_golden_output};
use super::super::{
    describe_exit, ExpectedOutcome, OutputNormalization, SimulationBackend, SimulationContract,
    SimulationPhase,
};
use crate::CommandResult;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct PhaseFailure {
    pub phase: SimulationPhase,
    pub message: String,
    pub output: Option<String>,
}

impl PhaseFailure {
    pub(crate) fn new(phase: SimulationPhase, message: String) -> Self {
        Self {
            phase,
            message,
            output: None,
        }
    }

    pub(crate) fn command(
        phase: SimulationPhase,
        action: &str,
        result: CommandResult,
        log_path: &Path,
    ) -> Self {
        Self {
            phase,
            message: format!(
                "{action} exited {}; see {}",
                describe_exit(result.exit_code),
                log_path.display()
            ),
            output: Some(result.output),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContractRunOutcome {
    Passed,
    XFailed(String),
}

pub(crate) fn evaluate_contract_outcome(
    contract: &SimulationContract,
    execution: Result<(), PhaseFailure>,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<ContractRunOutcome, String> {
    match (contract.expectation, execution) {
        (ExpectedOutcome::Pass { .. }, Ok(())) => Ok(ContractRunOutcome::Passed),
        (ExpectedOutcome::Pass { .. }, Err(failure)) => Err(failure.message),
        (
            ExpectedOutcome::Fail {
                phase: expected_phase,
                output: expected_output,
            },
            Err(failure),
        ) if failure.phase == expected_phase => {
            if let Some(expected_output) = expected_output {
                let actual = failure.output.as_deref().ok_or_else(|| {
                    format!(
                        "expected {} failure output for {}, but the failure produced no output: {}",
                        expected_phase.as_str(),
                        contract.name,
                        failure.message
                    )
                })?;
                let actual = normalize_phase_output(
                    failure.phase,
                    contract.backend,
                    contract.output,
                    actual,
                );
                let actual_path = artifact_dir.join("expected-failure.out");
                fs::write(&actual_path, &actual).map_err(|error| {
                    format!(
                        "write expected failure output {}: {error}",
                        actual_path.display()
                    )
                })?;
                compare_golden_output(
                    &actual,
                    &work_dir.join(expected_output),
                    &actual_path,
                    &artifact_dir.join("expected-failure.diff"),
                )?;
            }
            check_artifact_assertions(contract.assertions, work_dir, artifact_dir, contract.name)?;
            Ok(ContractRunOutcome::Passed)
        }
        (
            ExpectedOutcome::XFail {
                phase: expected_phase,
                reason,
            },
            Err(failure),
        ) if failure.phase == expected_phase => Ok(ContractRunOutcome::XFailed(format!(
            "{reason}: {}",
            failure.message
        ))),
        (expectation, Ok(())) => Err(format!(
            "XPASS: {} was expected to fail during {}",
            contract.name,
            expectation
                .expected_failure_phase()
                .expect("non-pass expectation has a failure phase")
                .as_str()
        )),
        (expectation, Err(failure)) => Err(format!(
            "{} failed during {}, but was expected to fail during {}: {}",
            contract.name,
            failure.phase.as_str(),
            expectation
                .expected_failure_phase()
                .expect("non-pass expectation has a failure phase")
                .as_str(),
            failure.message
        )),
    }
}

fn normalize_phase_output(
    phase: SimulationPhase,
    backend: SimulationBackend,
    normalization: OutputNormalization,
    output: &str,
) -> String {
    let output = if matches!(phase, SimulationPhase::Simulation | SimulationPhase::Vcd) {
        normalize_backend_output(backend, output)
    } else {
        output.to_owned()
    };
    normalize_contract_output(normalization, &output)
}

pub(crate) fn normalize_backend_output(backend: SimulationBackend, output: &str) -> String {
    match backend {
        SimulationBackend::Bluesim => output.to_owned(),
        SimulationBackend::Icarus => clean_iverilog_output(output),
    }
}

pub(crate) fn normalize_contract_output(
    normalization: OutputNormalization,
    output: &str,
) -> String {
    match normalization {
        OutputNormalization::Preserve => output.to_owned(),
        OutputNormalization::SortedLines => {
            let mut lines: Vec<_> = output.lines().collect();
            lines.sort_unstable();
            let mut output = lines.join("\n");
            if !output.is_empty() {
                output.push('\n');
            }
            output
        }
    }
}

pub(crate) fn clean_iverilog_output(output: &str) -> String {
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
