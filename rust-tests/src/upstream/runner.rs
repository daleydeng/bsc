use super::compile::run_compile_case;
use super::simulation::{
    ensure_simulation_generation, evaluate_generation_outcome, run_simulation_contract,
    validate_simulation_scenario, ContractRunOutcome, PhaseFailure,
};
use super::{
    reset_directory, sanitize_case_name, stage_fixture_paths, CompileCase, ExecutionPlan,
    GenerationStrategy, ResourceClass, RunnerPolicy, SimulationContract, SimulationPhase,
    SimulationScenario,
};
use crate::cache::{BscResultCache, GenerationCache};
use crate::Toolchain;
use std::collections::VecDeque;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
    XFailed(String),
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
    pub xfailed: usize,
    pub skipped: usize,
    pub failed: usize,
}

pub fn summarize_outcomes(outcomes: &[CaseOutcome]) -> RunSummary {
    let mut summary = RunSummary::default();
    for outcome in outcomes {
        match outcome.result {
            CaseResult::Passed => summary.passed += 1,
            CaseResult::XFailed(_) => summary.xfailed += 1,
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

    fn label(&self) -> &'static str {
        match self {
            Self::Compile(case) => case.name,
            Self::Simulation { scenario, .. } => scenario.name,
        }
    }
}

const PROGRESS_INTERVAL: Duration = Duration::from_secs(10);
const PROGRESS_CONTRACT_STEP: usize = 100;

#[derive(Clone, Copy)]
enum ProgressPhase {
    Parallel = 0,
    Heavy = 1,
}

struct ProgressState {
    total: usize,
    completed: AtomicUsize,
    phase: AtomicUsize,
    stop: AtomicBool,
    started: Instant,
    output: Mutex<()>,
}

impl ProgressState {
    fn report(&self, detail: Option<&str>) {
        let _output = self.output.lock().expect("progress output lock poisoned");
        let completed = self.completed.load(Ordering::Relaxed).min(self.total);
        let phase = match self.phase.load(Ordering::Relaxed) {
            value if value == ProgressPhase::Heavy as usize => "heavy",
            _ => "parallel",
        };
        let percentage = if self.total == 0 {
            100.0
        } else {
            completed as f64 * 100.0 / self.total as f64
        };
        let detail = detail
            .map(|detail| format!(", {detail}"))
            .unwrap_or_default();
        let mut output = io::stdout().lock();
        let _ = writeln!(
            output,
            "progress: {completed}/{} ({percentage:.1}%), phase={phase}, elapsed={}{}",
            self.total,
            format_elapsed(self.started.elapsed()),
            detail
        );
        let _ = output.flush();
    }

    fn advance(&self, count: usize) {
        let previous = self.completed.fetch_add(count, Ordering::Relaxed);
        let completed = (previous + count).min(self.total);
        if completed < self.total
            && previous / PROGRESS_CONTRACT_STEP != completed / PROGRESS_CONTRACT_STEP
        {
            self.report(None);
        }
    }
}

struct ProgressReporter {
    state: Arc<ProgressState>,
    worker: Option<thread::JoinHandle<()>>,
}

impl ProgressReporter {
    fn start(total: usize) -> Self {
        let state = Arc::new(ProgressState {
            total,
            completed: AtomicUsize::new(0),
            phase: AtomicUsize::new(ProgressPhase::Parallel as usize),
            stop: AtomicBool::new(false),
            started: Instant::now(),
            output: Mutex::new(()),
        });
        state.report(Some("started"));

        let worker_state = Arc::clone(&state);
        let worker = thread::spawn(move || loop {
            thread::park_timeout(PROGRESS_INTERVAL);
            if worker_state.stop.load(Ordering::Acquire) {
                break;
            }
            worker_state.report(None);
        });
        Self {
            state,
            worker: Some(worker),
        }
    }

    fn state(&self) -> Arc<ProgressState> {
        Arc::clone(&self.state)
    }

    fn enter_heavy_phase(&self, scenario_count: usize) {
        self.state
            .phase
            .store(ProgressPhase::Heavy as usize, Ordering::Relaxed);
        self.state.report(Some(&format!(
            "entering {scenario_count} serialized heavy scenario(s)"
        )));
    }

    fn finish(mut self) {
        self.stop_worker();
        self.state.report(Some("complete"));
    }

    fn stop_worker(&mut self) {
        self.state.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    if minutes == 0 {
        format!("{seconds}s")
    } else {
        format!("{minutes}m {seconds:02}s")
    }
}

fn prepare_simulation_generation(
    toolchain: &Toolchain,
    run_paths: &RunPaths,
    generation_cache: &GenerationCache,
    scenario: &SimulationScenario,
    contracts: &[&SimulationContract],
) -> Result<PathBuf, PhaseFailure> {
    validate_simulation_scenario(scenario)
        .map_err(|error| PhaseFailure::new(SimulationPhase::Generation, error))?;
    let (work_dir, artifact_dir) = run_paths.for_name(scenario.name);
    reset_directory(&work_dir)
        .map_err(|error| PhaseFailure::new(SimulationPhase::Generation, error))?;
    reset_directory(&artifact_dir)
        .map_err(|error| PhaseFailure::new(SimulationPhase::Generation, error))?;
    stage_fixture_paths(
        toolchain,
        scenario.fixture_dir,
        scenario.fixtures,
        &work_dir,
    )
    .map_err(|error| PhaseFailure::new(SimulationPhase::Generation, error))?;
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
        Ok(result) => result,
        Err(panic) => Err(PhaseFailure::new(
            SimulationPhase::Generation,
            format!("generation panicked: {}", panic_message(panic)),
        )),
    };
    let (generation_work_dir, _) = run_paths.for_name(scenario.name);

    contracts
        .into_iter()
        .map(|contract| {
            let result = if let Some(reason) = policy.skip_reason(contract.requirement) {
                CaseResult::Skipped(reason)
            } else {
                let execution = match &generation {
                    Err(failure) => {
                        let (_, artifact_dir) = run_paths.for_name(contract.name);
                        evaluate_generation_outcome(
                            contract,
                            Err(failure.clone()),
                            &generation_work_dir,
                            &artifact_dir,
                        )
                    }
                    Ok(generation_dir)
                        if contract.expectation.expected_failure_phase()
                            == Some(SimulationPhase::Generation) =>
                    {
                        let (_, artifact_dir) = run_paths.for_name(contract.name);
                        evaluate_generation_outcome(contract, Ok(()), generation_dir, &artifact_dir)
                    }
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
                            Ok(result) => result,
                            Err(panic) => Err(format!("runner panicked: {}", panic_message(panic))),
                        }
                    }
                };
                match execution {
                    Ok(ContractRunOutcome::Passed) => CaseResult::Passed,
                    Ok(ContractRunOutcome::XFailed(reason)) => CaseResult::XFailed(reason),
                    Err(error) => CaseResult::Failed(error),
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

pub fn run_plan(
    toolchain: Toolchain,
    run_paths: RunPaths,
    plan: ExecutionPlan,
    policy: RunnerPolicy,
    test_threads: usize,
) -> Vec<CaseOutcome> {
    let contract_count = plan.contract_count();
    let mut work = plan
        .compile_cases
        .into_iter()
        .map(WorkItem::Compile)
        .collect::<Vec<_>>();
    work.extend(
        plan.simulations
            .into_iter()
            .map(|simulation| WorkItem::Simulation {
                scenario: simulation.scenario,
                contracts: simulation.contracts,
            }),
    );
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
    let progress = ProgressReporter::start(contract_count);

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
    let parallel_progress = progress.state();
    let mut outcomes: Vec<CaseOutcome> = run_fixed_queue(parallel, test_threads, move |work| {
        let outcomes = run_work_item(
            &worker_toolchain,
            &worker_run_paths,
            &worker_generation_cache,
            &worker_result_cache,
            work,
            policy,
        );
        parallel_progress.advance(outcomes.len());
        outcomes
    })
    .into_iter()
    .flatten()
    .collect();
    let heavy_generation_cache = Arc::clone(&generation_cache);
    let heavy_result_cache = Arc::clone(&result_cache);
    if !heavy.is_empty() {
        progress.enter_heavy_phase(heavy.len());
    }
    let heavy_progress = progress.state();
    outcomes.extend(
        run_fixed_queue(heavy, 1, move |work| {
            heavy_progress.report(Some(&format!("scenario={}", work.label())));
            let outcomes = run_work_item(
                &toolchain,
                &run_paths,
                &heavy_generation_cache,
                &heavy_result_cache,
                work,
                policy,
            );
            heavy_progress.advance(outcomes.len());
            outcomes
        })
        .into_iter()
        .flatten(),
    );
    progress.finish();
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

#[cfg(test)]
mod tests {
    use super::format_elapsed;
    use std::time::Duration;

    #[test]
    fn progress_elapsed_time_is_compact() {
        assert_eq!(format_elapsed(Duration::from_secs(9)), "9s");
        assert_eq!(format_elapsed(Duration::from_secs(125)), "2m 05s");
    }
}
