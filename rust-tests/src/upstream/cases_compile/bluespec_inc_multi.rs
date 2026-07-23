//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1191/b1191.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1225/b1225.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1263/b1263.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1305/b1305.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1325/b1325.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1326/b1326.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1349/b1349.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1429/b1429.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1591/b1591.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1610/b1610.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1654/b1654.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1718/b1718.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b257/b257.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b263/b263.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b304/b304.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b316/b316.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b351/b351.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b391/b391.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b418/b418.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b446/b446.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b447/b447.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b453/b453.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b487/b487.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b491/b491.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b589/b589.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b633/b633.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b667/b667.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b676/b676.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b941/b941.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/ek/ek_bug.exp`

use crate::upstream::{
    CompileCase, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

pub(super) const B1191_DOOM: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1191::Doom.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1191",
    "Doom.bsv"
);
pub(super) const B1191_TEST: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1191::Test.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1191",
    "Test.bsv"
);
pub(super) const B1191_TEST2: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1191::Test2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1191",
    "Test2.bsv"
);
pub(super) const B1191_TEST3: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1191::Test3.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1191",
    "Test3.bsv"
);

pub(super) const B1225_BUG1225: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1225::Bug1225.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1225",
    source: "Bug1225.bsv",
    fixtures: &["Bug1225.bsv", "Bug1225.bsv.bsc-out.expected"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0030",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Bug1225.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B1263_BUG1263: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b1263::Bug1263.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1263",
    "Bug1263.bsv",
    "T0107"
);
pub(super) const B1263_BUG1263_2: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b1263::Bug1263_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1263",
    "Bug1263_2.bsv",
    "T0013"
);

pub(super) const B1305_ZOW: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1305::Zow.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1305",
    "Zow.bsv"
);
pub(super) const B1305_ZOW2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1305::Zow2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1305",
    "Zow2.bsv"
);

pub(super) const B1325_TEST5: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1325::Test5.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1325",
    "Test5.bsv"
);
pub(super) const B1325_TOP: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1325::top.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1325",
    source: "top.bsv",
    fixtures: &["top.bsv", "param_test_case_new2.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B1326_SAT_TEST: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1326::SatTest.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1326",
    source: "SatTest.bsv",
    fixtures: &["SatTest.bsv", "SatMath.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};
pub(super) const B1326_SIZEOF_UNIFY_WITH_BITS_PROVISO: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1326::SizeOf_UnifyWithBitsProviso.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1326",
    "SizeOf_UnifyWithBitsProviso.bsv"
);
pub(super) const B1326_SIZEOF_MISSING_BITS_PROVISO: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1326::SizeOf_MissingBitsProviso.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1326",
    source: "SizeOf_MissingBitsProviso.bsv",
    fixtures: &[
        "SizeOf_MissingBitsProviso.bsv",
        "SizeOf_MissingBitsProviso.bsv.bsc-out.expected",
    ],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0030",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "SizeOf_MissingBitsProviso.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B1349_TEST: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1349::Test.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1349",
    "Test.bsv"
);
pub(super) const B1349_TEST2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1349::Test2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1349",
    "Test2.bsv"
);

pub(super) const B1429_TEST1: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1429::Test1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1429",
    "Test1.bsv"
);
pub(super) const B1429_TEST2: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1429::Test2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1429",
    "Test2.bsv"
);

pub(super) const B1591_BUG1591: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1591::Bug1591.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1591",
    source: "Bug1591.bsv",
    fixtures: &["Bug1591.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-opt-undetermined-vals"],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("sysBug1591"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B1610_BOARD: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1610::Board.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1610",
    "Board.bsv"
);
pub(super) const B1610_TEST2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1610::Test2.bs",
    "testsuite/bsc.bugs/bluespec_inc/b1610",
    "Test2.bs"
);

pub(super) const B1654_BUG: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1654::Bug.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1654",
    "Bug.bsv"
);
pub(super) const B1654_NOT_BUG_VALUE_METHOD: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1654::NotBug_ValueMethod.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1654",
    "NotBug_ValueMethod.bsv"
);
pub(super) const B1654_NOT_BUG_INIT: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1654::NotBug_Init.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1654",
    "NotBug_Init.bsv"
);

pub(super) const B1718_STRUCT_EXPLICIT_READ: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1718::StructExplicitRead.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1718",
    "StructExplicitRead.bsv"
);
pub(super) const B1718_STRUCT_EXPLICIT_WRITE: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1718::StructExplicitWrite.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1718",
    "StructExplicitWrite.bsv"
);
pub(super) const B1718_STRUCT_IMPLICIT_READ: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b1718::StructImplicitRead.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1718",
    "StructImplicitRead.bsv",
    "T0020"
);
pub(super) const B1718_STRUCT_IMPLICIT_WRITE: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b1718::StructImplicitWrite.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1718",
    "StructImplicitWrite.bsv",
    "T0066"
);

pub(super) const B257_BUG257: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b257::Bug257.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b257",
    source: "Bug257.bsv",
    fixtures: &["Bug257.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Fail,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("G0027"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B263_BUG263_1: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b263::Bug263-1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b263",
    "Bug263-1.bsv"
);
pub(super) const B263_BUG263_2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b263::Bug263-2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b263",
    "Bug263-2.bsv"
);
pub(super) const B263_BUG263_3: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b263::Bug263-3.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b263",
    "Bug263-3.bsv"
);
pub(super) const B263_BUG263_4: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b263::Bug263-4.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b263",
    "Bug263-4.bsv"
);

pub(super) const B304_BUG304_1: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b304::Bug304_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b304",
    "Bug304_1.bsv",
    "T0011"
);
pub(super) const B304_BUG304_2: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b304::Bug304_2.bs",
    "testsuite/bsc.bugs/bluespec_inc/b304",
    "Bug304_2.bs",
    "T0011"
);

pub(super) const B316_PPC_BRANCH_PREDICTOR: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b316::PPC_BranchPredictor.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b316",
    source: "PPC_BranchPredictor.bsv",
    fixtures: &["PPC_BranchPredictor.bsv", "PPC_Datatypes.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B351_DESIGN: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b351::Design.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b351",
    source: "Design.bsv",
    fixtures: &["Design.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("mkDesign"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B391_BUG391_1: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b391::Bug391_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b391",
    "Bug391_1.bsv",
    "T0011"
);
pub(super) const B391_BUG391_2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b391::Bug391_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b391",
    "Bug391_2.bsv"
);
pub(super) const B391_BUG391_3: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b391::Bug391_3.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b391",
    "Bug391_3.bsv",
    "T0011"
);
pub(super) const B391_BUG391_4: CompileCase = compile_verilog_fail_error_case!(
    "bsc.bugs/bluespec_inc/b391::Bug391_4.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b391",
    "Bug391_4.bsv",
    "G0027"
);
pub(super) const B391_BUG391_5: CompileCase = compile_verilog_fail_error_case!(
    "bsc.bugs/bluespec_inc/b391::Bug391_5.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b391",
    "Bug391_5.bsv",
    "G0027"
);

pub(super) const B418_TESTER: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b418::Tester.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b418",
    source: "Tester.bsv",
    fixtures: &[
        "Tester.bsv",
        "FPAdd.bsv",
        "FPLibrary.bsv",
        "Tester.bsv.bsc-out.expected",
    ],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0007",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Tester.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B446_BUG446_1: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b446::Bug446_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b446",
    "Bug446_1.bsv"
);
pub(super) const B446_BUG446_2: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b446::Bug446_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b446",
    "Bug446_2.bsv",
    "T0011"
);

pub(super) const B447_BUG447_1: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b447::Bug447_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b447",
    "Bug447_1.bsv",
    "P0039"
);
pub(super) const B447_BUG447_2: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b447::Bug447_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b447",
    "Bug447_2.bsv",
    "P0104"
);
pub(super) const B447_BUG447_3: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b447::Bug447_3.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b447",
    "Bug447_3.bsv",
    "T0066"
);

pub(super) const B453_MK_BGPD: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b453::MkBGPd.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b453",
    "MkBGPd.bsv"
);
pub(super) const B453_MK_BGPT: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b453::MkBGPt.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b453",
    "MkBGPt.bsv"
);

pub(super) const B487_SIMPLE_CLIENT: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b487::SimpleClient.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b487",
    source: "SimpleClient.bsv",
    fixtures: &["SimpleClient.bsv", "ZBusBuffer.bsv", "ZBusUtil.bs"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["+RTS", "-K200k", "-RTS"],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B491_BUG491_1: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b491::Bug491_1.bs",
    "testsuite/bsc.bugs/bluespec_inc/b491",
    "Bug491_1.bs"
);
pub(super) const B491_BUG491_2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b491::Bug491_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b491",
    "Bug491_2.bsv"
);

pub(super) const B589_BUG589_1: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b589::Bug589_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b589",
    "Bug589_1.bsv"
);
pub(super) const B589_BUG589_2: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b589::Bug589_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b589",
    "Bug589_2.bsv"
);

pub(super) const B633_TEST_IVEC1: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b633::TestIVec1.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b633",
    source: "TestIVec1.bsv",
    fixtures: &["TestIVec1.bsv", "IVec1.bs"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B667_RWIRE_OUTPUT: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b667::RWireOutput.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b667",
    source: "RWireOutput.bsv",
    fixtures: &["RWireOutput.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-inline-rwire"],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B676_BUG676_1: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b676::Bug676_1.bs",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b676",
    source: "Bug676_1.bs",
    fixtures: &["Bug676_1.bs", "Bug676_1.bs.bsc-out.expected"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0106",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Bug676_1.bs.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B676_BUG676_2: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b676::Bug676_2.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b676",
    source: "Bug676_2.bsv",
    fixtures: &["Bug676_2.bsv", "Bug676_2.bsv.bsc-out.expected"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0106",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Bug676_2.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B676_SELF_RECURSIVE_SYN: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b676::SelfRecursiveSyn.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b676",
    source: "SelfRecursiveSyn.bsv",
    fixtures: &[
        "SelfRecursiveSyn.bsv",
        "SelfRecursiveSyn.bsv.bsc-out.expected",
    ],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0106",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "SelfRecursiveSyn.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B941_UNDERSCORE_IFC: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b941::UnderscoreIfc.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b941",
    "UnderscoreIfc.bsv"
);
pub(super) const B941_UNDERSCORE_MODULE: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b941::UnderscoreModule.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b941",
    "UnderscoreModule.bsv"
);

pub(super) const EK_PARITY_SWITCH2: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/ek::ParitySwitch2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/ek",
    "ParitySwitch2.bsv"
);

pub(super) const CASES: &[CompileCase] = &[
    B1191_DOOM,
    B1191_TEST,
    B1191_TEST2,
    B1191_TEST3,
    B1225_BUG1225,
    B1263_BUG1263,
    B1263_BUG1263_2,
    B1305_ZOW,
    B1305_ZOW2,
    B1325_TEST5,
    B1325_TOP,
    B1326_SAT_TEST,
    B1326_SIZEOF_UNIFY_WITH_BITS_PROVISO,
    B1326_SIZEOF_MISSING_BITS_PROVISO,
    B1349_TEST,
    B1349_TEST2,
    B1429_TEST1,
    B1429_TEST2,
    B1591_BUG1591,
    B1610_BOARD,
    B1610_TEST2,
    B1654_BUG,
    B1654_NOT_BUG_VALUE_METHOD,
    B1654_NOT_BUG_INIT,
    B1718_STRUCT_EXPLICIT_READ,
    B1718_STRUCT_EXPLICIT_WRITE,
    B1718_STRUCT_IMPLICIT_READ,
    B1718_STRUCT_IMPLICIT_WRITE,
    B257_BUG257,
    B263_BUG263_1,
    B263_BUG263_2,
    B263_BUG263_3,
    B263_BUG263_4,
    B304_BUG304_1,
    B304_BUG304_2,
    B316_PPC_BRANCH_PREDICTOR,
    B351_DESIGN,
    B391_BUG391_1,
    B391_BUG391_2,
    B391_BUG391_3,
    B391_BUG391_4,
    B391_BUG391_5,
    B418_TESTER,
    B446_BUG446_1,
    B446_BUG446_2,
    B447_BUG447_1,
    B447_BUG447_2,
    B447_BUG447_3,
    B453_MK_BGPD,
    B453_MK_BGPT,
    B487_SIMPLE_CLIENT,
    B491_BUG491_1,
    B491_BUG491_2,
    B589_BUG589_1,
    B589_BUG589_2,
    B633_TEST_IVEC1,
    B667_RWIRE_OUTPUT,
    B676_BUG676_1,
    B676_BUG676_2,
    B676_SELF_RECURSIVE_SYN,
    B941_UNDERSCORE_IFC,
    B941_UNDERSCORE_MODULE,
    EK_PARITY_SWITCH2,
];
