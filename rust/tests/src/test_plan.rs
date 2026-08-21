use crate::assertion::{check_plan_assertion_typed, PlanAssertionFailure};
use crate::bluesim::{invocation as bluesim_invocation, resolve_executable};
use crate::cache::{ReadOnlyCacheLookup, ScenarioResultCache};
use crate::{
    reset_directory, run_bsc, run_bsc_with_options, run_bsc_with_options_and_environment,
    run_bsc_with_options_prepend, run_command, secure_directory_within, secure_file_within,
    Toolchain,
};
use bsc_test_plan::{
    simulation_executable_artifact, Action, BluetclInstalledScript, BluetclInvocation,
    BluetclMakedependCommand, BluetclPackage, BluetclSyntax, BscCompileMode, BscFlagPreflightMode,
    BscLinkMode, DependencyMode, ExpectedExit, Fixture, GoldenReplacement, IcarusSimulatorSelector,
    IntermediateDumpView, InterraOperatorSuite, OperationExpectation, OperationRecord, PlanStatus,
    Requirement, Scenario, SimulationBackend, SimulationGenerationMode, TestPlan,
    TextNormalization, UndeterminedValue, VerilogFilterProfile,
};
use filetime::{set_file_mtime, FileTime};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanRunSummary {
    pub passed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestPlanCacheSummary {
    pub enabled: bool,
    pub hits: usize,
    pub misses: usize,
    pub stores: usize,
}

#[derive(Default)]
struct BluetclPackageProbeCache {
    results: Mutex<BTreeMap<BluetclPackage, Result<bool, String>>>,
}

impl BluetclPackageProbeCache {
    fn available(&self, toolchain: &Toolchain, package: BluetclPackage) -> Result<bool, String> {
        let mut results = self
            .results
            .lock()
            .map_err(|_| "Bluetcl package probe cache lock was poisoned".to_owned())?;
        if let Some(result) = results.get(&package) {
            return result.clone();
        }
        let result = probe_bluetcl_package(toolchain, package);
        results.insert(package, result.clone());
        result
    }
}

pub struct TestPlanExecutor<'a> {
    toolchain: &'a Toolchain,
    cache: ScenarioResultCache,
    package_probes: BluetclPackageProbeCache,
}

impl<'a> TestPlanExecutor<'a> {
    pub fn new(toolchain: &'a Toolchain) -> Result<Self, String> {
        Ok(Self {
            toolchain,
            cache: ScenarioResultCache::new(toolchain)?,
            package_probes: BluetclPackageProbeCache::default(),
        })
    }

    pub fn execute(&self, plan: &TestPlan) -> Result<PlanRunSummary, String> {
        self.execute_scenarios(plan, &[])
    }

    /// Execute every scenario, or only the explicitly named scenarios in source order.
    pub fn execute_scenarios(
        &self,
        plan: &TestPlan,
        selected_scenarios: &[String],
    ) -> Result<PlanRunSummary, String> {
        plan.validate().map_err(|error| error.to_string())?;
        match plan.status {
            PlanStatus::Complete => {}
            PlanStatus::Disabled => {
                return Err(format!(
                    "Test Plan {} is disabled by upstream intent",
                    plan.id
                ));
            }
            PlanStatus::Blocked => {
                return Err(format!(
                    "Test Plan {} is blocked by {} import diagnostics",
                    plan.id,
                    plan.diagnostics.len()
                ));
            }
        }
        let selected = selected_scenario_ids(plan, selected_scenarios)?;
        verify_plan_origin(self.toolchain, plan)?;
        if plan
            .scenarios
            .iter()
            .filter(|scenario| {
                selected
                    .as_ref()
                    .is_none_or(|selected| selected.contains(&scenario.id))
            })
            .any(|scenario| scenario_skip_reason(self.toolchain, scenario).is_none())
        {
            verify_plan_fixtures(self.toolchain, plan)?;
        }
        let mut summary = PlanRunSummary::default();
        for scenario in &plan.scenarios {
            if selected
                .as_ref()
                .is_some_and(|selected| !selected.contains(&scenario.id))
            {
                continue;
            }
            if let Some(reason) = scenario_skip_reason(self.toolchain, scenario) {
                summary.skipped += scenario.stages.len();
                println!("SKIP {}::{}: {reason}", plan.id, scenario.id);
                continue;
            }
            println!("START {}::{}", plan.id, scenario.id);
            let started = Instant::now();
            match execute_scenario(
                self.toolchain,
                &self.cache,
                &self.package_probes,
                plan,
                scenario,
            ) {
                Ok(()) => {
                    summary.passed += scenario.stages.len();
                    println!(
                        "DONE  {}::{} ({:.3}s)",
                        plan.id,
                        scenario.id,
                        started.elapsed().as_secs_f64()
                    );
                }
                Err(error) => {
                    summary.failed += 1;
                    summary.failures.push(format!(
                        "{}::{} after {:.3}s: {error}",
                        plan.id,
                        scenario.id,
                        started.elapsed().as_secs_f64()
                    ));
                }
            }
        }
        Ok(summary)
    }

    pub fn cache_summary(&self) -> TestPlanCacheSummary {
        let summary = self.cache.summary();
        TestPlanCacheSummary {
            enabled: summary.enabled,
            hits: summary.hits,
            misses: summary.misses,
            stores: summary.stores,
        }
    }
}

pub fn execute_test_plan(toolchain: &Toolchain, plan: &TestPlan) -> Result<PlanRunSummary, String> {
    TestPlanExecutor::new(toolchain)?.execute(plan)
}

fn selected_scenario_ids(
    plan: &TestPlan,
    selected_scenarios: &[String],
) -> Result<Option<BTreeSet<String>>, String> {
    if selected_scenarios.is_empty() {
        return Ok(None);
    }
    let selected = selected_scenarios.iter().cloned().collect::<BTreeSet<_>>();
    let available = plan
        .scenarios
        .iter()
        .map(|scenario| scenario.id.as_str())
        .collect::<BTreeSet<_>>();
    let missing = selected
        .iter()
        .filter(|scenario| !available.contains(scenario.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(Some(selected))
    } else {
        Err(format!(
            "Test Plan {} has no scenario(s): {}",
            plan.id,
            missing.join(", ")
        ))
    }
}

fn no_main_icarus_link_arguments(
    builder: &Path,
    bluespecdir: &Path,
    top: &str,
    objects: &[String],
) -> Vec<String> {
    let libraries = bluespecdir.join("Libraries");
    let verilog = bluespecdir.join("Verilog");
    let mut arguments = vec![
        shell_path_for_platform(builder, cfg!(windows)),
        "link".to_owned(),
        top.to_owned(),
        top.to_owned(),
        "-y".to_owned(),
        ".".to_owned(),
        "-y".to_owned(),
        shell_path_for_platform(&libraries, cfg!(windows)),
        "-y".to_owned(),
        shell_path_for_platform(&verilog, cfg!(windows)),
    ];
    arguments.extend(objects.iter().cloned());
    arguments
}

fn shell_path_for_platform(path: &Path, windows: bool) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if windows {
        let bytes = path.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return format!("/{}{}", (bytes[0] as char).to_ascii_lowercase(), &path[2..]);
        }
    }
    path
}

fn make_test_data_arguments() -> [&'static str; 5] {
    ["-j1", "MAKEFLAGS=", "-f", "Makefile.data", "test_data"]
}

fn generate_interra_operator_vectors(
    toolchain: &Toolchain,
    scenario: &Scenario,
    suite: InterraOperatorSuite,
    work_dir: &Path,
    artifact_dir: &Path,
    operation_index: usize,
) -> Result<(), String> {
    let generate_dir = work_dir.join("generate");
    let timeout = Duration::from_secs(scenario.timeouts.generation_seconds);
    let run_checked =
        |program: &Path, arguments: &[&str], cwd: &Path, phase: &str| -> Result<String, String> {
            let log = artifact_dir.join(format!(
                "operation-{operation_index}-interra-vectors-{phase}.log"
            ));
            let result = run_command(toolchain, program, arguments, cwd, &log, timeout)?;
            result.success.then_some(result.output).ok_or_else(|| {
                format!(
                    "Interra operator vector {phase} failed with exit {}; see {}",
                    describe_exit(result.exit_code),
                    log.display()
                )
            })
        };

    let generated = run_checked(Path::new("perl"), &["gen.pl"], &generate_dir, "generate")?;
    fs::write(generate_dir.join(suite.generated_verilog()), generated)
        .map_err(|error| format!("write generated Interra Verilog: {error}"))?;

    let define = format!("-DTOP={}", suite.verilog_top());
    let main = toolchain
        .bluespecdir
        .join("Verilog/main.v")
        .to_string_lossy()
        .into_owned();
    run_checked(
        &toolchain.iverilog,
        &["-o", "a.out", &define, &main, suite.generated_verilog()],
        &generate_dir,
        "compile",
    )?;

    let executable = generate_dir.join("a.out");
    let (program, arguments) = icarus_invocation_for_platform(&executable, &[], cfg!(windows));
    let argument_refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let vectors = run_checked(&program, &argument_refs, &generate_dir, "simulate")?;
    fs::write(generate_dir.join("vectors"), vectors)
        .map_err(|error| format!("write generated Interra vectors: {error}"))?;

    let vectors_bsv = run_checked(
        Path::new("perl"),
        &["sort.pl", "vectors"],
        &generate_dir,
        "sort",
    )?;
    fs::write(generate_dir.join("Vectors.bsv"), &vectors_bsv)
        .map_err(|error| format!("write generated Interra Vectors.bsv: {error}"))?;
    fs::write(work_dir.join("Vectors.bsv"), vectors_bsv)
        .map_err(|error| format!("stage generated Interra Vectors.bsv: {error}"))?;
    Ok(())
}

fn simulation_generation_flags(mode: SimulationGenerationMode) -> &'static [&'static str] {
    match mode {
        SimulationGenerationMode::Bluesim => &["-sim"],
        SimulationGenerationMode::Verilog => &["-verilog"],
        SimulationGenerationMode::SharedElaboration => &["-verilog", "-elab"],
    }
}

fn resolve_icarus_simulator(
    toolchain: &Toolchain,
    selector: IcarusSimulatorSelector,
) -> Result<String, String> {
    match selector {
        IcarusSimulatorSelector::Default => Ok("iverilog".to_owned()),
        IcarusSimulatorSelector::BluespecDirInstalledBuilder => {
            let relative = Path::new("exec/bsc_build_vsim_iverilog");
            let builder = secure_file_within(
                &toolchain.bluespecdir,
                relative,
                "installed Icarus simulator builder",
            )?;
            ensure_executable_file(&builder, "installed Icarus simulator builder")?;
            Ok(shell_path_for_platform(&builder, cfg!(windows)))
        }
        IcarusSimulatorSelector::PosixEchoProbe => {
            if cfg!(windows) {
                return Err("posix_echo_probe is unavailable on Windows".to_owned());
            }
            let echo = Path::new("/bin/echo");
            let metadata = fs::symlink_metadata(echo)
                .map_err(|error| format!("inspect POSIX echo probe {}: {error}", echo.display()))?;
            if is_link_like(&metadata) || !metadata.is_file() {
                return Err(format!(
                    "POSIX echo probe must be a regular non-link file: {}",
                    echo.display()
                ));
            }
            ensure_executable_file(echo, "POSIX echo probe")?;
            Ok("/bin/echo".to_owned())
        }
        IcarusSimulatorSelector::LiteralBogus => Ok("bogus_sim".to_owned()),
        IcarusSimulatorSelector::BluespecDirBogus => {
            let exec = secure_directory_within(
                &toolchain.bluespecdir,
                Path::new("exec"),
                "installed simulator directory",
            )?;
            let candidate = exec.join("bogus_sim");
            match fs::symlink_metadata(&candidate) {
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(format!(
                        "inspect absent installed bogus simulator {}: {error}",
                        candidate.display()
                    ))
                }
                Ok(_) => {
                    return Err(format!(
                        "installed bogus simulator must remain absent: {}",
                        candidate.display()
                    ))
                }
            }
            Ok(shell_path_for_platform(&candidate, cfg!(windows)))
        }
    }
}

#[cfg(unix)]
fn ensure_executable_file(path: &Path, label: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect {label} {}: {error}", path.display()))?;
    if is_link_like(&metadata) || !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(format!(
            "{label} must be a regular executable non-link file: {}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_executable_file(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect {label} {}: {error}", path.display()))?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "{label} must be a regular executable non-link file: {}",
            path.display()
        ));
    }
    Ok(())
}

fn platform_bsc_path_list_arguments(arguments: &[String]) -> Vec<String> {
    let mut adjusted = arguments.to_vec();
    #[cfg(windows)]
    for index in 0..adjusted.len().saturating_sub(1) {
        if matches!(adjusted[index].as_str(), "-p" | "-vsearch") {
            adjusted[index + 1] = adjusted[index + 1].replace(':', ";");
        }
    }
    adjusted
}

fn simulation_model_extension(backend: SimulationBackend) -> &'static str {
    match backend {
        SimulationBackend::Bluesim => "ba",
        SimulationBackend::Icarus => "v",
    }
}

fn simulation_run_arguments(
    backend: SimulationBackend,
    arguments: &[String],
    vcd: Option<&str>,
) -> Vec<String> {
    let mut invocation_arguments = Vec::with_capacity(arguments.len() + 2);
    match (backend, vcd) {
        (SimulationBackend::Bluesim, Some(path)) => {
            invocation_arguments.extend(["-V".to_owned(), path.to_owned()]);
        }
        (SimulationBackend::Bluesim, None) => {}
        (SimulationBackend::Icarus, Some(_)) => invocation_arguments.push("+bscvcd".to_owned()),
        (SimulationBackend::Icarus, None) => invocation_arguments.push("-vcd-none".to_owned()),
    }
    invocation_arguments.extend_from_slice(arguments);
    invocation_arguments
}

fn c_object_build_arguments(source: &str, output: &str) -> Vec<String> {
    vec![
        "-fPIC".to_owned(),
        "-c".to_owned(),
        source.to_owned(),
        "-o".to_owned(),
        output.to_owned(),
    ]
}

fn validate_c_object_makefile(work_dir: &Path, makefile: &str) -> Result<(), String> {
    let path = work_dir.join(makefile);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("inspect C object makefile {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "C object makefile must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("read C object makefile {}: {error}", path.display()))?;
    if !matches!(
        contents.as_str(),
        "CFLAGS +=-fPIC" | "CFLAGS +=-fPIC\n" | "CFLAGS +=-fPIC\r\n"
    ) {
        return Err(format!(
            "unsupported C object makefile contents: {}",
            path.display()
        ));
    }
    Ok(())
}

fn bsc_flag_preflight_arguments(
    mode: BscFlagPreflightMode,
    input: &str,
    top: Option<&str>,
    unspecified_to: UndeterminedValue,
) -> Result<Vec<String>, String> {
    let value = match unspecified_to {
        UndeterminedValue::X => "X",
        UndeterminedValue::Z => "Z",
    };
    Ok(match mode {
        BscFlagPreflightMode::VerilogNoOptUndetermined => vec![
            "-verilog".to_owned(),
            "-no-opt-undetermined-vals".to_owned(),
            "-unspecified-to".to_owned(),
            value.to_owned(),
            "-no-show-timestamps".to_owned(),
            "-no-show-version".to_owned(),
            "-u".to_owned(),
            input.to_owned(),
        ],
        BscFlagPreflightMode::BluesimLink => {
            let top = top.ok_or_else(|| "Bluesim link flag preflight requires a top".to_owned())?;
            vec![
                "-no-show-timestamps".to_owned(),
                "-no-show-version".to_owned(),
                "-sim".to_owned(),
                "-e".to_owned(),
                top.to_owned(),
                "-o".to_owned(),
                simulation_executable_artifact(SimulationBackend::Bluesim, top),
                "-unspecified-to".to_owned(),
                value.to_ascii_lowercase(),
                input.to_owned(),
            ]
        }
    })
}

fn bsc_systemc_link_arguments(
    systemc_include: &Path,
    top: &str,
    objects: &[String],
) -> Vec<String> {
    let mut arguments = vec![
        "-no-show-timestamps".to_owned(),
        "-no-show-version".to_owned(),
        "-systemc".to_owned(),
        "-e".to_owned(),
        top.to_owned(),
        "-Xc++".to_owned(),
        format!("-I{}", systemc_include.display()),
    ];
    arguments.extend(objects.iter().cloned());
    arguments
}

fn systemc_cxx_link_arguments(
    toolchain: &Toolchain,
    executable: &str,
    sources: &[String],
    top_modules: &[String],
    other_modules: &[String],
    defines: &[String],
) -> Vec<String> {
    let bluesim = toolchain.bluespecdir.join("Bluesim");
    let mut arguments = Vec::new();
    arguments.extend(defines.iter().cloned());
    arguments.extend([
        format!("-I{}", toolchain.systemc_include.display()),
        format!("-L{}", toolchain.systemc_lib.display()),
        format!("-I{}", bluesim.display()),
        format!("-L{}", bluesim.display()),
        "-o".to_owned(),
        format!("{executable}.syscexe"),
    ]);
    arguments.extend(other_modules.iter().map(|module| format!("{module}.o")));
    arguments.extend(top_modules.iter().map(|module| format!("{module}.o")));
    arguments.extend(
        other_modules
            .iter()
            .map(|module| format!("{module}_systemc.o")),
    );
    arguments.extend(
        top_modules
            .iter()
            .map(|module| format!("{module}_systemc.o")),
    );
    arguments.extend(top_modules.iter().map(|module| format!("model_{module}.o")));
    arguments.push("-x".to_owned());
    arguments.push("c++".to_owned());
    arguments.extend(sources.iter().cloned());
    arguments.extend([
        "-lsystemc".to_owned(),
        "-lbskernel".to_owned(),
        "-lbsprim".to_owned(),
    ]);
    if cfg!(windows) {
        arguments.push("-lwinpthread".to_owned());
    } else {
        arguments.push("-pthread".to_owned());
        arguments.push(format!("-Wl,-rpath,{}", toolchain.systemc_lib.display()));
    }
    arguments
}

fn normalize_systemc_output(output: &str, sort_output: bool) -> String {
    let normalized_newlines = output.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized_newlines.lines().skip(4).collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        if lines[index] == "SystemC: simulation stopped by user" {
            index += 1;
            continue;
        }
        if lines[index].is_empty()
            && lines
                .get(index + 1)
                .is_some_and(|line| *line == "Info: /OSCI/SystemC: Simulation stopped by user.")
        {
            index += 2;
            continue;
        }
        normalized.push(lines[index].to_owned());
        index += 1;
    }
    if sort_output {
        normalized.sort_by(|left, right| {
            systemc_numeric_sort_key(left)
                .partial_cmp(&systemc_numeric_sort_key(right))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.cmp(right))
        });
    }
    if normalized.is_empty() {
        String::new()
    } else {
        format!("{}\n", normalized.join("\n"))
    }
}

fn systemc_numeric_sort_key(line: &str) -> f64 {
    let prefix = line.trim_start();
    let end = prefix
        .char_indices()
        .take_while(|(_, character)| {
            character.is_ascii_digit() || matches!(character, '+' | '-' | '.')
        })
        .last()
        .map_or(0, |(index, character)| index + character.len_utf8());
    prefix[..end].parse().unwrap_or(0.0)
}

fn resolve_simulation_executable(
    work_dir: &Path,
    backend: SimulationBackend,
    executable: &str,
) -> Result<PathBuf, String> {
    match backend {
        SimulationBackend::Bluesim => resolve_executable(
            work_dir,
            &simulation_executable_artifact(backend, executable),
        ),
        SimulationBackend::Icarus => {
            let artifact = simulation_executable_artifact(backend, executable);
            let path = work_dir.join(&artifact);
            path.is_file().then_some(path).ok_or_else(|| {
                format!(
                    "BSC did not link Icarus executable {}",
                    work_dir.join(artifact).display()
                )
            })
        }
    }
}

fn icarus_invocation_for_platform(
    executable: &Path,
    arguments: &[String],
    windows: bool,
) -> (PathBuf, Vec<String>) {
    if windows {
        let mut invocation_arguments = Vec::with_capacity(arguments.len() + 1);
        invocation_arguments.push(executable.to_string_lossy().to_string());
        invocation_arguments.extend_from_slice(arguments);
        (PathBuf::from("vvp"), invocation_arguments)
    } else {
        (executable.to_owned(), arguments.to_owned())
    }
}

fn stage_fixture_paths(
    toolchain: &Toolchain,
    plan: &TestPlan,
    fixtures: &[&str],
    work_dir: &Path,
) -> Result<(), String> {
    let source_root = secure_directory_within(
        &toolchain.project_root,
        Path::new(&plan.fixture_dir),
        "fixture directory",
    )?;
    for path in fixtures {
        let fixture = plan
            .fixtures
            .iter()
            .find(|fixture| fixture.path == *path)
            .ok_or_else(|| format!("scenario fixture {path:?} is not registered by the plan"))?;
        let source = resolve_declared_fixture(&source_root, fixture)?;
        let destination = work_dir.join(path);
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

fn execute_scenario(
    toolchain: &Toolchain,
    cache: &ScenarioResultCache,
    package_probes: &BluetclPackageProbeCache,
    plan: &TestPlan,
    scenario: &Scenario,
) -> Result<(), String> {
    let name = sanitize_name(&format!("{}::{}", plan.id, scenario.id));
    let temporary = toolchain.project_root.join(".pixi").join("tmp");
    let work_dir = temporary.join("rust-test-work").join("plans").join(&name);
    let artifact_dir = temporary
        .join("rust-test-artifacts")
        .join("plans")
        .join(&name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;

    let fixtures = scenario
        .fixtures
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let fingerprint = serde_json::to_string(scenario)
        .map_err(|error| format!("serialize scenario fingerprint: {error}"))?;
    let package_states = scenario_bluetcl_packages(scenario)
        .into_iter()
        .map(|package| {
            package_probes
                .available(toolchain, package)
                .map(|available| format!("bluetcl-package:{package:?}={available}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut fingerprint_parts = vec![
        "bsc-test-plan-executor-v5",
        plan.id.as_str(),
        fingerprint.as_str(),
    ];
    fingerprint_parts.extend(package_states.iter().map(String::as_str));
    stage_fixture_paths(toolchain, plan, &fixtures, &work_dir)?;
    let cache_log = artifact_dir.join("plan-cache.log");
    let cache_key =
        match cache.lookup_read_only(&work_dir, &fixtures, &fingerprint_parts, &cache_log)? {
            ReadOnlyCacheLookup::Hit(assertion_snapshots) => {
                check_cached_assertions(plan, scenario, &assertion_snapshots, &artifact_dir)?;
                println!("CACHE {}::{}", plan.id, scenario.id);
                return Ok(());
            }
            ReadOnlyCacheLookup::Miss(key) => Some(key),
            ReadOnlyCacheLookup::Disabled => None,
        };

    for (stage_index, stage) in scenario.stages.iter().enumerate() {
        let stage_artifacts = artifact_dir
            .join("stages")
            .join(format!("{stage_index}-{}", sanitize_name(&stage.id)));
        fs::create_dir_all(&stage_artifacts)
            .map_err(|error| format!("create stage artifact directory: {error}"))?;
        println!("STAGE {}::{}::{}", plan.id, scenario.id, stage.id);
        for (operation_index, operation) in stage.operations.iter().enumerate() {
            if let Some(reason) = operation_skip_reason(toolchain, package_probes, operation)? {
                println!(
                    "  SKIP  {}::{}::{}::{} {}: {reason}",
                    plan.id,
                    scenario.id,
                    stage.id,
                    operation_index + 1,
                    action_name(&operation.action)
                );
                continue;
            }
            println!(
                "  START {}::{}::{}::{} {}",
                plan.id,
                scenario.id,
                stage.id,
                operation_index + 1,
                action_name(&operation.action)
            );
            let started = Instant::now();
            execute_operation(
                toolchain,
                scenario,
                operation,
                stage_index,
                operation_index,
                &work_dir,
                &stage_artifacts,
            )?;
            println!(
                "  DONE  {}::{}::{}::{} {} ({:.3}s)",
                plan.id,
                scenario.id,
                stage.id,
                operation_index + 1,
                action_name(&operation.action),
                started.elapsed().as_secs_f64()
            );
        }
    }
    if let Some(key) = cache_key {
        cache.store(&key, &work_dir)?;
    }
    Ok(())
}

fn execute_operation(
    toolchain: &Toolchain,
    scenario: &Scenario,
    operation: &OperationRecord,
    stage_index: usize,
    operation_index: usize,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), String> {
    let action = &operation.action;
    let expectation = &operation.expectation;
    let result = match action {
        Action::BscOptions {
            args,
            expected_exit,
            bsc_options_prepend,
            stdout,
        } => {
            let mut arguments = vec![
                "-i".to_owned(),
                shell_path_for_platform(&toolchain.bluespecdir, cfg!(windows)),
            ];
            arguments.extend(args.iter().cloned());
            arguments.extend(
                ["-no-show-timestamps", "-no-show-version"]
                    .into_iter()
                    .map(str::to_owned),
            );
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-options.log"));
            let result = if let Some(prepend) = bsc_options_prepend {
                if scenario.bsc_options_append.is_some() {
                    return Err(
                        "bsc.options cannot combine action prepend and scenario append".to_owned(),
                    );
                }
                run_bsc_with_options_prepend(
                    toolchain,
                    &refs,
                    work_dir,
                    &log,
                    Duration::from_secs(scenario.timeouts.generation_seconds),
                    prepend,
                )?
            } else {
                run_bsc_with_options(
                    toolchain,
                    &refs,
                    work_dir,
                    &log,
                    Duration::from_secs(scenario.timeouts.generation_seconds),
                    scenario.bsc_options_append.as_deref(),
                )?
            };
            let output = work_dir.join(stdout);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "create options output directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&output, &result.output)
                .map_err(|error| format!("write options output {}: {error}", output.display()))?;
            let expected_success = *expected_exit == ExpectedExit::Success;
            (result.success == expected_success)
                .then_some(())
                .ok_or_else(|| {
                    format!(
                        "BSC options command {} but expected {}; see {}",
                        if result.success {
                            "succeeded"
                        } else {
                            "failed"
                        },
                        if expected_success {
                            "success"
                        } else {
                            "failure"
                        },
                        log.display()
                    )
                })
        }
        Action::BluetclRun {
            invocation,
            working_directory,
            expected_exit,
            stdout,
            ..
        } => {
            let arguments = bluetcl_arguments(toolchain, invocation)?;
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-bluetcl.log"));
            let execution_directory = match working_directory {
                Some(directory) => secure_directory_within(
                    work_dir,
                    Path::new(directory),
                    "bluetcl.run working directory",
                )?,
                None => work_dir.to_owned(),
            };
            let result = run_command(
                toolchain,
                &toolchain.bluetcl,
                &refs,
                &execution_directory,
                &log,
                Duration::from_secs(scenario.timeouts.assertion_seconds),
            )?;
            let output = work_dir.join(stdout);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "create Bluetcl output directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&output, &result.output)
                .map_err(|error| format!("write Bluetcl output {}: {error}", output.display()))?;
            let contract_result = bluetcl_contract_result(*expected_exit, result.success, &log);
            apply_operation_expectation(contract_result, expectation, action)
        }
        Action::MakeTestData => {
            let arguments = make_test_data_arguments();
            let log = artifact_dir.join(format!("operation-{operation_index}-make-test-data.log"));
            let result = run_command(
                toolchain,
                &toolchain.make,
                &arguments,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.generation_seconds),
            )?;
            result.success.then_some(()).ok_or_else(|| {
                format!(
                    "upstream make test_data failed with exit {}; see {}",
                    describe_exit(result.exit_code),
                    log.display()
                )
            })
        }
        Action::InterraOperatorVectors { suite } => generate_interra_operator_vectors(
            toolchain,
            scenario,
            *suite,
            work_dir,
            artifact_dir,
            operation_index,
        ),
        Action::M4CurdirRender { template, output } => render_m4_curdir(work_dir, template, output),
        Action::TextNormalize {
            source,
            destination,
            transform,
        } => {
            let filter_iverilog_10_1 = if *transform == TextNormalization::IverilogQuietOutput {
                let log =
                    artifact_dir.join(format!("operation-{operation_index}-iverilog-version.log"));
                let result = run_command(
                    toolchain,
                    &toolchain.iverilog,
                    &["-V"],
                    work_dir,
                    &log,
                    Duration::from_secs(scenario.timeouts.assertion_seconds),
                )?;
                if !result.success {
                    return Err(format!(
                        "Icarus version probe failed; see {}",
                        log.display()
                    ));
                }
                Regex::new(r"(?i)Icarus Verilog version\s+10\.1(?:\D|$)")
                    .expect("fixed Icarus 10.1 version regex")
                    .is_match(&result.output)
            } else {
                false
            };
            normalize_text_artifact_with_iverilog_version(
                work_dir,
                source,
                destination,
                *transform,
                filter_iverilog_10_1,
            )
        }
        Action::VerilogFilter {
            path,
            profiles,
            expected_exit,
        } => apply_verilog_filter_pipeline(work_dir, path, profiles, *expected_exit),
        Action::RenderGolden {
            template,
            output,
            replacement,
        } => {
            let template_path = work_dir.join(template);
            let template_contents = fs::read_to_string(&template_path).map_err(|error| {
                format!("read golden template {}: {error}", template_path.display())
            })?;
            let contents = match replacement {
                GoldenReplacement::BluespecDir => template_contents.replace(
                    "BLUESPECDIR",
                    &shell_path_for_platform(&toolchain.bluespecdir, cfg!(windows)),
                ),
                GoldenReplacement::WorkDir => template_contents
                    .replace("HERE", &shell_path_for_platform(work_dir, cfg!(windows))),
                GoldenReplacement::FifoWarningLocations => {
                    derive_fifo_warning_locations(&template_contents)?
                }
            };
            let output_path = work_dir.join(output);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "create rendered golden directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&output_path, contents).map_err(|error| {
                format!("write rendered golden {}: {error}", output_path.display())
            })
        }
        Action::BscFlagPreflight {
            mode,
            input,
            top,
            unspecified_to,
            stdout,
        } => {
            let arguments =
                bsc_flag_preflight_arguments(*mode, input, top.as_deref(), *unspecified_to)?;
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-flag-preflight.log"));
            let result = run_bsc_with_options(
                toolchain,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.generation_seconds),
                scenario.bsc_options_append.as_deref(),
            )?;
            let output = work_dir.join(stdout);
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "create flag preflight output directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&output, &result.output).map_err(|error| {
                format!("write flag preflight output {}: {error}", output.display())
            })?;
            if result.success {
                Err(format!(
                    "BSC flag preflight unexpectedly succeeded; see {}",
                    log.display()
                ))
            } else {
                Ok(())
            }
        }
        Action::BscCompile {
            source,
            working_directory,
            mode,
            module,
            args,
            absolute_import_paths,
            dependency_mode,
            expected_exit,
            unexpected_success_forbidden_regex,
            environment,
            stdout,
        } => {
            let mut arguments = platform_bsc_path_list_arguments(args);
            for path in absolute_import_paths {
                let directory = secure_directory_within(
                    work_dir,
                    Path::new(path),
                    "bsc.compile absolute import path",
                )?;
                arguments.extend([
                    "-p".to_owned(),
                    format!("+:{}", shell_path_for_platform(&directory, cfg!(windows))),
                ]);
            }
            arguments.extend(
                ["-no-show-timestamps", "-no-show-version"]
                    .into_iter()
                    .map(str::to_owned),
            );
            match mode {
                BscCompileMode::Frontend => {
                    if matches!(dependency_mode, DependencyMode::Update) {
                        arguments.push("-u".to_owned());
                    }
                }
                BscCompileMode::BluesimObject => {
                    arguments.extend(["-u".to_owned(), "-sim".to_owned()]);
                    if let Some(module) = module {
                        arguments.extend(["-g".to_owned(), module.clone()]);
                    }
                }
                BscCompileMode::Verilog | BscCompileMode::VerilogSchedule => {
                    arguments.push("-u".to_owned());
                    if matches!(mode, BscCompileMode::VerilogSchedule) {
                        arguments.extend(
                            [
                                "-resource-simple",
                                "-show-schedule",
                                "-dschedule",
                                "-dresources",
                                "-dvschedinfo",
                            ]
                            .into_iter()
                            .map(str::to_owned),
                        );
                    }
                    arguments.push("-verilog".to_owned());
                    if let Some(module) = module {
                        arguments.extend(["-g".to_owned(), module.clone()]);
                    }
                }
                BscCompileMode::Synthesize => {
                    arguments.extend(["-synthesize".to_owned(), "-verilog".to_owned()]);
                    if let Some(module) = module {
                        arguments.extend(["-g".to_owned(), module.clone()]);
                    }
                }
            }
            arguments.push(source.clone());
            let execution_directory = match working_directory {
                Some(directory) => secure_directory_within(
                    work_dir,
                    Path::new(directory),
                    "bsc.compile working directory",
                )?,
                None => work_dir.to_owned(),
            };
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-compile.log"));
            let result = run_bsc_with_options_and_environment(
                toolchain,
                &refs,
                &execution_directory,
                &log,
                Duration::from_secs(scenario.timeouts.generation_seconds),
                scenario.bsc_options_append.as_deref(),
                *environment,
            )?;
            let output = execution_directory.join(stdout);
            fs::write(&output, &result.output)
                .map_err(|error| format!("write compile output {}: {error}", output.display()))?;
            let object_exists = compile_object_exists(
                work_dir,
                &execution_directory,
                source,
                args,
                &operation.artifacts.outputs,
            )?;
            let contract_result = compile_contract_result(
                source,
                *expected_exit,
                unexpected_success_forbidden_regex.as_deref(),
                result.success,
                result.exit_code,
                &result.output,
                object_exists,
                &log,
            );
            apply_operation_expectation(contract_result, expectation, action)
        }
        Action::BscGenerate {
            source,
            mode,
            module,
            args,
        } => {
            let mut arguments = platform_bsc_path_list_arguments(args);
            arguments.extend(
                ["-no-show-timestamps", "-no-show-version", "-u"]
                    .into_iter()
                    .map(str::to_owned),
            );
            arguments.extend(
                simulation_generation_flags(*mode)
                    .iter()
                    .map(|flag| (*flag).to_owned()),
            );
            if let Some(module) = module {
                arguments.extend(["-g".to_owned(), module.clone()]);
            }
            arguments.push(source.clone());
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-generation.log"));
            let result = run_bsc_with_options(
                toolchain,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.generation_seconds),
                scenario.bsc_options_append.as_deref(),
            )?;
            fs::write(
                work_dir.join(mode.compiler_output_path(source)),
                &result.output,
            )
            .map_err(|error| format!("write generation output for {source}: {error}"))?;
            result
                .success
                .then_some(())
                .ok_or_else(|| format!("BSC generation for {source} failed; see {}", log.display()))
        }
        Action::CObjectBuild {
            source,
            makefile,
            output,
        } => {
            validate_c_object_makefile(work_dir, makefile)?;
            let arguments = c_object_build_arguments(source, output);
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-c-object.log"));
            let result = run_command(
                toolchain,
                &toolchain.cc,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.link_seconds),
            )?;
            result
                .success
                .then_some(())
                .ok_or_else(|| format!("C object build for {source} failed; see {}", log.display()))
        }
        Action::BscLink {
            objects,
            top,
            args,
            backend,
            mode,
            expected_exit,
            simulator,
            ..
        } => {
            let log = artifact_dir.join(format!("operation-{operation_index}-link.log"));
            let result = match mode {
                BscLinkMode::Standard => {
                    let mut arguments = vec![
                        "-no-show-timestamps".to_owned(),
                        "-no-show-version".to_owned(),
                    ];
                    match backend {
                        SimulationBackend::Bluesim => arguments.push("-sim".to_owned()),
                        SimulationBackend::Icarus => {
                            arguments.push("-verilog".to_owned());
                            arguments.push("-vsim".to_owned());
                            arguments.push(resolve_icarus_simulator(toolchain, *simulator)?);
                        }
                    }
                    arguments.extend([
                        "-e".to_owned(),
                        top.clone(),
                        "-o".to_owned(),
                        simulation_executable_artifact(*backend, top),
                    ]);
                    arguments.extend(platform_bsc_path_list_arguments(args));
                    let extension = simulation_model_extension(*backend);
                    arguments.extend(objects.iter().map(|object| {
                        if Path::new(object).extension().is_some() {
                            object.clone()
                        } else {
                            format!("{object}.{extension}")
                        }
                    }));
                    let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
                    run_bsc_with_options(
                        toolchain,
                        &refs,
                        work_dir,
                        &log,
                        Duration::from_secs(scenario.timeouts.link_seconds),
                        scenario.bsc_options_append.as_deref(),
                    )?
                }
                BscLinkMode::NoMain => {
                    if *backend != SimulationBackend::Icarus {
                        return Err("no-main linking requires the Icarus backend".to_owned());
                    }
                    let builder = toolchain
                        .bluespecdir
                        .join("exec")
                        .join("bsc_build_vsim_iverilog");
                    if !builder.is_file() {
                        return Err(format!(
                            "Icarus no-main builder is missing at {}",
                            builder.display()
                        ));
                    }
                    let arguments = no_main_icarus_link_arguments(
                        &builder,
                        &toolchain.bluespecdir,
                        top,
                        objects,
                    );
                    let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
                    run_command(
                        toolchain,
                        Path::new("sh"),
                        &refs,
                        work_dir,
                        &log,
                        Duration::from_secs(scenario.timeouts.link_seconds),
                    )?
                }
            };
            let output_name = match (backend, mode) {
                (SimulationBackend::Bluesim, BscLinkMode::Standard) => {
                    format!("{top}.bsc-ccomp-out")
                }
                (_, BscLinkMode::NoMain) | (SimulationBackend::Icarus, BscLinkMode::Standard) => {
                    format!("{top}.bsc-vcomp-out")
                }
            };
            let compiler_output = if matches!(
                (backend, mode),
                (SimulationBackend::Bluesim, BscLinkMode::Standard)
            ) {
                clean_bluesim_link_output(&result.output, cfg!(windows))
            } else {
                result.output.clone()
            };
            fs::write(work_dir.join(output_name), compiler_output)
                .map_err(|error| format!("write link output for {top}: {error}"))?;
            let expected_success = *expected_exit == ExpectedExit::Success;
            let contract_result = if result.success != expected_success {
                Err(format!(
                    "BSC link for {top} {} but expected {}; see {}",
                    if result.success {
                        "succeeded"
                    } else {
                        "failed"
                    },
                    if expected_success {
                        "success"
                    } else {
                        "failure"
                    },
                    log.display()
                ))
            } else if expected_success && simulator.produces_executable() {
                resolve_simulation_executable(work_dir, *backend, top).map(|_| ())
            } else {
                Ok(())
            };
            apply_operation_expectation(contract_result, expectation, action)
        }
        Action::BscSystemcLink {
            objects,
            top,
            expected_exit,
        } => {
            let arguments = bsc_systemc_link_arguments(&toolchain.systemc_include, top, objects);
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-systemc-link.log"));
            let result = run_bsc(
                toolchain,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.link_seconds),
            )?;
            fs::write(
                work_dir.join(format!("{top}.bsc-ccomp-out")),
                &result.output,
            )
            .map_err(|error| format!("write SystemC link output for {top}: {error}"))?;
            let expected_success = *expected_exit == ExpectedExit::Success;
            (result.success == expected_success)
                .then_some(())
                .ok_or_else(|| {
                    format!(
                        "BSC SystemC link for {top} {} but expected {}; see {}",
                        if result.success {
                            "succeeded"
                        } else {
                            "failed"
                        },
                        if expected_success {
                            "success"
                        } else {
                            "failure"
                        },
                        log.display()
                    )
                })
        }
        Action::SystemcCxxLink {
            executable,
            sources,
            top_modules,
            other_modules,
            defines,
        } => {
            let arguments = systemc_cxx_link_arguments(
                toolchain,
                executable,
                sources,
                top_modules,
                other_modules,
                defines,
            );
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log =
                artifact_dir.join(format!("operation-{operation_index}-systemc-cxx-link.log"));
            let result = run_command(
                toolchain,
                &toolchain.cxx,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.link_seconds),
            )?;
            fs::write(
                work_dir.join(format!("{executable}.cxx-comp-out")),
                &result.output,
            )
            .map_err(|error| format!("write SystemC C++ output for {executable}: {error}"))?;
            result.success.then_some(()).ok_or_else(|| {
                format!(
                    "SystemC C++ link for {executable} failed; see {}",
                    log.display()
                )
            })
        }
        Action::SystemcRun {
            executable,
            stdout,
            sort_output,
        } => {
            let program = work_dir.join(format!("{executable}.syscexe"));
            if !program.is_file() {
                return Err(format!(
                    "SystemC executable is missing: {}",
                    program.display()
                ));
            }
            let log = artifact_dir.join(format!("operation-{operation_index}-systemc-run.log"));
            let result = run_command(
                toolchain,
                &program,
                &[],
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.simulation_seconds),
            )?;
            fs::write(
                work_dir.join(format!("{executable}.raw.out")),
                &result.output,
            )
            .map_err(|error| format!("write SystemC raw output for {executable}: {error}"))?;
            fs::write(
                work_dir.join(stdout),
                normalize_systemc_output(&result.output, *sort_output),
            )
            .map_err(|error| format!("write SystemC output {stdout}: {error}"))?;
            (result.exit_code == Some(0)).then_some(()).ok_or_else(|| {
                format!(
                    "SystemC simulation exit code {:?}, expected 0; see {}",
                    result.exit_code,
                    log.display()
                )
            })
        }
        Action::BscParsePretty {
            source,
            args,
            pretty_output,
        } => {
            let roundtrip = (|| {
                let mut first_arguments = args.clone();
                first_arguments.extend(
                    ["-no-show-timestamps", "-no-show-version", "-u"]
                        .into_iter()
                        .map(str::to_owned),
                );
                first_arguments.push(format!("-dparsed={pretty_output}"));
                first_arguments.push(source.clone());
                let first_refs = first_arguments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                let first_log = artifact_dir.join(format!(
                    "operation-{operation_index}-parse-pretty-first.log"
                ));
                let first = run_bsc_with_options(
                    toolchain,
                    &first_refs,
                    work_dir,
                    &first_log,
                    Duration::from_secs(scenario.timeouts.generation_seconds),
                    scenario.bsc_options_append.as_deref(),
                )?;
                fs::write(work_dir.join(format!("{source}.bsc-out")), &first.output).map_err(
                    |error| format!("write first parse-pretty output for {source}: {error}"),
                )?;
                if !first.success {
                    return Err(format!(
                        "BSC parse-pretty compile of {source} failed; see {}",
                        first_log.display()
                    ));
                }

                strip_dump_wrapper(&work_dir.join(pretty_output))?;

                let mut second_arguments = args.clone();
                second_arguments.extend(
                    ["-no-show-timestamps", "-no-show-version", "-u"]
                        .into_iter()
                        .map(str::to_owned),
                );
                second_arguments.push(pretty_output.clone());
                let second_refs = second_arguments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                let second_log = artifact_dir.join(format!(
                    "operation-{operation_index}-parse-pretty-second.log"
                ));
                let second = run_bsc_with_options(
                    toolchain,
                    &second_refs,
                    work_dir,
                    &second_log,
                    Duration::from_secs(scenario.timeouts.generation_seconds),
                    scenario.bsc_options_append.as_deref(),
                )?;
                fs::write(
                    work_dir.join(format!("{pretty_output}.bsc-out")),
                    &second.output,
                )
                .map_err(|error| {
                    format!("write second parse-pretty output for {pretty_output}: {error}")
                })?;
                second.success.then_some(()).ok_or_else(|| {
                    format!(
                        "BSC compile of pretty-printed source {pretty_output} failed; see {}",
                        second_log.display()
                    )
                })
            })();
            apply_operation_expectation(roundtrip, expectation, action)
        }
        Action::Bsc2Bsv { source, stdout } => {
            let log = artifact_dir.join(format!("operation-{operation_index}-bsc2bsv.log"));
            let result = run_command(
                toolchain,
                &toolchain.bsc2bsv,
                &[source.as_str()],
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.assertion_seconds),
            )?;
            fs::write(work_dir.join(stdout), &result.output)
                .map_err(|error| format!("write bsc2bsv output {stdout}: {error}"))?;
            result.success.then_some(()).ok_or_else(|| {
                format!(
                    "bsc2bsv conversion of {source} failed; see {}",
                    log.display()
                )
            })
        }
        Action::DumpIntermediate {
            input,
            output,
            view,
        } => {
            let arguments = match view {
                IntermediateDumpView::Bi => vec!["-bi", input.as_str()],
                IntermediateDumpView::Bo => vec![input.as_str()],
            };
            let log = artifact_dir.join(format!("operation-{operation_index}-dump.log"));
            let result = run_command(
                toolchain,
                &toolchain.dumpbo,
                &arguments,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.assertion_seconds),
            )?;
            fs::write(work_dir.join(output), &result.output)
                .map_err(|error| format!("write intermediate dump {output}: {error}"))?;
            result.success.then_some(()).ok_or_else(|| {
                format!("intermediate dump of {input} failed; see {}", log.display())
            })
        }
        Action::SimulationRun {
            backend,
            executable,
            args,
            stdout,
            expected_exits,
            vcd,
        } => {
            let executable_path = resolve_simulation_executable(work_dir, *backend, executable)?;
            let simulation_arguments = simulation_run_arguments(*backend, args, vcd.as_deref());
            let (program, invocation_arguments) = match backend {
                SimulationBackend::Bluesim => {
                    let artifact = simulation_executable_artifact(*backend, executable);
                    let invocation =
                        bluesim_invocation(&executable_path, &artifact, &simulation_arguments);
                    (invocation.program, invocation.arguments)
                }
                SimulationBackend::Icarus => icarus_invocation_for_platform(
                    &executable_path,
                    &simulation_arguments,
                    cfg!(windows),
                ),
            };
            let refs = invocation_arguments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-simulation.log"));
            let result = run_command(
                toolchain,
                &program,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.simulation_seconds),
            )?;
            let output = match backend {
                SimulationBackend::Bluesim => result.output.clone(),
                SimulationBackend::Icarus => clean_iverilog_output(&result.output),
            };
            fs::write(work_dir.join(stdout), output)
                .map_err(|error| format!("write simulation output {stdout}: {error}"))?;
            if matches!(backend, SimulationBackend::Icarus) {
                if let Some(path) = vcd {
                    let generated = work_dir.join("dump.vcd");
                    if generated.is_file() && path != "dump.vcd" {
                        strict_transfer(work_dir, "dump.vcd", path, true)?;
                    }
                }
            }
            result
                .exit_code
                .is_some_and(|code| expected_exits.accepts_current_platform(code))
                .then_some(())
                .ok_or_else(|| {
                    format!(
                        "{backend:?} simulation exit code {:?}, expected {:?}; see {}",
                        result.exit_code,
                        expected_exits,
                        log.display()
                    )
                })
        }
        Action::ShowRules {
            top,
            input,
            output,
            design_inputs,
            stdout,
        } => {
            let program = toolchain.showrules.as_ref().ok_or_else(|| {
                "showrules is unavailable despite the scenario requirement".to_owned()
            })?;
            ensure_executable_file(program, "showrules executable")?;
            ensure_regular_artifact(work_dir, input, "showrules input VCD")?;
            for design_input in design_inputs {
                ensure_regular_artifact(work_dir, design_input, "showrules design input")?;
            }
            ensure_artifact_target_absent(work_dir, output, "showrules output VCD")?;
            ensure_artifact_target_absent(work_dir, stdout, "showrules stdout")?;
            let arguments = showrules_arguments(top, input, output);
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-showrules.log"));
            let result = run_command(
                toolchain,
                program,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.assertion_seconds),
            )?;
            let stdout_path = secure_artifact_path(work_dir, stdout)?;
            fs::write(&stdout_path, &result.output).map_err(|error| {
                format!("write showrules stdout {}: {error}", stdout_path.display())
            })?;
            if !result.success {
                return Err(format!(
                    "showrules for {input} failed with exit {}; see {}",
                    describe_exit(result.exit_code),
                    log.display()
                ));
            }
            ensure_regular_artifact(work_dir, output, "showrules output VCD")?;
            ensure_regular_artifact(work_dir, stdout, "showrules stdout")
        }
        Action::VcdCheck {
            path,
            checks,
            expected_exit,
        } => {
            let mut arguments = Vec::with_capacity(checks.len() * 2 + 1);
            for check in checks {
                arguments.extend(["-c".to_owned(), check.clone()]);
            }
            arguments.push(path.clone());
            let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let log = artifact_dir.join(format!("operation-{operation_index}-vcdcheck.log"));
            let result = run_command(
                toolchain,
                &toolchain.vcdcheck,
                &refs,
                work_dir,
                &log,
                Duration::from_secs(scenario.timeouts.assertion_seconds),
            )?;
            let expected_success = *expected_exit == ExpectedExit::Success;
            let contract_result = (result.success == expected_success)
                .then_some(())
                .ok_or_else(|| {
                    format!(
                        "vcdcheck for {path} {} but expected {}; see {}",
                        if result.success {
                            "succeeded"
                        } else {
                            "failed"
                        },
                        if expected_success {
                            "success"
                        } else {
                            "failure"
                        },
                        log.display()
                    )
                });
            apply_operation_expectation(contract_result, expectation, action)
        }
        Action::FsCopy {
            source,
            destination,
        } => strict_transfer(work_dir, source, destination, false),
        Action::FsCopyReplace {
            source,
            destination,
        } => strict_copy_replace(work_dir, source, destination),
        Action::FsRewriteDarwinCppIncludePath {
            source,
            destination,
        } => rewrite_darwin_cpp_include_path(work_dir, source, destination),
        Action::FsMove {
            source,
            destination,
        } => strict_transfer(work_dir, source, destination, true),
        Action::FsMoveReplace {
            source,
            destination,
        } => strict_move_replace(work_dir, source, destination),
        Action::FsRemove { path } => {
            let path = secure_artifact_path(work_dir, path)?;
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                format!("inspect removable artifact {}: {error}", path.display())
            })?;
            if is_link_like(&metadata) || !metadata.is_file() {
                return Err(format!(
                    "removable artifact must be a regular non-link file: {}",
                    path.display()
                ));
            }
            fs::remove_file(&path)
                .map_err(|error| format!("remove artifact {}: {error}", path.display()))
        }
        Action::FsEnsureAbsent { path } => ensure_file_absent(work_dir, path),
        Action::FsEnsureDirectoryAbsent { path } => ensure_directory_absent(work_dir, path),
        Action::FsMkdir { path } => fs::create_dir(work_dir.join(path))
            .map_err(|error| format!("create plan directory {path}: {error}")),
        Action::FsCreateDirAll { path } => ensure_directory_exists(work_dir, path),
        Action::FsTouch { path } => strict_touch(work_dir, path),
        Action::FsTouchCreate {
            path,
            delay_milliseconds,
        } => touch_create_update(work_dir, path, *delay_milliseconds),
        Action::FsRemoveUserRead { path } => remove_user_read(work_dir, path),
        Action::Delay { milliseconds } => {
            std::thread::sleep(Duration::from_millis(*milliseconds));
            Ok(())
        }
        action if action.is_assertion() => {
            let snapshot =
                snapshot_asserted_artifact(action, stage_index, operation_index, work_dir)?;
            apply_assertion_expectation(
                check_plan_assertion_typed(action, &snapshot, &snapshot, artifact_dir, "Test Plan"),
                expectation,
                action,
            )
        }
        _ => Err("unsupported Test Plan operation".to_owned()),
    };
    result.and_then(|()| {
        if matches!(operation.expectation, OperationExpectation::Xfail { .. }) {
            Ok(())
        } else {
            verify_artifact_contract(work_dir, operation)
        }
    })
}

fn verify_artifact_contract(work_dir: &Path, operation: &OperationRecord) -> Result<(), String> {
    for output in &operation.artifacts.outputs {
        if is_optional_link_intermediate(&operation.action, output) {
            continue;
        }
        let path = work_dir.join(output);
        if !path.exists() {
            return Err(format!(
                "{} did not produce declared artifact {}",
                action_name(&operation.action),
                path.display()
            ));
        }
    }
    for alternatives in &operation.artifacts.output_alternatives {
        if !alternatives
            .iter()
            .any(|output| work_dir.join(output).exists())
        {
            return Err(format!(
                "{} did not produce any declared artifact alternative {:?}",
                action_name(&operation.action),
                alternatives
            ));
        }
    }
    for directory in &operation.artifacts.directories {
        let path = work_dir.join(directory);
        if !path.is_dir() {
            return Err(format!(
                "{} did not create declared directory {}",
                action_name(&operation.action),
                path.display()
            ));
        }
    }
    for removed in &operation.artifacts.removes {
        let path = work_dir.join(removed);
        if path.exists() {
            return Err(format!(
                "{} did not remove declared artifact {}",
                action_name(&operation.action),
                path.display()
            ));
        }
    }
    Ok(())
}

fn is_optional_link_intermediate(action: &Action, output: &str) -> bool {
    let Action::BscLink {
        backend: SimulationBackend::Bluesim,
        top,
        ..
    } = action
    else {
        return false;
    };
    matches!(
        output,
        path if path == format!("{top}.cxx")
            || path == format!("model_{top}.cxx")
            || path == format!("{top}.o")
            || path == format!("model_{top}.o")
    )
}

fn bluetcl_arguments(
    toolchain: &Toolchain,
    invocation: &BluetclInvocation,
) -> Result<Vec<String>, String> {
    let installed_script = match invocation {
        BluetclInvocation::InstalledScript { script, .. } => Some(
            installed_bluetcl_script(toolchain, *script)?
                .ok_or_else(|| format!("installed Bluetcl script {script:?} is unavailable"))?,
        ),
        _ => None,
    };
    Ok(bluetcl_arguments_with_installed_path(
        invocation,
        installed_script.as_deref(),
    ))
}

fn bluetcl_arguments_with_installed_path(
    invocation: &BluetclInvocation,
    installed_script: Option<&Path>,
) -> Vec<String> {
    match invocation {
        BluetclInvocation::Script {
            script,
            args,
            syntax,
        } => {
            let mut arguments = Vec::with_capacity(args.len() + 2);
            arguments.push(script.clone());
            arguments.extend(args.iter().cloned());
            if *syntax == BluetclSyntax::Bh {
                arguments.push("-bh".to_owned());
            }
            arguments
        }
        BluetclInvocation::Exec { script, args } => {
            let mut arguments = Vec::with_capacity(args.len() + 2);
            arguments.extend(["-exec".to_owned(), script.clone()]);
            arguments.extend(args.iter().cloned());
            arguments
        }
        BluetclInvocation::InstalledScript { args, .. } => {
            let script = installed_script.expect("installed script path is resolved before argv");
            let mut arguments = Vec::with_capacity(args.len() + 1);
            arguments.push(shell_path_for_platform(script, cfg!(windows)));
            arguments.extend(args.iter().cloned());
            arguments
        }
        BluetclInvocation::Makedepend { command, args } => {
            let executable = match command {
                BluetclMakedependCommand::Makedepend => "makedepend",
                BluetclMakedependCommand::MakedependTcl => "makedepend.tcl",
            };
            let mut arguments = Vec::with_capacity(args.len() + 2);
            arguments.extend(["-exec".to_owned(), executable.to_owned()]);
            arguments.extend(args.iter().cloned());
            arguments
        }
    }
}

fn installed_bluetcl_script(
    toolchain: &Toolchain,
    script: BluetclInstalledScript,
) -> Result<Option<PathBuf>, String> {
    let (relative, companions): (&Path, &[&str]) = match script {
        BluetclInstalledScript::ExpandPorts => (
            Path::new("tcllib/bluespec/expandPorts.tcl"),
            &["tcllib/bluespec/portUtil.tcl"],
        ),
    };
    let candidate = toolchain.bluespecdir.join(relative);
    match fs::symlink_metadata(&candidate) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "inspect installed Bluetcl script {}: {error}",
                candidate.display()
            ))
        }
        Ok(_) => {}
    }
    let script_path =
        secure_file_within(&toolchain.bluespecdir, relative, "installed Bluetcl script")?;
    for companion in companions {
        let relative = Path::new(companion);
        let candidate = toolchain.bluespecdir.join(relative);
        match fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "inspect installed Bluetcl companion {}: {error}",
                    candidate.display()
                ))
            }
            Ok(_) => {}
        }
        secure_file_within(
            &toolchain.bluespecdir,
            relative,
            "installed Bluetcl companion",
        )?;
    }
    Ok(Some(script_path))
}

fn probe_bluetcl_package(toolchain: &Toolchain, package: BluetclPackage) -> Result<bool, String> {
    match package {
        BluetclPackage::ExpandPorts => {
            installed_bluetcl_script(toolchain, BluetclInstalledScript::ExpandPorts)
                .map(|path| path.is_some())
        }
        BluetclPackage::InstSynth => {
            let probe_dir = toolchain
                .project_root
                .join(".pixi")
                .join("tmp")
                .join("rust-test-package-probes")
                .join(std::process::id().to_string())
                .join("instsynth");
            reset_directory(&probe_dir)?;
            let script_name = Path::new("probe-inst-synth.tcl");
            let script_path = probe_dir.join(script_name);
            fs::write(&script_path, b"package require InstSynth\nexit 0\n").map_err(|error| {
                format!(
                    "write fixed InstSynth package probe {}: {error}",
                    script_path.display()
                )
            })?;
            secure_file_within(&probe_dir, script_name, "InstSynth package probe script")?;
            let log = probe_dir.join("probe.log");
            let result = run_command(
                toolchain,
                &toolchain.bluetcl,
                &["probe-inst-synth.tcl"],
                &probe_dir,
                &log,
                Duration::from_secs(30),
            )?;
            Ok(result.success)
        }
    }
}

fn bluetcl_contract_result(
    expected_exit: ExpectedExit,
    success: bool,
    log: &Path,
) -> Result<(), String> {
    let expected_success = expected_exit == ExpectedExit::Success;
    (success == expected_success).then_some(()).ok_or_else(|| {
        format!(
            "Bluetcl {} but expected {}; see {}",
            if success { "succeeded" } else { "failed" },
            if expected_success {
                "success"
            } else {
                "failure"
            },
            log.display()
        )
    })
}

fn derive_fifo_warning_locations(contents: &str) -> Result<String, String> {
    let mut derived = String::new();
    for (index, line) in contents.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let (kind, remainder) = line.split_once(": ").ok_or_else(|| {
            format!(
                "FIFO warning golden line {} has no warning category separator",
                index + 1
            )
        })?;
        let (_, details) = remainder.split_once(": ").ok_or_else(|| {
            format!(
                "FIFO warning golden line {} has no simulator location separator",
                index + 1
            )
        })?;
        if kind != "Warning" {
            return Err(format!(
                "FIFO warning golden line {} has unexpected category {kind:?}",
                index + 1
            ));
        }
        let details = details.strip_prefix("main.").ok_or_else(|| {
            format!(
                "FIFO warning golden line {} has no main. location prefix",
                index + 1
            )
        })?;
        if !details.contains(".error_checks") {
            return Err(format!(
                "FIFO warning golden line {} has no .error_checks location",
                index + 1
            ));
        }
        derived.push_str("Warning: ");
        derived.push_str(&details.replace(".error_checks", ""));
        derived.push('\n');
    }
    if derived.is_empty() {
        return Err("FIFO warning golden contains no warning lines".to_owned());
    }
    Ok(derived)
}

fn strip_dump_wrapper(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("read parse-pretty dump {}: {error}", path.display()))?;
    let mut lines = content.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("parse-pretty dump {} has no header", path.display()))?;
    if !header.starts_with("===") {
        return Err(format!(
            "parse-pretty dump {} has an invalid header",
            path.display()
        ));
    }
    let body = lines.collect::<Vec<_>>();
    let footer = body
        .iter()
        .position(|line| *line == "-----")
        .ok_or_else(|| format!("parse-pretty dump {} has no footer marker", path.display()))?;
    fs::write(path, body[..footer].join("\n")).map_err(|error| {
        format!(
            "write stripped parse-pretty dump {}: {error}",
            path.display()
        )
    })
}

fn action_name(action: &Action) -> &'static str {
    match action {
        Action::BscCompile { .. } => "bsc.compile",
        Action::BscOptions { .. } => "bsc.options",
        Action::BscFlagPreflight { .. } => "bsc.flag_preflight",
        Action::BluetclRun { .. } => "bluetcl.run",
        Action::MakeTestData => "upstream.make_test_data",
        Action::InterraOperatorVectors { .. } => "fixture.interra_operator_vectors",
        Action::Bsc2Bsv { .. } => "internal.bsc2bsv",
        Action::BscParsePretty { .. } => "bsc.parse_pretty_roundtrip",
        Action::DumpIntermediate { .. } => "internal.dump",
        Action::RenderGolden { .. } => "golden.render",
        Action::M4CurdirRender { .. } => "template.m4_curdir",
        Action::TextNormalize { .. } => "text.normalize",
        Action::VerilogFilter { .. } => "verilog.filter",
        Action::BscGenerate { .. } => "bsc.generate",
        Action::CObjectBuild { .. } => "c.compile_object",
        Action::BscLink { .. } => "bsc.link",
        Action::BscSystemcLink { .. } => "bsc.systemc_link",
        Action::SystemcCxxLink { .. } => "systemc.cxx_link",
        Action::SystemcRun { .. } => "systemc.run",
        Action::SimulationRun { .. } => "simulation.run",
        Action::ShowRules { .. } => "vcd.showrules",
        Action::VcdCheck { .. } => "vcd.check",
        Action::FsCopy { .. } => "fs.copy",
        Action::FsCopyReplace { .. } => "fs.copy_replace",
        Action::FsRewriteDarwinCppIncludePath { .. } => "fs.rewrite_darwin_cpp_include_path",
        Action::FsMove { .. } => "fs.move",
        Action::FsMoveReplace { .. } => "fs.move_replace",
        Action::FsRemove { .. } => "fs.remove",
        Action::FsEnsureAbsent { .. } => "fs.ensure_absent",
        Action::FsEnsureDirectoryAbsent { .. } => "fs.ensure_dir_absent",
        Action::FsMkdir { .. } => "fs.mkdir",
        Action::FsCreateDirAll { .. } => "fs.create_dir_all",
        Action::FsTouch { .. } => "fs.touch",
        Action::FsTouchCreate { .. } => "fs.touch_create",
        Action::FsRemoveUserRead { .. } => "fs.remove_user_read",
        Action::Delay { .. } => "time.delay",
        _ => "assertion",
    }
}

fn compile_object_exists(
    work_dir: &Path,
    execution_directory: &Path,
    source: &str,
    args: &[String],
    declared_outputs: &[String],
) -> Result<Option<bool>, String> {
    if args.iter().any(|argument| argument.starts_with("-KILL")) {
        return Ok(Some(true));
    }
    let declared_objects = declared_outputs
        .iter()
        .filter(|path| Path::new(path).extension().and_then(|value| value.to_str()) == Some("bo"))
        .collect::<Vec<_>>();
    if !declared_objects.is_empty() {
        for path in declared_objects {
            let path = secure_artifact_path(work_dir, path)?;
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Some(false)),
                Err(error) => {
                    return Err(format!(
                        "inspect declared compile object {}: {error}",
                        path.display()
                    ))
                }
            };
            if is_link_like(&metadata) || !metadata.is_file() {
                return Ok(Some(false));
            }
        }
        return Ok(Some(true));
    }
    let Some(stem) = Path::new(source).file_stem() else {
        return Ok(None);
    };
    let path = execution_directory.join(stem).with_extension("bo");
    match fs::symlink_metadata(&path) {
        Ok(metadata) => Ok(Some(!is_link_like(&metadata) && metadata.is_file())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Some(false)),
        Err(error) => Err(format!(
            "inspect default compile object {}: {error}",
            path.display()
        )),
    }
}

fn compile_contract_result(
    source: &str,
    expected_exit: ExpectedExit,
    unexpected_success_forbidden_regex: Option<&str>,
    success: bool,
    exit_code: Option<i32>,
    output: &str,
    object_exists: Option<bool>,
    log: &Path,
) -> Result<(), String> {
    match expected_exit {
        ExpectedExit::Success if !success => Err(format!(
            "BSC should compile {source} but exited {}; see {}",
            describe_exit(exit_code),
            log.display()
        )),
        ExpectedExit::Failure if success => {
            let forbidden_match = unexpected_success_forbidden_regex
                .and_then(|pattern| Regex::new(pattern).ok().map(|regex| (pattern, regex)))
                .is_some_and(|(_, regex)| regex.is_match(output));
            Err(if forbidden_match {
                format!(
                    "BSC should reject {source} but succeeded and its output matched forbidden regex {:?}; see {}",
                    unexpected_success_forbidden_regex.unwrap_or_default(),
                    log.display()
                )
            } else {
                format!(
                    "BSC should reject {source} but succeeded; see {}",
                    log.display()
                )
            })
        }
        ExpectedExit::Success => match object_exists {
            Some(true) => Ok(()),
            Some(false) => Err(format!(
                "BSC succeeded but did not create the expected object for {source}; see {}",
                log.display()
            )),
            None => Err(format!("source has no file stem: {source}")),
        },
        ExpectedExit::Failure | ExpectedExit::Unchecked => Ok(()),
    }
}

fn apply_assertion_expectation(
    result: Result<(), PlanAssertionFailure>,
    expectation: &OperationExpectation,
    action: &Action,
) -> Result<(), String> {
    match result {
        Ok(()) => apply_operation_expectation(Ok(()), expectation, action),
        Err(PlanAssertionFailure::ContractMismatch(message)) => {
            apply_operation_expectation(Err(message), expectation, action)
        }
        Err(PlanAssertionFailure::Infrastructure(message)) => Err(message),
    }
}

fn apply_operation_expectation(
    result: Result<(), String>,
    expectation: &OperationExpectation,
    action: &Action,
) -> Result<(), String> {
    match (expectation, result) {
        (OperationExpectation::Required, result) => result,
        (OperationExpectation::Xfail { reason }, Err(message)) => {
            println!("XFAIL: {message} ({reason})");
            Ok(())
        }
        (OperationExpectation::Xfail { reason }, Ok(())) => Err(format!(
            "XPASS: {action:?} unexpectedly satisfied its contract ({reason})"
        )),
    }
}

fn check_cached_assertions(
    plan: &TestPlan,
    scenario: &Scenario,
    assertion_snapshots: &Path,
    artifact_dir: &Path,
) -> Result<(), String> {
    for (stage_index, stage) in scenario.stages.iter().enumerate() {
        let stage_artifacts = artifact_dir
            .join("stages")
            .join(format!("{stage_index}-{}", sanitize_name(&stage.id)));
        fs::create_dir_all(&stage_artifacts)
            .map_err(|error| format!("create cached stage artifact directory: {error}"))?;
        for (operation_index, operation) in stage.operations.iter().enumerate() {
            if platform_operation_skip_reason(operation).is_some() {
                continue;
            }
            if operation.action.is_assertion() {
                let snapshot = assertion_snapshots
                    .join(stage_index.to_string())
                    .join(operation_index.to_string());
                apply_assertion_expectation(
                    check_plan_assertion_typed(
                        &operation.action,
                        &snapshot,
                        &snapshot,
                        &stage_artifacts,
                        &plan.id,
                    ),
                    &operation.expectation,
                    &operation.action,
                )?;
            }
        }
    }
    Ok(())
}

fn snapshot_asserted_artifact(
    action: &Action,
    stage_index: usize,
    operation_index: usize,
    work_dir: &Path,
) -> Result<PathBuf, String> {
    let actual = action
        .asserted_path()
        .ok_or_else(|| "cannot snapshot a non-assertion operation".to_owned())?;
    let root = assertion_snapshot_root(work_dir, stage_index, operation_index);
    if matches!(action, Action::AssertVcdValidIfPresent { .. }) && !work_dir.join(actual).is_file()
    {
        fs::create_dir_all(&root)
            .map_err(|error| format!("create optional assertion snapshot: {error}"))?;
        return Ok(root);
    }
    snapshot_path(work_dir, &root, actual)?;
    for expected in action.expected_paths() {
        snapshot_path(work_dir, &root, expected)?;
    }
    Ok(root)
}

fn snapshot_path(work_dir: &Path, root: &Path, path: &str) -> Result<(), String> {
    let source = work_dir.join(path);
    if !source.is_file() {
        return Err(format!(
            "asserted artifact does not exist: {}",
            source.display()
        ));
    }
    let destination = root.join(path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create assertion snapshot directory: {error}"))?;
    }
    fs::copy(&source, &destination).map_err(|error| {
        format!(
            "snapshot asserted artifact {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn assertion_snapshot_root(work_dir: &Path, stage_index: usize, operation_index: usize) -> PathBuf {
    work_dir
        .join(".bsc-test-plan")
        .join("assertions")
        .join(stage_index.to_string())
        .join(operation_index.to_string())
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
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

fn is_link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || is_reparse_point(metadata)
}

fn validate_removable_directory_tree(path: &Path) -> Result<(), String> {
    for entry in fs::read_dir(path)
        .map_err(|error| format!("read removable directory {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| {
            format!("read removable directory entry {}: {error}", path.display())
        })?;
        let child = entry.path();
        let metadata = fs::symlink_metadata(&child)
            .map_err(|error| format!("inspect removable path {}: {error}", child.display()))?;
        if is_link_like(&metadata) {
            return Err(format!(
                "refusing to remove directory tree containing a symlink or reparse point: {}",
                child.display()
            ));
        }
        if metadata.is_dir() {
            validate_removable_directory_tree(&child)?;
        } else if !metadata.is_file() {
            return Err(format!(
                "refusing to remove directory tree containing a special filesystem node: {}",
                child.display()
            ));
        }
    }
    Ok(())
}

fn ensure_file_absent(work_dir: &Path, path: &str) -> Result<(), String> {
    let root = fs::canonicalize(work_dir).map_err(|error| {
        format!(
            "canonicalize plan work directory {}: {error}",
            work_dir.display()
        )
    })?;
    let root_metadata = fs::symlink_metadata(&root)
        .map_err(|error| format!("inspect plan work directory {}: {error}", root.display()))?;
    if is_link_like(&root_metadata) || !root_metadata.is_dir() {
        return Err(format!(
            "plan work directory must be a regular non-link directory: {}",
            root.display()
        ));
    }
    let components = Path::new(path).components().collect::<Vec<_>>();
    if components.is_empty() {
        return Err("artifact path must not be empty".to_owned());
    }
    let mut target = root;
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(format!("invalid plan artifact path: {path}"));
        };
        target.push(component);
        let metadata = match fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!(
                    "inspect ensure-absent artifact {}: {error}",
                    target.display()
                ))
            }
        };
        if index + 1 == components.len() {
            if is_link_like(&metadata) || !metadata.is_file() {
                return Err(format!(
                    "ensure-absent target must be a regular non-link file: {}",
                    target.display()
                ));
            }
            return fs::remove_file(&target).map_err(|error| {
                format!("ensure artifact {} is absent: {error}", target.display())
            });
        }
        if is_link_like(&metadata) || !metadata.is_dir() {
            return Err(format!(
                "plan artifact parent must be a regular non-link directory: {}",
                target.display()
            ));
        }
    }
    unreachable!("non-empty artifact path has a final component")
}

fn ensure_directory_absent(work_dir: &Path, path: &str) -> Result<(), String> {
    let root_metadata = fs::symlink_metadata(work_dir).map_err(|error| {
        format!(
            "inspect plan work directory {}: {error}",
            work_dir.display()
        )
    })?;
    if is_link_like(&root_metadata) || !root_metadata.is_dir() {
        return Err(format!(
            "plan work directory must be a regular non-link directory: {}",
            work_dir.display()
        ));
    }
    let root = fs::canonicalize(work_dir).map_err(|error| {
        format!(
            "canonicalize plan work directory {}: {error}",
            work_dir.display()
        )
    })?;
    let relative = Path::new(path);
    let mut parent = root.clone();
    if let Some(components) = relative.parent() {
        for component in components.components() {
            let Component::Normal(component) = component else {
                return Err(format!("invalid plan directory path: {path}"));
            };
            parent.push(component);
            let metadata = fs::symlink_metadata(&parent).map_err(|error| {
                format!(
                    "inspect plan directory parent {}: {error}",
                    parent.display()
                )
            })?;
            if is_link_like(&metadata) || !metadata.is_dir() {
                return Err(format!(
                    "plan directory parent must be a regular non-link directory: {}",
                    parent.display()
                ));
            }
        }
    }
    let target = root.join(relative);
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "inspect plan directory {}: {error}",
                target.display()
            ))
        }
    };
    if is_link_like(&metadata) || !metadata.is_dir() {
        return Err(format!(
            "plan directory removal target must be a regular non-link directory: {}",
            target.display()
        ));
    }
    validate_removable_directory_tree(&target)?;
    fs::remove_dir_all(&target)
        .map_err(|error| format!("remove plan directory {}: {error}", target.display()))?;
    match fs::symlink_metadata(&target) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(format!(
            "plan directory still exists after removal: {}",
            target.display()
        )),
        Err(error) => Err(format!(
            "verify removed plan directory {}: {error}",
            target.display()
        )),
    }
}

fn ensure_directory_exists(work_dir: &Path, path: &str) -> Result<(), String> {
    let root = fs::canonicalize(work_dir).map_err(|error| {
        format!(
            "canonicalize plan work directory {}: {error}",
            work_dir.display()
        )
    })?;
    let metadata = fs::symlink_metadata(&root)
        .map_err(|error| format!("inspect plan work directory {}: {error}", root.display()))?;
    if is_link_like(&metadata) || !metadata.is_dir() {
        return Err(format!(
            "plan work directory must be a regular non-link directory: {}",
            root.display()
        ));
    }
    let mut current = root;
    for component in Path::new(path).components() {
        let Component::Normal(component) = component else {
            return Err(format!("invalid plan directory path: {path}"));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if !is_link_like(&metadata) && metadata.is_dir() => {}
            Ok(_) => {
                return Err(format!(
                    "plan directory component must be a regular non-link directory: {}",
                    current.display()
                ))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|error| {
                    format!("create plan directory {}: {error}", current.display())
                })?;
            }
            Err(error) => {
                return Err(format!(
                    "inspect plan directory component {}: {error}",
                    current.display()
                ))
            }
        }
    }
    Ok(())
}

fn showrules_arguments(top: &str, input: &str, output: &str) -> [String; 4] {
    [
        top.to_owned(),
        input.to_owned(),
        "-o".to_owned(),
        output.to_owned(),
    ]
}

fn ensure_regular_artifact(work_dir: &Path, path: &str, label: &str) -> Result<(), String> {
    let path = secure_artifact_path(work_dir, path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("inspect {label} {}: {error}", path.display()))?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "{label} must be a regular non-link file: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_artifact_target_absent(work_dir: &Path, path: &str, label: &str) -> Result<(), String> {
    let path = secure_artifact_path(work_dir, path)?;
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("inspect {label} {}: {error}", path.display())),
        Ok(_) => Err(format!(
            "{label} must not exist before execution: {}",
            path.display()
        )),
    }
}

fn secure_artifact_path(work_dir: &Path, path: &str) -> Result<PathBuf, String> {
    let root = fs::canonicalize(work_dir).map_err(|error| {
        format!(
            "canonicalize plan work directory {}: {error}",
            work_dir.display()
        )
    })?;
    let mut target = root.clone();
    let relative = Path::new(path);
    let components = relative.components().collect::<Vec<_>>();
    if components.is_empty() {
        return Err("artifact path must not be empty".to_owned());
    }
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(format!("invalid plan artifact path: {path}"));
        };
        target.push(component);
        if index + 1 == components.len() {
            break;
        }
        let metadata = fs::symlink_metadata(&target).map_err(|error| {
            format!("inspect plan artifact parent {}: {error}", target.display())
        })?;
        if is_link_like(&metadata) || !metadata.is_dir() {
            return Err(format!(
                "plan artifact parent must be a regular non-link directory: {}",
                target.display()
            ));
        }
    }
    Ok(target)
}

#[cfg(test)]
fn normalize_text_artifact(
    work_dir: &Path,
    source: &str,
    destination: &str,
    transform: TextNormalization,
) -> Result<(), String> {
    normalize_text_artifact_with_iverilog_version(work_dir, source, destination, transform, false)
}

fn normalize_text_artifact_with_iverilog_version(
    work_dir: &Path,
    source: &str,
    destination: &str,
    transform: TextNormalization,
    filter_iverilog_10_1: bool,
) -> Result<(), String> {
    let source_path = secure_artifact_path(work_dir, source)?;
    let metadata = fs::symlink_metadata(&source_path).map_err(|error| {
        format!(
            "inspect text normalization source {}: {error}",
            source_path.display()
        )
    })?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "text normalization source must be a regular non-link file: {}",
            source_path.display()
        ));
    }
    let contents = fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "read text normalization source {}: {error}",
            source_path.display()
        )
    })?;
    let normalized = match transform {
        TextNormalization::SortNumericField1ThenField2 => {
            let mut lines = contents.lines().collect::<Vec<_>>();
            let key = |line: &str| -> Result<(i128, String), String> {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                let first = fields
                    .first()
                    .ok_or_else(|| "numeric task sort does not accept an empty line".to_owned())?;
                let numeric = first.parse::<i128>().map_err(|error| {
                    format!("numeric task sort has invalid first field {first:?}: {error}")
                })?;
                Ok((numeric, fields[1..].join(" ")))
            };
            let mut keyed = lines
                .drain(..)
                .map(|line| key(line).map(|key| (key, line)))
                .collect::<Result<Vec<_>, _>>()?;
            keyed.sort_by(|left, right| {
                left.0
                     .0
                    .cmp(&right.0 .0)
                    .then_with(|| left.0 .1.cmp(&right.0 .1))
                    .then_with(|| left.1.cmp(right.1))
            });
            let mut output = keyed
                .into_iter()
                .map(|(_, line)| line)
                .collect::<Vec<_>>()
                .join("\n");
            if !output.is_empty() {
                output.push('\n');
            }
            output
        }
        TextNormalization::VerilogTaskProjection | TextNormalization::BluesimTaskProjection => {
            let mut output = String::new();
            for line in contents.lines() {
                let line = if transform == TextNormalization::VerilogTaskProjection {
                    line.replace("main.", "")
                } else {
                    line.to_owned()
                };
                for field in line.split_whitespace().skip(1) {
                    output.push_str(field);
                    output.push(' ');
                }
                output.push('\n');
            }
            output
        }
        TextNormalization::IfNestedToSplitIfNested
        | TextNormalization::IfNestedToNoSplitIfNested => {
            let replacement = if transform == TextNormalization::IfNestedToSplitIfNested {
                "SplitIfNested"
            } else {
                "NoSplitIfNested"
            };
            Regex::new(r"\bIfNested\b")
                .expect("fixed IfNested token regex")
                .replace_all(&contents, replacement)
                .into_owned()
        }
        TextNormalization::MakeDirectoryMessages => {
            let is_make_directory_message = |line: &str| {
                line.find("make").is_some_and(|make_index| {
                    line[make_index..].contains(": Entering directory")
                        || line[make_index..].contains(": Leaving directory")
                })
            };
            let mut output = contents
                .lines()
                .filter(|line| !is_make_directory_message(line))
                .collect::<Vec<_>>()
                .join("\n");
            if contents.ends_with('\n') && !output.is_empty() {
                output.push('\n');
            }
            output
        }
        TextNormalization::IverilogQuietOutput => {
            let mut output = contents
                .lines()
                .filter(|line| {
                    !line.contains("WARNING: IVerilog")
                        && !line.contains("not guaranteed")
                        && !(filter_iverilog_10_1 && line.contains("inherits dimensions from var"))
                })
                .collect::<Vec<_>>()
                .join("\n");
            if contents.ends_with('\n') && !output.is_empty() {
                output.push('\n');
            }
            output
        }
    };
    create_text_output(
        work_dir,
        destination,
        normalized.as_bytes(),
        "text normalization",
    )
}

fn create_text_output(
    work_dir: &Path,
    destination: &str,
    contents: &[u8],
    label: &str,
) -> Result<(), String> {
    let destination = secure_artifact_path(work_dir, destination)?;
    match fs::symlink_metadata(&destination) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "inspect {label} output {}: {error}",
                destination.display()
            ))
        }
        Ok(_) => {
            return Err(format!(
                "{label} output already exists: {}",
                destination.display()
            ))
        }
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .and_then(|mut file| file.write_all(contents))
        .map_err(|error| format!("write {label} output {}: {error}", destination.display()))
}

fn apply_verilog_filter_pipeline(
    work_dir: &Path,
    path: &str,
    profiles: &[VerilogFilterProfile],
    expected_exit: ExpectedExit,
) -> Result<(), String> {
    const RENAME_FIRE: &[u8] = b"#!/usr/bin/perl --\n# -*-Perl-*-\n# A sample -verilog-filter: shorten BSC's -keep-fires rule-signal\n# prefixes (CAN_FIRE_ -> CF_, WILL_FIRE_ -> WF_) in the generated\n# Verilog.  bsc invokes the filter with the generated file as its\n# argument, and the filter rewrites the file in place.\n\nforeach my $outfile (@ARGV) {\n    next unless open(FILE, $outfile);\n    my @lines = <FILE>;\n    close(FILE);\n\n    foreach my $line (@lines) {\n        $line =~ s/\\bCAN_FIRE_/CF_/g;\n        $line =~ s/\\bWILL_FIRE_/WF_/g;\n    }\n\n    next unless open(FILE, \">\", $outfile);\n    print FILE @lines;\n    close(FILE);\n}\n";
    const SIMPLE_SED: &[u8] = b"s/CLK/CLOCK/\n";
    const ORDER_SED: &[u8] = b"s/WF_/W_F_/g\n";
    for profile in profiles {
        let expected = match profile {
            VerilogFilterProfile::RenameFire => Some(("renamefire.pl", RENAME_FIRE)),
            VerilogFilterProfile::ClockToClock => Some(("simple.sed", SIMPLE_SED)),
            VerilogFilterProfile::WfToWF => Some(("order.sed", ORDER_SED)),
            VerilogFilterProfile::MissingSed => None,
        };
        if let Some((fixture, expected)) = expected {
            let fixture_path = secure_artifact_path(work_dir, fixture)?;
            let metadata = fs::symlink_metadata(&fixture_path).map_err(|error| {
                format!(
                    "inspect Verilog filter fixture {}: {error}",
                    fixture_path.display()
                )
            })?;
            if is_link_like(&metadata) || !metadata.is_file() {
                return Err(format!(
                    "Verilog filter fixture must be a regular non-link file: {}",
                    fixture_path.display()
                ));
            }
            let actual = fs::read(&fixture_path).map_err(|error| {
                format!(
                    "read Verilog filter fixture {}: {error}",
                    fixture_path.display()
                )
            })?;
            if actual != expected {
                return Err(format!(
                    "Verilog filter fixture contents are not the audited profile: {}",
                    fixture_path.display()
                ));
            }
        }
    }
    let target = secure_artifact_path(work_dir, path)?;
    let metadata = fs::symlink_metadata(&target).map_err(|error| {
        format!(
            "inspect generated Verilog filter target {}: {error}",
            target.display()
        )
    })?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "generated Verilog filter target must be a regular non-link file: {}",
            target.display()
        ));
    }
    let mut contents = fs::read_to_string(&target).map_err(|error| {
        format!(
            "read generated Verilog filter target {}: {error}",
            target.display()
        )
    })?;
    static RENAME_CAN: OnceLock<Regex> = OnceLock::new();
    static RENAME_WILL: OnceLock<Regex> = OnceLock::new();
    for profile in profiles {
        match profile {
            VerilogFilterProfile::RenameFire => {
                contents = RENAME_CAN
                    .get_or_init(|| Regex::new(r"\bCAN_FIRE_").expect("closed CAN_FIRE regex"))
                    .replace_all(&contents, "CF_")
                    .into_owned();
                contents = RENAME_WILL
                    .get_or_init(|| Regex::new(r"\bWILL_FIRE_").expect("closed WILL_FIRE regex"))
                    .replace_all(&contents, "WF_")
                    .into_owned();
            }
            VerilogFilterProfile::ClockToClock => {
                contents = contents
                    .split_inclusive('\n')
                    .map(|line| line.replacen("CLK", "CLOCK", 1))
                    .collect();
            }
            VerilogFilterProfile::WfToWF => contents = contents.replace("WF_", "W_F_"),
            VerilogFilterProfile::MissingSed => {
                let missing = secure_artifact_path(work_dir, "doesnotexist.sed")?;
                match fs::symlink_metadata(&missing) {
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(format!(
                            "inspect missing filter fixture {}: {error}",
                            missing.display()
                        ))
                    }
                    Ok(_) => {
                        return Err(format!(
                            "missing filter fixture must remain absent: {}",
                            missing.display()
                        ))
                    }
                }
            }
        }
    }
    let mut output = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&target)
        .map_err(|error| {
            format!(
                "open generated Verilog filter target {}: {error}",
                target.display()
            )
        })?;
    let output_metadata = output.metadata().map_err(|error| {
        format!(
            "inspect open Verilog filter target {}: {error}",
            target.display()
        )
    })?;
    if !output_metadata.is_file() {
        return Err(format!(
            "open Verilog filter target is not regular: {}",
            target.display()
        ));
    }
    output.write_all(contents.as_bytes()).map_err(|error| {
        format!(
            "rewrite generated Verilog filter target {}: {error}",
            target.display()
        )
    })?;
    let failed = profiles.last() == Some(&VerilogFilterProfile::MissingSed);
    if failed == (expected_exit == ExpectedExit::Failure) {
        Ok(())
    } else {
        Err("Verilog filter pipeline exit expectation mismatch".to_owned())
    }
}

fn render_m4_curdir(work_dir: &Path, template: &str, output: &str) -> Result<(), String> {
    const CHANGEQUOTE: &str = "changequote(`[', `]')";
    let template_path = secure_artifact_path(work_dir, template)?;
    let metadata = fs::symlink_metadata(&template_path).map_err(|error| {
        format!(
            "inspect M4 CURDIR template {}: {error}",
            template_path.display()
        )
    })?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "M4 CURDIR template must be a regular non-link file: {}",
            template_path.display()
        ));
    }
    let mut contents = fs::read_to_string(&template_path).map_err(|error| {
        format!(
            "read M4 CURDIR template {}: {error}",
            template_path.display()
        )
    })?;
    if let Some(remainder) = contents.strip_prefix(CHANGEQUOTE) {
        contents = remainder.to_owned();
    }
    if contents.contains("changequote") || !contents.contains("CURDIR") {
        return Err(format!(
            "M4 CURDIR template {} is outside the audited literal subset",
            template_path.display()
        ));
    }
    let replacement = work_dir.to_string_lossy().replace('\\', "/");
    let rendered = contents.replace("CURDIR", &replacement);
    let output_path = secure_artifact_path(work_dir, output)?;
    match fs::symlink_metadata(&output_path) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(format!(
                "M4 CURDIR output already exists: {}",
                output_path.display()
            ))
        }
        Err(error) => {
            return Err(format!(
                "inspect M4 CURDIR output {}: {error}",
                output_path.display()
            ))
        }
    }
    fs::write(&output_path, rendered)
        .map_err(|error| format!("write M4 CURDIR output {}: {error}", output_path.display()))
}

fn touch_create_update(work_dir: &Path, path: &str, delay_milliseconds: u64) -> Result<(), String> {
    if !(1..=10_000).contains(&delay_milliseconds) {
        return Err("touch-create delay must be between 1 and 10000 milliseconds".to_owned());
    }
    std::thread::sleep(Duration::from_millis(delay_milliseconds));
    let target = secure_artifact_path(work_dir, path)?;
    match fs::symlink_metadata(&target) {
        Ok(metadata) => {
            if is_link_like(&metadata) || !metadata.is_file() {
                return Err(format!(
                    "touch-create target must be a regular non-link file: {}",
                    target.display()
                ));
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|error| format!("create touch artifact {}: {error}", target.display()))?;
        }
        Err(error) => {
            return Err(format!(
                "inspect touch artifact {}: {error}",
                target.display()
            ))
        }
    }
    set_file_mtime(&target, FileTime::now())
        .map_err(|error| format!("update touch artifact {}: {error}", target.display()))
}

#[cfg(unix)]
fn remove_user_read(work_dir: &Path, path: &str) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let target = secure_artifact_path(work_dir, path)?;
    let metadata = fs::symlink_metadata(&target)
        .map_err(|error| format!("inspect unreadable artifact {}: {error}", target.display()))?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "unreadable artifact must be a regular non-link file: {}",
            target.display()
        ));
    }
    let mode = metadata.mode() & !0o400;
    fs::set_permissions(&target, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("remove user-read permission {}: {error}", target.display()))?;
    match fs::File::open(&target) {
        Err(error) if error.kind() == ErrorKind::PermissionDenied => Ok(()),
        Err(error) => Err(format!(
            "cannot prove unreadability for {}: open failed with {error}",
            target.display()
        )),
        Ok(_) => Err(format!(
            "cannot prove unreadability for uid {} after chmod u-r: {}",
            metadata.uid(),
            target.display()
        )),
    }
}

#[cfg(not(unix))]
fn remove_user_read(_work_dir: &Path, path: &str) -> Result<(), String> {
    Err(format!(
        "POSIX user-read removal is unavailable on this platform: {path}"
    ))
}

fn posix_unreadability_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(probe_posix_unreadability)
}

#[cfg(unix)]
fn probe_posix_unreadability() -> bool {
    use std::os::unix::fs::PermissionsExt;

    let path = std::env::temp_dir().join(format!("bsc-posix-unreadability-{}", std::process::id()));
    let result = (|| {
        fs::write(&path, b"probe").ok()?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o200)).ok()?;
        Some(matches!(
            fs::File::open(&path),
            Err(error) if error.kind() == ErrorKind::PermissionDenied
        ))
    })()
    .unwrap_or(false);
    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    let _ = fs::remove_file(path);
    result
}

#[cfg(not(unix))]
fn probe_posix_unreadability() -> bool {
    false
}

fn strict_touch(work_dir: &Path, path: &str) -> Result<(), String> {
    let path = secure_artifact_path(work_dir, path)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("touch source does not exist {}: {error}", path.display()))?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "touch source must be an existing regular non-link file: {}",
            path.display()
        ));
    }
    set_file_mtime(&path, FileTime::now())
        .map_err(|error| format!("touch artifact {}: {error}", path.display()))
}

fn strict_copy_replace(work_dir: &Path, source: &str, destination: &str) -> Result<(), String> {
    let source = secure_artifact_path(work_dir, source)?;
    let destination = secure_artifact_path(work_dir, destination)?;
    for (label, path) in [("source", &source), ("destination", &destination)] {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            format!(
                "copy-replace {label} is unavailable {}: {error}",
                path.display()
            )
        })?;
        if is_link_like(&metadata) || !metadata.is_file() {
            return Err(format!(
                "copy-replace {label} must be a regular non-link file: {}",
                path.display()
            ));
        }
    }
    fs::copy(&source, &destination).map_err(|error| {
        format!(
            "replace artifact {} with {}: {error}",
            destination.display(),
            source.display()
        )
    })?;
    set_file_mtime(&destination, FileTime::now())
        .map_err(|error| format!("set replacement mtime {}: {error}", destination.display()))
}

fn rewrite_darwin_cpp_include_path(
    work_dir: &Path,
    source: &str,
    destination: &str,
) -> Result<(), String> {
    let source = secure_artifact_path(work_dir, source)?;
    let metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("inspect cpp rewrite source {}: {error}", source.display()))?;
    if is_link_like(&metadata) || !metadata.is_file() {
        return Err(format!(
            "cpp rewrite source must be a regular non-link file: {}",
            source.display()
        ));
    }
    let destination = secure_artifact_path(work_dir, destination)?;
    match fs::symlink_metadata(&destination) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(format!(
                "cpp rewrite destination already exists: {}",
                destination.display()
            ))
        }
        Err(error) => {
            return Err(format!(
                "inspect cpp rewrite destination {}: {error}",
                destination.display()
            ))
        }
    }
    let contents = fs::read_to_string(&source)
        .map_err(|error| format!("read cpp rewrite source {}: {error}", source.display()))?;
    let rewritten = normalize_darwin_cpp_include_paths(&contents);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .and_then(|mut file| std::io::Write::write_all(&mut file, rewritten.as_bytes()))
        .map_err(|error| {
            format!(
                "write cpp rewrite output {}: {error}",
                destination.display()
            )
        })
}

fn normalize_darwin_cpp_include_paths(contents: &str) -> String {
    static QUOTED_MORE_PATH: OnceLock<Regex> = OnceLock::new();
    QUOTED_MORE_PATH
        .get_or_init(|| Regex::new(r#""[^"\r\n]*/more\.bsv""#).expect("closed cpp path regex"))
        .replace_all(contents, "\"more.bsv\"")
        .into_owned()
}

fn strict_move_replace(work_dir: &Path, source: &str, destination: &str) -> Result<(), String> {
    if source == destination {
        return Err("move-replace source and destination must differ".to_owned());
    }
    strict_copy_replace(work_dir, source, destination)?;
    let source = secure_artifact_path(work_dir, source)?;
    fs::remove_file(&source)
        .map_err(|error| format!("remove moved artifact {}: {error}", source.display()))
}

fn strict_transfer(
    work_dir: &Path,
    source: &str,
    destination: &str,
    remove_source: bool,
) -> Result<(), String> {
    let source = secure_artifact_path(work_dir, source)?;
    let source_metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("inspect transfer source {}: {error}", source.display()))?;
    if is_link_like(&source_metadata) || !source_metadata.is_file() {
        return Err(format!(
            "transfer source must be a regular non-link file: {}",
            source.display()
        ));
    }
    let destination_relative = Path::new(destination);
    if let Some(parent) = destination_relative
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        let parent = parent
            .to_str()
            .ok_or_else(|| format!("transfer destination parent is not UTF-8: {destination}"))?;
        ensure_directory_exists(work_dir, parent)?;
    }
    let destination = secure_artifact_path(work_dir, destination)?;
    match fs::symlink_metadata(&destination) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(format!(
                "transfer destination already exists: {}",
                destination.display()
            ))
        }
        Err(error) => {
            return Err(format!(
                "inspect transfer destination {}: {error}",
                destination.display()
            ))
        }
    }
    let mut input = fs::File::open(&source)
        .map_err(|error| format!("open transfer source {}: {error}", source.display()))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| {
            format!(
                "create transfer destination {}: {error}",
                destination.display()
            )
        })?;
    if let Err(error) = io::copy(&mut input, &mut output) {
        drop(output);
        let _ = fs::remove_file(&destination);
        return Err(format!(
            "copy artifact {} to {}: {error}",
            source.display(),
            destination.display()
        ));
    }
    if remove_source {
        fs::remove_file(&source)
            .map_err(|error| format!("remove moved artifact {}: {error}", source.display()))?;
    }
    Ok(())
}

fn describe_exit(exit_code: Option<i32>) -> String {
    exit_code.map_or_else(
        || "after termination by signal".to_owned(),
        |code| format!("with status {code}"),
    )
}

fn clean_bluesim_link_output(output: &str, windows: bool) -> String {
    let lines = output.lines().collect::<Vec<_>>();
    let mut kept = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_end_matches('\r');
        let is_mingw_fpic_warning = windows
            && line.ends_with(
                ":1:0: warning: -fPIC ignored for target (all code is position independent)",
            )
            && lines
                .get(index + 1)
                .is_some_and(|line| line.trim_end_matches('\r') == " /*")
            && lines
                .get(index + 2)
                .is_some_and(|line| line.trim_end_matches('\r') == " ^");
        if is_mingw_fpic_warning {
            index += 3;
            continue;
        }
        kept.push(line);
        index += 1;
    }

    let mut cleaned = kept.join("\n");
    if output.ends_with('\n') {
        cleaned.push('\n');
    }
    cleaned
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

fn scenario_bluetcl_packages(scenario: &Scenario) -> BTreeSet<BluetclPackage> {
    scenario
        .stages
        .iter()
        .flat_map(|stage| &stage.operations)
        .flat_map(|operation| &operation.requires)
        .filter_map(|requirement| match requirement {
            Requirement::BluetclPackage(package) => Some(*package),
            _ => None,
        })
        .collect()
}

fn operation_skip_reason(
    toolchain: &Toolchain,
    package_probes: &BluetclPackageProbeCache,
    operation: &OperationRecord,
) -> Result<Option<String>, String> {
    for requirement in &operation.requires {
        if let Requirement::BluetclPackage(package) = requirement {
            if !package_probes.available(toolchain, *package)? {
                return Ok(Some(format!("Bluetcl package {package:?} is unavailable")));
            }
        }
    }
    Ok(platform_operation_skip_reason(operation))
}

fn platform_operation_skip_reason(operation: &OperationRecord) -> Option<String> {
    operation
        .requires
        .iter()
        .find_map(|requirement| match requirement {
            Requirement::InternalChecks if std::env::var_os("BSC_INTERNAL_CHECKS").is_none() => {
                Some("internal checks disabled (set BSC_INTERNAL_CHECKS=1 to enable)".to_owned())
            }
            Requirement::PosixUnreadability if !posix_unreadability_available() => Some(
                "POSIX unreadability cannot be proven for the current user/filesystem".to_owned(),
            ),
            Requirement::NonWindows if cfg!(windows) => {
                Some("requires a non-Windows filesystem".to_owned())
            }
            Requirement::Darwin if !cfg!(target_os = "macos") => Some("requires Darwin".to_owned()),
            _ => None,
        })
}

fn scenario_skip_reason(toolchain: &Toolchain, scenario: &Scenario) -> Option<String> {
    for requirement in &scenario.requires {
        match requirement {
            Requirement::Bluesim
            | Requirement::Verilog
            | Requirement::Frontend
            | Requirement::Icarus
            | Requirement::Bluetcl => {}
            Requirement::ShowRules if toolchain.showrules.is_some() => {}
            Requirement::ShowRules => return Some("showrules is unavailable".to_owned()),
            Requirement::BluetclPackage(_) => {
                return Some(
                    "Bluetcl package availability must be attached to operations, not scenarios"
                        .to_owned(),
                )
            }
            Requirement::NonWindows if !cfg!(windows) => {}
            Requirement::NonWindows => return Some("requires a non-Windows filesystem".to_owned()),
            Requirement::InternalChecks => {
                return Some(
                    "internal checks must be attached to operations, not scenarios".to_owned(),
                )
            }
            Requirement::PosixUnreadability => {
                return Some(
                    "POSIX unreadability must be attached to dependent operations, not scenarios"
                        .to_owned(),
                )
            }
            Requirement::Darwin => {
                return Some(
                    "Darwin must be attached to dependent operations, not scenarios".to_owned(),
                )
            }
            Requirement::SystemC => {}
        }
    }
    None
}

fn verify_plan_origin(toolchain: &Toolchain, plan: &TestPlan) -> Result<(), String> {
    verify_hash_within(
        &toolchain.project_root,
        Path::new(&plan.origin.path),
        &plan.origin.sha256,
    )
}

fn verify_plan_fixtures(toolchain: &Toolchain, plan: &TestPlan) -> Result<(), String> {
    let fixture_root = secure_directory_within(
        &toolchain.project_root,
        Path::new(&plan.fixture_dir),
        "fixture directory",
    )?;
    for fixture in &plan.fixtures {
        let path = resolve_declared_fixture(&fixture_root, fixture)?;
        verify_hash_at(&path, &fixture.sha256)?;
    }
    Ok(())
}

fn resolve_declared_fixture(root: &Path, fixture: &Fixture) -> Result<PathBuf, String> {
    let Some(declared_source) = &fixture.source else {
        return secure_file_within(root, Path::new(&fixture.path), "Test Plan fixture");
    };
    let logical = Path::new(&fixture.path);
    ensure_fixture_parents_are_regular(root, logical.parent().unwrap_or_else(|| Path::new("")))?;
    let alias = root.join(logical);
    let metadata = fs::symlink_metadata(&alias).map_err(|error| {
        format!(
            "inspect Test Plan fixture alias {}: {error}",
            alias.display()
        )
    })?;
    if !metadata.file_type().is_symlink() {
        return Err(format!(
            "declared fixture alias must be a symbolic link: {}",
            alias.display()
        ));
    }
    let target = fs::read_link(&alias)
        .map_err(|error| format!("read Test Plan fixture alias {}: {error}", alias.display()))?;
    let resolved = resolve_fixture_alias_target(logical, &target)?;
    if &resolved != declared_source {
        return Err(format!(
            "fixture alias {} changed target (expected {declared_source:?}, found {resolved:?})",
            alias.display()
        ));
    }
    let source = Path::new(declared_source);
    ensure_fixture_parents_are_regular(root, source.parent().unwrap_or_else(|| Path::new("")))?;
    let target = root.join(source);
    let target_metadata = fs::symlink_metadata(&target).map_err(|error| {
        format!(
            "inspect Test Plan fixture alias target {}: {error}",
            target.display()
        )
    })?;
    if is_link_like(&target_metadata) || !target_metadata.is_file() {
        return Err(format!(
            "fixture alias target must be a regular non-link file: {}",
            target.display()
        ));
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("canonicalize fixture root {}: {error}", root.display()))?;
    let canonical = fs::canonicalize(&target).map_err(|error| {
        format!(
            "canonicalize Test Plan fixture alias target {}: {error}",
            target.display()
        )
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "fixture alias target escapes its fixture root: {}",
            target.display()
        ));
    }
    Ok(canonical)
}

fn ensure_fixture_parents_are_regular(root: &Path, relative: &Path) -> Result<(), String> {
    let mut current = root.to_owned();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(format!(
                "invalid fixture parent path: {}",
                relative.display()
            ));
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("inspect fixture parent {}: {error}", current.display()))?;
        if is_link_like(&metadata) || !metadata.is_dir() {
            return Err(format!(
                "fixture parent must be a regular non-link directory: {}",
                current.display()
            ));
        }
    }
    Ok(())
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
    let resolved = resolved.to_string_lossy().replace('\\', "/");
    if resolved.is_empty() || resolved == logical.to_string_lossy() {
        return Err("fixture alias target must be a distinct fixture path".to_owned());
    }
    Ok(resolved)
}

fn verify_hash_within(root: &Path, relative: &Path, expected: &str) -> Result<(), String> {
    let path = secure_file_within(root, relative, "Test Plan input")?;
    verify_hash_at(&path, expected)
}

fn verify_hash_at(path: &Path, expected: &str) -> Result<(), String> {
    let contents = fs::read(path)
        .map_err(|error| format!("read Test Plan input {}: {error}", path.display()))?;
    let actual = format!("{:x}", Sha256::digest(contents));
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "Test Plan input {} changed (expected {expected}, found {actual}); run `pixi run just plans-update`",
            path.display()
        )
    })
}

fn sanitize_name(name: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_preflights_keep_non_materialized_inputs_after_all_flags() {
        assert_eq!(
            bsc_flag_preflight_arguments(
                BscFlagPreflightMode::VerilogNoOptUndetermined,
                "NoOptUndet_UnspecToX.bsv",
                None,
                UndeterminedValue::X,
            )
            .unwrap(),
            [
                "-verilog",
                "-no-opt-undetermined-vals",
                "-unspecified-to",
                "X",
                "-no-show-timestamps",
                "-no-show-version",
                "-u",
                "NoOptUndet_UnspecToX.bsv",
            ]
            .map(str::to_owned)
        );
        assert_eq!(
            bsc_flag_preflight_arguments(
                BscFlagPreflightMode::BluesimLink,
                "m.ba",
                Some("mkBluesimLink_UnspecToZ"),
                UndeterminedValue::Z,
            )
            .unwrap(),
            [
                "-no-show-timestamps",
                "-no-show-version",
                "-sim",
                "-e",
                "mkBluesimLink_UnspecToZ",
                "-o",
                "mkBluesimLink_UnspecToZ.cexe",
                "-unspecified-to",
                "z",
                "m.ba",
            ]
            .map(str::to_owned)
        );
        assert!(bsc_flag_preflight_arguments(
            BscFlagPreflightMode::BluesimLink,
            "m.ba",
            None,
            UndeterminedValue::X,
        )
        .is_err());
    }

    #[test]
    fn showrules_uses_only_the_verified_fixed_argv() {
        assert_eq!(
            showrules_arguments("mkTop", "raw.vcd", "rules.vcd"),
            ["mkTop", "raw.vcd", "-o", "rules.vcd"].map(str::to_owned)
        );
    }

    #[cfg(unix)]
    #[test]
    fn fake_showrules_executable_observes_only_fixed_argv_in_the_workspace() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_workspace("bsc-rust-tests-showrules-fake");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let program = root.join("showrules");
        fs::write(
            &program,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > argv.txt\ncp \"$2\" \"$4\"\nprintf 'showrules stdout\\n'\nprintf 'showrules stderr\\n' >&2\n",
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(root.join("raw.vcd"), "$enddefinitions $end\n").unwrap();
        let arguments = showrules_arguments("mkTop", "raw.vcd", "rules.vcd");
        let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let result = run_command(
            &requirement_toolchain(Some(program.clone())),
            &program,
            &refs,
            &root,
            &root.join("showrules.log"),
            Duration::from_secs(5),
        )
        .unwrap();
        assert!(result.success);
        assert_eq!(
            fs::read_to_string(root.join("argv.txt")).unwrap(),
            "mkTop\nraw.vcd\n-o\nrules.vcd\n"
        );
        assert!(result.output.contains("showrules stdout"));
        assert!(result.output.contains("showrules stderr"));
        assert!(root.join("rules.vcd").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn showrules_artifact_guards_reject_unsafe_paths_and_existing_outputs() {
        let root = test_workspace("bsc-rust-tests-showrules-artifacts");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("raw.vcd"), "$enddefinitions $end\n").unwrap();
        ensure_regular_artifact(&root, "raw.vcd", "input").unwrap();
        assert!(ensure_regular_artifact(&root, "../raw.vcd", "input").is_err());
        ensure_artifact_target_absent(&root, "rules.vcd", "output").unwrap();
        fs::write(root.join("rules.vcd"), "existing").unwrap();
        assert!(ensure_artifact_target_absent(&root, "rules.vcd", "output").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn strips_only_a_closed_parse_pretty_dump_wrapper() {
        let root = test_workspace("bsc-rust-tests-parse-pretty-wrapper");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test workspace");
        let dump = root.join("Demo.bsv-pretty-out.bsv");
        fs::write(
            &dump,
            "=== parsed (Demo):\npackage Demo;\nendpackage\n-----\nfooter\n",
        )
        .expect("write wrapped dump");
        strip_dump_wrapper(&dump).expect("strip closed wrapper");
        assert_eq!(
            fs::read_to_string(&dump).expect("read stripped dump"),
            "package Demo;\nendpackage"
        );

        fs::write(&dump, "=== parsed (Demo):\npackage Demo;\n").expect("write open dump");
        assert!(strip_dump_wrapper(&dump).is_err());
        fs::write(&dump, "package Demo;\n-----\n").expect("write invalid header");
        assert!(strip_dump_wrapper(&dump).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_replace_requires_regular_files_and_refreshes_mtime() {
        let root = test_workspace("bsc-rust-tests-copy-replace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test workspace");
        fs::write(root.join("source.bs"), "new\n").expect("write source");
        fs::write(root.join("destination.bs"), "old\n").expect("write destination");
        let old = FileTime::from_unix_time(1, 0);
        set_file_mtime(root.join("destination.bs"), old).expect("set old mtime");
        strict_copy_replace(&root, "source.bs", "destination.bs").expect("replace fixture");
        assert_eq!(
            fs::read_to_string(root.join("destination.bs")).unwrap(),
            "new\n"
        );
        assert!(
            FileTime::from_last_modification_time(
                &fs::metadata(root.join("destination.bs")).unwrap()
            ) > old
        );
        assert!(strict_copy_replace(&root, "missing.bs", "destination.bs").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stages_the_checked_in_cpp_alias_as_regular_target_bytes() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|candidate| {
                candidate.join("Cargo.toml").is_file() && candidate.join("testsuite").is_dir()
            })
            .expect("workspace root")
            .to_owned();
        let toolchain = Toolchain {
            project_root: project_root.clone(),
            bsc: PathBuf::new(),
            bluetcl: PathBuf::new(),
            bsc2bsv: PathBuf::new(),
            dumpbo: PathBuf::new(),
            dumpba: PathBuf::new(),
            vcdcheck: PathBuf::new(),
            showrules: None,
            make: PathBuf::new(),
            iverilog: PathBuf::new(),
            bluespecdir: PathBuf::new(),
            systemc_include: PathBuf::new(),
            systemc_lib: PathBuf::new(),
            cc: PathBuf::new(),
            cxx: PathBuf::new(),
        };
        let plan: TestPlan =
            serde_json::from_str(include_str!("../plans/bsc.driver/cpp/cpp.test.json"))
                .expect("decode generated cpp plan");
        verify_plan_fixtures(&toolchain, &plan).expect("verify cpp fixture aliases and hashes");

        let work = test_workspace("bsc-rust-tests-cpp-fixture-alias");
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work).expect("create staging workspace");
        stage_fixture_paths(&toolchain, &plan, &["Cpreprocess1.bsv"], &work)
            .expect("materialize alias target bytes");

        let staged = work.join("Cpreprocess1.bsv");
        let metadata = fs::symlink_metadata(&staged).expect("inspect staged alias");
        assert!(metadata.is_file());
        assert!(!is_link_like(&metadata));
        assert_eq!(
            fs::read(&staged).unwrap(),
            fs::read(project_root.join("testsuite/bsc.driver/cpp/Cpreprocess.bsv")).unwrap()
        );
        let _ = fs::remove_dir_all(&work);
    }

    #[test]
    fn normalizes_only_the_audited_quoted_darwin_cpp_more_path() {
        let input = concat!(
            "Error in \"/private/tmp/build/more.bsv\"\n",
            "keep \"more.bsv\"\n",
            "keep /private/tmp/build/more.bsv\n",
            "keep \"/private/tmp/build/not-more.bsv\"\n",
            "keep \"/private/tmp/build/moreXbsv\"\n",
        );
        assert_eq!(
            normalize_darwin_cpp_include_paths(input),
            concat!(
                "Error in \"more.bsv\"\n",
                "keep \"more.bsv\"\n",
                "keep /private/tmp/build/more.bsv\n",
                "keep \"/private/tmp/build/not-more.bsv\"\n",
                "keep \"/private/tmp/build/moreXbsv\"\n",
            )
        );
    }

    #[test]
    fn derives_bluesim_fifo_warning_locations_from_verilog_golden() {
        let input = "Warning: FIFO2: main.top.d_fifo2.error_checks -- Dequeuing from empty fifo\nWarning: FIFO1: main.top.e_fifo1.error_checks -- Enqueuing to a full fifo\n";
        assert_eq!(
            derive_fifo_warning_locations(input).unwrap(),
            "Warning: top.d_fifo2 -- Dequeuing from empty fifo\nWarning: top.e_fifo1 -- Enqueuing to a full fifo\n"
        );
        assert!(derive_fifo_warning_locations("not a warning\n").is_err());
    }
    use bsc_test_plan::{ResourceClass, Stage, Timeouts};

    fn test_workspace(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root")
            .join(".pixi/tmp")
            .join(format!("{name}-{}", std::process::id()))
    }

    #[test]
    fn native_task_text_transforms_preserve_exact_order_and_spacing() {
        let root = test_workspace("bsc-rust-tests-task-text-normalization");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create task transform workspace");
        fs::write(
            root.join("raw.out"),
            "10 main.beta z\n2 main.foo b\n2 main.foo a\n",
        )
        .expect("write task output");

        normalize_text_artifact(
            &root,
            "raw.out",
            "sorted.out",
            TextNormalization::SortNumericField1ThenField2,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("sorted.out")).unwrap(),
            "2 main.foo a\n2 main.foo b\n10 main.beta z\n"
        );
        normalize_text_artifact(
            &root,
            "sorted.out",
            "verilog.out",
            TextNormalization::VerilogTaskProjection,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("verilog.out")).unwrap(),
            "foo a \nfoo b \nbeta z \n"
        );
        normalize_text_artifact(
            &root,
            "sorted.out",
            "bluesim.out",
            TextNormalization::BluesimTaskProjection,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("bluesim.out")).unwrap(),
            "main.foo a \nmain.foo b \nmain.beta z \n"
        );

        assert!(normalize_text_artifact(
            &root,
            "raw.out",
            "sorted.out",
            TextNormalization::SortNumericField1ThenField2,
        )
        .is_err());
        assert!(normalize_text_artifact(
            &root,
            "../raw.out",
            "escaped.out",
            TextNormalization::BluesimTaskProjection,
        )
        .is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn iverilog_quiet_filter_preserves_version_specific_warning_semantics() {
        let root = test_workspace("bsc-rust-tests-iverilog-quiet-filter");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create Icarus filter workspace");
        fs::write(
            root.join("raw.out"),
            concat!(
                "WARNING: IVerilog compatibility warning\n",
                "behavior is not guaranteed\n",
                "signal inherits dimensions from var\n",
                "keep this line\n",
            ),
        )
        .unwrap();

        normalize_text_artifact_with_iverilog_version(
            &root,
            "raw.out",
            "modern.out",
            TextNormalization::IverilogQuietOutput,
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("modern.out")).unwrap(),
            "signal inherits dimensions from var\nkeep this line\n"
        );
        normalize_text_artifact_with_iverilog_version(
            &root,
            "raw.out",
            "v10-1.out",
            TextNormalization::IverilogQuietOutput,
            true,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("v10-1.out")).unwrap(),
            "keep this line\n"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn native_verilog_filters_are_ordered_pinned_and_fail_closed() {
        let root = test_workspace("bsc-rust-tests-verilog-filter");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create Verilog filter workspace");
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|candidate| candidate.join("testsuite").is_dir())
            .expect("workspace root containing testsuite");
        let fixture_root = project_root.join("testsuite/bsc.verilog/filter");
        for fixture in ["renamefire.pl", "simple.sed", "order.sed"] {
            fs::copy(fixture_root.join(fixture), root.join(fixture))
                .expect("stage audited filter fixture");
        }
        fs::write(
            root.join("mkTop.v"),
            "CAN_FIRE_A WILL_FIRE_B XCAN_FIRE_C CLK CLK WF_D\n",
        )
        .unwrap();
        apply_verilog_filter_pipeline(
            &root,
            "mkTop.v",
            &[
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::ClockToClock,
                VerilogFilterProfile::WfToWF,
            ],
            ExpectedExit::Success,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("mkTop.v")).unwrap(),
            "CF_A W_F_B XCAN_FIRE_C CLOCK CLK W_F_D\n"
        );

        fs::write(root.join("simple.sed"), "s/CLK/CLOCK/g\n").unwrap();
        let before = fs::read(root.join("mkTop.v")).unwrap();
        assert!(apply_verilog_filter_pipeline(
            &root,
            "mkTop.v",
            &[VerilogFilterProfile::ClockToClock],
            ExpectedExit::Success,
        )
        .is_err());
        assert_eq!(fs::read(root.join("mkTop.v")).unwrap(), before);
        fs::copy(fixture_root.join("simple.sed"), root.join("simple.sed")).unwrap();

        fs::write(root.join("missing.v"), "CAN_FIRE_A\n").unwrap();
        apply_verilog_filter_pipeline(
            &root,
            "missing.v",
            &[
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::MissingSed,
            ],
            ExpectedExit::Failure,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("missing.v")).unwrap(),
            "CF_A\n"
        );
        fs::write(root.join("doesnotexist.sed"), "unexpected\n").unwrap();
        assert!(apply_verilog_filter_pipeline(
            &root,
            "missing.v",
            &[VerilogFilterProfile::MissingSed],
            ExpectedExit::Failure,
        )
        .is_err());
        assert!(apply_verilog_filter_pipeline(
            &root,
            "../mkTop.v",
            &[VerilogFilterProfile::RenameFire],
            ExpectedExit::Success,
        )
        .is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn native_text_and_filter_transforms_reject_symlink_artifacts() {
        use std::os::unix::fs::symlink;

        let root = test_workspace("bsc-rust-tests-native-transform-symlinks");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("real.out"), "1 value\n").unwrap();
        symlink("real.out", root.join("linked.out")).unwrap();
        assert!(normalize_text_artifact(
            &root,
            "linked.out",
            "normalized.out",
            TextNormalization::BluesimTaskProjection,
        )
        .is_err());

        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|candidate| candidate.join("testsuite").is_dir())
            .unwrap();
        fs::copy(
            project_root.join("testsuite/bsc.verilog/filter/renamefire.pl"),
            root.join("renamefire.pl"),
        )
        .unwrap();
        fs::write(root.join("real.v"), "CAN_FIRE_A\n").unwrap();
        symlink("real.v", root.join("linked.v")).unwrap();
        assert!(apply_verilog_filter_pipeline(
            &root,
            "linked.v",
            &[VerilogFilterProfile::RenameFire],
            ExpectedExit::Success,
        )
        .is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finite_icarus_simulator_selectors_resolve_only_audited_targets() {
        let root = test_workspace("bsc-rust-tests-icarus-selectors");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("exec")).unwrap();
        let builder = root.join("exec/bsc_build_vsim_iverilog");
        fs::write(&builder, "builder\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&builder, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let toolchain = Toolchain {
            project_root: root.clone(),
            bsc: PathBuf::new(),
            bluetcl: PathBuf::new(),
            bsc2bsv: PathBuf::new(),
            dumpbo: PathBuf::new(),
            dumpba: PathBuf::new(),
            vcdcheck: PathBuf::new(),
            showrules: None,
            make: PathBuf::new(),
            iverilog: PathBuf::new(),
            bluespecdir: root.clone(),
            systemc_include: PathBuf::new(),
            systemc_lib: PathBuf::new(),
            cc: PathBuf::new(),
            cxx: PathBuf::new(),
        };

        assert_eq!(
            resolve_icarus_simulator(&toolchain, IcarusSimulatorSelector::Default).unwrap(),
            "iverilog"
        );
        assert_eq!(
            resolve_icarus_simulator(&toolchain, IcarusSimulatorSelector::LiteralBogus).unwrap(),
            "bogus_sim"
        );
        let installed = resolve_icarus_simulator(
            &toolchain,
            IcarusSimulatorSelector::BluespecDirInstalledBuilder,
        )
        .unwrap();
        assert!(installed
            .replace('\\', "/")
            .ends_with("/exec/bsc_build_vsim_iverilog"));
        let absent =
            resolve_icarus_simulator(&toolchain, IcarusSimulatorSelector::BluespecDirBogus)
                .unwrap();
        assert!(absent.replace('\\', "/").ends_with("/exec/bogus_sim"));
        fs::write(root.join("exec/bogus_sim"), "unexpected\n").unwrap();
        assert!(
            resolve_icarus_simulator(&toolchain, IcarusSimulatorSelector::BluespecDirBogus,)
                .is_err()
        );

        #[cfg(not(windows))]
        assert_eq!(
            resolve_icarus_simulator(&toolchain, IcarusSimulatorSelector::PosixEchoProbe).unwrap(),
            "/bin/echo"
        );
        #[cfg(windows)]
        assert!(
            resolve_icarus_simulator(&toolchain, IcarusSimulatorSelector::PosixEchoProbe,).is_err()
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn installed_icarus_selector_rejects_symlink_and_non_executable_builders() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = test_workspace("bsc-rust-tests-icarus-selector-safety");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("exec")).unwrap();
        let builder = root.join("exec/bsc_build_vsim_iverilog");
        fs::write(&builder, "builder\n").unwrap();
        fs::set_permissions(&builder, fs::Permissions::from_mode(0o644)).unwrap();
        let toolchain = Toolchain {
            project_root: root.clone(),
            bsc: PathBuf::new(),
            bluetcl: PathBuf::new(),
            bsc2bsv: PathBuf::new(),
            dumpbo: PathBuf::new(),
            dumpba: PathBuf::new(),
            vcdcheck: PathBuf::new(),
            showrules: None,
            make: PathBuf::new(),
            iverilog: PathBuf::new(),
            bluespecdir: root.clone(),
            systemc_include: PathBuf::new(),
            systemc_lib: PathBuf::new(),
            cc: PathBuf::new(),
            cxx: PathBuf::new(),
        };
        assert!(resolve_icarus_simulator(
            &toolchain,
            IcarusSimulatorSelector::BluespecDirInstalledBuilder,
        )
        .is_err());
        fs::remove_file(&builder).unwrap();
        fs::write(root.join("real-builder"), "builder\n").unwrap();
        symlink("../real-builder", &builder).unwrap();
        assert!(resolve_icarus_simulator(
            &toolchain,
            IcarusSimulatorSelector::BluespecDirInstalledBuilder,
        )
        .is_err());
        let _ = fs::remove_dir_all(&root);
    }

    fn scenario(requirement: Requirement) -> Scenario {
        Scenario {
            id: "requirement-policy".to_owned(),
            fixtures: Vec::new(),
            resource: ResourceClass::Normal,
            requires: vec![requirement],
            bsc_options_append: None,
            timeouts: Timeouts::default(),
            stages: vec![Stage {
                id: "stage".to_owned(),
                operations: Vec::new(),
            }],
        }
    }

    fn requirement_toolchain(showrules: Option<PathBuf>) -> Toolchain {
        Toolchain {
            project_root: PathBuf::new(),
            bsc: PathBuf::new(),
            bluetcl: PathBuf::new(),
            bsc2bsv: PathBuf::new(),
            dumpbo: PathBuf::new(),
            dumpba: PathBuf::new(),
            vcdcheck: PathBuf::new(),
            showrules,
            make: PathBuf::new(),
            iverilog: PathBuf::new(),
            bluespecdir: PathBuf::new(),
            systemc_include: PathBuf::new(),
            systemc_lib: PathBuf::new(),
            cc: PathBuf::new(),
            cxx: PathBuf::new(),
        }
    }

    #[test]
    fn ensure_directory_absent_is_recursive_idempotent_and_directory_only() {
        let root = test_workspace("bsc-rust-tests-ensure-dir-absent");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("work/nested")).expect("create removable tree");
        fs::write(root.join("work/nested/output.txt"), "output\n").expect("write nested file");

        ensure_directory_absent(&root, "work").expect("remove regular directory tree");
        assert!(!root.join("work").exists());
        ensure_directory_absent(&root, "work").expect("missing directory is already absent");

        fs::write(root.join("work"), "not a directory\n").expect("write conflicting file");
        let error = ensure_directory_absent(&root, "work").unwrap_err();
        assert!(error.contains("regular non-link directory"));
        assert!(root.join("work").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn ensure_directory_absent_rejects_symlink_targets_and_children() {
        use std::os::unix::fs::symlink;

        let root = test_workspace("bsc-rust-tests-ensure-dir-links");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("outside")).expect("create link target");
        symlink(root.join("outside"), root.join("work")).expect("create target symlink");
        assert!(ensure_directory_absent(&root, "work").is_err());
        fs::remove_file(root.join("work")).expect("remove target symlink");

        fs::create_dir(root.join("work")).expect("create removable root");
        symlink(root.join("outside"), root.join("work/link")).expect("create child symlink");
        assert!(ensure_directory_absent(&root, "work").is_err());
        assert!(root.join("outside").is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn ensure_directory_absent_rejects_directory_reparse_points() {
        use std::os::windows::fs::symlink_dir;

        let root = test_workspace("bsc-rust-tests-ensure-dir-reparse");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("outside")).expect("create reparse target");
        if symlink_dir(root.join("outside"), root.join("work")).is_ok() {
            assert!(ensure_directory_absent(&root, "work").is_err());
            assert!(root.join("outside").is_dir());
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn touch_requires_an_existing_regular_file_and_preserves_contents() {
        let root =
            std::env::temp_dir().join(format!("bsc-rust-tests-touch-{}", std::process::id()));
        fs::create_dir_all(&root).expect("create test workspace");
        let file = root.join("Source.bsv");
        fs::write(&file, "package Source; endpackage\n").expect("write fixture");

        strict_touch(&root, "Source.bsv").expect("touch fixture");
        assert_eq!(
            fs::read_to_string(&file).expect("read touched fixture"),
            "package Source; endpackage\n"
        );

        let missing = strict_touch(&root, "Missing.bsv").unwrap_err();
        assert!(missing.contains("does not exist"));
        assert!(!root.join("Missing.bsv").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn typed_create_touch_render_and_erase_are_closed_and_idempotent() {
        let root = test_workspace("bsc-rust-tests-typed-workspace-actions");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test workspace");

        ensure_directory_exists(&root, "generated/nested").expect("create directory tree");
        ensure_directory_exists(&root, "generated/nested")
            .expect("directory creation is idempotent");
        touch_create_update(&root, "generated/nested/Source.bsv", 1)
            .expect("create touched source");
        fs::write(root.join("generated/nested/Source.bsv"), "source\n")
            .expect("populate touched source");
        touch_create_update(&root, "generated/nested/Source.bsv", 1)
            .expect("update existing source");
        assert_eq!(
            fs::read_to_string(root.join("generated/nested/Source.bsv")).unwrap(),
            "source\n"
        );
        assert!(touch_create_update(&root, "invalid.bsv", 0).is_err());
        assert!(!root.join("invalid.bsv").exists());

        let template = "changequote(`[', `]')`include \"CURDIR/generated/defines\"\n";
        fs::write(root.join("source.pre-m4"), template).expect("write source template");
        render_m4_curdir(&root, "source.pre-m4", "rendered.bsv").expect("render CURDIR");
        let expected_root = root.to_string_lossy().replace('\\', "/");
        assert_eq!(
            fs::read_to_string(root.join("rendered.bsv")).unwrap(),
            format!("`include \"{expected_root}/generated/defines\"\n")
        );
        assert!(render_m4_curdir(&root, "source.pre-m4", "rendered.bsv").is_err());
        fs::write(root.join("invalid.pre-m4"), "changequote(foo) CURDIR\n")
            .expect("write invalid template");
        assert!(render_m4_curdir(&root, "invalid.pre-m4", "invalid.out").is_err());

        ensure_file_absent(&root, "generated/nested/Source.bsv").expect("erase generated source");
        ensure_file_absent(&root, "generated/nested/Source.bsv").expect("erase is idempotent");
        ensure_file_absent(&root, "missing/child.bsv").expect("missing parent is absent");
        fs::create_dir(root.join("directory-target")).expect("create conflicting directory");
        assert!(ensure_file_absent(&root, "directory-target").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn strict_transfer_is_no_clobber_and_creates_only_safe_parents() {
        let root = test_workspace("bsc-rust-tests-strict-transfer");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test workspace");
        fs::write(root.join("source.txt"), "new\n").expect("write source");
        strict_transfer(&root, "source.txt", "nested/destination.txt", false)
            .expect("copy without clobber");
        assert_eq!(
            fs::read_to_string(root.join("nested/destination.txt")).unwrap(),
            "new\n"
        );
        fs::write(root.join("other.txt"), "other\n").expect("write second source");
        assert!(strict_transfer(&root, "other.txt", "nested/destination.txt", false).is_err());
        assert_eq!(
            fs::read_to_string(root.join("nested/destination.txt")).unwrap(),
            "new\n"
        );
        assert!(strict_transfer(&root, "../outside.txt", "copy.txt", false).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn compile_object_verification_uses_declared_bdir_output() {
        let root = test_workspace("bsc-rust-tests-compile-object-output");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bdir")).expect("create bdir");
        fs::write(root.join("bdir/Source.bo"), "object\n").expect("write object");
        assert_eq!(
            compile_object_exists(
                &root,
                &root,
                "Source.bsv",
                &["-bdir".to_owned(), "bdir".to_owned()],
                &["Source.bsv.bsc-out".to_owned(), "bdir/Source.bo".to_owned()],
            )
            .unwrap(),
            Some(true)
        );
        fs::remove_file(root.join("bdir/Source.bo")).expect("remove object");
        assert_eq!(
            compile_object_exists(
                &root,
                &root,
                "Source.bsv",
                &[],
                &["bdir/Source.bo".to_owned()],
            )
            .unwrap(),
            Some(false)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn conditional_compile_failure_check_preserves_upstream_control_flow() {
        let log = Path::new("compile.log");
        assert!(compile_contract_result(
            "Demo.bsv",
            ExpectedExit::Failure,
            Some("Internal.*Error"),
            false,
            Some(1),
            "Internal Compiler Error",
            Some(false),
            log,
        )
        .is_ok());

        let error = compile_contract_result(
            "Demo.bsv",
            ExpectedExit::Failure,
            Some("Internal.*Error"),
            true,
            Some(0),
            "ordinary output",
            Some(true),
            log,
        )
        .unwrap_err();
        assert!(error.contains("should reject"));
        assert!(!error.contains("forbidden regex"));

        let error = compile_contract_result(
            "Demo.bsv",
            ExpectedExit::Failure,
            Some("Internal.*Error"),
            true,
            Some(0),
            "Internal Compiler Error",
            Some(true),
            log,
        )
        .unwrap_err();
        assert!(error.contains("should reject"));
        assert!(error.contains("forbidden regex"));

        for (success, exit_code, object_exists) in
            [(true, Some(0), Some(true)), (false, Some(1), Some(false))]
        {
            assert!(compile_contract_result(
                "Demo.bsv",
                ExpectedExit::Unchecked,
                None,
                success,
                exit_code,
                "ignored helper result",
                object_exists,
                log,
            )
            .is_ok());
        }
    }

    #[test]
    fn builds_closed_bluetcl_argv_and_honors_expected_failure() {
        assert_eq!(
            bluetcl_arguments_with_installed_path(
                &BluetclInvocation::Script {
                    script: "utils_test.tcl".to_owned(),
                    args: vec!["arg".to_owned()],
                    syntax: BluetclSyntax::Bsv,
                },
                None,
            ),
            ["utils_test.tcl", "arg"]
        );
        assert_eq!(
            bluetcl_arguments_with_installed_path(
                &BluetclInvocation::Script {
                    script: "utils_test.tcl".to_owned(),
                    args: vec!["arg".to_owned()],
                    syntax: BluetclSyntax::Bh,
                },
                None,
            ),
            ["utils_test.tcl", "arg", "-bh"]
        );
        assert_eq!(
            bluetcl_arguments_with_installed_path(
                &BluetclInvocation::Exec {
                    script: "dump_poss.tcl".to_owned(),
                    args: vec!["sysTop".to_owned()],
                },
                None,
            ),
            ["-exec", "dump_poss.tcl", "sysTop"]
        );

        let log = Path::new("bluetcl.log");
        assert!(bluetcl_contract_result(ExpectedExit::Failure, false, log).is_ok());
        assert!(bluetcl_contract_result(ExpectedExit::Success, true, log).is_ok());
        assert!(bluetcl_contract_result(ExpectedExit::Failure, true, log).is_err());
        assert!(bluetcl_contract_result(ExpectedExit::Success, false, log).is_err());
    }

    #[test]
    fn non_windows_requirement_is_skipped_only_on_windows() {
        assert_eq!(
            scenario_skip_reason(
                &requirement_toolchain(None),
                &scenario(Requirement::NonWindows),
            )
            .is_some(),
            cfg!(windows)
        );
        let mut operation = OperationRecord::new(
            Action::FsMkdir {
                path: "BOUTDIR".to_owned(),
            },
            OperationExpectation::Required,
            bsc_test_plan::Provenance {
                span: bsc_test_plan::SourceSpan {
                    start_byte: 0,
                    end_byte: 0,
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                },
                expansion: Vec::new(),
            },
        );
        operation.requires.push(Requirement::NonWindows);
        assert_eq!(
            platform_operation_skip_reason(&operation).is_some(),
            cfg!(windows)
        );
    }

    #[test]
    fn supports_compiler_profile_requirements() {
        let toolchain = requirement_toolchain(None);
        assert!(scenario_skip_reason(&toolchain, &scenario(Requirement::Verilog)).is_none());
        assert!(scenario_skip_reason(&toolchain, &scenario(Requirement::Frontend)).is_none());
        assert!(scenario_skip_reason(&toolchain, &scenario(Requirement::Icarus)).is_none());
        assert!(scenario_skip_reason(&toolchain, &scenario(Requirement::SystemC)).is_none());
        assert!(scenario_skip_reason(&toolchain, &scenario(Requirement::Bluetcl)).is_none());
    }

    #[test]
    fn showrules_unavailability_skips_the_whole_scenario() {
        assert_eq!(
            scenario_skip_reason(
                &requirement_toolchain(None),
                &scenario(Requirement::ShowRules),
            ),
            Some("showrules is unavailable".to_owned())
        );
        assert!(scenario_skip_reason(
            &requirement_toolchain(Some(PathBuf::from("showrules"))),
            &scenario(Requirement::ShowRules),
        )
        .is_none());
    }

    #[test]
    fn builds_c_object_with_fixed_shell_free_arguments() {
        assert_eq!(
            c_object_build_arguments("convert.c", "convert.o"),
            ["-fPIC", "-c", "convert.c", "-o", "convert.o"]
        );

        let root = std::env::temp_dir().join(format!(
            "bsc-rust-tests-c-object-makefile-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("convert.mk"), "CFLAGS +=-fPIC\n").unwrap();
        validate_c_object_makefile(&root, "convert.mk").unwrap();
        fs::write(root.join("convert.mk"), "CFLAGS +=-fPIC -O2\n").unwrap();
        assert!(validate_c_object_makefile(&root, "convert.mk").is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn builds_systemc_link_argv_with_a_separate_cxx_include_argument() {
        let objects = vec!["mkTop.ba".to_owned(), "mkHelper.ba".to_owned()];
        assert_eq!(
            bsc_systemc_link_arguments(Path::new("C:/pixi/Library/include"), "mkTop", &objects),
            [
                "-no-show-timestamps",
                "-no-show-version",
                "-systemc",
                "-e",
                "mkTop",
                "-Xc++",
                "-IC:/pixi/Library/include",
                "mkTop.ba",
                "mkHelper.ba",
            ]
        );
    }

    #[test]
    fn builds_only_typed_installed_script_and_makedepend_bluetcl_argv() {
        let installed = BluetclInvocation::InstalledScript {
            script: BluetclInstalledScript::ExpandPorts,
            args: vec![
                "-quiet".to_owned(),
                "-wrapper".to_owned(),
                "mkTest1.wrapper.got.v".to_owned(),
                "-include".to_owned(),
                "mkTest1.includes.got.vh".to_owned(),
                "Test1".to_owned(),
                "mkTest1".to_owned(),
                "mkTest1.v".to_owned(),
            ],
        };
        let installed_path = Path::new(r"D:\bsc\inst\lib\tcllib\bluespec\expandPorts.tcl");
        let mut expected = vec![shell_path_for_platform(installed_path, cfg!(windows))];
        expected.extend(installed.args().iter().cloned());
        assert_eq!(
            bluetcl_arguments_with_installed_path(&installed, Some(installed_path)),
            expected
        );

        assert_eq!(
            bluetcl_arguments_with_installed_path(
                &BluetclInvocation::Makedepend {
                    command: BluetclMakedependCommand::Makedepend,
                    args: vec!["-no-show-timestamps".to_owned(), "*.bsv".to_owned()],
                },
                None,
            ),
            ["-exec", "makedepend", "-no-show-timestamps", "*.bsv"]
        );
        assert_eq!(
            bluetcl_arguments_with_installed_path(
                &BluetclInvocation::Makedepend {
                    command: BluetclMakedependCommand::MakedependTcl,
                    args: Vec::new(),
                },
                None,
            ),
            ["-exec", "makedepend.tcl"]
        );
    }

    #[test]
    fn package_unavailability_skips_only_the_guarded_operation() {
        let package = BluetclPackage::InstSynth;
        let cache = BluetclPackageProbeCache::default();
        cache.results.lock().unwrap().insert(package, Ok(false));
        let toolchain = Toolchain {
            project_root: PathBuf::from("C:/bsc"),
            bsc: PathBuf::from("C:/bsc/inst/bin/core/bsc.exe"),
            bluetcl: PathBuf::from("C:/bsc/inst/bin/core/bluetcl.exe"),
            bsc2bsv: PathBuf::from("C:/bsc/inst/bin/core/bsc2bsv.exe"),
            dumpbo: PathBuf::from("C:/bsc/inst/bin/core/dumpbo.exe"),
            dumpba: PathBuf::from("C:/bsc/inst/bin/core/dumpba.exe"),
            vcdcheck: PathBuf::from("C:/bsc/inst/bin/core/vcdcheck.exe"),
            showrules: Some(PathBuf::from("C:/bsc/inst/bin/core/showrules.exe")),
            make: PathBuf::from("C:/pixi/Library/bin/make.exe"),
            iverilog: PathBuf::from("C:/pixi/Library/bin/iverilog.exe"),
            bluespecdir: PathBuf::from("C:/bsc/inst/lib"),
            systemc_include: PathBuf::from("C:/pixi/Library/include"),
            systemc_lib: PathBuf::from("C:/pixi/Library/lib"),
            cc: PathBuf::from("C:/pixi/Library/mingw-w64/bin/gcc.exe"),
            cxx: PathBuf::from("C:/pixi/Library/mingw-w64/bin/g++.exe"),
        };
        let mut guarded = OperationRecord::new(
            Action::AssertExists {
                path: "generated.txt".to_owned(),
            },
            OperationExpectation::Required,
            bsc_test_plan::Provenance {
                span: bsc_test_plan::SourceSpan {
                    start_byte: 0,
                    end_byte: 1,
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 2,
                },
                expansion: Vec::new(),
            },
        );
        guarded.requires.push(Requirement::BluetclPackage(package));
        assert_eq!(
            operation_skip_reason(&toolchain, &cache, &guarded).unwrap(),
            Some("Bluetcl package InstSynth is unavailable".to_owned())
        );

        let unguarded = OperationRecord::new(
            Action::AssertExists {
                path: "other.txt".to_owned(),
            },
            OperationExpectation::Required,
            guarded.provenance.clone(),
        );
        assert_eq!(
            operation_skip_reason(&toolchain, &cache, &unguarded).unwrap(),
            None
        );
    }

    #[test]
    fn builds_systemc_cxx_link_argv_with_fixed_module_object_order() {
        let toolchain = Toolchain {
            project_root: PathBuf::from("C:/bsc"),
            bsc: PathBuf::from("C:/bsc/inst/bin/core/bsc.exe"),
            bluetcl: PathBuf::from("C:/bsc/inst/bin/core/bluetcl.exe"),
            bsc2bsv: PathBuf::from("C:/bsc/inst/bin/core/bsc2bsv.exe"),
            dumpbo: PathBuf::from("C:/bsc/inst/bin/core/dumpbo.exe"),
            dumpba: PathBuf::from("C:/bsc/inst/bin/core/dumpba.exe"),
            vcdcheck: PathBuf::from("C:/bsc/inst/bin/core/vcdcheck.exe"),
            showrules: Some(PathBuf::from("C:/bsc/inst/bin/core/showrules.exe")),
            make: PathBuf::from("C:/pixi/Library/bin/make.exe"),
            iverilog: PathBuf::from("C:/pixi/Library/bin/iverilog.exe"),
            bluespecdir: PathBuf::from("C:/bsc/inst/lib"),
            systemc_include: PathBuf::from("C:/pixi/Library/include"),
            systemc_lib: PathBuf::from("C:/pixi/Library/lib"),
            cc: PathBuf::from("C:/pixi/Library/mingw-w64/bin/gcc.exe"),
            cxx: PathBuf::from("C:/pixi/Library/mingw-w64/bin/g++.exe"),
        };
        assert_eq!(
            systemc_cxx_link_arguments(
                &toolchain,
                "top",
                &["main.cpp".to_owned()],
                &["mkTop".to_owned()],
                &["mkHelper".to_owned()],
                &["-DENABLE=1".to_owned()],
            ),
            [
                "-DENABLE=1",
                "-IC:/pixi/Library/include",
                "-LC:/pixi/Library/lib",
                &format!(
                    "-I{}",
                    Path::new("C:/bsc/inst/lib").join("Bluesim").display()
                ),
                &format!(
                    "-L{}",
                    Path::new("C:/bsc/inst/lib").join("Bluesim").display()
                ),
                "-o",
                "top.syscexe",
                "mkHelper.o",
                "mkTop.o",
                "mkHelper_systemc.o",
                "mkTop_systemc.o",
                "model_mkTop.o",
                "-x",
                "c++",
                "main.cpp",
                "-lsystemc",
                "-lbskernel",
                "-lbsprim",
                "-lwinpthread",
            ]
        );
    }

    #[test]
    fn removes_only_complete_mingw_fpic_warning_blocks_from_bluesim_link_output() {
        let output = concat!(
            "sd/mkTop.cxx:1:0: warning: -fPIC ignored for target (all code is position independent)\r\n",
            " /*\r\n",
            " ^\r\n",
            "Bluesim object created: sd/mkTop.{h,o}\r\n",
        );
        assert_eq!(
            clean_bluesim_link_output(output, true),
            "Bluesim object created: sd/mkTop.{h,o}\n"
        );
        assert_eq!(
            clean_bluesim_link_output(output, false),
            output.replace("\r\n", "\n")
        );

        let incomplete = concat!(
            "sd/mkTop.cxx:1:0: warning: -fPIC ignored for target (all code is position independent)\n",
            " /*\n",
            "different caret\n",
        );
        assert_eq!(clean_bluesim_link_output(incomplete, true), incomplete);
    }

    #[test]
    fn normalizes_osci_banner_stop_messages_and_numeric_sorting() {
        let output = concat!(
            "        SystemC 2.3.4-Accellera --- Aug 30 2023 11:23:29\n",
            "        Copyright (c) 1996-2023 by all Contributors,\n",
            "        ALL RIGHTS RESERVED\n",
            "\n",
            "10 ten\n",
            "2 two\n",
            "\n",
            "Info: /OSCI/SystemC: Simulation stopped by user.\n",
        );
        assert_eq!(normalize_systemc_output(output, false), "10 ten\n2 two\n");
        assert_eq!(normalize_systemc_output(output, true), "2 two\n10 ten\n");
        assert_eq!(
            normalize_systemc_output(
                "a\nb\nc\nd\npayload\nSystemC: simulation stopped by user\n",
                false
            ),
            "payload\n"
        );
    }

    #[test]
    fn adapts_bsc_search_paths_to_the_platform_separator() {
        for option in ["-p", "-vsearch"] {
            let arguments = vec![option.to_owned(), "+:vlib".to_owned()];
            let adjusted = platform_bsc_path_list_arguments(&arguments);
            assert_eq!(
                adjusted,
                if cfg!(windows) {
                    vec![option.to_owned(), "+;vlib".to_owned()]
                } else {
                    arguments
                }
            );
        }
    }

    #[test]
    fn builds_no_main_icarus_link_argv_without_main_v() {
        let objects = vec!["Tb.v".to_owned(), "mkDesign.v".to_owned()];
        let arguments = no_main_icarus_link_arguments(
            Path::new(r"D:\bsc\inst\lib\exec\bsc_build_vsim_iverilog"),
            Path::new(r"D:\bsc\inst\lib"),
            "Tb",
            &objects,
        );
        assert_eq!(
            arguments,
            [
                "/d/bsc/inst/lib/exec/bsc_build_vsim_iverilog",
                "link",
                "Tb",
                "Tb",
                "-y",
                ".",
                "-y",
                "/d/bsc/inst/lib/Libraries",
                "-y",
                "/d/bsc/inst/lib/Verilog",
                "Tb.v",
                "mkDesign.v",
            ]
        );
        assert!(!arguments.iter().any(|argument| argument == "main.v"));
    }

    #[test]
    fn make_test_data_uses_the_fixed_upstream_argv() {
        assert_eq!(
            make_test_data_arguments(),
            ["-j1", "MAKEFLAGS=", "-f", "Makefile.data", "test_data"]
        );
    }

    #[test]
    fn selects_generation_flags_and_model_extensions_by_backend() {
        assert_eq!(
            simulation_generation_flags(SimulationGenerationMode::Bluesim),
            ["-sim"]
        );
        assert_eq!(
            simulation_generation_flags(SimulationGenerationMode::Verilog),
            ["-verilog"]
        );
        assert_eq!(
            simulation_generation_flags(SimulationGenerationMode::SharedElaboration),
            ["-verilog", "-elab"]
        );

        assert_eq!(simulation_model_extension(SimulationBackend::Bluesim), "ba");
        assert_eq!(simulation_model_extension(SimulationBackend::Icarus), "v");
    }

    #[test]
    fn selects_backend_vcd_arguments() {
        let arguments = vec!["user-argument".to_owned()];
        assert_eq!(
            simulation_run_arguments(SimulationBackend::Bluesim, &arguments, None),
            ["user-argument"]
        );
        assert_eq!(
            simulation_run_arguments(SimulationBackend::Bluesim, &arguments, Some("trace.vcd")),
            ["-V", "trace.vcd", "user-argument"]
        );
        assert_eq!(
            simulation_run_arguments(SimulationBackend::Icarus, &arguments, None),
            ["-vcd-none", "user-argument"]
        );
        assert_eq!(
            simulation_run_arguments(SimulationBackend::Icarus, &arguments, Some("trace.vcd")),
            ["+bscvcd", "user-argument"]
        );
    }

    #[test]
    fn windows_icarus_launcher_runs_vexe_through_vvp() {
        let (program, arguments) = icarus_invocation_for_platform(
            Path::new("work/mkTestbench.vexe"),
            &["+bscvcd".to_owned()],
            true,
        );
        assert_eq!(program, Path::new("vvp"));
        assert_eq!(arguments, ["work/mkTestbench.vexe", "+bscvcd"]);
    }

    #[test]
    fn bluesim_executable_artifact_is_cexe() {
        assert_eq!(
            simulation_executable_artifact(SimulationBackend::Bluesim, "mkTest"),
            "mkTest.cexe"
        );
    }

    #[test]
    fn icarus_executable_artifact_is_vexe() {
        assert_eq!(
            simulation_executable_artifact(SimulationBackend::Icarus, "sysTest"),
            "sysTest.vexe"
        );
    }
}
