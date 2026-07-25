//! Origin: `testsuite/bsc.bugs/bluespec_inc/b752/b752.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b752";

pub(super) const BUG_752: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b752::Bug752.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "Bug752.bsv",
    fixtures: &["Bug752.bsv"],
    assertions: &[ArtifactAssertion::Text {
        path: "Bug752.bsv.bsc-out",
        assertion: TextAssertion::LineCount {
            text: "RL_b, RL_c",
            count: 1,
        },
    }],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0030",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const BUG_752_2: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b752::Bug752-2.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "Bug752-2.bsv",
    fixtures: &["Bug752-2.bsv"],
    assertions: &[ArtifactAssertion::Text {
        path: "Bug752-2.bsv.bsc-out",
        assertion: TextAssertion::LineCount {
            text: "RL_e, RL_f, RL_g, RL_h",
            count: 1,
        },
    }],
    expectation: CompileExpectation::Fail,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[BUG_752, BUG_752_2];
