//! Origin: `testsuite/bsc.arrays/bounds/update/update.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.arrays/bounds/update";

macro_rules! bounds_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.arrays/bounds/update::", $module),
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
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.arrays/bounds/update::", $module, "::bluesim"),
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
                    name: concat!("bsc.arrays/bounds/update::", $module, "::icarus"),
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
        };
    };
}

bounds_scenario!(ARRAY_1, "ArrayInBounds1");
bounds_scenario!(ARRAY_2, "ArrayInBounds2");
bounds_scenario!(LIST_1, "ListInBounds1");
bounds_scenario!(LIST_2, "ListInBounds2");
bounds_scenario!(VECTOR_1, "VectorInBounds1");
bounds_scenario!(VECTOR_2, "VectorInBounds2");
bounds_scenario!(LIST_N_1, "ListNInBounds1");
bounds_scenario!(LIST_N_2, "ListNInBounds2");
bounds_scenario!(BIT_1, "BitInBounds1");
bounds_scenario!(BIT_2, "BitInBounds2");

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    ARRAY_1, ARRAY_2, LIST_1, LIST_2, VECTOR_1, VECTOR_2, LIST_N_1, LIST_N_2, BIT_1, BIT_2,
];
