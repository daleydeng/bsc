//! Origin: `testsuite/bsc.syntax/bsv05/case/case.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
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
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.syntax/bsv05/case::", $module, "::bluesim"),
                    expected: concat!("sys", $module, ".out.expected"),
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.syntax/bsv05/case::", $module, "::icarus"),
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

case_scenario!(MATCHES_MIXED_LIT, "CaseMatches_MixedLit");
case_scenario!(MIXED_HEX, "CaseMixedHex");
case_scenario!(MIXED_OCT, "CaseMixedOct");

pub(super) const SCENARIOS: &[SimulationScenario] = &[MATCHES_MIXED_LIT, MIXED_HEX, MIXED_OCT];
