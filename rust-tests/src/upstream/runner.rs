use super::compile::run_compile_case;
use super::simulation::{
    ensure_simulation_generation, run_simulation_contract, validate_simulation_scenario,
};
use super::{
    reset_directory, sanitize_case_name, stage_fixture_paths, CompileCase, GenerationStrategy,
    ResourceClass, RunnerPolicy, SimulationContract, SimulationScenario, UpstreamCase,
};
use crate::cache::{BscResultCache, GenerationCache};
use crate::Toolchain;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

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

    pub(super) fn for_name(&self, name: &str) -> (PathBuf, PathBuf) {
        let directory = sanitize_case_name(name);
        (
            self.work_root.join(&directory),
            self.artifact_root.join(directory),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseResult {
    Passed,
    Skipped(String),
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

#[derive(Debug)]
pub(super) enum WorkItem {
    Compile(CompileCase),
    Simulation {
        scenario: &'static SimulationScenario,
        contracts: Vec<&'static SimulationContract>,
    },
}

impl WorkItem {
    fn resource_class(&self) -> ResourceClass {
        match self {
            Self::Compile(_) => ResourceClass::Normal,
            Self::Simulation { scenario, .. } => scenario.resource,
        }
    }
}

pub(super) fn build_work_items(cases: Vec<UpstreamCase>) -> Vec<WorkItem> {
    let mut work = Vec::new();
    for case in cases {
        match case {
            UpstreamCase::Compile(case) => work.push(WorkItem::Compile(case)),
            UpstreamCase::Simulation { scenario, contract } => {
                let existing = work.iter_mut().find_map(|item| match item {
                    WorkItem::Simulation {
                        scenario: registered,
                        contracts,
                    } if std::ptr::eq(*registered, scenario) => Some(contracts),
                    _ => None,
                });
                if let Some(contracts) = existing {
                    contracts.push(contract);
                } else {
                    work.push(WorkItem::Simulation {
                        scenario,
                        contracts: vec![contract],
                    });
                }
            }
        }
    }
    work
}

fn prepare_simulation_generation(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    generation_cache: &GenerationCache,
    scenario: &SimulationScenario,
    contracts: &[&SimulationContract],
) -> Result<PathBuf, String> {
    validate_simulation_scenario(scenario)?;
    let (work_dir, artifact_dir) = run_paths.for_name(scenario.name);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;
    stage_fixture_paths(
        toolchain,
        scenario.fixture_dir,
        scenario.fixtures,
        &work_dir,
    )?;
    let mut backends = Vec::new();
    for contract in contracts {
        if !backends.contains(&contract.backend) {
            backends.push(contract.backend);
        }
    }
    ensure_simulation_generation(
        toolchain,
        generation_cache,
        scenario,
        &backends,
        &work_dir,
        &artifact_dir,
    )?;
    Ok(work_dir)
}

fn run_compile_work_item(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    result_cache: &BscResultCache,
    case: CompileCase,
    policy: RunnerPolicy,
) -> CaseOutcome {
    let result = if let Some(reason) = policy.skip_reason(case.requirement) {
        CaseResult::Skipped(reason)
    } else {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_compile_case(toolchain, run_paths, result_cache, &case)
        })) {
            Ok(Ok(())) => CaseResult::Passed,
            Ok(Err(error)) => CaseResult::Failed(error),
            Err(panic) => CaseResult::Failed(format!("runner panicked: {}", panic_message(panic))),
        }
    };
    CaseOutcome {
        name: case.name,
        result,
    }
}

fn run_simulation_work_item(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    generation_cache: &GenerationCache,
    scenario: &'static SimulationScenario,
    contracts: Vec<&'static SimulationContract>,
    policy: RunnerPolicy,
) -> Vec<CaseOutcome> {
    let enabled = contracts
        .iter()
        .copied()
        .filter(|contract| policy.skip_reason(contract.requirement).is_none())
        .collect::<Vec<_>>();
    if enabled.is_empty() {
        return contracts
            .into_iter()
            .map(|contract| CaseOutcome {
                name: contract.name,
                result: CaseResult::Skipped(
                    policy
                        .skip_reason(contract.requirement)
                        .expect("disabled contract has a skip reason"),
                ),
            })
            .collect();
    }

    let preparation = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepare_simulation_generation(toolchain, run_paths, generation_cache, scenario, &enabled)
    }));
    let generation = match preparation {
        Ok(Ok(work_dir)) => Ok(work_dir),
        Ok(Err(error)) => Err(error),
        Err(panic) => Err(format!("generation panicked: {}", panic_message(panic))),
    };

    contracts
        .into_iter()
        .map(|contract| {
            let result = if let Some(reason) = policy.skip_reason(contract.requirement) {
                CaseResult::Skipped(reason)
            } else {
                match &generation {
                    Err(error) => CaseResult::Failed(format!(
                        "generation for scenario {} failed: {error}",
                        scenario.name
                    )),
                    Ok(generation_dir) => {
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            run_simulation_contract(
                                toolchain,
                                run_paths,
                                scenario,
                                contract,
                                generation_dir,
                            )
                        })) {
                            Ok(Ok(())) => CaseResult::Passed,
                            Ok(Err(error)) => CaseResult::Failed(error),
                            Err(panic) => CaseResult::Failed(format!(
                                "runner panicked: {}",
                                panic_message(panic)
                            )),
                        }
                    }
                }
            };
            CaseOutcome {
                name: contract.name,
                result,
            }
        })
        .collect()
}

fn run_work_item(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    generation_cache: &GenerationCache,
    result_cache: &BscResultCache,
    work: WorkItem,
    policy: RunnerPolicy,
) -> Vec<CaseOutcome> {
    match work {
        WorkItem::Compile(case) => vec![run_compile_work_item(
            toolchain,
            run_paths,
            result_cache,
            case,
            policy,
        )],
        WorkItem::Simulation {
            scenario,
            contracts,
        } => run_simulation_work_item(
            toolchain,
            run_paths,
            generation_cache,
            scenario,
            contracts,
            policy,
        ),
    }
}

pub fn run_cases(
    toolchain: Toolchain,
    run_paths: RunPaths,
    cases: Vec<UpstreamCase>,
    policy: RunnerPolicy,
    test_threads: usize,
) -> Vec<CaseOutcome> {
    let contract_count = cases.len();
    let work = build_work_items(cases);
    let simulation_scenarios = work
        .iter()
        .filter(|item| matches!(item, WorkItem::Simulation { .. }))
        .count();
    let shared_scenarios = work
        .iter()
        .filter(|item| {
            matches!(
                item,
                WorkItem::Simulation { scenario, .. }
                    if scenario.generation == GenerationStrategy::SharedElaboration
            )
        })
        .count();
    let simulation_contracts = work
        .iter()
        .map(|item| match item {
            WorkItem::Compile(_) => 0,
            WorkItem::Simulation { contracts, .. } => contracts.len(),
        })
        .sum::<usize>();
    println!(
        "execution plan: {simulation_contracts} simulation contracts in {simulation_scenarios} generation scenarios ({shared_scenarios} shared); {} compile contracts",
        contract_count - simulation_contracts
    );

    let generation_cache = Arc::new(match GenerationCache::new(&toolchain) {
        Ok(cache) => cache,
        Err(error) => {
            eprintln!(
                "warning: generation cache initialization failed; continuing uncached: {error}"
            );
            GenerationCache::disabled(&toolchain)
        }
    });
    let result_cache = Arc::new(match BscResultCache::new(&toolchain) {
        Ok(cache) => cache,
        Err(error) => {
            eprintln!(
                "warning: BSC result cache initialization failed; continuing uncached: {error}"
            );
            BscResultCache::disabled(&toolchain)
        }
    });
    let toolchain = Arc::new(toolchain);
    let run_paths = Arc::new(run_paths);
    let (heavy, parallel): (Vec<_>, Vec<_>) = work
        .into_iter()
        .partition(|item| item.resource_class() == ResourceClass::Heavy);
    let worker_toolchain = Arc::clone(&toolchain);
    let worker_run_paths = Arc::clone(&run_paths);
    let worker_generation_cache = Arc::clone(&generation_cache);
    let worker_result_cache = Arc::clone(&result_cache);
    let mut outcomes: Vec<CaseOutcome> = run_fixed_queue(parallel, test_threads, move |work| {
        run_work_item(
            &worker_toolchain,
            &worker_run_paths,
            &worker_generation_cache,
            &worker_result_cache,
            work,
            policy,
        )
    })
    .into_iter()
    .flatten()
    .collect();
    let heavy_generation_cache = Arc::clone(&generation_cache);
    let heavy_result_cache = Arc::clone(&result_cache);
    outcomes.extend(
        run_fixed_queue(heavy, 1, move |work| {
            run_work_item(
                &toolchain,
                &run_paths,
                &heavy_generation_cache,
                &heavy_result_cache,
                work,
                policy,
            )
        })
        .into_iter()
        .flatten(),
    );
    let cache = generation_cache.summary();
    if cache.enabled {
        println!(
            "generation cache: {} hits, {} misses, {} stores",
            cache.hits, cache.misses, cache.stores
        );
    } else {
        println!("generation cache: disabled");
    }
    let cache = result_cache.summary();
    if cache.enabled {
        println!(
            "BSC result cache: {} hits, {} misses, {} stores",
            cache.hits, cache.misses, cache.stores
        );
    } else {
        println!("BSC result cache: disabled");
    }
    outcomes
}

pub(super) fn run_fixed_queue<T, R, F>(items: Vec<T>, thread_count: usize, worker: F) -> Vec<R>
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
