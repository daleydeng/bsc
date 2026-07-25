//! Origin: `testsuite/bsc.verilog/derived_bits/derived_bits.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
    TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.verilog/derived_bits";
const OPTIMIZE_UNDETERMINED: &[&str] = &["-opt-undetermined-vals"];

macro_rules! verilog_match {
    ($path:literal) => {
        ArtifactAssertion::Matches {
            actual: $path,
            expected: concat!($path, ".expected"),
            normalization: ArtifactNormalization::Verilog,
        }
    };
}

macro_rules! verilog_case {
    ($constant:ident, $source:literal, [$($fixture:expr),* $(,)?], [$($assertion:expr),* $(,)?], $options:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.verilog/derived_bits::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, $($fixture,)*],
            assertions: &[$($assertion,)*],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: $options,
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

verilog_case!(
    LEFT_BIG,
    "LeftBig.bs",
    ["mkLeftBigReg.v.expected"],
    [verilog_match!("mkLeftBigReg.v")],
    OPTIMIZE_UNDETERMINED
);
verilog_case!(
    RIGHT_BIG,
    "RightBig.bs",
    ["mkRightBigReg.v.expected"],
    [verilog_match!("mkRightBigReg.v")],
    OPTIMIZE_UNDETERMINED
);
verilog_case!(
    MAYBE,
    "Maybe.bs",
    ["mkMaybeReg.v.expected"],
    [verilog_match!("mkMaybeReg.v")],
    OPTIMIZE_UNDETERMINED
);

macro_rules! derived_case {
    ($constant:ident, $stem:literal, $source:literal) => {
        verilog_case!(
            $constant,
            $source,
            [
                concat!("mk", $stem, "Reg.v.expected"),
                concat!("mkMaybe", $stem, "Reg.v.expected"),
                concat!("mk", $stem, "Test.v.expected"),
            ],
            [
                ArtifactAssertion::Matches {
                    actual: concat!("mk", $stem, "Reg.v"),
                    expected: concat!("mk", $stem, "Reg.v.expected"),
                    normalization: ArtifactNormalization::Verilog,
                },
                ArtifactAssertion::Matches {
                    actual: concat!("mkMaybe", $stem, "Reg.v"),
                    expected: concat!("mkMaybe", $stem, "Reg.v.expected"),
                    normalization: ArtifactNormalization::Verilog,
                },
                ArtifactAssertion::Matches {
                    actual: concat!("mk", $stem, "Test.v"),
                    expected: concat!("mk", $stem, "Test.v.expected"),
                    normalization: ArtifactNormalization::Verilog,
                },
            ],
            OPTIMIZE_UNDETERMINED
        );
    };
}

derived_case!(ORIG, "Orig", "Orig.bs");
derived_case!(ALT_1, "Alt1", "Alt1.bs");
derived_case!(ALT_1A, "Alt1a", "Alt1a.bs");
derived_case!(ALT_2, "Alt2", "Alt2.bs");
derived_case!(ALT_3, "Alt3", "Alt3.bs");
derived_case!(ALT_4, "Alt4", "Alt4.bs");
derived_case!(ALT_5, "Alt5", "Alt5.bs");
derived_case!(ALT_6, "Alt6", "Alt6.bs");
derived_case!(C0, "C0", "C0.bs");
derived_case!(C1, "C1", "C1.bs");

verilog_case!(
    ENUMS,
    "Enums.bsv",
    [
        "mkEnumType1Reg.v.expected",
        "mkEnumType2Reg.v.expected",
        "mkEnumType3Reg.v.expected",
        "mkEnumType1Test.v.expected",
        "mkEnumType2Test.v.expected",
        "mkEnumType3Test.v.expected",
    ],
    [
        verilog_match!("mkEnumType1Reg.v"),
        verilog_match!("mkEnumType2Reg.v"),
        verilog_match!("mkEnumType3Reg.v"),
        verilog_match!("mkEnumType1Test.v"),
        verilog_match!("mkEnumType2Test.v"),
        verilog_match!("mkEnumType3Test.v"),
    ],
    &[]
);

verilog_case!(
    DECODER,
    "Decoder.bs",
    [],
    [
        ArtifactAssertion::Text {
            path: "mkDecoder.v",
            assertion: TextAssertion::DoesNotContain { text: "xxxx" },
        },
        ArtifactAssertion::Text {
            path: "mkDecoder.v",
            assertion: TextAssertion::DoesNotContain {
                text: "unspecified value",
            },
        },
        ArtifactAssertion::Text {
            path: "mkDecoder.v",
            assertion: TextAssertion::DoesNotContain { text: "?" },
        },
        ArtifactAssertion::Text {
            path: "mkDecoder.v",
            assertion: TextAssertion::DoesNotContain { text: "case" },
        },
    ],
    &["-opt-undetermined-vals", "-unspecified-to", "x"]
);

macro_rules! noop_case {
    ($constant:ident, $source:literal, $output:literal, $operator:literal) => {
        verilog_case!(
            $constant,
            $source,
            [],
            [
                ArtifactAssertion::Text {
                    path: $output,
                    assertion: TextAssertion::LineCount {
                        text: $operator,
                        count: 0,
                    },
                },
                ArtifactAssertion::Text {
                    path: $output,
                    assertion: TextAssertion::Regex {
                        pattern: r"assign _read = r ;",
                    },
                },
                ArtifactAssertion::Text {
                    path: $output,
                    assertion: TextAssertion::Regex {
                        pattern: r"assign r\$D_IN = _write_1 ;",
                    },
                },
            ],
            OPTIMIZE_UNDETERMINED
        );
    };
}

noop_case!(NOOP_CASE, "TestNoopCase.bs", "mkTestNoopCase.v", "case");
noop_case!(
    NOOP_TERNARY,
    "TestNoopTernary.bs",
    "mkTestNoopTernary.v",
    "?"
);

pub(super) const CASES: &[CompileCase] = &[
    LEFT_BIG,
    RIGHT_BIG,
    MAYBE,
    ORIG,
    ALT_1,
    ALT_1A,
    ALT_2,
    ALT_3,
    ALT_4,
    ALT_5,
    ALT_6,
    C0,
    C1,
    ENUMS,
    DECODER,
    NOOP_CASE,
    NOOP_TERNARY,
];
