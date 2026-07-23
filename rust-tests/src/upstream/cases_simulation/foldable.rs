//! Origin: `testsuite/bsc.lib/Foldable/Foldable.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/Foldable";

macro_rules! foldable_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.lib/Foldable::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bs"),
            fixtures: &[
                concat!($module, ".bs"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.lib/Foldable::", $module, "::bluesim"),
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
                    name: concat!("bsc.lib/Foldable::", $module, "::icarus"),
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

foldable_scenario!(FOLDABLE, "FoldableTest");
foldable_scenario!(FOLDABLE_ONE, "Foldable1Test");

pub(super) const SCENARIOS: &[SimulationScenario] = &[FOLDABLE, FOLDABLE_ONE];
