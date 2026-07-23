//! Origin: `testsuite/bsc.bugs/bluespec_inc/b378/b378.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b378";

macro_rules! generated_verilog_case {
    ($constant:ident, $source:literal, $module:literal, $expected:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.bugs/bluespec_inc/b378::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, $expected],
            assertions: &[ArtifactAssertion::Matches {
                actual: concat!($module, ".v"),
                expected: $expected,
                normalization: ArtifactNormalization::Verilog,
            }],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog {
                module: Some($module),
            },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

generated_verilog_case!(DISJOINT, "Disjoint.bsv", "mkTest", "mkTest.v.expected");
generated_verilog_case!(
    DISJOINT_CONFLICT,
    "DisjointConflict.bsv",
    "mkCTest",
    "mkCTest.v.expected"
);

pub(super) const CASES: &[CompileCase] = &[DISJOINT, DISJOINT_CONFLICT];
