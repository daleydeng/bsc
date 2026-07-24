//! Origins:
//! - `testsuite/bsc.bsv_examples/FIRFilter/firfilter.exp`
//! - `testsuite/bsc.assertions/properties/properties.exp`
//! - `testsuite/bsc.bsv_examples/Hamming/hamming.exp`
//! - `testsuite/bsc.lib/BRAM/BRAMTest/BRAMTest.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario {
    (
        $constant:ident,
        $name:literal,
        $fixture_dir:expr,
        $module:literal,
        $expected:literal,
        $fixtures:expr,
        $timeout:expr,
        $resource:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform($timeout),
            resource: $resource,
            contracts: &[
                SimulationContract {
                    name: concat!($name, "::bluesim"),
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
                    name: concat!($name, "::icarus"),
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

macro_rules! backend_scenario {
    (
        $constant:ident,
        $name_prefix:literal,
        $fixture_dir:expr,
        $module:literal,
        $expected:literal,
        $fixtures:expr,
        $backend:ident,
        $backend_name:literal,
        $vcd:expr,
        $requirement:expr,
        $timeout:expr,
        $resource:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($name_prefix, $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform($timeout),
            resource: $resource,
            contracts: &[SimulationContract {
                name: concat!($name_prefix, $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: $requirement,
            }],
        };
    };
}

const FIR_DIR: &str = "testsuite/bsc.bsv_examples/FIRFilter";
const FIR_FIXTURES: &[&str] = &[
    "FIRTest.bsv",
    "FIRMain.bsv",
    "SyncFIR.bsv",
    "sysFIRTest.out.expected",
];
shared_scenario!(
    FIR,
    "bsc.bsv_examples/FIRFilter::FIRTest",
    FIR_DIR,
    "FIRTest",
    "sysFIRTest.out.expected",
    FIR_FIXTURES,
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);

const PROPERTIES_DIR: &str = "testsuite/bsc.assertions/properties";
shared_scenario!(
    PROPERTIES,
    "bsc.assertions/properties::SemanticsTest",
    PROPERTIES_DIR,
    "SemanticsTest",
    "sysSemanticsTest.out.expected",
    &["SemanticsTest.bsv", "sysSemanticsTest.out.expected"],
    std::time::Duration::from_secs(600),
    ResourceClass::Heavy
);

const HAMMING_DIR: &str = "testsuite/bsc.bsv_examples/Hamming";
shared_scenario!(
    HAMMING,
    "bsc.bsv_examples/Hamming::Hamming",
    HAMMING_DIR,
    "Hamming",
    "sysHamming.out.expected",
    &["Hamming.bsv", "sysHamming.out.expected"],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);

const BRAM_DIR: &str = "testsuite/bsc.lib/BRAM/BRAMTest";
macro_rules! bram_scenario {
    (
        $constant:ident,
        $module:literal,
        $expected:literal,
        $backend:ident,
        $backend_name:literal,
        $vcd:expr,
        $requirement:expr,
        $timeout:expr,
        $resource:expr
    ) => {
        backend_scenario!(
            $constant,
            "bsc.lib/BRAM/BRAMTest::",
            BRAM_DIR,
            $module,
            $expected,
            &[concat!($module, ".bsv"), $expected, "bram2.txt"],
            $backend,
            $backend_name,
            $vcd,
            $requirement,
            $timeout,
            $resource
        );
    };
}

bram_scenario!(
    BRAM_BLUESIM,
    "BRAMTest",
    "sysBRAMTest.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled,
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
bram_scenario!(
    BRAM_ICARUS,
    "BRAMTest",
    "sysBRAMTest.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled,
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
bram_scenario!(
    BRAM_1_BLUESIM,
    "BRAM1Test",
    "sysBRAM1Test.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled,
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
bram_scenario!(
    BRAM_1_ICARUS,
    "BRAM1Test",
    "sysBRAM1Test.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled,
    std::time::Duration::from_secs(600),
    ResourceClass::Heavy
);
bram_scenario!(
    BRAM_PIPELINED_BLUESIM,
    "BRAMPipelined",
    "sysBRAMPipelined.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled,
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
bram_scenario!(
    BRAM_PIPELINED_ICARUS,
    "BRAMPipelined",
    "sysBRAMPipelined.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled,
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    FIR,
    PROPERTIES,
    HAMMING,
    BRAM_BLUESIM,
    BRAM_ICARUS,
    BRAM_1_BLUESIM,
    BRAM_1_ICARUS,
    BRAM_PIPELINED_BLUESIM,
    BRAM_PIPELINED_ICARUS,
];
