use bsc_rust_tests::upstream::{
    parse_cli, probe_iverilog_major, run_plan, select_plan, summarize_outcomes, CaseResult,
    RunPaths, RunnerPolicy,
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
            eprintln!(
                "usage: upstream [--list] [FILTER] [--exact] [--no-bluesim] [--no-verilog] [--test-threads N]"
            );
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let options = parse_cli(env::args_os().skip(1))?;
    let policy = RunnerPolicy::new(options.bluesim_enabled, options.verilog_enabled)
        .with_iverilog_major(probe_iverilog_major());
    let plan = select_plan(&options);
    let total = plan.contract_count();
    if options.list {
        for name in plan.contract_names() {
            println!("{name}: test");
        }
        println!();
        println!("{total} tests");
        return Ok(true);
    }

    println!("running {total} tests");
    if total == 0 {
        println!();
        println!("test result: ok. 0 passed; 0 xfailed; 0 skipped; 0 failed");
        return Ok(true);
    }

    let toolchain = Toolchain::discover()?;
    let run_paths = RunPaths::new(&toolchain.project_root, current_run_id());
    let outcomes = run_plan(toolchain, run_paths, plan, policy, options.test_threads);
    let summary = summarize_outcomes(&outcomes);
    let mut failures = Vec::new();
    for outcome in &outcomes {
        match &outcome.result {
            CaseResult::Passed => {}
            CaseResult::XFailed(reason) => {
                println!("test {} ... XFAIL: {reason}", outcome.name)
            }
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
            "test result: ok. {} passed; {} xfailed; {} skipped; 0 failed",
            summary.passed, summary.xfailed, summary.skipped
        );
    } else {
        println!(
            "test result: FAILED. {} passed; {} xfailed; {} skipped; {} failed",
            summary.passed, summary.xfailed, summary.skipped, summary.failed
        );
    }
    Ok(summary.failed == 0)
}
