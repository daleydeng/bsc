use super::{
    compare_legacy_golden, count_diagnostics, describe_exit, is_safe_relative, reset_directory,
    stage_fixture_paths, CompileCase, CompileExpectation, CompileMode, Requirement, RunPaths,
};
use crate::cache::{BscResultCache, ResultCacheLookup};
use crate::{run_bsc, Toolchain, BSC_TIMEOUT};
use std::fs;
use std::path::Path;

pub(super) fn run_compile_case(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    result_cache: &BscResultCache,
    case: &CompileCase,
) -> Result<(), String> {
    validate_case(case)?;
    let (work_dir, artifact_dir) = run_paths.for_name(case.name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;
    stage_fixtures(toolchain, case, &work_dir)?;

    let arguments = compile_arguments(case);

    let log_path = artifact_dir.join("bsc.log");
    let fixture_root = toolchain.project_root.join(case.fixture_dir);
    let (result, cache_key) = match result_cache.lookup(
        &fixture_root,
        case.fixtures,
        &arguments,
        &work_dir,
        &log_path,
    ) {
        Ok(ResultCacheLookup::Hit(result)) => (result, None),
        Ok(ResultCacheLookup::Miss(key)) => (
            run_bsc(toolchain, &arguments, &work_dir, &log_path, BSC_TIMEOUT)?,
            Some(key),
        ),
        Ok(ResultCacheLookup::Disabled) => (
            run_bsc(toolchain, &arguments, &work_dir, &log_path, BSC_TIMEOUT)?,
            None,
        ),
        Err(error) => {
            eprintln!(
                "warning: BSC result cache lookup failed for {}: {error}",
                case.name
            );
            (
                run_bsc(toolchain, &arguments, &work_dir, &log_path, BSC_TIMEOUT)?,
                None,
            )
        }
    };
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

    if let Some(key) = cache_key {
        if let Err(error) = result_cache.store(&key, &work_dir, &result) {
            eprintln!(
                "warning: BSC result cache store failed for {}: {error}",
                case.name
            );
        }
    }

    Ok(())
}

pub(super) fn compile_arguments(case: &CompileCase) -> Vec<&str> {
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
            check_compile_success(case, success, exit_code, work_dir, log_path)?;
        }
        CompileExpectation::PassWithDiagnostic { kind, tag, count } => {
            check_compile_success(case, success, exit_code, work_dir, log_path)?;
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

fn check_compile_success(
    case: &CompileCase,
    success: bool,
    exit_code: Option<i32>,
    work_dir: &Path,
    log_path: &Path,
) -> Result<(), String> {
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
    Ok(())
}

fn stage_fixtures(
    toolchain: &Toolchain,
    case: &CompileCase,
    work_dir: &Path,
) -> Result<(), String> {
    stage_fixture_paths(toolchain, case.fixture_dir, case.fixtures, work_dir)
}

pub(super) fn validate_case(case: &CompileCase) -> Result<(), String> {
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
