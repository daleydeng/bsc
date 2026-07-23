//! Origins:
//! - `testsuite/bsc.mcd/Synchronizers/synchronizers.exp`
//! - `testsuite/bsc.mcd/SyncReset/SyncReset.exp`
//! - `testsuite/bsc.lib/BuildVector/BuildVector.exp`
//! - `testsuite/bsc.lib/dreg/dreg.exp`
//! - `testsuite/bsc.lib/Memory/Memory.exp`
//! - `testsuite/bsc.lib/Printf/Printf.exp`
//! - `testsuite/bsc.lib/TreeMap/libtreemap.exp`
//! - `testsuite/bsc.lib/BRAM/SyncBRAMFIFO/SyncBRAMFIFO.exp`

use super::SimulationCase;

macro_rules! backend_pair {
    ($bluesim:ident, $icarus:ident, $prefix:literal, $fixture_dir:expr, $module:literal, $bluesim_expected:literal, $icarus_expected:literal) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!($prefix, "::", $module, "::bluesim"),
            $fixture_dir,
            $module,
            $bluesim_expected
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!($prefix, "::", $module, "::icarus"),
            $fixture_dir,
            $module,
            $icarus_expected
        );
    };
}

const SYNCHRONIZERS_DIR: &str = "testsuite/bsc.mcd/Synchronizers";
backend_pair!(
    SYNC_BIT_BLUESIM,
    SYNC_BIT_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncBitTest",
    "sysSyncBitTest.out.expected",
    "sysSyncBitTest.out.expected"
);
backend_pair!(
    SYNC_BIT_1_BLUESIM,
    SYNC_BIT_1_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncBit1Test",
    "sysSyncBit1Test.out.expected",
    "sysSyncBit1Test.out.expected"
);
backend_pair!(
    SYNC_BIT_05_BLUESIM,
    SYNC_BIT_05_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncBit05Test",
    "sysSyncBit05Test.out.expected",
    "sysSyncBit05Test.out.expected"
);
backend_pair!(
    SYNC_BIT_15_BLUESIM,
    SYNC_BIT_15_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncBit15Test",
    "sysSyncBit15Test.out.expected",
    "sysSyncBit15Test.out.expected"
);
backend_pair!(
    SYNC_PULSE_BLUESIM,
    SYNC_PULSE_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncPulseTest",
    "sysSyncPulseTest.out.expected",
    "sysSyncPulseTest.out.expected"
);
backend_pair!(
    SYNC_HANDSHAKE_BLUESIM,
    SYNC_HANDSHAKE_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncHandshakeTest",
    "sysSyncHandshakeTest.out.expected",
    "sysSyncHandshakeTest.out.expected"
);
backend_pair!(
    SYNC_REG_BLUESIM,
    SYNC_REG_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncRegTest",
    "sysSyncRegTest.out.expected",
    "sysSyncRegTest.out.expected"
);
backend_pair!(
    SYNC_HANDSHAKE_2_BLUESIM,
    SYNC_HANDSHAKE_2_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncHandshakeTest2",
    "sysSyncHandshakeTest2.c.out.expected",
    "sysSyncHandshakeTest2.out.expected"
);
backend_pair!(
    SYNC_REG_3_BLUESIM,
    SYNC_REG_3_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncRegTest3",
    "sysSyncRegTest3.c.out.expected",
    "sysSyncRegTest3.out.expected"
);
backend_pair!(
    SYNC_FIFO_BLUESIM,
    SYNC_FIFO_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncFIFOTest",
    "sysSyncFIFOTest.c.out.expected",
    "sysSyncFIFOTest.out.expected"
);
backend_pair!(
    SYNC_FIFO_1_BLUESIM,
    SYNC_FIFO_1_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncFIFOTest1",
    "sysSyncFIFOTest1.c.out.expected",
    "sysSyncFIFOTest1.out.expected"
);
backend_pair!(
    SYNC_FIFO_1A_BLUESIM,
    SYNC_FIFO_1A_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncFIFOTest1A",
    "sysSyncFIFOTest1A.c.out.expected",
    "sysSyncFIFOTest1A.out.expected"
);
backend_pair!(
    SYNC_FIFO_2_BLUESIM,
    SYNC_FIFO_2_ICARUS,
    "bsc.mcd/Synchronizers",
    SYNCHRONIZERS_DIR,
    "SyncFIFOTest2",
    "sysSyncFIFOTest2.c.out.expected",
    "sysSyncFIFOTest2.out.expected"
);

const SYNC_RESET_DIR: &str = "testsuite/bsc.mcd/SyncReset";
backend_pair!(
    RST_TEST_BLUESIM,
    RST_TEST_ICARUS,
    "bsc.mcd/SyncReset",
    SYNC_RESET_DIR,
    "RstTest",
    "sysRstTest.out.expected",
    "sysRstTest.v.out.expected"
);
backend_pair!(
    RST_TEST_V1_BLUESIM,
    RST_TEST_V1_ICARUS,
    "bsc.mcd/SyncReset",
    SYNC_RESET_DIR,
    "RstTest_V1",
    "sysRstTest_V1.out.expected",
    "sysRstTest_V1.v.out.expected"
);
backend_pair!(
    RST_TEST_V2_BLUESIM,
    RST_TEST_V2_ICARUS,
    "bsc.mcd/SyncReset",
    SYNC_RESET_DIR,
    "RstTest_V2",
    "sysRstTest_V2.out.expected",
    "sysRstTest_V2.v.out.expected"
);

const BUILD_VECTOR_DIR: &str = "testsuite/bsc.lib/BuildVector";
pub(super) const TEST_BUILD_VECTOR_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/BuildVector::TestBuildVector::icarus",
    BUILD_VECTOR_DIR,
    "TestBuildVector",
    "sysTestBuildVector.out.expected"
);

const DREG_DIR: &str = "testsuite/bsc.lib/dreg";
backend_pair!(
    DREG_TEST_0_BLUESIM,
    DREG_TEST_0_ICARUS,
    "bsc.lib/dreg",
    DREG_DIR,
    "DRegTest0",
    "sysDRegTest0.out.expected",
    "sysDRegTest0.v.out.expected"
);
backend_pair!(
    DREG_TEST_1_BLUESIM,
    DREG_TEST_1_ICARUS,
    "bsc.lib/dreg",
    DREG_DIR,
    "DRegTest1",
    "sysDRegTest1.out.expected",
    "sysDRegTest1.v.out.expected"
);

const MEMORY_DIR: &str = "testsuite/bsc.lib/Memory";
backend_pair!(
    MEMORY_TEST_BLUESIM,
    MEMORY_TEST_ICARUS,
    "bsc.lib/Memory",
    MEMORY_DIR,
    "MemoryTest",
    "sysMemoryTest.out.expected",
    "sysMemoryTest.out.expected"
);

const PRINTF_DIR: &str = "testsuite/bsc.lib/Printf";
pub(super) const PRINTF_TEST_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/Printf::PrintfTest::icarus",
    PRINTF_DIR,
    "PrintfTest",
    "sysPrintfTest.out.expected"
);

const TREE_MAP_DIR: &str = "testsuite/bsc.lib/TreeMap";
backend_pair!(
    TREE_MAP_LOOKUP_BLUESIM,
    TREE_MAP_LOOKUP_ICARUS,
    "bsc.lib/TreeMap",
    TREE_MAP_DIR,
    "TreeMapLookup",
    "sysTreeMapLookup.out.expected",
    "sysTreeMapLookup.out.expected"
);
backend_pair!(
    TREE_MAP_MEMBER_BLUESIM,
    TREE_MAP_MEMBER_ICARUS,
    "bsc.lib/TreeMap",
    TREE_MAP_DIR,
    "TreeMapMember",
    "sysTreeMapMember.out.expected",
    "sysTreeMapMember.out.expected"
);
backend_pair!(
    TREE_MAP_ORDER_BLUESIM,
    TREE_MAP_ORDER_ICARUS,
    "bsc.lib/TreeMap",
    TREE_MAP_DIR,
    "TreeMapOrder",
    "sysTreeMapOrder.out.expected",
    "sysTreeMapOrder.out.expected"
);
backend_pair!(
    TREE_MAP_INSERT_WITH_BLUESIM,
    TREE_MAP_INSERT_WITH_ICARUS,
    "bsc.lib/TreeMap",
    TREE_MAP_DIR,
    "TreeMapInsertWith",
    "sysTreeMapInsertWith.out.expected",
    "sysTreeMapInsertWith.out.expected"
);

const SYNC_BRAM_FIFO_DIR: &str = "testsuite/bsc.lib/BRAM/SyncBRAMFIFO";
backend_pair!(
    SYNC_BRAM_FIFO_TO_BLUESIM,
    SYNC_BRAM_FIFO_TO_ICARUS,
    "bsc.lib/BRAM/SyncBRAMFIFO",
    SYNC_BRAM_FIFO_DIR,
    "SyncBRAMFIFOToTest",
    "sysSyncBRAMFIFOToTest.c.out.expected",
    "sysSyncBRAMFIFOToTest.v.out.expected"
);
backend_pair!(
    SYNC_BRAM_FIFO_FROM_BLUESIM,
    SYNC_BRAM_FIFO_FROM_ICARUS,
    "bsc.lib/BRAM/SyncBRAMFIFO",
    SYNC_BRAM_FIFO_DIR,
    "SyncBRAMFIFOFromTest",
    "sysSyncBRAMFIFOFromTest.c.out.expected",
    "sysSyncBRAMFIFOFromTest.v.out.expected"
);
backend_pair!(
    SYNC_BRAM_FIFO_BLUESIM,
    SYNC_BRAM_FIFO_ICARUS,
    "bsc.lib/BRAM/SyncBRAMFIFO",
    SYNC_BRAM_FIFO_DIR,
    "SyncBRAMFIFOTest",
    "sysSyncBRAMFIFOTest.c.out.expected",
    "sysSyncBRAMFIFOTest.v.out.expected"
);

pub(super) const CASES: &[SimulationCase] = &[
    SYNC_BIT_BLUESIM,
    SYNC_BIT_ICARUS,
    SYNC_BIT_1_BLUESIM,
    SYNC_BIT_1_ICARUS,
    SYNC_BIT_05_BLUESIM,
    SYNC_BIT_05_ICARUS,
    SYNC_BIT_15_BLUESIM,
    SYNC_BIT_15_ICARUS,
    SYNC_PULSE_BLUESIM,
    SYNC_PULSE_ICARUS,
    SYNC_HANDSHAKE_BLUESIM,
    SYNC_HANDSHAKE_ICARUS,
    SYNC_REG_BLUESIM,
    SYNC_REG_ICARUS,
    SYNC_HANDSHAKE_2_BLUESIM,
    SYNC_HANDSHAKE_2_ICARUS,
    SYNC_REG_3_BLUESIM,
    SYNC_REG_3_ICARUS,
    SYNC_FIFO_BLUESIM,
    SYNC_FIFO_ICARUS,
    SYNC_FIFO_1_BLUESIM,
    SYNC_FIFO_1_ICARUS,
    SYNC_FIFO_1A_BLUESIM,
    SYNC_FIFO_1A_ICARUS,
    SYNC_FIFO_2_BLUESIM,
    SYNC_FIFO_2_ICARUS,
    RST_TEST_BLUESIM,
    RST_TEST_ICARUS,
    RST_TEST_V1_BLUESIM,
    RST_TEST_V1_ICARUS,
    RST_TEST_V2_BLUESIM,
    RST_TEST_V2_ICARUS,
    TEST_BUILD_VECTOR_ICARUS,
    DREG_TEST_0_BLUESIM,
    DREG_TEST_0_ICARUS,
    DREG_TEST_1_BLUESIM,
    DREG_TEST_1_ICARUS,
    MEMORY_TEST_BLUESIM,
    MEMORY_TEST_ICARUS,
    PRINTF_TEST_ICARUS,
    TREE_MAP_LOOKUP_BLUESIM,
    TREE_MAP_LOOKUP_ICARUS,
    TREE_MAP_MEMBER_BLUESIM,
    TREE_MAP_MEMBER_ICARUS,
    TREE_MAP_ORDER_BLUESIM,
    TREE_MAP_ORDER_ICARUS,
    TREE_MAP_INSERT_WITH_BLUESIM,
    TREE_MAP_INSERT_WITH_ICARUS,
    SYNC_BRAM_FIFO_TO_BLUESIM,
    SYNC_BRAM_FIFO_TO_ICARUS,
    SYNC_BRAM_FIFO_FROM_BLUESIM,
    SYNC_BRAM_FIFO_FROM_ICARUS,
    SYNC_BRAM_FIFO_BLUESIM,
    SYNC_BRAM_FIFO_ICARUS,
];
