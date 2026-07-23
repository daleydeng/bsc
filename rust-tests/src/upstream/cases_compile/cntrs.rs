//! Origin: `testsuite/bsc.lib/Cntrs/Cntrs.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/Cntrs";

pub(super) const SCHEDULE: CompileCase = CompileCase {
    name: "bsc.lib/Cntrs::CntrSched.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "CntrSched.bsv",
    fixtures: &[
        "CntrSched.bsv",
        "sysCntrSched.v.expected",
        "sysCntrSched.sched.expected",
    ],
    assertions: &[
        ArtifactAssertion::ParsesAsSystemVerilog {
            path: "sysCntrSched.v",
        },
        ArtifactAssertion::Matches {
            actual: "sysCntrSched.v",
            expected: "sysCntrSched.v.expected",
            normalization: ArtifactNormalization::Verilog,
        },
        ArtifactAssertion::Matches {
            actual: "sysCntrSched.sched",
            expected: "sysCntrSched.sched.expected",
            normalization: ArtifactNormalization::GoldenOutput,
        },
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::VerilogSchedule { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const LIMITS: CompileCase = CompileCase {
    name: "bsc.lib/Cntrs::CntrsLimits.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "CntrsLimits.bsv",
    fixtures: &["CntrsLimits.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Fail,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[SCHEDULE, LIMITS];
