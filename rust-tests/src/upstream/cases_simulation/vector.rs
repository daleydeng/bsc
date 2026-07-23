//! Origin: `testsuite/bsc.lib/vector/libvector.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/vector";

macro_rules! vector_scenario {
    ($constant:ident, $module:literal, $extension:literal, $compile_options:expr) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.lib/vector::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, $extension),
            fixtures: &[
                concat!($module, $extension),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.lib/vector::", $module, "::bluesim"),
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
                    name: concat!("bsc.lib/vector::", $module, "::icarus"),
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
    ($constant:ident, $module:literal, $extension:literal) => {
        vector_scenario!($constant, $module, $extension, &[]);
    };
}

vector_scenario!(MISC_FUNC, "MiscFunc", ".bsv", &["-let-gen"]);
vector_scenario!(SHIFT_OUT_TEST, "ShiftOutTest", ".bsv");
vector_scenario!(SHIFT_TEST, "ShiftTest", ".bsv");
vector_scenario!(COUNT_ELEM, "CountElem", ".bsv");
vector_scenario!(COUNT_IF, "CountIf", ".bsv");
vector_scenario!(FIND_ELEM, "FindElem", ".bsv");
vector_scenario!(FIND_INDEX, "FindIndex", ".bsv");
vector_scenario!(ROTATE_BY, "RotateBy", ".bsv");
vector_scenario!(ZERO_VECTOR, "ZeroVector", ".bsv");
vector_scenario!(FROM_CHUNKS_TEST, "FromChunksTest", ".bsv");
vector_scenario!(CONCAT_TUPLE, "ConcatTuple", ".bs");
vector_scenario!(APPLICATIVE_VECTOR, "ApplicativeVector", ".bs");

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    MISC_FUNC,
    SHIFT_OUT_TEST,
    SHIFT_TEST,
    COUNT_ELEM,
    COUNT_IF,
    FIND_ELEM,
    FIND_INDEX,
    ROTATE_BY,
    ZERO_VECTOR,
    FROM_CHUNKS_TEST,
    CONCAT_TUPLE,
    APPLICATIVE_VECTOR,
];
