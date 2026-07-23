//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1043/b1043.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1213/b1213.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1235/b1235.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1265/b1265.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1267/b1267.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1332/b1332.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1356/b1356.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1389/b1389.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1396/b1396.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b265/b265.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b290/b290.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b308/b308.exp`

use crate::upstream::{CompileCase, CompileExpectation, CompileMode, Requirement};

pub(super) const B1043: CompileCase = compile_verilog_fail_error_case!(
    "bsc.bugs/bluespec_inc/b1043::PrimitiveBVI_BadPortName.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1043",
    "PrimitiveBVI_BadPortName.bsv",
    "G0124"
);

pub(super) const B1213: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1213::Example.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1213",
    source: "Example.bsv",
    fixtures: &["Example.bsv", "Zaz.bsv", "XReg.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B1235: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1235::HasSizeTest.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1235",
    source: "HasSizeTest.bsv",
    fixtures: &["HasSizeTest.bsv", "HasSize.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B1265: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1265::Test2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1265",
    "Test2.bsv"
);

pub(super) const B1267: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1267::VectorBug.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1267",
    "VectorBug.bsv"
);

pub(super) const B1332: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1332::Bug1332.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1332",
    "Bug1332.bsv"
);

pub(super) const B1356: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1356::Bug.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1356",
    source: "Bug.bsv",
    fixtures: &["Bug.bsv", "BugFn.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B1389: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1389::Test0.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1389",
    "Test0.bsv"
);

pub(super) const B1396: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1396::Example.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1396",
    source: "Example.bsv",
    fixtures: &["Example.bsv", "TLM.bsv", "TLMDefines.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B265: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b265::Design_1.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b265",
    source: "Design_1.bsv",
    fixtures: &["Design_1.bsv", "Design_0.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B290: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b290::Bug290.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b290",
    "Bug290.bsv"
);

pub(super) const B308: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b308::Bug308.bs",
    "testsuite/bsc.bugs/bluespec_inc/b308",
    "Bug308.bs"
);

pub(super) const CASES: &[CompileCase] = &[
    B1043,
    B1213,
    B1235,
    B1265,
    B1267,
    B1332,
    B1356,
    B1389,
    B1396,
    B265,
    B290,
    B308,
];
