//! Origins:
//! - `testsuite/bsc.mcd/Synchronizers/synchronizers.exp`
//! - `testsuite/bsc.mcd/SyncReset/SyncReset.exp`
//! - `testsuite/bsc.lib/BuildVector/BuildVector.exp`
//! - `testsuite/bsc.lib/dreg/dreg.exp`
//! - `testsuite/bsc.lib/Memory/Memory.exp`
//! - `testsuite/bsc.lib/Printf/Printf.exp`
//! - `testsuite/bsc.lib/TreeMap/libtreemap.exp`
//! - `testsuite/bsc.lib/BRAM/SyncBRAMFIFO/SyncBRAMFIFO.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
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
        }
    };
}

macro_rules! backend_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal, $backend_name:literal, $backend:ident, $vcd:expr, $requirement:ident) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        }
    };
}

macro_rules! bluesim_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "bluesim",
            Bluesim,
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        )
    };
}

macro_rules! icarus_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "icarus",
            Icarus,
            Some(VcdContract::parse()),
            VerilogEnabled
        )
    };
}

const SYNCHRONIZERS_DIR: &str = "testsuite/bsc.mcd/Synchronizers";
const SYNC_RESET_DIR: &str = "testsuite/bsc.mcd/SyncReset";
const BUILD_VECTOR_DIR: &str = "testsuite/bsc.lib/BuildVector";
const DREG_DIR: &str = "testsuite/bsc.lib/dreg";
const MEMORY_DIR: &str = "testsuite/bsc.lib/Memory";
const PRINTF_DIR: &str = "testsuite/bsc.lib/Printf";
const TREE_MAP_DIR: &str = "testsuite/bsc.lib/TreeMap";
const SYNC_BRAM_FIFO_DIR: &str = "testsuite/bsc.lib/BRAM/SyncBRAMFIFO";

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    // testsuite/bsc.mcd/Synchronizers/synchronizers.exp
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncBitTest",
        "sysSyncBitTest.out.expected"
    ),
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncBit1Test",
        "sysSyncBit1Test.out.expected"
    ),
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncBit05Test",
        "sysSyncBit05Test.out.expected"
    ),
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncBit15Test",
        "sysSyncBit15Test.out.expected"
    ),
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncPulseTest",
        "sysSyncPulseTest.out.expected"
    ),
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncHandshakeTest",
        "sysSyncHandshakeTest.out.expected"
    ),
    shared_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncRegTest",
        "sysSyncRegTest.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncHandshakeTest2",
        "sysSyncHandshakeTest2.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncHandshakeTest2",
        "sysSyncHandshakeTest2.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncRegTest3",
        "sysSyncRegTest3.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncRegTest3",
        "sysSyncRegTest3.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest",
        "sysSyncFIFOTest.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest",
        "sysSyncFIFOTest.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest1",
        "sysSyncFIFOTest1.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest1",
        "sysSyncFIFOTest1.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest1A",
        "sysSyncFIFOTest1A.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest1A",
        "sysSyncFIFOTest1A.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest2",
        "sysSyncFIFOTest2.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/Synchronizers",
        SYNCHRONIZERS_DIR,
        "SyncFIFOTest2",
        "sysSyncFIFOTest2.out.expected"
    ),
    // testsuite/bsc.mcd/SyncReset/SyncReset.exp
    bluesim_scenario!(
        "bsc.mcd/SyncReset",
        SYNC_RESET_DIR,
        "RstTest",
        "sysRstTest.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/SyncReset",
        SYNC_RESET_DIR,
        "RstTest",
        "sysRstTest.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/SyncReset",
        SYNC_RESET_DIR,
        "RstTest_V1",
        "sysRstTest_V1.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/SyncReset",
        SYNC_RESET_DIR,
        "RstTest_V1",
        "sysRstTest_V1.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/SyncReset",
        SYNC_RESET_DIR,
        "RstTest_V2",
        "sysRstTest_V2.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/SyncReset",
        SYNC_RESET_DIR,
        "RstTest_V2",
        "sysRstTest_V2.v.out.expected"
    ),
    // testsuite/bsc.lib/BuildVector/BuildVector.exp
    icarus_scenario!(
        "bsc.lib/BuildVector",
        BUILD_VECTOR_DIR,
        "TestBuildVector",
        "sysTestBuildVector.out.expected"
    ),
    // testsuite/bsc.lib/dreg/dreg.exp
    bluesim_scenario!(
        "bsc.lib/dreg",
        DREG_DIR,
        "DRegTest0",
        "sysDRegTest0.out.expected"
    ),
    icarus_scenario!(
        "bsc.lib/dreg",
        DREG_DIR,
        "DRegTest0",
        "sysDRegTest0.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.lib/dreg",
        DREG_DIR,
        "DRegTest1",
        "sysDRegTest1.out.expected"
    ),
    icarus_scenario!(
        "bsc.lib/dreg",
        DREG_DIR,
        "DRegTest1",
        "sysDRegTest1.v.out.expected"
    ),
    // testsuite/bsc.lib/Memory/Memory.exp
    shared_scenario!(
        "bsc.lib/Memory",
        MEMORY_DIR,
        "MemoryTest",
        "sysMemoryTest.out.expected"
    ),
    // testsuite/bsc.lib/Printf/Printf.exp
    icarus_scenario!(
        "bsc.lib/Printf",
        PRINTF_DIR,
        "PrintfTest",
        "sysPrintfTest.out.expected"
    ),
    // testsuite/bsc.lib/TreeMap/libtreemap.exp
    shared_scenario!(
        "bsc.lib/TreeMap",
        TREE_MAP_DIR,
        "TreeMapLookup",
        "sysTreeMapLookup.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/TreeMap",
        TREE_MAP_DIR,
        "TreeMapMember",
        "sysTreeMapMember.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/TreeMap",
        TREE_MAP_DIR,
        "TreeMapOrder",
        "sysTreeMapOrder.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/TreeMap",
        TREE_MAP_DIR,
        "TreeMapInsertWith",
        "sysTreeMapInsertWith.out.expected"
    ),
    // testsuite/bsc.lib/BRAM/SyncBRAMFIFO/SyncBRAMFIFO.exp
    bluesim_scenario!(
        "bsc.lib/BRAM/SyncBRAMFIFO",
        SYNC_BRAM_FIFO_DIR,
        "SyncBRAMFIFOToTest",
        "sysSyncBRAMFIFOToTest.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.lib/BRAM/SyncBRAMFIFO",
        SYNC_BRAM_FIFO_DIR,
        "SyncBRAMFIFOToTest",
        "sysSyncBRAMFIFOToTest.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.lib/BRAM/SyncBRAMFIFO",
        SYNC_BRAM_FIFO_DIR,
        "SyncBRAMFIFOFromTest",
        "sysSyncBRAMFIFOFromTest.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.lib/BRAM/SyncBRAMFIFO",
        SYNC_BRAM_FIFO_DIR,
        "SyncBRAMFIFOFromTest",
        "sysSyncBRAMFIFOFromTest.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.lib/BRAM/SyncBRAMFIFO",
        SYNC_BRAM_FIFO_DIR,
        "SyncBRAMFIFOTest",
        "sysSyncBRAMFIFOTest.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.lib/BRAM/SyncBRAMFIFO",
        SYNC_BRAM_FIFO_DIR,
        "SyncBRAMFIFOTest",
        "sysSyncBRAMFIFOTest.v.out.expected"
    ),
];
