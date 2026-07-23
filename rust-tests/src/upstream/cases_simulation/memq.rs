//! Origin: `testsuite/bsc.bsv_examples/memq/priq.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const DYNAMIC_PRIORITY_QUEUE: SimulationScenario = SimulationScenario {
    name: "bsc.bsv_examples/memq::DQueueTb",
    fixture_dir: "testsuite/bsc.bsv_examples/memq",
    source: "DQueueTb.bsv",
    fixtures: &[
        "DQueueTb.bsv",
        "DQueue.bsv",
        "DQueueConfig.bsv",
        "PriQ.bsv",
        "Priority.bsv",
        "QType.bsv",
        "sysDQueueTb.out.expected",
    ],
    top: "sysDQueueTb",
    generated_modules: &["mkQueue"],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.bsv_examples/memq::DQueueTb::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysDQueueTb.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.bsv_examples/memq::DQueueTb::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysDQueueTb.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[DYNAMIC_PRIORITY_QUEUE];
