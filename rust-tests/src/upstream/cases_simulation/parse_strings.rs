//! Origin: `testsuite/bsc.syntax/bsv05/strings/parse_strings.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[SimulationScenario {
    name: "bsc.syntax/bsv05/strings::OctalCharsAsInteger",
    fixture_dir: "testsuite/bsc.syntax/bsv05/strings",
    source: "OctalCharsAsInteger.bsv",
    fixtures: &[
        "OctalCharsAsInteger.bsv",
        "sysOctalCharsAsInteger.out.expected",
    ],
    top: "sysOctalCharsAsInteger",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.syntax/bsv05/strings::OctalCharsAsInteger::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysOctalCharsAsInteger.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.syntax/bsv05/strings::OctalCharsAsInteger::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysOctalCharsAsInteger.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
}];
