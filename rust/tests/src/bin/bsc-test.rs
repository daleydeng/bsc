use bsc_rust_tests::test_plan::{BluesimEngine, TestPlanExecutor};
use bsc_rust_tests::{secure_directory_within, secure_read_file, Toolchain};
use bsc_test_plan::{PlanStatus, TestPlan, TestPlanIndex};
use clap::Parser;
use rayon::prelude::*;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Parser)]
#[command(
    name = "bsc-test",
    about = "Execute versioned BSC Test Plans with the canonical Rust runner"
)]
struct Cli {
    /// Select plans whose stable ID contains this value.
    filter: Option<String>,
    /// Require the filter to match one complete plan ID exactly.
    #[arg(long)]
    exact: bool,
    /// Start at this exact stable plan ID in index order.
    #[arg(long, value_name = "PLAN_ID")]
    start_at: Option<String>,
    /// List plans without executing them.
    #[arg(long)]
    list: bool,
    /// Execute only this scenario within the exactly selected Test Plan. May be repeated.
    #[arg(long = "scenario", value_name = "SCENARIO_ID")]
    scenarios: Vec<String>,
    /// Select the Bluesim engine scenarios to execute. `both` runs each engine in its own workspace/cache identity.
    #[arg(long, value_enum, default_value_t = BluesimEngine::Legacy)]
    bluesim_engine: BluesimEngine,
    /// Maximum number of Test Plans to execute concurrently.
    #[arg(long, default_value_t = available_jobs())]
    jobs: usize,
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    if cli.exact && cli.filter.is_none() {
        return Err("--exact requires a plan ID".to_owned());
    }
    if !cli.scenarios.is_empty() && !cli.exact {
        return Err("--scenario requires --exact and a complete plan ID".to_owned());
    }
    if !cli.scenarios.is_empty() && cli.list {
        return Err("--scenario cannot be used with --list".to_owned());
    }
    if cli.jobs == 0 {
        return Err("--jobs must be a positive integer".to_owned());
    }
    let toolchain = Toolchain::discover()?;
    let plans_root = secure_directory_within(
        &toolchain.project_root,
        Path::new("rust/tests/plans"),
        "Test Plan root",
    )?;
    let index: TestPlanIndex = serde_json::from_slice(&secure_read_file(
        &plans_root,
        Path::new("index.json"),
        "Test Plan index",
    )?)
    .map_err(|error| format!("decode Test Plan index: {error}"))?;
    index.validate().map_err(|error| error.to_string())?;

    let start_index = match cli.start_at.as_deref() {
        None => 0,
        Some(start_at) => index
            .plans
            .iter()
            .position(|entry| entry.id == start_at)
            .ok_or_else(|| format!("--start-at plan ID does not exist: {start_at}"))?,
    };
    if cli.start_at.is_some() && !cli.list {
        let entry = &index.plans[start_index];
        match entry.status {
            PlanStatus::Complete => {}
            PlanStatus::Disabled => {
                return Err(format!(
                    "--start-at plan is disabled by upstream intent: {}",
                    entry.id
                ));
            }
            PlanStatus::Blocked => {
                return Err(format!("--start-at plan is blocked: {}", entry.id));
            }
        }
    }

    let matches = |id: &str| match cli.filter.as_deref() {
        None => true,
        Some(filter) if cli.exact => id == filter,
        Some(filter) => id.contains(filter),
    };
    let selected = index
        .plans
        .iter()
        .skip(start_index)
        .filter(|entry| {
            matches(&entry.id)
                && (cli.list || cli.filter.is_some() || entry.status == PlanStatus::Complete)
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("no Test Plan matched the requested filter".to_owned());
    }
    if cli.list {
        for entry in selected {
            println!("{:?}\t{}", entry.status, entry.id);
        }
        return Ok(());
    }

    if cli.filter.is_none() {
        let disabled_count = index
            .plans
            .iter()
            .filter(|entry| entry.status == PlanStatus::Disabled)
            .count();
        let blocked_count = index
            .plans
            .iter()
            .filter(|entry| entry.status == PlanStatus::Blocked)
            .count();
        println!(
            "Test Plan selection: running {} complete plans; {disabled_count} disabled and {blocked_count} blocked plans excluded",
            selected.len()
        );
    }

    for (status, label) in [
        (PlanStatus::Disabled, "disabled by upstream intent"),
        (PlanStatus::Blocked, "blocked"),
    ] {
        let unavailable = selected
            .iter()
            .filter(|entry| entry.status == status)
            .collect::<Vec<_>>();
        if unavailable.is_empty() {
            continue;
        }
        let preview = unavailable
            .iter()
            .take(10)
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "selection contains {} {label} Test Plans: {preview}{}",
            unavailable.len(),
            if unavailable.len() > 10 { ", ..." } else { "" }
        ));
    }

    let executor = TestPlanExecutor::new(&toolchain)?.with_bluesim_engine(cli.bluesim_engine);
    let plans = selected
        .into_iter()
        .map(|entry| load_plan(&plans_root, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let total = plans.len();
    let completed = AtomicUsize::new(0);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(cli.jobs.min(total))
        .thread_name(|index| format!("bsc-test-plan-{index}"))
        .build()
        .map_err(|error| format!("create Test Plan worker pool: {error}"))?;
    let results = pool.install(|| {
        plans
            .par_iter()
            .map(|(id, plan)| {
                let result = if cli.scenarios.is_empty() {
                    executor.execute(plan)
                } else {
                    executor.execute_scenarios(plan, &cli.scenarios)
                };
                let current = completed.fetch_add(1, Ordering::Relaxed) + 1;
                println!("[{current}/{total}] {id}");
                (id, result)
            })
            .collect::<Vec<_>>()
    });

    let mut passed = 0;
    let mut skipped = 0;
    let mut failed = 0;
    for (id, result) in results {
        match result {
            Ok(summary) if summary.failed == 0 => {
                passed += summary.passed;
                skipped += summary.skipped;
                println!(
                    "PASS {id} ({} stages passed, {} skipped)",
                    summary.passed, summary.skipped
                );
            }
            Ok(summary) => {
                passed += summary.passed;
                skipped += summary.skipped;
                failed += summary.failed;
                for failure in summary.failures {
                    eprintln!("FAIL {failure}");
                }
            }
            Err(error) => {
                failed += 1;
                eprintln!("FAIL {id}: {error}");
            }
        }
    }
    println!(
        "Test Plan summary: {passed} stages passed, {skipped} skipped, {failed} scenarios failed"
    );
    let cache = executor.cache_summary();
    println!(
        "scenario result cache summary: {} hits, {} misses, {} stores{}",
        cache.hits,
        cache.misses,
        cache.stores,
        if cache.enabled { "" } else { " (disabled)" }
    );
    if failed == 0 {
        Ok(())
    } else {
        Err(format!("{failed} Test Plan scenarios failed"))
    }
}

fn load_plan(
    plans_root: &Path,
    entry: &bsc_test_plan::TestPlanIndexEntry,
) -> Result<(String, TestPlan), String> {
    let path = plans_root.join(&entry.path);
    let plan: TestPlan = serde_json::from_slice(&secure_read_file(
        plans_root,
        Path::new(&entry.path),
        "Test Plan",
    )?)
    .map_err(|error| format!("decode Test Plan {}: {error}", path.display()))?;
    let stage_count = plan
        .scenarios
        .iter()
        .map(|scenario| scenario.stages.len())
        .sum::<usize>();
    let operation_count = plan
        .scenarios
        .iter()
        .flat_map(|scenario| &scenario.stages)
        .map(|stage| stage.operations.len())
        .sum::<usize>();
    if entry.id != plan.id
        || entry.status != plan.status
        || entry.origin != plan.origin
        || entry.scenario_count != plan.scenarios.len()
        || entry.stage_count != stage_count
        || entry.operation_count != operation_count
        || entry.diagnostic_count != plan.diagnostics.len()
    {
        return Err(format!(
            "Test Plan index entry {} does not match {}",
            entry.id,
            path.display()
        ));
    }
    Ok((entry.id.clone(), plan))
}

fn available_jobs() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(16)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("bsc-test: {error}");
        std::process::exit(1);
    }
}
