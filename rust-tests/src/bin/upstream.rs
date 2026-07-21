use bsc_rust_tests::upstream::{
    all_cases, parse_cli, run_cases, select_cases, summarize_outcomes, CaseResult, RunPaths,
    RunnerPolicy, UpstreamCase,
};
use bsc_rust_tests::{current_run_id, Toolchain};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(success) if success => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("usage: upstream [--list] [FILTER] [--exact] [--test-threads N]");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let options = parse_cli(env::args_os().skip(1))?;
    let policy = RunnerPolicy::from_environment(
        env::var_os("CTEST").as_deref(),
        env::var_os("VTEST").as_deref(),
    );
    let available = all_cases();
    let selected = select_cases(&available, &options);
    if options.list {
        for case in &selected {
            println!("{}: test", case.name());
        }
        println!();
        println!("{} tests", selected.len());
        return Ok(true);
    }

    let cases: Vec<UpstreamCase> = selected.into_iter().copied().collect();
    let total = cases.len();
    println!("running {total} tests");
    if cases.is_empty() {
        println!();
        println!("test result: ok. 0 passed; 0 skipped; 0 failed");
        return Ok(true);
    }

    let toolchain = Toolchain::discover()?;
    let run_paths = RunPaths::new(&toolchain.project_root, current_run_id());
    let outcomes = run_cases(toolchain, run_paths, cases, policy, options.test_threads);
    let summary = summarize_outcomes(&outcomes);
    let mut failures = Vec::new();
    for outcome in &outcomes {
        match &outcome.result {
            CaseResult::Passed => println!("test {} ... ok", outcome.name),
            CaseResult::Skipped(reason) => {
                println!("test {} ... SKIPPED: {reason}", outcome.name)
            }
            CaseResult::Failed(error) => {
                println!("test {} ... FAILED", outcome.name);
                failures.push((outcome.name, error));
            }
        }
    }

    if !failures.is_empty() {
        println!();
        println!("failures:");
        for (name, error) in &failures {
            println!();
            println!("---- {name} ----");
            println!("{error}");
        }
    }

    println!();
    if summary.failed == 0 {
        println!(
            "test result: ok. {} passed; {} skipped; 0 failed",
            summary.passed, summary.skipped
        );
    } else {
        println!(
            "test result: FAILED. {} passed; {} skipped; {} failed",
            summary.passed, summary.skipped, summary.failed
        );
    }
    Ok(summary.failed == 0)
}
