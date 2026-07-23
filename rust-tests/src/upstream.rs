use crate::{normalize_diff_b_text, readable_diff, Toolchain};
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
pub enum CompileMode {
    Frontend,
    Verilog { module: Option<&'static str> },
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
    pub fn from_environment(
        ctest: Option<&std::ffi::OsStr>,
        vtest: Option<&std::ffi::OsStr>,
    ) -> Self {
        Self {
            bluesim_enabled: ctest != Some(std::ffi::OsStr::new("0")),
            verilog_enabled: vtest != Some(std::ffi::OsStr::new("0")),
            iverilog_major: None,
        }
    }

    pub fn from_vtest(value: Option<&std::ffi::OsStr>) -> Self {
        Self::from_environment(None, value)
    }

    pub fn with_iverilog_major(mut self, major: Option<u32>) -> Self {
        self.iverilog_major = major;
        self
    }

    fn skip_reason(self, requirement: Requirement) -> Option<String> {
        match requirement {
            Requirement::Always => None,
            Requirement::BluesimEnabled if self.bluesim_enabled => None,
            Requirement::BluesimEnabled => Some("Bluesim backend disabled by CTEST=0".to_owned()),
            Requirement::VerilogEnabled if self.verilog_enabled => None,
            Requirement::VerilogEnabled => Some("Verilog backend disabled by VTEST=0".to_owned()),
            Requirement::IcarusAtLeast(_) if !self.verilog_enabled => {
                Some("Verilog backend disabled by VTEST=0".to_owned())
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
        Self::from_vtest(None)
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
pub enum VcdExpectation {
    None,
    BluesimOutputMatchesNormal,
    IcarusSmoke,
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
pub struct SimulationScenario {
    pub name: &'static str,
    pub fixture_dir: &'static str,
    pub source: &'static str,
    pub fixtures: &'static [&'static str],
    pub top: &'static str,
    pub generated_modules: &'static [&'static str],
    pub compile_options: &'static [&'static str],
    pub generation: GenerationStrategy,
    pub timeout: std::time::Duration,
    pub resource: ResourceClass,
    pub contracts: &'static [SimulationContract],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationContract {
    pub name: &'static str,
    pub expected: &'static str,
    pub link_options: &'static [&'static str],
    pub simulation_options: &'static [&'static str],
    pub sort_output: bool,
    pub backend: SimulationBackend,
    pub vcd: VcdExpectation,
    pub requirement: Requirement,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CaseModule<C: 'static> {
    pub name: &'static str,
    pub cases: &'static [C],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamCase {
    Compile(CompileCase),
    Simulation {
        scenario: &'static SimulationScenario,
        contract: &'static SimulationContract,
    },
}

impl UpstreamCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Compile(case) => case.name,
            Self::Simulation { contract, .. } => contract.name,
        }
    }

    pub fn requirement(self) -> Requirement {
        match self {
            Self::Compile(case) => case.requirement,
            Self::Simulation { contract, .. } => contract.requirement,
        }
    }
}

mod cases_compile;
mod cases_simulation;
mod compile;
mod runner;
mod simulation;

#[cfg(test)]
use compile::{compile_arguments, validate_case};
#[cfg(test)]
use runner::{build_work_items, run_fixed_queue, WorkItem};
pub use runner::{run_cases, summarize_outcomes, CaseOutcome, CaseResult, RunPaths, RunSummary};
#[cfg(test)]
use simulation::clean_iverilog_output;
pub(crate) use simulation::validate_simulation_scenario;

pub fn compile_cases() -> &'static [CompileCase] {
    cases_compile::cases()
}

pub fn simulation_scenarios() -> &'static [SimulationScenario] {
    cases_simulation::scenarios()
}

pub(crate) fn compile_case_modules() -> &'static [CaseModule<CompileCase>] {
    cases_compile::MODULES
}

pub(crate) fn simulation_scenario_modules() -> &'static [CaseModule<SimulationScenario>] {
    cases_simulation::MODULES
}

pub fn all_cases() -> Vec<UpstreamCase> {
    compile_cases()
        .iter()
        .copied()
        .map(UpstreamCase::Compile)
        .chain(simulation_scenarios().iter().flat_map(|scenario| {
            scenario
                .contracts
                .iter()
                .map(|contract| UpstreamCase::Simulation { scenario, contract })
        }))
        .collect()
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

pub fn normalize_legacy_golden(text: &str) -> String {
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

fn compare_legacy_golden(
    actual: &str,
    expected_path: &Path,
    actual_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    let expected = fs::read_to_string(expected_path)
        .map_err(|error| format!("read golden {}: {error}", expected_path.display()))?;
    let expected = normalize_legacy_golden(&expected);
    let actual = normalize_legacy_golden(actual);
    if expected == actual {
        return Ok(());
    }

    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        &actual_path.display().to_string(),
    );
    fs::write(diff_path, diff)
        .map_err(|error| format!("write golden diff {}: {error}", diff_path.display()))?;
    Err(format!(
        "{} differs from {}; see {}",
        actual_path.display(),
        expected_path.display(),
        diff_path.display()
    ))
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
    pub test_threads: usize,
    pub filter: Option<String>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            list: false,
            exact: false,
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

pub fn select_cases<'a>(cases: &'a [UpstreamCase], options: &CliOptions) -> Vec<&'a UpstreamCase> {
    cases
        .iter()
        .filter(|case| match &options.filter {
            None => true,
            Some(filter) if options.exact => case.name() == filter,
            Some(filter) => case.name().contains(filter),
        })
        .collect()
}

#[cfg(test)]
mod tests;
