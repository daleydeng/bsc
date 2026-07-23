//! Origin: `testsuite/bsc.bsv_examples/Life/example_life.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const LIFE_5_BY_5: SimulationScenario = SimulationScenario {
    name: "bsc.bsv_examples/Life::Life",
    fixture_dir: "testsuite/bsc.bsv_examples/Life",
    source: "Life.bsv",
    fixtures: &["Life.bsv", "sysLife.out.expected"],
    top: "sysLife",
    generated_modules: &["mkLife55"],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.bsv_examples/Life::Life::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysLife.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.bsv_examples/Life::Life::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysLife.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[LIFE_5_BY_5];
