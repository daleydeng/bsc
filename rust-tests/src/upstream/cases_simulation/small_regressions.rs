//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1037/b1037.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1045/b1045.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
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
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeout: crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::icarus"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Icarus,
                    vcd: VcdExpectation::IcarusSmoke,
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
