//! Origin: `testsuite/bsc.bugs/bluespec_inc/b621/b621.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b621";

macro_rules! vector_interface_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.bugs/bluespec_inc/b621::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[
                concat!($module, ".bsv"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            link_inputs: &[SimulationLinkInput::GeneratedModule("mkVectored")],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.bugs/bluespec_inc/b621::", $module, "::bluesim"),
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
                    name: concat!("bsc.bugs/bluespec_inc/b621::", $module, "::icarus"),
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

vector_interface_scenario!(STATIC_VECTOR_INTERFACE, "StaticVectorIfc");
vector_interface_scenario!(DYNAMIC_VECTOR_INTERFACE, "DynamicVectorIfc");

pub(super) const SCENARIOS: &[SimulationScenario] =
    &[STATIC_VECTOR_INTERFACE, DYNAMIC_VECTOR_INTERFACE];
