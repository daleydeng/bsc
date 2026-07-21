use crate::{normalize_diff_b_text, readable_diff, run_bsc, run_command, Toolchain, BSC_TIMEOUT};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunnerPolicy {
    pub bluesim_enabled: bool,
    pub verilog_enabled: bool,
}

impl RunnerPolicy {
    pub fn from_environment(
        ctest: Option<&std::ffi::OsStr>,
        vtest: Option<&std::ffi::OsStr>,
    ) -> Self {
        Self {
            bluesim_enabled: ctest != Some(std::ffi::OsStr::new("0")),
            verilog_enabled: vtest != Some(std::ffi::OsStr::new("0")),
        }
    }

    pub fn from_vtest(value: Option<&std::ffi::OsStr>) -> Self {
        Self::from_environment(None, value)
    }

    fn skip_reason(self, requirement: Requirement) -> Option<&'static str> {
        match requirement {
            Requirement::Always => None,
            Requirement::BluesimEnabled if self.bluesim_enabled => None,
            Requirement::BluesimEnabled => Some("Bluesim backend disabled by CTEST=0"),
            Requirement::VerilogEnabled if self.verilog_enabled => None,
            Requirement::VerilogEnabled => Some("Verilog backend disabled by VTEST=0"),
        }
    }
}

impl Default for RunnerPolicy {
    fn default() -> Self {
        Self::from_vtest(None)
    }
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
pub struct SimulationCase {
    pub name: &'static str,
    pub fixture_dir: &'static str,
    pub source: &'static str,
    pub fixtures: &'static [&'static str],
    pub top: &'static str,
    pub expected: &'static str,
    pub compile_options: &'static [&'static str],
    pub link_options: &'static [&'static str],
    pub simulation_options: &'static [&'static str],
    pub sort_output: bool,
    pub backend: SimulationBackend,
    pub requirement: Requirement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamCase {
    Compile(CompileCase),
    Simulation(SimulationCase),
}

impl UpstreamCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Compile(case) => case.name,
            Self::Simulation(case) => case.name,
        }
    }

    fn requirement(self) -> Requirement {
        match self {
            Self::Compile(case) => case.requirement,
            Self::Simulation(case) => case.requirement,
        }
    }
}

mod cases_compile;
mod cases_simulation;

pub use cases_compile::CASES;
pub use cases_simulation::SIMULATION_CASES;

pub fn all_cases() -> Vec<UpstreamCase> {
    CASES
        .iter()
        .copied()
        .map(UpstreamCase::Compile)
        .chain(
            SIMULATION_CASES
                .iter()
                .copied()
                .map(UpstreamCase::Simulation),
        )
        .collect()
}

#[derive(Debug, Clone)]
pub struct RunPaths {
    pub work_root: PathBuf,
    pub artifact_root: PathBuf,
}

impl RunPaths {
    pub fn new(project_root: &Path, run_id: &str) -> Self {
        let temp_root = project_root.join(".pixi").join("tmp");
        Self {
            work_root: temp_root
                .join("rust-test-work")
                .join("upstream")
                .join(run_id),
            artifact_root: temp_root
                .join("rust-test-artifacts")
                .join("upstream")
                .join(run_id),
        }
    }

    fn for_name(&self, name: &str) -> (PathBuf, PathBuf) {
        let directory = sanitize_case_name(name);
        (
            self.work_root.join(&directory),
            self.artifact_root.join(directory),
        )
    }
}

pub fn run_compile_case(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    case: &CompileCase,
) -> Result<(), String> {
    validate_case(case)?;
    let (work_dir, artifact_dir) = run_paths.for_name(case.name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;
    stage_fixtures(toolchain, case, &work_dir)?;

    let arguments = compile_arguments(case);

    let log_path = artifact_dir.join("bsc.log");
    let result = run_bsc(toolchain, &arguments, &work_dir, &log_path, BSC_TIMEOUT)?;
    let output_path = work_dir.join(format!("{}.bsc-out", case.source));
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create output directory {}: {error}", parent.display()))?;
    }
    fs::write(&output_path, &result.output)
        .map_err(|error| format!("write BSC output {}: {error}", output_path.display()))?;

    check_expectation(
        case,
        result.success,
        result.exit_code,
        &result.output,
        &work_dir,
        &log_path,
    )?;

    if let Some(golden) = case.golden {
        let expected_path = work_dir.join(golden.expected);
        compare_legacy_golden(
            &result.output,
            &expected_path,
            &output_path,
            &artifact_dir.join("golden.diff"),
        )?;
    }

    Ok(())
}

pub fn run_simulation_case(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    case: &SimulationCase,
) -> Result<(), String> {
    validate_simulation_case(case)?;
    let (work_dir, artifact_dir) = run_paths.for_name(case.name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;
    stage_fixture_paths(toolchain, case.fixture_dir, case.fixtures, &work_dir)?;

    let mut compile_arguments = Vec::with_capacity(case.compile_options.len() + 8);
    compile_arguments.extend_from_slice(case.compile_options);
    compile_arguments.extend_from_slice(&["-no-show-timestamps", "-no-show-version", "-u"]);
    match case.backend {
        SimulationBackend::Bluesim => compile_arguments.push("-sim"),
        SimulationBackend::Icarus => compile_arguments.push("-verilog"),
    }
    compile_arguments.extend_from_slice(&["-g", case.top, case.source]);
    run_required_bsc_step(
        toolchain,
        &compile_arguments,
        &work_dir,
        &artifact_dir.join("compile.log"),
        "generate simulation model",
    )?;

    let generated = match case.backend {
        SimulationBackend::Bluesim => format!("{}.ba", case.top),
        SimulationBackend::Icarus => format!("{}.v", case.top),
    };
    if !work_dir.join(&generated).is_file() {
        return Err(format!(
            "BSC did not generate {} for {}; see {}",
            generated,
            case.name,
            artifact_dir.join("compile.log").display()
        ));
    }

    let mut link_arguments = Vec::with_capacity(case.link_options.len() + 10);
    link_arguments.extend_from_slice(&["-no-show-timestamps", "-no-show-version"]);
    match case.backend {
        SimulationBackend::Bluesim => link_arguments.push("-sim"),
        SimulationBackend::Icarus => {
            link_arguments.extend_from_slice(&["-verilog", "-vsim", "iverilog"]);
        }
    }
    link_arguments.extend_from_slice(&["-e", case.top, "-o", case.top]);
    link_arguments.extend_from_slice(case.link_options);
    link_arguments.push(&generated);
    run_required_bsc_step(
        toolchain,
        &link_arguments,
        &work_dir,
        &artifact_dir.join("link.log"),
        "link simulation executable",
    )?;

    let mut executable = work_dir.join(case.top);
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
        Some(match case.backend {
            SimulationBackend::Bluesim => "sh",
            SimulationBackend::Icarus => "vvp",
        })
    } else {
        None
    };
    let mut simulation_arguments =
        Vec::with_capacity(case.simulation_options.len() + usize::from(launcher.is_some()) + 1);
    if launcher.is_some() {
        simulation_arguments.push(case.top);
    }
    if case.backend == SimulationBackend::Icarus {
        simulation_arguments.push("-vcd-none");
    }
    simulation_arguments.extend_from_slice(case.simulation_options);
    let simulation_log = artifact_dir.join("simulation.log");
    let program = launcher.map_or(executable.as_path(), Path::new);
    let result = run_command(
        toolchain,
        program,
        &simulation_arguments,
        &work_dir,
        &simulation_log,
        BSC_TIMEOUT,
    )?;
    if !result.success {
        return Err(format!(
            "simulation for {} exited {}; see {}",
            case.name,
            describe_exit(result.exit_code),
            simulation_log.display()
        ));
    }

    let mut output = match case.backend {
        SimulationBackend::Bluesim => result.output,
        SimulationBackend::Icarus => clean_iverilog_output(&result.output),
    };
    if case.sort_output {
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
        &work_dir.join(case.expected),
        &output_path,
        &artifact_dir.join("golden.diff"),
    )
}

fn run_required_bsc_step(
    toolchain: &Toolchain,
    arguments: &[&str],
    work_dir: &Path,
    log_path: &Path,
    action: &str,
) -> Result<(), String> {
    let result = run_bsc(toolchain, arguments, work_dir, log_path, BSC_TIMEOUT)?;
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

fn clean_iverilog_output(output: &str) -> String {
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

fn compile_arguments(case: &CompileCase) -> Vec<&str> {
    let mut arguments = Vec::with_capacity(case.options.len() + 7);
    arguments.extend_from_slice(case.options);
    arguments.push("-no-show-timestamps");
    arguments.push("-no-show-version");
    match case.mode {
        CompileMode::Frontend => {
            if !case.nodeps {
                arguments.push("-u");
            }
        }
        CompileMode::Verilog { module } => {
            arguments.push("-u");
            arguments.push("-verilog");
            if let Some(module) = module.filter(|module| !module.is_empty()) {
                arguments.push("-g");
                arguments.push(module);
            }
        }
    }
    arguments.push(case.source);
    arguments
}

fn check_expectation(
    case: &CompileCase,
    success: bool,
    exit_code: Option<i32>,
    output: &str,
    work_dir: &Path,
    log_path: &Path,
) -> Result<(), String> {
    match case.expectation {
        CompileExpectation::Pass => {
            if !success {
                return Err(format!(
                    "BSC should compile {} but exited {}; see {}",
                    case.source,
                    describe_exit(exit_code),
                    log_path.display()
                ));
            }
            let stem = Path::new(case.source)
                .file_stem()
                .ok_or_else(|| format!("source has no file stem: {}", case.source))?;
            let object_path = work_dir.join(stem).with_extension("bo");
            if !object_path.is_file() {
                return Err(format!(
                    "BSC succeeded but did not create {}; see {}",
                    object_path.display(),
                    log_path.display()
                ));
            }
        }
        CompileExpectation::Fail => {
            if success {
                return Err(format!(
                    "BSC should reject {} but succeeded; see {}",
                    case.source,
                    log_path.display()
                ));
            }
        }
        CompileExpectation::FailWithDiagnostic { kind, tag, count } => {
            if success {
                return Err(format!(
                    "BSC should reject {} with {} {tag} but succeeded; see {}",
                    case.source,
                    kind.as_str(),
                    log_path.display()
                ));
            }
            let actual = count_diagnostics(output, kind, tag);
            if actual != count {
                return Err(format!(
                    "expected {count} copies of {} {tag} for {}, found {actual}; see {}",
                    kind.as_str(),
                    case.source,
                    log_path.display()
                ));
            }
        }
    }
    Ok(())
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

pub fn normalize_legacy_golden(text: &str) -> String {
    let normalized_newlines = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut filtered = String::with_capacity(normalized_newlines.len());
    for line in normalized_newlines.split_inclusive('\n') {
        if !line.contains("SystemC") && !line.contains("dumpfile parameter") {
            filtered.push_str(line);
        }
    }
    normalize_diff_b_text(&filtered)
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

fn stage_fixtures(
    toolchain: &Toolchain,
    case: &CompileCase,
    work_dir: &Path,
) -> Result<(), String> {
    stage_fixture_paths(toolchain, case.fixture_dir, case.fixtures, work_dir)
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

fn validate_case(case: &CompileCase) -> Result<(), String> {
    if case.name.is_empty() {
        return Err("compile case name must not be empty".to_owned());
    }
    if !is_safe_relative(case.fixture_dir) || !is_safe_relative(case.source) {
        return Err(format!(
            "compile case {} contains an unsafe path",
            case.name
        ));
    }
    if !case.fixtures.contains(&case.source) {
        return Err(format!(
            "compile case {} must declare source {} as a fixture",
            case.name, case.source
        ));
    }
    for fixture in case.fixtures {
        if !is_safe_relative(fixture) {
            return Err(format!(
                "compile case {} contains unsafe fixture path {fixture}",
                case.name
            ));
        }
    }
    if let Some(golden) = case.golden {
        if !is_safe_relative(golden.expected) || !case.fixtures.contains(&golden.expected) {
            return Err(format!(
                "compile case {} must declare golden {} as a fixture",
                case.name, golden.expected
            ));
        }
    }
    match case.mode {
        CompileMode::Frontend if case.requirement == Requirement::Always => {}
        CompileMode::Frontend => {
            return Err(format!(
                "frontend compile case {} must always run",
                case.name
            ))
        }
        CompileMode::Verilog { .. } if case.requirement == Requirement::VerilogEnabled => {}
        CompileMode::Verilog { .. } => {
            return Err(format!(
                "Verilog compile case {} must require the Verilog backend",
                case.name
            ))
        }
    }
    if matches!(case.mode, CompileMode::Verilog { .. }) && case.nodeps {
        return Err(format!(
            "Verilog compile case {} cannot disable the required -u option",
            case.name
        ));
    }
    Ok(())
}

fn validate_simulation_case(case: &SimulationCase) -> Result<(), String> {
    if case.name.is_empty()
        || !is_safe_relative(case.fixture_dir)
        || !is_safe_relative(case.source)
        || !is_safe_relative(case.expected)
        || case.top.is_empty()
    {
        return Err(format!(
            "simulation case {} contains an empty name or unsafe path",
            case.name
        ));
    }
    if !case.fixtures.contains(&case.source) || !case.fixtures.contains(&case.expected) {
        return Err(format!(
            "simulation case {} must declare source and expected output as fixtures",
            case.name
        ));
    }
    if case
        .fixtures
        .iter()
        .any(|fixture| !is_safe_relative(fixture))
    {
        return Err(format!(
            "simulation case {} contains an unsafe fixture path",
            case.name
        ));
    }
    let expected_requirement = match case.backend {
        SimulationBackend::Bluesim => Requirement::BluesimEnabled,
        SimulationBackend::Icarus => Requirement::VerilogEnabled,
    };
    if case.requirement != expected_requirement {
        return Err(format!(
            "simulation case {} has a backend/requirement mismatch",
            case.name
        ));
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseResult {
    Passed,
    Skipped(&'static str),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseOutcome {
    pub name: &'static str,
    pub result: CaseResult,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub passed: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn summarize_outcomes(outcomes: &[CaseOutcome]) -> RunSummary {
    let mut summary = RunSummary::default();
    for outcome in outcomes {
        match outcome.result {
            CaseResult::Passed => summary.passed += 1,
            CaseResult::Skipped(_) => summary.skipped += 1,
            CaseResult::Failed(_) => summary.failed += 1,
        }
    }
    summary
}

pub fn run_cases(
    toolchain: Toolchain,
    run_paths: RunPaths,
    cases: Vec<UpstreamCase>,
    policy: RunnerPolicy,
    test_threads: usize,
) -> Vec<CaseOutcome> {
    let toolchain = Arc::new(toolchain);
    let run_paths = Arc::new(run_paths);
    run_fixed_queue(cases, test_threads, move |case| {
        let result = if let Some(reason) = policy.skip_reason(case.requirement()) {
            CaseResult::Skipped(reason)
        } else {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match case {
                UpstreamCase::Compile(case) => run_compile_case(&toolchain, &run_paths, &case),
                UpstreamCase::Simulation(case) => {
                    run_simulation_case(&toolchain, &run_paths, &case)
                }
            })) {
                Ok(Ok(())) => CaseResult::Passed,
                Ok(Err(error)) => CaseResult::Failed(error),
                Err(panic) => {
                    CaseResult::Failed(format!("runner panicked: {}", panic_message(panic)))
                }
            }
        };
        CaseOutcome {
            name: case.name(),
            result,
        }
    })
}

fn run_fixed_queue<T, R, F>(items: Vec<T>, thread_count: usize, worker: F) -> Vec<R>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> R + Send + Sync + 'static,
{
    let item_count = items.len();
    if item_count == 0 {
        return Vec::new();
    }
    let queue = Arc::new(Mutex::new(VecDeque::from(items)));
    let worker = Arc::new(worker);
    let (sender, receiver) = mpsc::channel();
    let actual_threads = thread_count.max(1).min(item_count);
    let mut workers = Vec::with_capacity(actual_threads);

    for _ in 0..actual_threads {
        let queue = Arc::clone(&queue);
        let worker = Arc::clone(&worker);
        let sender = sender.clone();
        workers.push(thread::spawn(move || loop {
            let item = queue.lock().expect("worker queue poisoned").pop_front();
            let Some(item) = item else {
                break;
            };
            if sender.send(worker(item)).is_err() {
                break;
            }
        }));
    }
    drop(sender);

    let results: Vec<R> = receiver.into_iter().collect();
    for worker in workers {
        worker.join().expect("runner worker panicked");
    }
    assert_eq!(results.len(), item_count, "runner lost a case result");
    results
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn data_model_has_126_valid_cases_from_48_scripts() {
        assert_eq!(CASES.len(), 126);

        let names: BTreeSet<_> = CASES.iter().map(|case| case.name).collect();
        let scripts: BTreeSet<_> = CASES.iter().map(|case| case.fixture_dir).collect();
        assert_eq!(names.len(), 126, "case names must be unique");
        assert_eq!(scripts.len(), 48);

        let mut category_counts = [0; 3];
        let mut mode_counts = [0; 2];
        for case in CASES {
            validate_case(case).unwrap();
            assert!(case.options.is_empty());
            assert!(!case.nodeps);
            match case.expectation {
                CompileExpectation::Pass => category_counts[0] += 1,
                CompileExpectation::Fail => category_counts[1] += 1,
                CompileExpectation::FailWithDiagnostic { count, .. } => {
                    assert_eq!(count, 1);
                    category_counts[2] += 1;
                }
            }
            match case.mode {
                CompileMode::Frontend => {
                    assert_eq!(case.requirement, Requirement::Always);
                    mode_counts[0] += 1;
                }
                CompileMode::Verilog { module } => {
                    assert_eq!(module, None);
                    assert_eq!(case.requirement, Requirement::VerilogEnabled);
                    mode_counts[1] += 1;
                }
            }
        }
        assert_eq!(category_counts, [36, 32, 58]);
        assert_eq!(mode_counts, [81, 45]);
        assert_eq!(
            CASES.iter().filter(|case| case.golden.is_some()).count(),
            34
        );

        let phase_one_names = [
            "b600::Bug600.bsv",
            "b267::Bug267.bs",
            "b1040::Bug1040.bsv",
            "b417::Bug417.bsv",
            "b492::Bug492_1.bs",
            "b1586::Bug1586.bsv",
            "b269::Bug269.bsv",
            "b1493::Bug1493.bsv",
            "b1493::Bug1493_Bad.bsv",
        ];
        assert!(phase_one_names.into_iter().all(|name| names.contains(name)));
    }

    #[test]
    fn compile_modes_build_distinct_unix_exp_argv() {
        let mut frontend = CASES[0];
        frontend.options = &["-keep-fires"];
        assert_eq!(
            compile_arguments(&frontend),
            [
                "-keep-fires",
                "-no-show-timestamps",
                "-no-show-version",
                "-u",
                frontend.source,
            ]
        );
        frontend.nodeps = true;
        assert_eq!(
            compile_arguments(&frontend),
            [
                "-keep-fires",
                "-no-show-timestamps",
                "-no-show-version",
                frontend.source,
            ]
        );

        let mut verilog = *CASES
            .iter()
            .find(|case| matches!(case.mode, CompileMode::Verilog { .. }))
            .unwrap();
        assert_eq!(
            compile_arguments(&verilog),
            [
                "-no-show-timestamps",
                "-no-show-version",
                "-u",
                "-verilog",
                verilog.source,
            ]
        );
        verilog.mode = CompileMode::Verilog {
            module: Some("mkTop"),
        };
        assert_eq!(
            compile_arguments(&verilog),
            [
                "-no-show-timestamps",
                "-no-show-version",
                "-u",
                "-verilog",
                "-g",
                "mkTop",
                verilog.source,
            ]
        );
    }

    #[test]
    fn vtest_policy_defaults_enabled_and_zero_disables_verilog() {
        let default_policy = RunnerPolicy::from_vtest(None);
        assert!(default_policy.verilog_enabled);
        assert!(CASES
            .iter()
            .all(|case| default_policy.skip_reason(case.requirement).is_none()));

        let disabled = RunnerPolicy::from_vtest(Some(std::ffi::OsStr::new("0")));
        assert!(!disabled.verilog_enabled);
        let skipped: Vec<_> = CASES
            .iter()
            .filter(|case| disabled.skip_reason(case.requirement).is_some())
            .map(|case| case.name)
            .collect();
        assert_eq!(skipped.len(), 45);
        assert!(skipped.iter().any(|name| name.contains("dynamic/errors")));
        assert!(skipped.iter().any(|name| name.contains("bounds/select")));

        assert!(RunnerPolicy::from_vtest(Some(std::ffi::OsStr::new("1"))).verilog_enabled);
    }

    #[test]
    fn simulation_data_model_has_62_valid_backend_cases() {
        assert_eq!(SIMULATION_CASES.len(), 62);
        let mut backend_counts = [0; 2];
        for case in SIMULATION_CASES {
            validate_simulation_case(case).unwrap();
            assert!(case.compile_options.is_empty());
            assert!(case.link_options.is_empty());
            assert!(case.simulation_options.is_empty());
            assert!(!case.sort_output);
            match case.backend {
                SimulationBackend::Bluesim => backend_counts[0] += 1,
                SimulationBackend::Icarus => backend_counts[1] += 1,
            }
        }
        assert_eq!(backend_counts, [31, 31]);

        let all = all_cases();
        assert_eq!(all.len(), 188);
        let names: BTreeSet<_> = all.iter().map(|case| case.name()).collect();
        assert_eq!(names.len(), 188, "all upstream case names must be unique");
    }

    #[test]
    fn ctest_and_vtest_capabilities_skip_their_simulation_backends() {
        let cases = all_cases();
        let no_bluesim = RunnerPolicy::from_environment(
            Some(std::ffi::OsStr::new("0")),
            Some(std::ffi::OsStr::new("1")),
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| no_bluesim.skip_reason(case.requirement()).is_some())
                .count(),
            31
        );

        let no_verilog = RunnerPolicy::from_environment(
            Some(std::ffi::OsStr::new("1")),
            Some(std::ffi::OsStr::new("0")),
        );
        assert_eq!(
            cases
                .iter()
                .filter(|case| no_verilog.skip_reason(case.requirement()).is_some())
                .count(),
            76
        );
    }

    #[test]
    fn iverilog_output_filter_matches_legacy_noise_rules() {
        let output = concat!(
            "$readmem ignored\n",
            "WARNING: file: $readmem changed\n",
            "keep this\n",
            "foo $finish called\n",
            "VCD info: dumpfile\n"
        );
        assert_eq!(clean_iverilog_output(output), "keep this\n");
    }

    #[test]
    fn outcome_summary_counts_skips_without_turning_them_into_failures() {
        let outcomes = [
            CaseOutcome {
                name: "pass",
                result: CaseResult::Passed,
            },
            CaseOutcome {
                name: "skip",
                result: CaseResult::Skipped("capability disabled"),
            },
            CaseOutcome {
                name: "fail",
                result: CaseResult::Failed("broken".to_owned()),
            },
        ];
        assert_eq!(
            summarize_outcomes(&outcomes),
            RunSummary {
                passed: 1,
                skipped: 1,
                failed: 1,
            }
        );
    }

    #[test]
    fn diagnostic_count_matches_tcl_line_regexp_shape() {
        let output = concat!(
            "Error: \"file\", line 1, column 2: (P0070)\n",
            "  details (P0070)\n",
            "prefix Error: Unknown position: (P0070)\r\n",
            "Error:(P0070)\n",
            "Error: x (OTHER)\n",
            "Warning: x (P0070)\n"
        );
        assert_eq!(count_diagnostics(output, DiagnosticKind::Error, "P0070"), 2);
        assert_eq!(
            count_diagnostics(output, DiagnosticKind::Warning, "P0070"),
            1
        );
    }

    #[test]
    fn legacy_golden_uses_diff_b_and_line_filters() {
        let expected = "alpha  beta\nSystemC banner\ndumpfile parameter ignored\nlast\tvalue\n";
        let actual = "alpha\tbeta\nlast value   \r\n";
        assert_eq!(
            normalize_legacy_golden(expected),
            normalize_legacy_golden(actual)
        );
    }

    #[test]
    fn cli_parses_filter_exact_and_thread_count() {
        let options = parse_cli(["b1493::Bug1493_Bad.bsv", "--exact", "--test-threads=3"]).unwrap();
        assert!(options.exact);
        assert_eq!(options.test_threads, 3);
        assert_eq!(options.filter.as_deref(), Some("b1493::Bug1493_Bad.bsv"));
        let cases = all_cases();
        assert_eq!(select_cases(&cases, &options).len(), 1);
    }

    #[test]
    fn cli_list_and_substring_selection() {
        let options = parse_cli(["--list", "b1493", "--test-threads", "2"]).unwrap();
        assert!(options.list);
        let cases = all_cases();
        let names: Vec<&str> = select_cases(&cases, &options)
            .into_iter()
            .map(|case| case.name())
            .collect();
        assert_eq!(names, ["b1493::Bug1493.bsv", "b1493::Bug1493_Bad.bsv"]);
    }

    #[test]
    fn cli_rejects_bad_thread_counts_and_multiple_filters() {
        assert!(parse_cli(["--test-threads", "0"]).is_err());
        assert!(parse_cli(["one", "two"]).is_err());
        assert!(parse_cli(["--unknown"]).is_err());
    }

    #[test]
    fn fixed_worker_queue_processes_every_item_once() {
        let results = run_fixed_queue((0..20).collect(), 4, |value| value * value);
        let actual: BTreeSet<_> = results.into_iter().collect();
        let expected: BTreeSet<_> = (0..20).map(|value| value * value).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn run_paths_are_scoped_by_run_id() {
        let paths = RunPaths::new(Path::new("project"), "123-456");
        assert!(paths.work_root.ends_with("upstream/123-456"));
        assert!(paths.artifact_root.ends_with("upstream/123-456"));
    }
}
