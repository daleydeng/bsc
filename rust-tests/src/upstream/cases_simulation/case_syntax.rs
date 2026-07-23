//! Origin: `testsuite/bsc.syntax/bsv05/case/case.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bsv05/case";

macro_rules! case_scenario {
    ($constant:ident, $module:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.syntax/bsv05/case::", $module),
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
                    name: concat!("bsc.syntax/bsv05/case::", $module, "::bluesim"),
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
                    name: concat!("bsc.syntax/bsv05/case::", $module, "::icarus"),
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

case_scenario!(MATCHES_MIXED_LIT, "CaseMatches_MixedLit");
case_scenario!(MIXED_HEX, "CaseMixedHex");
case_scenario!(MIXED_OCT, "CaseMixedOct");

pub(super) const SCENARIOS: &[SimulationScenario] = &[MATCHES_MIXED_LIT, MIXED_HEX, MIXED_OCT];
