//! Origin: `testsuite/bsc.misc/eq3/eq3.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, Requirement, TextAssertion,
};

pub(super) const CASES: &[CompileCase] = &[CompileCase {
    name: "bsc.misc/eq3::EQ3.bs::verilog",
    fixture_dir: "testsuite/bsc.misc/eq3",
    source: "EQ3.bs",
    fixtures: &["EQ3.bs"],
    assertions: &[
        ArtifactAssertion::Text {
            path: "mkEQ3.v",
            assertion: TextAssertion::LineCount {
                text: "===",
                count: 1,
            },
        },
        ArtifactAssertion::Text {
            path: "mkEQ3.v",
            assertion: TextAssertion::LineCount {
                text: "!==",
                count: 1,
            },
        },
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
}];
