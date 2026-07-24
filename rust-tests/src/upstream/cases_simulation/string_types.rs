//! Origin: `testsuite/bsc.typechecker/string/string.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/string";

macro_rules! string_scenario {
    ($constant:ident, $module:literal, $extension:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.typechecker/string::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, $extension),
            fixtures: &[
                concat!($module, $extension),
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
                    name: concat!("bsc.typechecker/string::", $module, "::bluesim"),
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
                    name: concat!("bsc.typechecker/string::", $module, "::icarus"),
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

string_scenario!(STRING_OF, "StringOf", ".bs");
string_scenario!(STRING_OF_BSV, "StringOfBSV", ".bsv");
string_scenario!(TYPE_CLASS_STRING, "TypeClassString", ".bs");
string_scenario!(T_STR_CAT, "TStrCat", ".bs");
string_scenario!(T_NUM_TO_STR, "TNumToStr", ".bs");

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    STRING_OF,
    STRING_OF_BSV,
    TYPE_CLASS_STRING,
    T_STR_CAT,
    T_NUM_TO_STR,
];
