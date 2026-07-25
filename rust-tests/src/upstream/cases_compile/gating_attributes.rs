//! Origin: `testsuite/bsc.mcd/Gating/attributes/attributes.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.mcd/Gating/attributes";

macro_rules! has_port {
    ($output:literal, $port:literal) => {
        ArtifactAssertion::Text {
            path: $output,
            assertion: TextAssertion::Regex {
                pattern: concat!("input  ", $port, ";"),
            },
        }
    };
}

macro_rules! lacks_port {
    ($output:literal, $port:literal) => {
        ArtifactAssertion::Text {
            path: $output,
            assertion: TextAssertion::RegexDoesNotMatch {
                pattern: concat!("input  ", $port, ";"),
            },
        }
    };
}

macro_rules! gating_pass {
    ($constant:ident, $source:literal, $module:literal, [$($assertion:expr),* $(,)?]) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.mcd/Gating/attributes::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[$($assertion,)*],
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

gating_pass!(
    GATE_ALL_CLOCKS,
    "GateAllClocks.bsv",
    "sysGateAllClocks",
    [
        has_port!("sysGateAllClocks.v", "CLK_GATE"),
        has_port!("sysGateAllClocks.v", "CLK_GATE_c1"),
        has_port!("sysGateAllClocks.v", "CLK_GATE_c2"),
        has_port!("sysGateAllClocks.v", "CLK_GATE_c3"),
    ]
);
gating_pass!(
    GATE_DEFAULT_CLOCK,
    "GateDefaultClock.bsv",
    "sysGateDefaultClock",
    [
        has_port!("sysGateDefaultClock.v", "CLK_GATE"),
        lacks_port!("sysGateDefaultClock.v", "CLK_GATE_c1"),
        lacks_port!("sysGateDefaultClock.v", "CLK_GATE_c2"),
        lacks_port!("sysGateDefaultClock.v", "CLK_GATE_c3"),
    ]
);
gating_pass!(
    GATE_INPUT_CLOCKS_1,
    "GateInputClocks1.bsv",
    "sysGateInputClocks1",
    [
        lacks_port!("sysGateInputClocks1.v", "CLK_GATE"),
        has_port!("sysGateInputClocks1.v", "CLK_GATE_c1"),
        lacks_port!("sysGateInputClocks1.v", "CLK_GATE_c2"),
        has_port!("sysGateInputClocks1.v", "CLK_GATE_c3"),
    ]
);
gating_pass!(
    GATE_INPUT_CLOCKS_2,
    "GateInputClocks2.bsv",
    "sysGateInputClocks2",
    [
        lacks_port!("sysGateInputClocks2.v", "CLK_GATE"),
        has_port!("sysGateInputClocks2.v", "CLK_GATE_c1"),
        has_port!("sysGateInputClocks2.v", "CLK_GATE_c2"),
        lacks_port!("sysGateInputClocks2.v", "CLK_GATE_c3"),
    ]
);

macro_rules! gating_fail {
    ($constant:ident, $source:literal, $module:literal, $tag:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.mcd/Gating/attributes::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
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

gating_fail!(
    EMPTY_CLOCK_LIST,
    "GateInputClocks3.bsv",
    "sysGateInputClocks3",
    "P0063"
);
gating_fail!(
    NON_CLOCK_NAME,
    "GateInputClocks4.bsv",
    "sysGateInputClocks4",
    "P0182"
);

pub(super) const CASES: &[CompileCase] = &[
    GATE_ALL_CLOCKS,
    GATE_DEFAULT_CLOCK,
    GATE_INPUT_CLOCKS_1,
    GATE_INPUT_CLOCKS_2,
    EMPTY_CLOCK_LIST,
    NON_CLOCK_NAME,
];
