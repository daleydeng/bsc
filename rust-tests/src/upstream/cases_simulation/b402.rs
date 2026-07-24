//! Origin: `testsuite/bsc.bugs/bluespec_inc/b402/b402.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

pub(super) const BIT_SWAP_DESIGN: SimulationScenario = SimulationScenario {
    name: "bsc.bugs/bluespec_inc/b402::Test",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b402",
    source: "Test.bsv",
    fixtures: &["Test.bsv", "Design.bsv", "sysTest.out.expected"],
    top: "sysTest",
    link_inputs: &[SimulationLinkInput::GeneratedModule("mkDesign")],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.bugs/bluespec_inc/b402::Test::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTest.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.bugs/bluespec_inc/b402::Test::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTest.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[BIT_SWAP_DESIGN];
