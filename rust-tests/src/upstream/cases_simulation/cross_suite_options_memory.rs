//! Origins:
//! - `testsuite/bsc.mcd/MakeClock/MakeClock.exp`
//! - `testsuite/bsc.lib/BRAM/Lat/Lat.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario {
    ($constant:ident, $prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal, $compile_options:expr, [$($extra_fixture:literal),* $(,)?]) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected, $($extra_fixture),*],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
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
                    name: concat!($prefix, "::", $module, "::icarus"),
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
        };
    };
}

shared_scenario!(
    MAKE_CLOCK,
    "bsc.mcd/MakeClock",
    "testsuite/bsc.mcd/MakeClock",
    "MakeClockTest",
    "sysMakeClockTest.out.expected",
    &["-keep-fires"],
    []
);

shared_scenario!(
    LAT_112,
    "bsc.lib/BRAM/Lat",
    "testsuite/bsc.lib/BRAM/Lat",
    "Lat112",
    "sysLat112.out.expected",
    &[],
    ["Latency1Port.bsv"]
);
shared_scenario!(
    LAT_122,
    "bsc.lib/BRAM/Lat",
    "testsuite/bsc.lib/BRAM/Lat",
    "Lat122",
    "sysLat122.out.expected",
    &[],
    ["Latency1Port.bsv"]
);
shared_scenario!(
    LAT_124,
    "bsc.lib/BRAM/Lat",
    "testsuite/bsc.lib/BRAM/Lat",
    "Lat124",
    "sysLat124.out.expected",
    &[],
    ["Latency1Port.bsv"]
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[MAKE_CLOCK, LAT_112, LAT_122, LAT_124];
