//! Origins:
//! - `testsuite/bsc.bsv_examples/FIRFilter/firfilter.exp`
//! - `testsuite/bsc.assertions/properties/properties.exp`
//! - `testsuite/bsc.bsv_examples/Hamming/hamming.exp`
//! - `testsuite/bsc.lib/BRAM/BRAMTest/BRAMTest.exp`

use super::SimulationCase;
use crate::upstream::{Requirement, SimulationBackend};

macro_rules! simulation_case {
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr, $fixtures:expr, $backend:expr, $requirement:expr) => {
        simulation_case!(
            $name,
            $fixture_dir,
            $module,
            $expected,
            $fixtures,
            $backend,
            $requirement,
            $crate::BSC_TIMEOUT,
            false
        )
    };
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr, $fixtures:expr, $backend:expr, $requirement:expr, $timeout:expr, $heavy:expr) => {
        SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            expected: $expected,
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: $backend,
            requirement: $requirement,
            timeout: $timeout,
            heavy: $heavy,
        }
    };
}

const FIR_DIR: &str = "testsuite/bsc.bsv_examples/FIRFilter";
const FIR_FIXTURES: &[&str] = &[
    "FIRTest.bsv",
    "FIRMain.bsv",
    "SyncFIR.bsv",
    "sysFIRTest.out.expected",
];
pub(super) const FIR_BLUESIM: SimulationCase = simulation_case!(
    "bsc.bsv_examples/FIRFilter::FIRTest::bluesim",
    FIR_DIR,
    "FIRTest",
    "sysFIRTest.out.expected",
    FIR_FIXTURES,
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const FIR_ICARUS: SimulationCase = simulation_case!(
    "bsc.bsv_examples/FIRFilter::FIRTest::icarus",
    FIR_DIR,
    "FIRTest",
    "sysFIRTest.out.expected",
    FIR_FIXTURES,
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled
);

const PROPERTIES_DIR: &str = "testsuite/bsc.assertions/properties";
pub(super) const PROPERTIES_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.assertions/properties::SemanticsTest::bluesim",
    PROPERTIES_DIR,
    "SemanticsTest",
    "sysSemanticsTest.out.expected"
);
pub(super) const PROPERTIES_ICARUS: SimulationCase = simulation_case!(
    "bsc.assertions/properties::SemanticsTest::icarus",
    PROPERTIES_DIR,
    "SemanticsTest",
    "sysSemanticsTest.out.expected",
    &["SemanticsTest.bsv", "sysSemanticsTest.out.expected"],
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled,
    std::time::Duration::from_secs(600),
    true
);

const HAMMING_DIR: &str = "testsuite/bsc.bsv_examples/Hamming";
pub(super) const HAMMING_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bsv_examples/Hamming::Hamming::bluesim",
    HAMMING_DIR,
    "Hamming",
    "sysHamming.out.expected"
);
pub(super) const HAMMING_ICARUS: SimulationCase = icarus_case!(
    "bsc.bsv_examples/Hamming::Hamming::icarus",
    HAMMING_DIR,
    "Hamming",
    "sysHamming.out.expected"
);

const BRAM_DIR: &str = "testsuite/bsc.lib/BRAM/BRAMTest";
macro_rules! bram_case {
    ($name:expr, $module:expr, $expected:expr, $backend:expr, $requirement:expr) => {
        bram_case!(
            $name,
            $module,
            $expected,
            $backend,
            $requirement,
            $crate::BSC_TIMEOUT,
            false
        )
    };
    ($name:expr, $module:expr, $expected:expr, $backend:expr, $requirement:expr, $timeout:expr, $heavy:expr) => {
        simulation_case!(
            $name,
            BRAM_DIR,
            $module,
            $expected,
            &[concat!($module, ".bsv"), $expected, "bram2.txt"],
            $backend,
            $requirement,
            $timeout,
            $heavy
        )
    };
}

pub(super) const BRAM_BLUESIM: SimulationCase = bram_case!(
    "bsc.lib/BRAM/BRAMTest::BRAMTest::bluesim",
    "BRAMTest",
    "sysBRAMTest.c.out.expected",
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const BRAM_ICARUS: SimulationCase = bram_case!(
    "bsc.lib/BRAM/BRAMTest::BRAMTest::icarus",
    "BRAMTest",
    "sysBRAMTest.v.out.expected",
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled
);
pub(super) const BRAM_1_BLUESIM: SimulationCase = bram_case!(
    "bsc.lib/BRAM/BRAMTest::BRAM1Test::bluesim",
    "BRAM1Test",
    "sysBRAM1Test.c.out.expected",
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const BRAM_1_ICARUS: SimulationCase = bram_case!(
    "bsc.lib/BRAM/BRAMTest::BRAM1Test::icarus",
    "BRAM1Test",
    "sysBRAM1Test.v.out.expected",
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled,
    std::time::Duration::from_secs(600),
    true
);
pub(super) const BRAM_PIPELINED_BLUESIM: SimulationCase = bram_case!(
    "bsc.lib/BRAM/BRAMTest::BRAMPipelined::bluesim",
    "BRAMPipelined",
    "sysBRAMPipelined.c.out.expected",
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const BRAM_PIPELINED_ICARUS: SimulationCase = bram_case!(
    "bsc.lib/BRAM/BRAMTest::BRAMPipelined::icarus",
    "BRAMPipelined",
    "sysBRAMPipelined.v.out.expected",
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled
);
