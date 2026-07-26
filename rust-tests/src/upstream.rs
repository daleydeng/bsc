use crate::{normalize_diff_b_text, Toolchain};
use regex::Regex;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path};
use std::process::Command;
use std::sync::OnceLock;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
    Warning,
}

impl DiagnosticKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileExpectation {
    Pass,
    PassWithDiagnostic {
        kind: DiagnosticKind,
        tag: &'static str,
        count: usize,
    },
    Fail,
    FailWithDiagnostic {
        kind: DiagnosticKind,
        tag: &'static str,
        count: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoldenExpectation {
    pub expected: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAssertion {
    Contains {
        text: &'static str,
    },
    DoesNotContain {
        text: &'static str,
    },
    LineCount {
        text: &'static str,
        count: usize,
    },
    Regex {
        pattern: &'static str,
    },
    RegexDoesNotMatch {
        pattern: &'static str,
    },
    RegexCount {
        pattern: &'static str,
        count: usize,
    },
    DiagnosticCount {
        kind: DiagnosticKind,
        tag: &'static str,
        count: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactNormalization {
    Exact,
    GoldenOutput,
    Verilog,
    DecimalTolerance {
        fractional_digits: u8,
        max_units: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactAssertion {
    Exists {
        path: &'static str,
    },
    Text {
        path: &'static str,
        assertion: TextAssertion,
    },
    ParsesAsSystemVerilog {
        path: &'static str,
    },
    Matches {
        actual: &'static str,
        expected: &'static str,
        normalization: ArtifactNormalization,
    },
}

impl ArtifactAssertion {
    pub fn actual_path(self) -> &'static str {
        match self {
            Self::Exists { path }
            | Self::Text { path, .. }
            | Self::ParsesAsSystemVerilog { path } => path,
            Self::Matches { actual, .. } => actual,
        }
    }

    pub fn expected_path(self) -> Option<&'static str> {
        match self {
            Self::Matches { expected, .. } => Some(expected),
            Self::Exists { .. } | Self::Text { .. } | Self::ParsesAsSystemVerilog { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileMode {
    Frontend,
    Verilog { module: Option<&'static str> },
    VerilogSchedule { module: Option<&'static str> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    Always,
    BluesimEnabled,
    VerilogEnabled,
    IcarusAtLeast(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunnerPolicy {
    pub bluesim_enabled: bool,
    pub verilog_enabled: bool,
    pub iverilog_major: Option<u32>,
}

impl RunnerPolicy {
    pub fn new(bluesim_enabled: bool, verilog_enabled: bool) -> Self {
        Self {
            bluesim_enabled,
            verilog_enabled,
            iverilog_major: None,
        }
    }

    pub fn with_iverilog_major(mut self, major: Option<u32>) -> Self {
        self.iverilog_major = major;
        self
    }

    fn skip_reason(self, requirement: Requirement) -> Option<String> {
        match requirement {
            Requirement::Always => None,
            Requirement::BluesimEnabled if self.bluesim_enabled => None,
            Requirement::BluesimEnabled => {
                Some("Bluesim backend disabled by --no-bluesim".to_owned())
            }
            Requirement::VerilogEnabled if self.verilog_enabled => None,
            Requirement::VerilogEnabled => {
                Some("Verilog backend disabled by --no-verilog".to_owned())
            }
            Requirement::IcarusAtLeast(_) if !self.verilog_enabled => {
                Some("Verilog backend disabled by --no-verilog".to_owned())
            }
            Requirement::IcarusAtLeast(required) => match self.iverilog_major {
                Some(actual) if actual >= required => None,
                Some(actual) => Some(format!(
                    "Icarus Verilog {actual} is older than required version {required}"
                )),
                None => Some(format!(
                    "Icarus Verilog version could not be determined (requires >= {required})"
                )),
            },
        }
    }
}

impl Default for RunnerPolicy {
    fn default() -> Self {
        Self::new(true, true)
    }
}

pub fn probe_iverilog_major() -> Option<u32> {
    let output = Command::new("iverilog").arg("-V").output().ok()?;
    parse_iverilog_major(&String::from_utf8_lossy(&output.stdout))
        .or_else(|| parse_iverilog_major(&String::from_utf8_lossy(&output.stderr)))
}

fn parse_iverilog_major(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let version = line.trim().strip_prefix("Icarus Verilog version ")?;
        version.split('.').next()?.parse().ok()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileCase {
    pub name: &'static str,
    pub fixture_dir: &'static str,
    pub source: &'static str,
    pub fixtures: &'static [&'static str],
    pub assertions: &'static [ArtifactAssertion],
    pub expectation: CompileExpectation,
    pub golden: Option<GoldenExpectation>,
    pub options: &'static [&'static str],
    pub nodeps: bool,
    pub mode: CompileMode,
    pub requirement: Requirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationBackend {
    Bluesim,
    Icarus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationPhase {
    Generation,
    Link,
    Simulation,
    OutputComparison,
    Vcd,
}

impl SimulationPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generation => "generation",
            Self::Link => "link",
            Self::Simulation => "simulation",
            Self::OutputComparison => "output comparison",
            Self::Vcd => "VCD simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedOutcome {
    Pass {
        output: &'static str,
    },
    Fail {
        phase: SimulationPhase,
        output: Option<&'static str>,
    },
    XFail {
        phase: SimulationPhase,
        reason: &'static str,
    },
    XFailOutput {
        output: &'static str,
        reason: &'static str,
    },
}

impl ExpectedOutcome {
    pub const fn expected_failure_phase(self) -> Option<SimulationPhase> {
        match self {
            Self::Pass { .. } => None,
            Self::Fail { phase, .. } | Self::XFail { phase, .. } => Some(phase),
            Self::XFailOutput { .. } => Some(SimulationPhase::OutputComparison),
        }
    }

    pub const fn expected_output(self) -> Option<&'static str> {
        match self {
            Self::Pass { output } | Self::XFailOutput { output, .. } => Some(output),
            Self::Fail { output, .. } => output,
            Self::XFail { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputNormalization {
    Preserve,
    SortedLines,
    MaskedLines { prefix: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcdOutputExpectation {
    ParseOnly,
    MatchesNormal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VcdContract {
    pub output: VcdOutputExpectation,
}

impl VcdContract {
    pub const fn parse() -> Self {
        Self {
            output: VcdOutputExpectation::ParseOnly,
        }
    }

    pub const fn output_matches_normal() -> Self {
        Self {
            output: VcdOutputExpectation::MatchesNormal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationTimeouts {
    pub generation: std::time::Duration,
    pub link: std::time::Duration,
    pub simulation: std::time::Duration,
    pub vcd: std::time::Duration,
}

impl SimulationTimeouts {
    pub const fn uniform(timeout: std::time::Duration) -> Self {
        Self {
            generation: timeout,
            link: timeout,
            simulation: timeout,
            vcd: timeout,
        }
    }

    pub const fn with_generation(mut self, timeout: std::time::Duration) -> Self {
        self.generation = timeout;
        self
    }

    pub const fn with_link(mut self, timeout: std::time::Duration) -> Self {
        self.link = timeout;
        self
    }

    pub const fn with_simulation(mut self, timeout: std::time::Duration) -> Self {
        self.simulation = timeout;
        self
    }

    pub const fn with_vcd(mut self, timeout: std::time::Duration) -> Self {
        self.vcd = timeout;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStrategy {
    BackendSpecific(SimulationBackend),
    SharedElaboration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceClass {
    Normal,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactTransferOperation {
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactTransfer {
    pub operation: ArtifactTransferOperation,
    pub source: &'static str,
    pub destination: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BluesimGeneration {
    pub source: &'static str,
    pub module: Option<&'static str>,
    pub options: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BluesimLink {
    pub objects: &'static [&'static str],
    pub top: &'static str,
    pub options: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BluesimWorkflowRun {
    pub name: &'static str,
    pub options: &'static [&'static str],
    pub stdout: &'static str,
    pub transfers: &'static [ArtifactTransfer],
    pub assertions: &'static [ArtifactAssertion],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BluesimWorkflowScenario {
    pub name: &'static str,
    pub fixture_dir: &'static str,
    pub fixtures: &'static [&'static str],
    pub generations: &'static [BluesimGeneration],
    pub link: BluesimLink,
    pub link_assertions: &'static [ArtifactAssertion],
    pub runs: &'static [BluesimWorkflowRun],
    pub timeouts: SimulationTimeouts,
    pub resource: ResourceClass,
    pub requirement: Requirement,
}

impl BluesimWorkflowScenario {
    pub const fn contract_count(&self) -> usize {
        if self.runs.is_empty() {
            1
        } else {
            self.runs.len()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationLinkInput {
    GeneratedModule(&'static str),
    ExactFile(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationScenario {
    pub name: &'static str,
    pub fixture_dir: &'static str,
    pub source: &'static str,
    pub fixtures: &'static [&'static str],
    pub top: &'static str,
    pub link_inputs: &'static [SimulationLinkInput],
    pub compile_options: &'static [&'static str],
    pub generation: GenerationStrategy,
    pub timeouts: SimulationTimeouts,
    pub resource: ResourceClass,
    pub contracts: &'static [SimulationContract],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationContract {
    pub name: &'static str,
    pub assertions: &'static [ArtifactAssertion],
    pub link_options: &'static [&'static str],
    pub simulation_options: &'static [&'static str],
    pub expectation: ExpectedOutcome,
    pub output: OutputNormalization,
    pub backend: SimulationBackend,
    pub vcd: Option<VcdContract>,
    pub requirement: Requirement,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CaseModule<C: 'static> {
    pub name: &'static str,
    pub cases: &'static [C],
}

#[derive(Debug, Default)]
pub struct ExecutionPlan {
    pub(super) compile_cases: Vec<CompileCase>,
    pub(super) simulations: Vec<PlannedSimulation>,
    pub(super) bluesim_workflows: Vec<PlannedBluesimWorkflow>,
}

impl ExecutionPlan {
    pub fn contract_count(&self) -> usize {
        self.compile_cases.len()
            + self
                .simulations
                .iter()
                .map(|simulation| simulation.contracts.len())
                .sum::<usize>()
            + self
                .bluesim_workflows
                .iter()
                .map(|workflow| {
                    if workflow.scenario.runs.is_empty() {
                        1
                    } else {
                        workflow.runs.len()
                    }
                })
                .sum::<usize>()
    }

    pub fn contract_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.compile_cases
            .iter()
            .map(|case| case.name)
            .chain(
                self.simulations.iter().flat_map(|simulation| {
                    simulation.contracts.iter().map(|contract| contract.name)
                }),
            )
            .chain(self.bluesim_workflows.iter().flat_map(|workflow| {
                workflow.runs.iter().map(|run| run.name).chain(
                    workflow
                        .scenario
                        .runs
                        .is_empty()
                        .then_some(workflow.scenario.name),
                )
            }))
    }
}

#[derive(Debug)]
pub(super) struct PlannedSimulation {
    pub scenario: &'static SimulationScenario,
    pub contracts: Vec<&'static SimulationContract>,
}

#[derive(Debug)]
pub(super) struct PlannedBluesimWorkflow {
    pub scenario: &'static BluesimWorkflowScenario,
    pub runs: Vec<&'static BluesimWorkflowRun>,
}

mod artifact;
mod bluesim_workflow;
mod cases_bluesim_workflow;
mod cases_compile;
mod cases_simulation;
mod compile;
mod runner;
mod simulation;

#[cfg(test)]
use artifact::{check_artifact_assertions, check_text_assertion, validate_artifact_assertions};
pub(crate) use bluesim_workflow::validate_bluesim_workflow;
#[cfg(test)]
use bluesim_workflow::{
    generation_arguments as bluesim_generation_arguments, link_arguments as bluesim_link_arguments,
    normalized_link_objects,
};
#[cfg(test)]
use compile::{compile_arguments, validate_case};
#[cfg(test)]
use runner::run_fixed_queue;
pub use runner::{run_plan, summarize_outcomes, CaseOutcome, CaseResult, RunPaths, RunSummary};
pub(crate) use simulation::validate_simulation_scenario;
#[cfg(test)]
use simulation::{
    clean_iverilog_output, evaluate_contract_outcome, expected_generated_files,
    normalize_contract_output, simulation_link_files, validate_vcd, ContractRunOutcome,
    PhaseFailure,
};

pub fn compile_cases() -> &'static [CompileCase] {
    cases_compile::cases()
}

pub fn simulation_scenarios() -> &'static [SimulationScenario] {
    cases_simulation::scenarios()
}

pub fn bluesim_workflow_scenarios() -> &'static [BluesimWorkflowScenario] {
    cases_bluesim_workflow::scenarios()
}

pub(crate) fn bluesim_workflow_scenario_modules() -> &'static [CaseModule<BluesimWorkflowScenario>]
{
    cases_bluesim_workflow::MODULES
}

pub(crate) fn compile_case_modules() -> &'static [CaseModule<CompileCase>] {
    cases_compile::MODULES
}

pub(crate) fn simulation_scenario_modules() -> &'static [CaseModule<SimulationScenario>] {
    cases_simulation::MODULES
}

pub fn count_diagnostics(output: &str, kind: DiagnosticKind, tag: &str) -> usize {
    let marker = format!("{}:", kind.as_str());
    let suffix = format!("({tag})");
    output
        .lines()
        .filter(|raw_line| {
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
            if !line.ends_with(&suffix) {
                return false;
            }
            line.find(&marker).is_some_and(|start| {
                let message_start = start + marker.len();
                let tag_start = line.len() - suffix.len();
                message_start < tag_start
            })
        })
        .count()
}

fn normalize_windows_scientific_exponents(text: &str) -> String {
    static WINDOWS_EXPONENT: OnceLock<Regex> = OnceLock::new();
    let pattern = WINDOWS_EXPONENT.get_or_init(|| {
        Regex::new(r"([0-9][eE][+-])0([0-9]{2})([^0-9]|$)")
            .expect("Windows scientific exponent regex is valid")
    });
    pattern.replace_all(text, "${1}${2}${3}").into_owned()
}

pub fn normalize_golden_output(text: &str) -> String {
    let normalized_newlines = text.replace("\r\n", "\n").replace('\r', "\n");
    let normalized_newlines = normalize_windows_scientific_exponents(&normalized_newlines);
    let mut filtered = String::with_capacity(normalized_newlines.len());
    for line in normalized_newlines.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let isolated_dependency_progress = trimmed.starts_with("compiling ./");
        if !line.contains("SystemC")
            && !line.contains("dumpfile parameter")
            && !isolated_dependency_progress
        {
            filtered.push_str(line);
        }
    }
    let normalized = normalize_diff_b_text(&filtered);
    normalized
        .strip_suffix('\n')
        .unwrap_or(&normalized)
        .to_owned()
}

fn stage_fixture_paths(
    toolchain: &Toolchain,
    fixture_dir: &str,
    fixtures: &[&str],
    work_dir: &Path,
) -> Result<(), String> {
    let source_root = toolchain.project_root.join(fixture_dir);
    for fixture in fixtures {
        let source = source_root.join(fixture);
        let destination = work_dir.join(fixture);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("create fixture directory {}: {error}", parent.display())
            })?;
        }
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "copy fixture {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    Ok(())
}

fn is_safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn reset_directory(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "remove old test directory {}: {error}",
                path.display()
            ))
        }
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("create test directory {}: {error}", path.display()))
}

fn sanitize_case_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn describe_exit(exit_code: Option<i32>) -> String {
    exit_code.map_or_else(
        || "after termination by signal".to_owned(),
        |code| format!("with status {code}"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    pub list: bool,
    pub exact: bool,
    pub bluesim_enabled: bool,
    pub verilog_enabled: bool,
    pub test_threads: usize,
    pub filter: Option<String>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            list: false,
            exact: false,
            bluesim_enabled: true,
            verilog_enabled: true,
            test_threads: thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            filter: None,
        }
    }
}

pub fn parse_cli<I, S>(arguments: I) -> Result<CliOptions, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let arguments: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    let mut options = CliOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index]
            .to_str()
            .ok_or_else(|| "runner arguments must be valid UTF-8".to_owned())?;
        match argument {
            "--list" => options.list = true,
            "--exact" => options.exact = true,
            "--no-bluesim" => options.bluesim_enabled = false,
            "--no-verilog" => options.verilog_enabled = false,
            "--test-threads" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| "--test-threads requires a positive integer".to_owned())?
                    .to_str()
                    .ok_or_else(|| "--test-threads must be valid UTF-8".to_owned())?;
                options.test_threads = parse_thread_count(value)?;
            }
            "--" => {
                for filter in &arguments[index + 1..] {
                    set_filter(
                        &mut options,
                        filter
                            .to_str()
                            .ok_or_else(|| "filter must be valid UTF-8".to_owned())?,
                    )?;
                }
                break;
            }
            _ if argument.starts_with("--test-threads=") => {
                options.test_threads = parse_thread_count(&argument["--test-threads=".len()..])?;
            }
            _ if argument.starts_with('-') => {
                return Err(format!("unrecognized runner option: {argument}"));
            }
            _ => set_filter(&mut options, argument)?,
        }
        index += 1;
    }
    Ok(options)
}

fn parse_thread_count(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(count) if count > 0 => Ok(count),
        _ => Err(format!(
            "--test-threads requires a positive integer, got {value:?}"
        )),
    }
}

fn set_filter(options: &mut CliOptions, filter: &str) -> Result<(), String> {
    if options.filter.is_some() {
        return Err("only one substring filter may be supplied".to_owned());
    }
    options.filter = Some(filter.to_owned());
    Ok(())
}

pub fn select_plan(options: &CliOptions) -> ExecutionPlan {
    let filter = options
        .filter
        .as_deref()
        .map(|filter| filter.replace('\\', "/"));
    let matches = |name: &str| match &filter {
        None => true,
        Some(filter) if options.exact => name == filter,
        Some(filter) => name.contains(filter),
    };
    let compile_cases = compile_cases()
        .iter()
        .copied()
        .filter(|case| matches(case.name))
        .collect();
    let simulations = simulation_scenarios()
        .iter()
        .filter_map(|scenario| {
            let contracts = scenario
                .contracts
                .iter()
                .filter(|contract| matches(contract.name))
                .collect::<Vec<_>>();
            (!contracts.is_empty()).then_some(PlannedSimulation {
                scenario,
                contracts,
            })
        })
        .collect();
    let bluesim_workflows = bluesim_workflow_scenarios()
        .iter()
        .filter_map(|scenario| {
            let runs = scenario
                .runs
                .iter()
                .filter(|run| matches(run.name))
                .collect::<Vec<_>>();
            (if scenario.runs.is_empty() {
                matches(scenario.name)
            } else {
                !runs.is_empty()
            })
            .then_some(PlannedBluesimWorkflow { scenario, runs })
        })
        .collect();
    ExecutionPlan {
        compile_cases,
        simulations,
        bluesim_workflows,
    }
}

#[cfg(test)]
mod tests;
