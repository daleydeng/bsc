//! Origin: `testsuite/bsc.lib/SShow/SShow.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/SShow";

pub(super) const TEST_SSHOW: CompileCase = CompileCase {
    name: "bsc.lib/SShow::TestSShow.bs",
    fixture_dir: FIXTURE_DIR,
    source: "TestSShow.bs",
    fixtures: &["TestSShow.bs", "sysTestSShow.out.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "sysTestSShow.out",
        expected: "sysTestSShow.out.expected",
        normalization: ArtifactNormalization::GoldenOutput,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("sysTestSShow"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[TEST_SSHOW];
