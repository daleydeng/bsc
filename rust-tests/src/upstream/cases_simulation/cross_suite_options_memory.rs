//! Origins:
//! - `testsuite/bsc.mcd/MakeClock/MakeClock.exp`
//! - `testsuite/bsc.lib/BRAM/Lat/Lat.exp`

use super::SimulationCase;
use crate::upstream::{GenerationStrategy, Requirement, ResourceClass, SimulationBackend};

macro_rules! backend_pair {
    ($bluesim:ident, $icarus:ident, $prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal, $compile_options:expr, [$($extra_fixture:literal),* $(,)?]) => {
        backend_pair!(
            $bluesim,
            $icarus,
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            $compile_options,
            [$($extra_fixture),*],
            ResourceClass::Normal,
            $crate::BSC_TIMEOUT
        );
    };
    ($bluesim:ident, $icarus:ident, $prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal, $compile_options:expr, [$($extra_fixture:literal),* $(,)?], $resource:expr, $timeout:expr) => {
        pub(super) const $bluesim: SimulationCase = SimulationCase {
            name: concat!($prefix, "::", $module, "::bluesim"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected, $($extra_fixture),*],
            top: concat!("sys", $module),
            generated_modules: &[],
            expected: $expected,
            compile_options: $compile_options,
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: SimulationBackend::Bluesim,
            generation: GenerationStrategy::SharedElaboration,
            vcd: $crate::upstream::VcdExpectation::None,
            requirement: Requirement::BluesimEnabled,
            timeout: $timeout,
            resource: $resource,
        };
        pub(super) const $icarus: SimulationCase = SimulationCase {
            name: concat!($prefix, "::", $module, "::icarus"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected, $($extra_fixture),*],
            top: concat!("sys", $module),
            generated_modules: &[],
            expected: $expected,
            compile_options: $compile_options,
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: SimulationBackend::Icarus,
            generation: GenerationStrategy::SharedElaboration,
            vcd: $crate::upstream::VcdExpectation::None,
            requirement: Requirement::VerilogEnabled,
            timeout: $timeout,
            resource: $resource,
        };
    };
}

backend_pair!(
    MAKE_CLOCK_BLUESIM,
    MAKE_CLOCK_ICARUS,
    "bsc.mcd/MakeClock",
    "testsuite/bsc.mcd/MakeClock",
    "MakeClockTest",
    "sysMakeClockTest.out.expected",
    &["-keep-fires"],
    []
);

backend_pair!(
    LAT_112_BLUESIM,
    LAT_112_ICARUS,
    "bsc.lib/BRAM/Lat",
    "testsuite/bsc.lib/BRAM/Lat",
    "Lat112",
    "sysLat112.out.expected",
    &[],
    ["Latency1Port.bsv"]
);
backend_pair!(
    LAT_122_BLUESIM,
    LAT_122_ICARUS,
    "bsc.lib/BRAM/Lat",
    "testsuite/bsc.lib/BRAM/Lat",
    "Lat122",
    "sysLat122.out.expected",
    &[],
    ["Latency1Port.bsv"]
);
backend_pair!(
    LAT_124_BLUESIM,
    LAT_124_ICARUS,
    "bsc.lib/BRAM/Lat",
    "testsuite/bsc.lib/BRAM/Lat",
    "Lat124",
    "sysLat124.out.expected",
    &[],
    ["Latency1Port.bsv"]
);

pub(super) const CASES: &[SimulationCase] = &[
    MAKE_CLOCK_BLUESIM,
    MAKE_CLOCK_ICARUS,
    LAT_112_BLUESIM,
    LAT_112_ICARUS,
    LAT_122_BLUESIM,
    LAT_122_ICARUS,
    LAT_124_BLUESIM,
    LAT_124_ICARUS,
];
