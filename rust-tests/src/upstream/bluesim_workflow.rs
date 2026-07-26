use super::artifact::{check_artifact_assertions, validate_artifact_assertions};
use super::{
    is_safe_relative, reset_directory, sanitize_case_name, stage_fixture_paths,
    ArtifactTransferOperation, BluesimGeneration, BluesimWorkflowRun, BluesimWorkflowScenario,
};
use crate::cache::{CacheLookup, GenerationCache};
use crate::{run_bsc, run_command, Toolchain};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn validate_bluesim_workflow(scenario: &BluesimWorkflowScenario) -> Result<(), String> {
    if scenario.name.is_empty() || scenario.link.top.is_empty() {
        return Err("Bluesim workflow and top names must not be empty".to_owned());
    }
    if scenario.generations.is_empty() {
        return Err(format!(
            "Bluesim workflow {} must declare at least one generation",
            scenario.name
        ));
    }
    for generation in scenario.generations {
        if !is_safe_relative(generation.source) || !scenario.fixtures.contains(&generation.source) {
            return Err(format!(
                "Bluesim workflow {} must declare generation source {} as a safe fixture",
                scenario.name, generation.source
            ));
        }
        validate_argv(generation.options, scenario.name)?;
    }
    for object in scenario.link.objects {
        if !is_safe_relative(object) {
            return Err(format!(
                "Bluesim workflow {} contains unsafe link object {object}",
                scenario.name
            ));
        }
    }
    validate_argv(scenario.link.options, scenario.name)?;
    validate_artifact_assertions(scenario.link_assertions, scenario.fixtures, scenario.name)?;
    for run in scenario.runs {
        if run.name.is_empty() || !is_safe_relative(run.stdout) {
            return Err(format!(
                "Bluesim workflow {} contains an invalid run declaration",
                scenario.name
            ));
        }
        validate_argv(run.options, run.name)?;
        for transfer in run.transfers {
            if !is_safe_relative(transfer.source) || !is_safe_relative(transfer.destination) {
                return Err(format!(
                    "Bluesim workflow run {} contains an unsafe artifact transfer",
                    run.name
                ));
            }
        }
        validate_artifact_assertions(run.assertions, scenario.fixtures, run.name)?;
    }
    Ok(())
}

fn validate_argv(arguments: &[&str], context: &str) -> Result<(), String> {
    if arguments.iter().any(|argument| argument.is_empty()) {
        return Err(format!("{context} contains an empty command argument"));
    }
    Ok(())
}

pub(super) fn generation_arguments(generation: &BluesimGeneration) -> Vec<&str> {
    let mut arguments = Vec::with_capacity(generation.options.len() + 7);
    arguments.extend_from_slice(generation.options);
    arguments.extend_from_slice(&["-no-show-timestamps", "-no-show-version", "-u", "-sim"]);
    if let Some(module) = generation.module {
        arguments.extend_from_slice(&["-g", module]);
    }
    arguments.push(generation.source);
    arguments
}

pub(super) fn normalized_link_objects(scenario: &BluesimWorkflowScenario) -> Vec<String> {
    scenario
        .link
        .objects
        .iter()
        .map(|object| {
            if object.contains('.') {
                (*object).to_owned()
            } else {
                format!("{object}.ba")
            }
        })
        .collect()
}

pub(super) fn link_arguments(scenario: &BluesimWorkflowScenario) -> Vec<String> {
    let mut arguments = vec![
        "-no-show-timestamps".to_owned(),
        "-no-show-version".to_owned(),
        "-sim".to_owned(),
        "-e".to_owned(),
        scenario.link.top.to_owned(),
        "-o".to_owned(),
        scenario.link.top.to_owned(),
    ];
    arguments.extend(
        scenario
            .link
            .options
            .iter()
            .map(|argument| (*argument).to_owned()),
    );
    arguments.extend(normalized_link_objects(scenario));
    arguments
}

pub(super) struct WorkflowExecution {
    pub build_error: Option<String>,
    pub run_results: Vec<(&'static str, Result<(), String>)>,
}

pub(super) fn execute_bluesim_workflow(
    toolchain: &Toolchain,
    generation_cache: &GenerationCache,
    scenario: &'static BluesimWorkflowScenario,
    selected_runs: &[&'static BluesimWorkflowRun],
    work_dir: &Path,
    artifact_dir: &Path,
) -> WorkflowExecution {
    let build = prepare_build(
        toolchain,
        generation_cache,
        scenario,
        work_dir,
        artifact_dir,
    );
    if let Err(error) = build {
        return WorkflowExecution {
            build_error: Some(error),
            run_results: Vec::new(),
        };
    }

    if let Err(error) = check_artifact_assertions(
        scenario.link_assertions,
        work_dir,
        artifact_dir,
        scenario.name,
    ) {
        return WorkflowExecution {
            build_error: Some(error),
            run_results: Vec::new(),
        };
    }
    if scenario.runs.is_empty() {
        return WorkflowExecution {
            build_error: None,
            run_results: Vec::new(),
        };
    }

    let run_results = selected_runs
        .iter()
        .map(|run| {
            let run_artifact_dir = artifact_dir.join("runs").join(sanitize_case_name(run.name));
            let result = run_bluesim(toolchain, scenario, run, work_dir, &run_artifact_dir);
            (run.name, result)
        })
        .collect();
    WorkflowExecution {
        build_error: None,
        run_results,
    }
}

fn prepare_build(
    toolchain: &Toolchain,
    generation_cache: &GenerationCache,
    scenario: &BluesimWorkflowScenario,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), String> {
    validate_bluesim_workflow(scenario)?;
    reset_directory(work_dir)?;
    reset_directory(artifact_dir)?;
    stage_fixture_paths(toolchain, scenario.fixture_dir, scenario.fixtures, work_dir)?;

    let fingerprint = workflow_fingerprint(scenario);
    let fingerprint_refs = fingerprint.iter().map(String::as_str).collect::<Vec<_>>();
    let fixture_root = toolchain.project_root.join(scenario.fixture_dir);
    let cache_log = artifact_dir.join("build-cache.log");
    let cache_key = match generation_cache.lookup(
        &fixture_root,
        scenario.fixtures,
        &fingerprint_refs,
        work_dir,
        &cache_log,
    )? {
        CacheLookup::Hit => {
            check_generation_intermediates(toolchain, scenario, work_dir, artifact_dir)?;
            resolve_executable(work_dir, scenario.link.top)?;
            return Ok(());
        }
        CacheLookup::Miss(key) => Some(key),
        CacheLookup::Disabled => None,
    };

    for (index, generation) in scenario.generations.iter().enumerate() {
        let arguments = generation_arguments(generation);
        let log = artifact_dir.join(format!("generation-{index}.log"));
        let result = run_bsc(
            toolchain,
            &arguments,
            work_dir,
            &log,
            scenario.timeouts.generation,
        )?;
        fs::write(
            work_dir.join(format!("{}.bsc-ccomp-out", generation.source)),
            &result.output,
        )
        .map_err(|error| format!("write generation output for {}: {error}", generation.source))?;
        if !result.success {
            return Err(format!(
                "Bluesim generation {} failed {}; see {}",
                generation.source,
                super::describe_exit(result.exit_code),
                log.display()
            ));
        }
        check_generation_intermediate(
            toolchain,
            scenario,
            generation,
            index,
            work_dir,
            artifact_dir,
        )?;
    }

    let link_arguments = link_arguments(scenario);
    let link_refs = link_arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let link_log = artifact_dir.join("link.log");
    let result = run_bsc(
        toolchain,
        &link_refs,
        work_dir,
        &link_log,
        scenario.timeouts.link,
    )?;
    fs::write(
        work_dir.join(format!("{}.bsc-ccomp-out", scenario.link.top)),
        &result.output,
    )
    .map_err(|error| format!("write link output for {}: {error}", scenario.link.top))?;
    if !result.success {
        return Err(format!(
            "Bluesim link {} failed {}; see {}",
            scenario.link.top,
            super::describe_exit(result.exit_code),
            link_log.display()
        ));
    }
    resolve_executable(work_dir, scenario.link.top)?;
    if let Some(key) = cache_key {
        generation_cache.store(&key, work_dir)?;
    }
    Ok(())
}

fn check_generation_intermediates(
    toolchain: &Toolchain,
    scenario: &BluesimWorkflowScenario,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), String> {
    for (index, generation) in scenario.generations.iter().enumerate() {
        check_generation_intermediate(
            toolchain,
            scenario,
            generation,
            index,
            work_dir,
            artifact_dir,
        )?;
    }
    Ok(())
}

fn check_generation_intermediate(
    toolchain: &Toolchain,
    scenario: &BluesimWorkflowScenario,
    generation: &BluesimGeneration,
    index: usize,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), String> {
    let output_root = generation_output_root(work_dir, generation.options)?;
    let object = output_root.join(Path::new(generation.source).with_extension("bo"));
    check_intermediate_file(
        toolchain,
        &toolchain.dumpbo,
        &object,
        &artifact_dir.join(format!("generation-{index}.dumpbo.log")),
        scenario.timeouts.generation,
        "compiler object",
    )?;

    if let Some(module) = generation.module {
        let elaborated = output_root.join(format!("{module}.ba"));
        check_intermediate_file(
            toolchain,
            &toolchain.dumpba,
            &elaborated,
            &artifact_dir.join(format!("generation-{index}.dumpba.log")),
            scenario.timeouts.generation,
            "elaborated module",
        )?;
    }
    Ok(())
}

fn generation_output_root(work_dir: &Path, options: &[&str]) -> Result<PathBuf, String> {
    let Some(index) = options.iter().position(|option| *option == "-bdir") else {
        return Ok(work_dir.to_owned());
    };
    let directory = options
        .get(index + 1)
        .ok_or_else(|| "Bluesim generation -bdir option is missing its directory".to_owned())?;
    let directory = Path::new(directory);
    Ok(if directory.is_absolute() {
        directory.to_owned()
    } else {
        work_dir.join(directory)
    })
}

fn check_intermediate_file(
    toolchain: &Toolchain,
    tool: &Path,
    path: &Path,
    log: &Path,
    timeout: std::time::Duration,
    kind: &str,
) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!(
            "BSC did not produce expected {kind} {}",
            path.display()
        ));
    }
    let absolute = path
        .canonicalize()
        .map_err(|error| format!("resolve {kind} {}: {error}", path.display()))?;
    let argument = absolute
        .to_str()
        .ok_or_else(|| format!("{kind} path is not valid UTF-8: {}", absolute.display()))?;
    let result = run_command(
        toolchain,
        tool,
        &[argument],
        path.parent().unwrap_or(path),
        log,
        timeout,
    )?;
    if result.success {
        Ok(())
    } else {
        Err(format!(
            "BSC {kind} {} could not be loaded {}; see {}",
            path.display(),
            super::describe_exit(result.exit_code),
            log.display()
        ))
    }
}

fn workflow_fingerprint(scenario: &BluesimWorkflowScenario) -> Vec<String> {
    let mut fingerprint = vec!["bluesim-workflow-v1".to_owned()];
    for generation in scenario.generations {
        fingerprint.push("generation".to_owned());
        fingerprint.extend(
            generation_arguments(generation)
                .into_iter()
                .map(str::to_owned),
        );
    }
    fingerprint.push("link".to_owned());
    fingerprint.extend(link_arguments(scenario));
    fingerprint
}

fn run_bluesim(
    toolchain: &Toolchain,
    scenario: &BluesimWorkflowScenario,
    run: &BluesimWorkflowRun,
    work_dir: &Path,
    artifact_dir: &Path,
) -> Result<(), String> {
    reset_directory(artifact_dir)?;
    let executable = resolve_executable(work_dir, scenario.link.top)?;
    let launcher = if cfg!(windows) && executable.extension().is_none() {
        Some("sh")
    } else {
        None
    };
    let mut arguments = Vec::with_capacity(run.options.len() + usize::from(launcher.is_some()));
    if launcher.is_some() {
        arguments.push(scenario.link.top);
    }
    arguments.extend_from_slice(run.options);
    let program = launcher.map_or(executable.as_path(), Path::new);
    let log = artifact_dir.join("simulation.log");
    let result = run_command(
        toolchain,
        program,
        &arguments,
        work_dir,
        &log,
        scenario.timeouts.simulation,
    )?;
    fs::write(work_dir.join(run.stdout), &result.output)
        .map_err(|error| format!("write Bluesim output {}: {error}", run.stdout))?;
    if !result.success {
        return Err(format!(
            "Bluesim run {} failed {}; see {}",
            run.name,
            super::describe_exit(result.exit_code),
            log.display()
        ));
    }
    for transfer in run.transfers {
        transfer_artifact(work_dir, transfer)?;
    }
    check_artifact_assertions(run.assertions, work_dir, artifact_dir, run.name)
}

fn resolve_executable(work_dir: &Path, top: &str) -> Result<PathBuf, String> {
    let executable = work_dir.join(top);
    if executable.is_file() {
        return Ok(executable);
    }
    if cfg!(windows) {
        let executable = executable.with_extension("exe");
        if executable.is_file() {
            return Ok(executable);
        }
    }
    Err(format!(
        "BSC did not link Bluesim executable {}",
        work_dir.join(top).display()
    ))
}

fn transfer_artifact(work_dir: &Path, transfer: &super::ArtifactTransfer) -> Result<(), String> {
    let source = work_dir.join(transfer.source);
    if !source.exists() {
        return Ok(());
    }
    let destination = work_dir.join(transfer.destination);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create artifact transfer directory: {error}"))?;
    }
    match transfer.operation {
        ArtifactTransferOperation::Copy => {
            fs::copy(&source, &destination)
                .map(|_| ())
                .map_err(|error| {
                    format!(
                        "copy {} to {}: {error}",
                        source.display(),
                        destination.display()
                    )
                })
        }
        ArtifactTransferOperation::Move => {
            if destination.exists() {
                if destination.is_dir() {
                    fs::remove_dir_all(&destination)
                } else {
                    fs::remove_file(&destination)
                }
                .map_err(|error| format!("remove old transfer destination: {error}"))?;
            }
            fs::rename(&source, &destination).map_err(|error| {
                format!(
                    "move {} to {}: {error}",
                    source.display(),
                    destination.display()
                )
            })
        }
    }
}
