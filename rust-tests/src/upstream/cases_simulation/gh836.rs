//! Origin: `testsuite/bsc.bugs/github/gh836/gh836.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/github/gh836";

macro_rules! zero_width_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.bugs/github/gh836::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bs"),
            fixtures: &[
                concat!($module, ".bs"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.bugs/github/gh836::", $module, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $module, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.bugs/github/gh836::", $module, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $module, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

zero_width_scenario!(ZERO_SIZE_WIRES, "ZeroSizeWires");
zero_width_scenario!(ZERO_UNDETERMINED, "ZeroUndet");

pub(super) const SCENARIOS: &[SimulationScenario] = &[ZERO_SIZE_WIRES, ZERO_UNDETERMINED];
