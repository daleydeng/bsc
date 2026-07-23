//! Origin: `testsuite/bsc.names/portRenaming/moduleArgs/moduleArgs.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.names/portRenaming/moduleArgs";

macro_rules! contains {
    ($path:literal, $text:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Contains { text: $text },
        }
    };
}

macro_rules! absent {
    ($path:literal, $text:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::DoesNotContain { text: $text },
        }
    };
}

macro_rules! module_arg_case {
    ($constant:ident, $source:literal, $expectation:expr, [$($assertion:expr),* $(,)?]) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.names/portRenaming/moduleArgs::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[$($assertion),*],
            expectation: $expectation,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! pass {
    ($constant:ident, $source:literal, [$($assertion:expr),* $(,)?]) => {
        module_arg_case!($constant, $source, CompileExpectation::Pass, [$($assertion),*]);
    };
}

macro_rules! warning {
    ($constant:ident, $source:literal, $tag:literal, [$($assertion:expr),* $(,)?]) => {
        module_arg_case!(
            $constant,
            $source,
            CompileExpectation::PassWithDiagnostic {
                kind: DiagnosticKind::Warning,
                tag: $tag,
                count: 1,
            },
            [$($assertion),*]
        );
    };
}

macro_rules! fail {
    ($constant:ident, $source:literal, $tag:literal) => {
        module_arg_case!(
            $constant,
            $source,
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            []
        );
    };
}

pass!(
    CLOCK_PREFIX_1,
    "ClockPrefix1.bsv",
    [
        absent!("sysClockPrefix1.v", "input  CLK;"),
        contains!("sysClockPrefix1.v", "input  CK;"),
        contains!("sysClockPrefix1.v", "input  RST_N;"),
        absent!("sysClockPrefix1.v", "input  CLK_clk2;"),
        contains!("sysClockPrefix1.v", "input  CK_clk2;"),
    ]
);
pass!(
    GATE_PREFIX_1,
    "GatePrefix1.bsv",
    [
        contains!("sysGatePrefix1.v", "input  CLK;"),
        absent!("sysGatePrefix1.v", "input  CLK_GATE;"),
        contains!("sysGatePrefix1.v", "input  GATE;"),
        contains!("sysGatePrefix1.v", "input  RST_N;"),
        contains!("sysGatePrefix1.v", "input  CLK_clk2;"),
        absent!("sysGatePrefix1.v", "input  CLK_GATE_clk2;"),
        contains!("sysGatePrefix1.v", "input  GATE_clk2;"),
    ]
);
pass!(
    RESET_PREFIX_1,
    "ResetPrefix1.bsv",
    [
        contains!("sysResetPrefix1.v", "input  CLK;"),
        absent!("sysResetPrefix1.v", "input  RST_N;"),
        contains!("sysResetPrefix1.v", "input  RESET;"),
        contains!("sysResetPrefix1.v", "input  CLK_clk2;"),
        absent!("sysResetPrefix1.v", "input  RST_rst2;"),
        contains!("sysResetPrefix1.v", "input  RESET_rst2;"),
    ]
);
pass!(
    OSC_ATTRIBUTE,
    "OscAttribute.bsv",
    [
        absent!("sysOscAttribute.v", "input  CLK;"),
        contains!("sysOscAttribute.v", "input  CK;"),
        absent!("sysOscAttribute.v", "input  CLK_GATE;"),
        contains!("sysOscAttribute.v", "input  RST_N;"),
        absent!("sysOscAttribute.v", "input  CLK_clk2;"),
        absent!("sysOscAttribute.v", "input  CK_clk2;"),
        contains!("sysOscAttribute.v", "input  BCLK;"),
    ]
);
pass!(
    GATE_ATTRIBUTE_1,
    "GateAttribute1.bsv",
    [
        contains!("sysGateAttribute1.v", "input  CLK;"),
        absent!("sysGateAttribute1.v", "input  CLK_GATE;"),
        contains!("sysGateAttribute1.v", "input  RST_N;"),
        contains!("sysGateAttribute1.v", "input  CLK_clk2;"),
        contains!("sysGateAttribute1.v", "input  GATE2;"),
    ]
);
pass!(
    RESET_ATTRIBUTE,
    "ResetAttribute.bsv",
    [
        contains!("sysResetAttribute.v", "input  CLK;"),
        absent!("sysResetAttribute.v", "input  CLK_GATE;"),
        contains!("sysResetAttribute.v", "input  RST_N;"),
        contains!("sysResetAttribute.v", "input  CLK_clk2;"),
        absent!("sysResetAttribute.v", "input  CLK_GATE_clk2;"),
        absent!("sysResetAttribute.v", "input  RST_rst2;"),
        contains!("sysResetAttribute.v", "input  RST2;"),
    ]
);
pass!(
    GATE_ATTRIBUTE_2,
    "GateAttribute2.bsv",
    [
        contains!("sysGateAttribute2.v", "input  CLK;"),
        contains!("sysGateAttribute2.v", "input  CLK_GATE;"),
        contains!("sysGateAttribute2.v", "input  RST_N;"),
        contains!("sysGateAttribute2.v", "input  CLK_clk2;"),
        absent!("sysGateAttribute2.v", "input  CLK_GATE_clk2;"),
    ]
);
pass!(
    GATE_ATTRIBUTE_3,
    "GateAttribute3.bsv",
    [
        contains!("sysGateAttribute3.v", "input  CLK;"),
        contains!("sysGateAttribute3.v", "input  CLK_GATE;"),
        contains!("sysGateAttribute3.v", "input  RST_N;"),
        contains!("sysGateAttribute3.v", "input  CLK_clk2;"),
        absent!("sysGateAttribute3.v", "input  CLK_GATE_clk2;"),
    ]
);
pass!(
    PORT_ATTRIBUTE,
    "PortAttribute.bsv",
    [
        contains!("sysPortAttribute.v", "input  CLK;"),
        absent!("sysPortAttribute.v", "input  CLK_GATE;"),
        contains!("sysPortAttribute.v", "input  RST_N;"),
        contains!("sysPortAttribute.v", "input  CLK_clk2;"),
        absent!("sysPortAttribute.v", "input  CLK_GATE_clk2;"),
        absent!("sysPortAttribute.v", "input  bin;"),
        contains!("sysPortAttribute.v", "input  BOOL_IN;"),
    ]
);
pass!(
    INOUT_PORT_ATTRIBUTE,
    "InoutPortAttribute.bsv",
    [
        contains!("sysInoutPortAttribute.v", "input  CLK;"),
        contains!("sysInoutPortAttribute.v", "input  RST_N;"),
        contains!("sysInoutPortAttribute.v", "inout  p;"),
        absent!("sysInoutPortAttribute.v", "in_io"),
        contains!("sysInoutPortAttribute.v", ".p(p)"),
        contains!("sysInoutPortAttribute.v", ".out_io(p)"),
    ]
);
pass!(
    PARAM_ATTRIBUTE,
    "ParamAttribute.bsv",
    [
        contains!("sysParamAttribute.v", "parameter [0 : 0] BOOL_IN = 1'b0;"),
        absent!("sysParamAttribute.v", "parameter [0 : 0] bin = 1'b0;"),
        absent!("sysParamAttribute.v", "input  bin;"),
        absent!("sysParamAttribute.v", "input  BOOL_IN;"),
    ]
);
pass!(
    DEFAULT_CLOCK,
    "DefaultClock.bsv",
    [
        absent!("sysDefaultClock.v", "input  CLK;"),
        contains!("sysDefaultClock.v", "input  CLOCK;"),
        absent!("sysDefaultClock.v", "input  CLK_GATE;"),
        contains!("sysDefaultClock.v", "input  GATE;"),
        contains!("sysDefaultClock.v", "input  RST_N;"),
    ]
);
pass!(
    DEFAULT_RESET,
    "DefaultReset.bsv",
    [
        contains!("sysDefaultReset.v", "input  CLK;"),
        absent!("sysDefaultReset.v", "input  CLK_GATE;"),
        absent!("sysDefaultReset.v", "input  RST_N;"),
        contains!("sysDefaultReset.v", "input  RESET;"),
    ]
);
pass!(
    DEFAULT_GATE_1,
    "DefaultGate1.bsv",
    [
        contains!("sysDefaultGate1.v", "input  CLK;"),
        absent!("sysDefaultGate1.v", "input  CLK_GATE;"),
        contains!("sysDefaultGate1.v", "input  RST_N;"),
        contains!("sysDefaultGate1.v", "input  CLK_clk2;"),
        contains!("sysDefaultGate1.v", "input  CLK_GATE_clk2;"),
    ]
);
pass!(
    DEFAULT_GATE_2,
    "DefaultGate2.bsv",
    [
        contains!("sysDefaultGate2.v", "input  CLK;"),
        absent!("sysDefaultGate2.v", "input  CLK_GATE;"),
        contains!("sysDefaultGate2.v", "input  RST_N;"),
        contains!("sysDefaultGate2.v", "input  CLK_clk2;"),
        contains!("sysDefaultGate2.v", "input  CLK_GATE_clk2;"),
    ]
);
pass!(
    NO_DEFAULT_CLOCK,
    "NoDefaultClock.bsv",
    [
        absent!("sysNoDefaultClock.v", "input  CLK;"),
        contains!("sysNoDefaultClock.v", "input  RST_N;"),
    ]
);
pass!(
    NO_DEFAULT_RESET,
    "NoDefaultReset.bsv",
    [
        contains!("sysNoDefaultReset.v", "input  CLK;"),
        absent!("sysNoDefaultReset.v", "input  RST_N;"),
    ]
);
pass!(
    ORPHANED_GATE,
    "OrphanedGate.bsv",
    [
        absent!("sysOrphanedGate.v", "input  ;"),
        absent!("sysOrphanedGate.v", "input  CLK;"),
        absent!("sysOrphanedGate.v", "input  CLK_GATE;"),
        contains!("sysOrphanedGate.v", "input  RST_N;"),
        contains!("sysOrphanedGate.v", "input  CLK_clk2;"),
        contains!("sysOrphanedGate.v", "input  CLK_GATE_clk2;"),
    ]
);
pass!(
    EMPTY_PREFIX_OK,
    "EmptyPrefixOK.bsv",
    [
        absent!("sysEmptyPrefixOK.v", "input  ;"),
        absent!("sysEmptyPrefixOK.v", "input  CLK;"),
        contains!("sysEmptyPrefixOK.v", "input  CLOCK;"),
        contains!("sysEmptyPrefixOK.v", "input  RST_N;"),
        contains!("sysEmptyPrefixOK.v", "input  clk2;"),
    ]
);

warning!(DEPRECATED_WARNING, "DeprecatedWarning.bsv", "P0072", []);
fail!(EMPTY_CLOCK_PREFIX, "EmptyClockPrefix.bsv", "P0177");
fail!(EMPTY_GATE_PREFIX, "EmptyGatePrefix.bsv", "P0177");
fail!(EMPTY_RESET_PREFIX, "EmptyResetPrefix.bsv", "P0177");
fail!(BAD_ARG_NAME, "BadArgName.bsv", "P0182");
fail!(NAME_CLASH, "NameClash.bsv", "P0183");
fail!(NAME_CLASH_2, "NameClash2.bsv", "P0183");
fail!(NAME_CLASH_KEYWORD, "NameClashKeyword.bsv", "P0184");
fail!(CONFLICTING_GATES, "ConflictingGates.bsv", "P0178");
fail!(EMPTY_PORT_NAME, "EmptyPortName.bsv", "P0063");
fail!(EMPTY_PARAM_NAME, "EmptyParamName.bsv", "P0063");
warning!(
    UNUSED_PREFIX,
    "UnusedPrefix.bsv",
    "P0179",
    [
        absent!("sysUnusedPrefix.v", "input  CLK;"),
        absent!("sysUnusedPrefix.v", "input  CK;"),
        contains!("sysUnusedPrefix.v", "input  CLKA;"),
        contains!("sysUnusedPrefix.v", "input  RST_N;"),
        absent!("sysUnusedPrefix.v", "input  CLK_clk2;"),
        absent!("sysUnusedPrefix.v", "input  CK_clk2;"),
        absent!("sysUnusedPrefix.v", "input  clk2;"),
        contains!("sysUnusedPrefix.v", "input  CLKB;"),
    ]
);
warning!(
    UNUSED_PREFIX_2,
    "UnusedPrefix2.bsv",
    "P0179",
    [
        contains!("sysUnusedPrefix2.v", "input  CLK;"),
        absent!("sysUnusedPrefix2.v", "input  RST_N;"),
        absent!("sysUnusedPrefix2.v", "input  RESET;"),
        absent!("sysUnusedPrefix2.v", "input  CLK_clk2;"),
        contains!("sysUnusedPrefix2.v", "input  CLKB;"),
    ]
);
fail!(WRONG_ARG_TYPE, "WrongArgType.bsv", "P0181");
fail!(WRONG_ARG_TYPE_2, "WrongArgType2.bsv", "P0155");
fail!(WRONG_ARG_TYPE_3, "WrongArgType3.bsv", "P0181");
fail!(WRONG_ARG_TYPE_4, "WrongArgType4.bsv", "P0155");
warning!(
    UNGATE_ALL_CLOCKS,
    "UngateAllClocks.bsv",
    "P0180",
    [
        contains!("sysUngateAllClocks.v", "input  CLK;"),
        absent!("sysUngateAllClocks.v", "input  CLK_GATE;"),
        contains!("sysUngateAllClocks.v", "input  RST_N;"),
        contains!("sysUngateAllClocks.v", "input  CLK_clk2;"),
        absent!("sysUngateAllClocks.v", "input  CLK_GATE_clk2;"),
    ]
);
fail!(TEST_INHIGH, "TestInhigh.bsv", "G0063");
fail!(TEST_UNUSED, "TestUnused.bsv", "G0077");

pub(super) const CASES: &[CompileCase] = &[
    CLOCK_PREFIX_1,
    GATE_PREFIX_1,
    RESET_PREFIX_1,
    OSC_ATTRIBUTE,
    GATE_ATTRIBUTE_1,
    RESET_ATTRIBUTE,
    GATE_ATTRIBUTE_2,
    GATE_ATTRIBUTE_3,
    PORT_ATTRIBUTE,
    INOUT_PORT_ATTRIBUTE,
    PARAM_ATTRIBUTE,
    DEFAULT_CLOCK,
    DEFAULT_RESET,
    DEFAULT_GATE_1,
    DEFAULT_GATE_2,
    NO_DEFAULT_CLOCK,
    NO_DEFAULT_RESET,
    ORPHANED_GATE,
    EMPTY_PREFIX_OK,
    DEPRECATED_WARNING,
    EMPTY_CLOCK_PREFIX,
    EMPTY_GATE_PREFIX,
    EMPTY_RESET_PREFIX,
    BAD_ARG_NAME,
    NAME_CLASH,
    NAME_CLASH_2,
    NAME_CLASH_KEYWORD,
    CONFLICTING_GATES,
    EMPTY_PORT_NAME,
    EMPTY_PARAM_NAME,
    UNUSED_PREFIX,
    UNUSED_PREFIX_2,
    WRONG_ARG_TYPE,
    WRONG_ARG_TYPE_2,
    WRONG_ARG_TYPE_3,
    WRONG_ARG_TYPE_4,
    UNGATE_ALL_CLOCKS,
    TEST_INHIGH,
    TEST_UNUSED,
];
