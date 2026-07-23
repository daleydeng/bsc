//! Origin: `testsuite/bsc.typechecker/error_recovery/error_recovery.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, DiagnosticKind,
    GoldenExpectation, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/error_recovery";

macro_rules! fail {
    ($constant:ident, $source:literal, $tag:literal, $count:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.typechecker/error_recovery::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        };
    };
}

fail!(ACTIONS, "Actions.bsv", "T0080", 2);
fail!(EXPLICIT_ACTIONS, "ExplicitActions.bsv", "T0080", 2);
fail!(EXPLICIT_BOOL, "ExplicitBool.bsv", "T0031", 2);
fail!(EXPLICIT_RULES, "ExplicitRules.bsv", "T0080", 2);
fail!(RULES_1, "Rules1.bsv", "T0080", 1);
fail!(RULES_2, "Rules2.bsv", "T0020", 2);
fail!(TWO_RULES, "TwoRules.bsv", "T0080", 2);
fail!(TWO_ACTIONS, "TwoActions.bsv", "T0080", 2);
fail!(LIFE, "Life.bsv", "T0080", 11);

pub(super) const TRIM_SKIP: CompileCase = CompileCase {
    name: "bsc.typechecker/error_recovery::TrimSkip.bs",
    fixture_dir: FIXTURE_DIR,
    source: "TrimSkip.bs",
    fixtures: &["TrimSkip.bs", "TrimSkip.bs.bsc-out.expected"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "TrimSkip.bs.bsc-out.expected",
    }),
    options: &["-dinternal", "-trace-skip-trim"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const DEF_ERROR_RECOVERY: CompileCase = CompileCase {
    name: "bsc.typechecker/error_recovery::DefErrorRecovery.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "DefErrorRecovery.bsv",
    fixtures: &["DefErrorRecovery.bsv", "sysDefErrorRecovery.v.expected"],
    assertions: &[
        ArtifactAssertion::ParsesAsSystemVerilog {
            path: "sysDefErrorRecovery.v",
        },
        ArtifactAssertion::Matches {
            actual: "sysDefErrorRecovery.v",
            expected: "sysDefErrorRecovery.v.expected",
            normalization: ArtifactNormalization::Verilog,
        },
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0080",
        count: 1,
    },
    golden: None,
    options: &["-continue-after-errors"],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[
    ACTIONS,
    EXPLICIT_ACTIONS,
    EXPLICIT_BOOL,
    EXPLICIT_RULES,
    RULES_1,
    RULES_2,
    TWO_RULES,
    TWO_ACTIONS,
    LIFE,
    TRIM_SKIP,
    DEF_ERROR_RECOVERY,
];
