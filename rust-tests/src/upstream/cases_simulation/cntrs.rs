//! Origin: `testsuite/bsc.lib/Cntrs/Cntrs.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/Cntrs";

macro_rules! scenario {
    ($module:literal, $expected:literal) => {
        SimulationScenario {
            name: concat!("bsc.lib/Cntrs::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.lib/Cntrs::", $module, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.lib/Cntrs::", $module, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        }
    };
}

pub(super) const CNTR_TEST: SimulationScenario = scenario!("CntrTest", "sysCntrTest.out.expected");
pub(super) const UCNTR_TEST: SimulationScenario =
    scenario!("UCntrTest", "sysUCntrTest.out.expected");
pub(super) const CNTRS_0: SimulationScenario = scenario!("Cntrs0", "sysCntrs0.out.expected");

pub(super) const SCENARIOS: &[SimulationScenario] = &[CNTR_TEST, UCNTR_TEST, CNTRS_0];
