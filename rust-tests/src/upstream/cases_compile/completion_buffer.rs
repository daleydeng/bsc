//! Origin: `testsuite/bsc.lib/CompletionBuffer/CompletionBuffer.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/CompletionBuffer";

pub(super) const SCHEDULE: CompileCase = CompileCase {
    name: "bsc.lib/CompletionBuffer::TestCompletionBufferSchedule.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "TestCompletionBufferSchedule.bsv",
    fixtures: &[
        "TestCompletionBufferSchedule.bsv",
        "mkCompletionBuffer_4_Bit32.sched.expected",
    ],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkCompletionBuffer_4_Bit32.sched",
        expected: "mkCompletionBuffer_4_Bit32.sched.expected",
        normalization: ArtifactNormalization::GoldenOutput,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::VerilogSchedule { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[SCHEDULE];
