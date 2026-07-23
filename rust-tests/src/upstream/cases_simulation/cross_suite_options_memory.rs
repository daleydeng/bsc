//! Origins:
//! - `testsuite/bsc.mcd/MakeClock/MakeClock.exp`
//! - `testsuite/bsc.lib/BRAM/Lat/Lat.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
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
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
                    expected: $expected,
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::icarus"),
                    expected: $expected,
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
