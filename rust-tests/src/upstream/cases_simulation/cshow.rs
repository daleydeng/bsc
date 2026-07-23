//! Origin: `testsuite/bsc.lib/CShow/CShow.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const CLASSIC_SHOW: SimulationScenario = SimulationScenario {
    name: "bsc.lib/CShow::TestCShow",
    fixture_dir: "testsuite/bsc.lib/CShow",
    source: "TestCShow.bs",
    fixtures: &["TestCShow.bs", "sysTestCShow.out.expected"],
    top: "sysTestCShow",
    generated_modules: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.lib/CShow::TestCShow::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTestCShow.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.lib/CShow::TestCShow::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTestCShow.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[CLASSIC_SHOW];
