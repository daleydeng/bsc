//! Origin: `testsuite/bsc.typechecker/read_desugaring/read_desugaring.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/read_desugaring";

macro_rules! read_desugaring_scenario {
    ($module:literal) => {
        SimulationScenario {
            name: concat!("bsc.typechecker/read_desugaring::", $module),
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
            timeout: crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.typechecker/read_desugaring::", $module, "::bluesim"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.typechecker/read_desugaring::", $module, "::icarus"),
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
    read_desugaring_scenario!("ListDesugar"),
    read_desugaring_scenario!("StructReg"),
    read_desugaring_scenario!("TwoDUpdateTest"),
];
