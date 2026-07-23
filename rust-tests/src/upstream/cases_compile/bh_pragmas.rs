//! Origin: `testsuite/bsc.syntax/bh/bh_pragmas/bh_pragmas.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, DiagnosticKind,
    Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bh/bh_pragmas";

macro_rules! verilog_pass_with_golden {
    ($constant:ident, $source:literal, $actual:literal, $expected:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.syntax/bh/bh_pragmas::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, $expected],
            assertions: &[ArtifactAssertion::Matches {
                actual: $actual,
                expected: $expected,
                normalization: ArtifactNormalization::Verilog,
            }],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! verilog_error {
    ($constant:ident, $source:literal, $tag:literal, $count:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.syntax/bh/bh_pragmas::", $source),
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
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! verilog_error_with_text {
    ($constant:ident, $source:literal, $tag:literal, $text:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.syntax/bh/bh_pragmas::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[ArtifactAssertion::Text {
                path: concat!($source, ".bsc-out"),
                assertion: TextAssertion::LineCount {
                    text: $text,
                    count: 1,
                },
            }],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! frontend_error_with_text {
    ($constant:ident, $source:literal, $text:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.syntax/bh/bh_pragmas::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[ArtifactAssertion::Text {
                path: concat!($source, ".bsc-out"),
                assertion: TextAssertion::LineCount {
                    text: $text,
                    count: 1,
                },
            }],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: "P0156",
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        };
    };
}

verilog_pass_with_golden!(
    GATE_DEFAULT_CLOCK,
    "GateDefaultClock.bs",
    "sysGateDefaultClock.v",
    "sysGateDefaultClock.v.expected"
);
verilog_pass_with_golden!(
    GATE_EXPLICIT_CLOCK,
    "GateExplicitClock.bs",
    "sysGateExplicitClock.v",
    "sysGateExplicitClock.v.expected"
);
verilog_error_with_text!(GATE_UNKNOWN_CLOCK, "GateUnknownClock.bs", "P0182", "gated");
verilog_error!(NO_CLOCK_FAMILY, "NoClockFamily.bs", "G0007", 2);
verilog_pass_with_golden!(
    CLOCK_FAMILY,
    "ClockFamily.bs",
    "mkClockFamily.v",
    "mkClockFamily.v.expected"
);
verilog_error_with_text!(
    UNKNOWN_CLOCK_FAMILY,
    "UnknownClockFamily.bs",
    "P0182",
    "ungated"
);
verilog_pass_with_golden!(
    PREFIXES,
    "Prefixes.bs",
    "sysPrefixes.v",
    "sysPrefixes.v.expected"
);
frontend_error_with_text!(DOUBLE_CLOCK_PREFIX, "DoubleClockPrefix.bs", "clock_prefix");
frontend_error_with_text!(DOUBLE_GATE_PREFIX, "DoubleGatePrefix.bs", "gate_prefix");
frontend_error_with_text!(DOUBLE_RESET_PREFIX, "DoubleResetPrefix.bs", "reset_prefix");
verilog_pass_with_golden!(
    SYNTHESIZE,
    "Synthesize.bs",
    "mkSynthesize.v",
    "mkSynthesize.v.expected"
);

pub(super) const CASES: &[CompileCase] = &[
    GATE_DEFAULT_CLOCK,
    GATE_EXPLICIT_CLOCK,
    GATE_UNKNOWN_CLOCK,
    NO_CLOCK_FAMILY,
    CLOCK_FAMILY,
    UNKNOWN_CLOCK_FAMILY,
    PREFIXES,
    DOUBLE_CLOCK_PREFIX,
    DOUBLE_GATE_PREFIX,
    DOUBLE_RESET_PREFIX,
    SYNTHESIZE,
];
