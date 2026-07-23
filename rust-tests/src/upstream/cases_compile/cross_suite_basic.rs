//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b373/b373.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b461/b461.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b522/b522.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b561/b561.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b68/b68.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b610/b610.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b637/b637.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b719/b719.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b851/b851.exp`
//! - `testsuite/bsc.bugs/github/gh334/gh334.exp`
//! - `testsuite/bsc.bugs/github/gh839/gh839.exp`
//! - `testsuite/bsc.bugs/github/gh841/gh841.exp`
//! - `testsuite/bsc.bsv_examples/fifo/fifo_Lennart_RWire/fifo_Lennart_RWire.exp`
//! - `testsuite/bsc.bsv_examples/typeclasses/typeclasses.exp`
//! - `testsuite/bsc.evaluator/prims/static_eval/static_eval.exp`

use crate::upstream::{CompileCase, CompileExpectation, CompileMode, DiagnosticKind, Requirement};

pub(super) const B373: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b373::Temp.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b373",
    source: "Temp.bsv",
    fixtures: &["Temp.bsv", "Wallace.bs"],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "S0015",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B461: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b461::Bug461.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b461",
    "Bug461.bsv"
);

pub(super) const B522: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b522::Bug522_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b522",
    "Bug522_1.bsv"
);

pub(super) const B561: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b561::Bug561_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b561",
    "Bug561_1.bsv"
);

pub(super) const B68: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b68::Bug68.bs",
    "testsuite/bsc.bugs/bluespec_inc/b68",
    "Bug68.bs"
);

pub(super) const B610: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b610::Test20.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b610",
    "Test20.bsv"
);

pub(super) const B637: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b637::Bug637.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b637",
    "Bug637.bsv"
);

pub(super) const B719: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b719::Bug719.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b719",
    "Bug719.bsv"
);

pub(super) const B851: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b851::Bug851.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b851",
    "Bug851.bsv"
);

pub(super) const GH334: CompileCase = compile_pass_case!(
    "bsc.bugs/github/gh334::IPv4.bsv",
    "testsuite/bsc.bugs/github/gh334",
    "IPv4.bsv"
);

pub(super) const GH839: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/github/gh839::OneHotSelectZero.bs",
    "testsuite/bsc.bugs/github/gh839",
    "OneHotSelectZero.bs"
);

pub(super) const GH841: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/github/gh841::GH841.bs",
    "testsuite/bsc.bugs/github/gh841",
    "GH841.bs"
);

pub(super) const FIFO_LENNART_RWIRE: CompileCase = compile_pass_case!(
    "bsc.bsv_examples/fifo/fifo_Lennart_RWire::Fifo_Lennart_RWire.bsv",
    "testsuite/bsc.bsv_examples/fifo/fifo_Lennart_RWire",
    "Fifo_Lennart_RWire.bsv"
);

pub(super) const TYPECLASSES_BITWISE: CompileCase = compile_pass_case!(
    "bsc.bsv_examples/typeclasses::Bitwise.bsv",
    "testsuite/bsc.bsv_examples/typeclasses",
    "Bitwise.bsv"
);

pub(super) const STATIC_EVAL_SIGNED_COMPARE_INT0: CompileCase = compile_verilog_pass_case!(
    "bsc.evaluator/prims/static_eval::SignedCompare_Int0.bsv",
    "testsuite/bsc.evaluator/prims/static_eval",
    "SignedCompare_Int0.bsv"
);

pub(super) const CASES: &[CompileCase] = &[
    B373,
    B461,
    B522,
    B561,
    B68,
    B610,
    B637,
    B719,
    B851,
    GH334,
    GH839,
    GH841,
    FIFO_LENNART_RWIRE,
    TYPECLASSES_BITWISE,
    STATIC_EVAL_SIGNED_COMPARE_INT0,
];
