//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1037/b1037.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1045/b1045.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario {
    ($prefix:literal, $fixture_dir:literal, $module:literal) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[
                concat!($module, ".bsv"),
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
                    name: concat!($prefix, "::", $module, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: concat!("sys", $module, ".out.expected") },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: concat!("sys", $module, ".out.expected") },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        }
    };
}

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1037",
        "testsuite/bsc.bugs/bluespec_inc/b1037",
        "Foo"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1045",
        "testsuite/bsc.bugs/bluespec_inc/b1045",
        "Design"
    ),
];
