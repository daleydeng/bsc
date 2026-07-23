//! Origins:
//! - `testsuite/bsc.verilog/positivereset/SyncReset/SyncReset.exp`
//! - `testsuite/bsc.real/evaluator/undef/undef.exp`

use super::CompileCase;
use crate::upstream::{CompileExpectation, CompileMode, DiagnosticKind, Requirement};

const POSITIVE_RESET_OPTIONS: &[&str] = &["-reset-prefix", "RESET_P", "-D", "BSV_POSITIVE_RESET"];

pub(super) const POSITIVE_RESET_INVALID_ARGUMENTS: CompileCase = CompileCase {
    name: "bsc.verilog/positivereset/SyncReset::RstTest_E1.bsv",
    fixture_dir: "testsuite/bsc.verilog/positivereset/SyncReset",
    source: "RstTest_E1.bsv",
    fixtures: &["RstTest_E1.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0042",
        count: 1,
    },
    golden: None,
    options: POSITIVE_RESET_OPTIONS,
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const UNDEF_REAL_PRIMITIVE: CompileCase = CompileCase {
    name: "bsc.real/evaluator/undef::DontCareRealPrim.bsv",
    fixture_dir: "testsuite/bsc.real/evaluator/undef",
    source: "DontCareRealPrim.bsv",
    fixtures: &["DontCareRealPrim.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[POSITIVE_RESET_INVALID_ARGUMENTS, UNDEF_REAL_PRIMITIVE];
