//! Origin: `testsuite/bsc.bugs/bluespec_inc/b810/b810.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b810";

macro_rules! b810_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.bugs/bluespec_inc/b810::", $module),
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
                    name: concat!("bsc.bugs/bluespec_inc/b810::", $module, "::bluesim"),
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
                    name: concat!("bsc.bugs/bluespec_inc/b810::", $module, "::icarus"),
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

b810_scenario!(BUG_810_1, "Bug810_1");
b810_scenario!(BUG_810_3, "Bug810_3");
b810_scenario!(OPT_BUG, "Opt_bug");

pub(super) const SCENARIOS: &[SimulationScenario] = &[BUG_810_1, BUG_810_3, OPT_BUG];
