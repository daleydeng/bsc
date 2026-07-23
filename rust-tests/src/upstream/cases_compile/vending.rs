//! Origin: `testsuite/bsc.bsv_examples/vending/vending.exp`.

use super::CompileCase;
use crate::upstream::{CompileExpectation, CompileMode, DiagnosticKind, Requirement};

pub(super) const VENDING_ZERO_WARNINGS: CompileCase = CompileCase {
    name: "bsc.bsv_examples/vending::Vending0.bsv",
    fixture_dir: "testsuite/bsc.bsv_examples/vending",
    source: "Vending0.bsv",
    fixtures: &["Vending0.bsv", "VendingIfc.bsv"],
    assertions: &[],
    expectation: CompileExpectation::PassWithDiagnostic {
        kind: DiagnosticKind::Warning,
        tag: "G0010",
        count: 3,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[VENDING_ZERO_WARNINGS];
