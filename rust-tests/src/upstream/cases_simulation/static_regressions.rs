//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1302/b1302.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1314/b1314.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1353/b1353.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b431/b431.exp`
//! - `testsuite/bsc.bsv_examples/Misc/example_misc.exp`
//! - `testsuite/bsc.bsv_examples/stepcounter/stepcounter.exp`
//! - `testsuite/bsc.evaluator/prims/isancestor/isancestor.exp`
//! - `testsuite/bsc.lib/ClientServer/ClientServer.exp`
//! - `testsuite/bsc.lib/fork/fork.exp`
//! - `testsuite/bsc.lib/list_ops/list_ops.exp`
//! - `testsuite/bsc.lib/RegA/rega.exp`
//! - `testsuite/bsc.lib/regtwo/regtwo.exp`
//! - `testsuite/bsc.lib/Reserved/Reserved.exp`
//! - `testsuite/bsc.lib/Stmt/Modules/Modules.exp`
//! - `testsuite/bsc.lib/Tieoff/Tieoff.exp`
//! - `testsuite/bsc.misc/crc/crc.exp`
//! - `testsuite/bsc.typechecker/reflect/reflect.exp`
//! - `testsuite/bsc.evaluator/prims/build_module/build_module.exp`
//! - `testsuite/bsc.lib/Complex/Complex.exp`
//! - `testsuite/bsc.bsv_examples/xbar/xbar.exp`

use super::SimulationCase;
use crate::upstream::{Requirement, SimulationBackend};

macro_rules! simulation_case_with_fixtures {
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr, $fixtures:expr, $backend:expr, $requirement:expr) => {
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
            timeout: crate::BSC_TIMEOUT,
            heavy: false,
        }
    };
}

const B1302_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b1302";
const B1302_FIXTURES: &[&str] = &[
    "RFile2.bsv",
    "EHR2.bsv",
    "EHR_new.bsv",
    "sysRFile2.out.expected",
];
pub(super) const B1302_RFILE2_BLUESIM: SimulationCase = simulation_case_with_fixtures!(
    "bsc.bugs/bluespec_inc/b1302::RFile2::bluesim",
    B1302_DIR,
    "RFile2",
    "sysRFile2.out.expected",
    B1302_FIXTURES,
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const B1302_RFILE2_ICARUS: SimulationCase = simulation_case_with_fixtures!(
    "bsc.bugs/bluespec_inc/b1302::RFile2::icarus",
    B1302_DIR,
    "RFile2",
    "sysRFile2.out.expected",
    B1302_FIXTURES,
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled
);

const B1314_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b1314";
pub(super) const B1314_TEST_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bugs/bluespec_inc/b1314::Test::bluesim",
    B1314_DIR,
    "Test",
    "sysTest.out.expected"
);
pub(super) const B1314_TEST_ICARUS: SimulationCase = icarus_case!(
    "bsc.bugs/bluespec_inc/b1314::Test::icarus",
    B1314_DIR,
    "Test",
    "sysTest.out.expected"
);

const B1353_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b1353";
pub(super) const B1353_BUG1353_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bugs/bluespec_inc/b1353::Bug1353::bluesim",
    B1353_DIR,
    "Bug1353",
    "sysBug1353.out.expected"
);
pub(super) const B1353_BUG1353_ICARUS: SimulationCase = icarus_case!(
    "bsc.bugs/bluespec_inc/b1353::Bug1353::icarus",
    B1353_DIR,
    "Bug1353",
    "sysBug1353.out.expected"
);

const B431_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b431";
pub(super) const B431_BUG431_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bugs/bluespec_inc/b431::Bug431::bluesim",
    B431_DIR,
    "Bug431",
    "sysBug431.out.expected"
);
pub(super) const B431_BUG431_ICARUS: SimulationCase = icarus_case!(
    "bsc.bugs/bluespec_inc/b431::Bug431::icarus",
    B431_DIR,
    "Bug431",
    "sysBug431.out.expected"
);

const MISC_DIR: &str = "testsuite/bsc.bsv_examples/Misc";
pub(super) const MISC_TEST_SHIFTER64_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bsv_examples/Misc::TestShifter64::bluesim",
    MISC_DIR,
    "TestShifter64",
    "sysTestShifter64.out.expected"
);
pub(super) const MISC_TEST_SHIFTER64_ICARUS: SimulationCase = icarus_case!(
    "bsc.bsv_examples/Misc::TestShifter64::icarus",
    MISC_DIR,
    "TestShifter64",
    "sysTestShifter64.out.expected"
);

const STEP_COUNTER_DIR: &str = "testsuite/bsc.bsv_examples/stepcounter";
const STEP_COUNTER_FIXTURES: &[&str] = &[
    "TestStepCounter.bsv",
    "StepCounter.bsv",
    "sysTestStepCounter.out.expected",
];
pub(super) const STEP_COUNTER_BLUESIM: SimulationCase = simulation_case_with_fixtures!(
    "bsc.bsv_examples/stepcounter::TestStepCounter::bluesim",
    STEP_COUNTER_DIR,
    "TestStepCounter",
    "sysTestStepCounter.out.expected",
    STEP_COUNTER_FIXTURES,
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const STEP_COUNTER_ICARUS: SimulationCase = simulation_case_with_fixtures!(
    "bsc.bsv_examples/stepcounter::TestStepCounter::icarus",
    STEP_COUNTER_DIR,
    "TestStepCounter",
    "sysTestStepCounter.out.expected",
    STEP_COUNTER_FIXTURES,
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled
);

const IS_ANCESTOR_DIR: &str = "testsuite/bsc.evaluator/prims/isancestor";
pub(super) const IS_ANCESTOR_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.evaluator/prims/isancestor::IsAncestor::bluesim",
    IS_ANCESTOR_DIR,
    "IsAncestor",
    "sysIsAncestor.out.expected"
);
pub(super) const IS_ANCESTOR_ICARUS: SimulationCase = icarus_case!(
    "bsc.evaluator/prims/isancestor::IsAncestor::icarus",
    IS_ANCESTOR_DIR,
    "IsAncestor",
    "sysIsAncestor.out.expected"
);

const CLIENT_SERVER_DIR: &str = "testsuite/bsc.lib/ClientServer";
pub(super) const CLIENT_SERVER_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/ClientServer::TestToGPClientServer::bluesim",
    CLIENT_SERVER_DIR,
    "TestToGPClientServer",
    "sysTestToGPClientServer.out.expected"
);
pub(super) const CLIENT_SERVER_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/ClientServer::TestToGPClientServer::icarus",
    CLIENT_SERVER_DIR,
    "TestToGPClientServer",
    "sysTestToGPClientServer.out.expected"
);

const FORK_DIR: &str = "testsuite/bsc.lib/fork";
pub(super) const FORK_TEST_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/fork::ForkTest::icarus",
    FORK_DIR,
    "ForkTest",
    "sysForkTest.out.expected"
);

const LIST_OPS_DIR: &str = "testsuite/bsc.lib/list_ops";
pub(super) const SORT_GROUP_TEST_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/list_ops::SortGroupTest::bluesim",
    LIST_OPS_DIR,
    "SortGroupTest",
    "sysSortGroupTest.out.expected"
);

const REG_A_DIR: &str = "testsuite/bsc.lib/RegA";
pub(super) const REG_A_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/RegA::TestRegA::bluesim",
    REG_A_DIR,
    "TestRegA",
    "sysTestRegA.out.expected"
);
pub(super) const REG_A_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/RegA::TestRegA::icarus",
    REG_A_DIR,
    "TestRegA",
    "sysTestRegA.out.expected"
);

const REG_TWO_DIR: &str = "testsuite/bsc.lib/regtwo";
pub(super) const REG_TWO_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/regtwo::RegTwoTest::bluesim",
    REG_TWO_DIR,
    "RegTwoTest",
    "sysRegTwoTest.out.expected"
);
pub(super) const REG_TWO_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/regtwo::RegTwoTest::icarus",
    REG_TWO_DIR,
    "RegTwoTest",
    "sysRegTwoTest.out.expected"
);

const RESERVED_DIR: &str = "testsuite/bsc.lib/Reserved";
pub(super) const RESERVED_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/Reserved::ReservedTest::bluesim",
    RESERVED_DIR,
    "ReservedTest",
    "sysReservedTest.out.expected"
);
pub(super) const RESERVED_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/Reserved::ReservedTest::icarus",
    RESERVED_DIR,
    "ReservedTest",
    "sysReservedTest.out.expected"
);

const STMT_MODULES_DIR: &str = "testsuite/bsc.lib/Stmt/Modules";
pub(super) const ALWAYS_FSM_ONE_ACTION_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/Stmt/Modules::AlwaysFSM_OneAction::bluesim",
    STMT_MODULES_DIR,
    "AlwaysFSM_OneAction",
    "sysAlwaysFSM_OneAction.out.expected"
);
pub(super) const ALWAYS_FSM_ONE_ACTION_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/Stmt/Modules::AlwaysFSM_OneAction::icarus",
    STMT_MODULES_DIR,
    "AlwaysFSM_OneAction",
    "sysAlwaysFSM_OneAction.out.expected"
);

const TIE_OFF_DIR: &str = "testsuite/bsc.lib/Tieoff";
pub(super) const TIE_OFF_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/Tieoff::TieOffTest::bluesim",
    TIE_OFF_DIR,
    "TieOffTest",
    "sysTieOffTest.out.expected"
);
pub(super) const TIE_OFF_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/Tieoff::TieOffTest::icarus",
    TIE_OFF_DIR,
    "TieOffTest",
    "sysTieOffTest.out.expected"
);

const CRC_DIR: &str = "testsuite/bsc.misc/crc";
pub(super) const CRC_TEST_1_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.misc/crc::CRCTest1::bluesim",
    CRC_DIR,
    "CRCTest1",
    "sysCRCTest1.out.expected"
);
pub(super) const CRC_TEST_1_ICARUS: SimulationCase = icarus_case!(
    "bsc.misc/crc::CRCTest1::icarus",
    CRC_DIR,
    "CRCTest1",
    "sysCRCTest1.out.expected"
);

const REFLECT_DIR: &str = "testsuite/bsc.typechecker/reflect";
pub(super) const TYPE_OF_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.typechecker/reflect::TypeOf::bluesim",
    REFLECT_DIR,
    "TypeOf",
    "sysTypeOf.out.expected"
);
pub(super) const TYPE_OF_ICARUS: SimulationCase = icarus_case!(
    "bsc.typechecker/reflect::TypeOf::icarus",
    REFLECT_DIR,
    "TypeOf",
    "sysTypeOf.out.expected"
);
pub(super) const TYPE_EQ_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.typechecker/reflect::TypeEQ::bluesim",
    REFLECT_DIR,
    "TypeEQ",
    "sysTypeEQ.out.expected"
);
pub(super) const TYPE_EQ_ICARUS: SimulationCase = icarus_case!(
    "bsc.typechecker/reflect::TypeEQ::icarus",
    REFLECT_DIR,
    "TypeEQ",
    "sysTypeEQ.out.expected"
);

const BUILD_MODULE_DIR: &str = "testsuite/bsc.evaluator/prims/build_module";
pub(super) const ROSE_TEST_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.evaluator/prims/build_module::RoseTest::bluesim",
    BUILD_MODULE_DIR,
    "RoseTest",
    "sysRoseTest.out.expected"
);
pub(super) const ROSE_TEST_ICARUS: SimulationCase = icarus_case!(
    "bsc.evaluator/prims/build_module::RoseTest::icarus",
    BUILD_MODULE_DIR,
    "RoseTest",
    "sysRoseTest.out.expected"
);
pub(super) const FSHOW_FIFO_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.evaluator/prims/build_module::FShowFIFO::bluesim",
    BUILD_MODULE_DIR,
    "FShowFIFO",
    "sysFShowFIFO.out.expected"
);
pub(super) const FSHOW_FIFO_ICARUS: SimulationCase = icarus_case!(
    "bsc.evaluator/prims/build_module::FShowFIFO::icarus",
    BUILD_MODULE_DIR,
    "FShowFIFO",
    "sysFShowFIFO.out.expected"
);

const COMPLEX_DIR: &str = "testsuite/bsc.lib/Complex";
pub(super) const CMPLX_TEST_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/Complex::CmplxTest::bluesim",
    COMPLEX_DIR,
    "CmplxTest",
    "sysCmplxTest.out.expected"
);
pub(super) const CMPLX_TEST_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/Complex::CmplxTest::icarus",
    COMPLEX_DIR,
    "CmplxTest",
    "sysCmplxTest.out.expected"
);
pub(super) const CMPLX_SAT_ADD_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.lib/Complex::CmplxSatAdd::bluesim",
    COMPLEX_DIR,
    "CmplxSatAdd",
    "sysCmplxSatAdd.out.expected"
);
pub(super) const CMPLX_SAT_ADD_ICARUS: SimulationCase = icarus_case!(
    "bsc.lib/Complex::CmplxSatAdd::icarus",
    COMPLEX_DIR,
    "CmplxSatAdd",
    "sysCmplxSatAdd.out.expected"
);

const XBAR_DIR: &str = "testsuite/bsc.bsv_examples/xbar";
const XBAR_FIXTURES: &[&str] = &["Tb.bsv", "XBar.bsv", "sysTb.out.expected"];
pub(super) const XBAR_TB_BLUESIM: SimulationCase = simulation_case_with_fixtures!(
    "bsc.bsv_examples/xbar::Tb::bluesim",
    XBAR_DIR,
    "Tb",
    "sysTb.out.expected",
    XBAR_FIXTURES,
    SimulationBackend::Bluesim,
    Requirement::BluesimEnabled
);
pub(super) const XBAR_TB_ICARUS: SimulationCase = simulation_case_with_fixtures!(
    "bsc.bsv_examples/xbar::Tb::icarus",
    XBAR_DIR,
    "Tb",
    "sysTb.out.expected",
    XBAR_FIXTURES,
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled
);

pub(super) const CASES: &[SimulationCase] = &[
    B1302_RFILE2_BLUESIM,
    B1302_RFILE2_ICARUS,
    B1314_TEST_BLUESIM,
    B1314_TEST_ICARUS,
    B1353_BUG1353_BLUESIM,
    B1353_BUG1353_ICARUS,
    B431_BUG431_BLUESIM,
    B431_BUG431_ICARUS,
    MISC_TEST_SHIFTER64_BLUESIM,
    MISC_TEST_SHIFTER64_ICARUS,
    STEP_COUNTER_BLUESIM,
    STEP_COUNTER_ICARUS,
    IS_ANCESTOR_BLUESIM,
    IS_ANCESTOR_ICARUS,
    CLIENT_SERVER_BLUESIM,
    CLIENT_SERVER_ICARUS,
    FORK_TEST_ICARUS,
    SORT_GROUP_TEST_BLUESIM,
    REG_A_BLUESIM,
    REG_A_ICARUS,
    REG_TWO_BLUESIM,
    REG_TWO_ICARUS,
    RESERVED_BLUESIM,
    RESERVED_ICARUS,
    ALWAYS_FSM_ONE_ACTION_BLUESIM,
    ALWAYS_FSM_ONE_ACTION_ICARUS,
    TIE_OFF_BLUESIM,
    TIE_OFF_ICARUS,
    CRC_TEST_1_BLUESIM,
    CRC_TEST_1_ICARUS,
    TYPE_OF_BLUESIM,
    TYPE_OF_ICARUS,
    TYPE_EQ_BLUESIM,
    TYPE_EQ_ICARUS,
    ROSE_TEST_BLUESIM,
    ROSE_TEST_ICARUS,
    FSHOW_FIFO_BLUESIM,
    FSHOW_FIFO_ICARUS,
    CMPLX_TEST_BLUESIM,
    CMPLX_TEST_ICARUS,
    CMPLX_SAT_ADD_BLUESIM,
    CMPLX_SAT_ADD_ICARUS,
    XBAR_TB_BLUESIM,
    XBAR_TB_ICARUS,
];
