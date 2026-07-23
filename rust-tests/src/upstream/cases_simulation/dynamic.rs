//! Origin: `testsuite/bsc.evaluator/dynamic/dynamic.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
};

const FIXTURE_DIR: &str = "testsuite/bsc.evaluator/dynamic";

macro_rules! dynamic_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.evaluator/dynamic::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[
                concat!($module, ".bsv"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.evaluator/dynamic::", $module, "::bluesim"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.evaluator/dynamic::", $module, "::icarus"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Icarus,
                    vcd: VcdExpectation::IcarusSmoke,
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

dynamic_scenario!(INTEGER, "DynamicInteger");
dynamic_scenario!(INTEGER_NESTED, "DynamicIntegerNested");
dynamic_scenario!(DIV, "DynamicDiv");
dynamic_scenario!(NEG, "DynamicNeg");
dynamic_scenario!(NEG_2, "DynamicNeg2");
dynamic_scenario!(LT, "DynamicLT");
dynamic_scenario!(ADD, "DynamicAdd");

pub(super) const SCENARIOS: &[SimulationScenario] =
    &[INTEGER, INTEGER_NESTED, DIV, NEG, NEG_2, LT, ADD];
