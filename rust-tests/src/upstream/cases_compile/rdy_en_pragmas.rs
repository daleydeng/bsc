//! Origin: `testsuite/bsc.codegen/rdy_en_pragmas/rdy_en_pragmas.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.codegen/rdy_en_pragmas";
const UNSAFE_ALWAYS_READY: &[&str] = &["-unsafe-always-ready"];

macro_rules! text {
    ($path:literal, contains $value:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Contains { text: $value },
        }
    };
    ($path:literal, excludes $value:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::DoesNotContain { text: $value },
        }
    };
    ($path:literal, lines $value:literal, $count:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $value,
                count: $count,
            },
        }
    };
    ($path:literal, warning $tag:literal, $count:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::DiagnosticCount {
                kind: DiagnosticKind::Warning,
                tag: $tag,
                count: $count,
            },
        }
    };
}

macro_rules! verilog_case {
    ($source:literal, $expectation:expr, $options:expr, [$($assertion:expr),* $(,)?]) => {
        CompileCase {
            name: concat!("bsc.codegen/rdy_en_pragmas::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[$($assertion,)*],
            expectation: $expectation,
            golden: None,
            options: $options,
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        }
    };
}

macro_rules! pass {
    ($source:literal) => {
        verilog_case!($source, CompileExpectation::Pass, &[], [])
    };
}

pub(super) const CASES: &[CompileCase] = &[
    verilog_case!(
        "AlwaysEnabledNotOK.bsv",
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "G0006",
            count: 1,
        },
        &[],
        []
    ),
    verilog_case!(
        "AlwaysReadyNotOK.bsv",
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "G0006",
            count: 1,
        },
        &[],
        []
    ),
    verilog_case!(
        "TestEnableFail.bsv",
        CompileExpectation::Pass,
        &[],
        [text!(
            "TestEnableFail.bsv.bsc-out",
            warning "G0015",
            2
        )]
    ),
    verilog_case!(
        "Test_RdyEn_Path.bsv",
        CompileExpectation::Pass,
        UNSAFE_ALWAYS_READY,
        [text!("sysTest_RdyEn_Path.v", lines "//   EN_m1 -> m3", 1)]
    ),
    verilog_case!(
        "Test_Path_AlwaysEn.bsv",
        CompileExpectation::Pass,
        &[],
        [text!("sysTest_Path_AlwaysEn.v", excludes "EN__write")]
    ),
    verilog_case!(
        "Test_AlwaysReady.bsv",
        CompileExpectation::Pass,
        UNSAFE_ALWAYS_READY,
        [
            text!("sysTest_AlwaysReady.v", excludes "RDY__write"),
            text!(
                "sysTest_AlwaysReady.v",
                contains "assign rg$EN = EN__write ;"
            ),
        ]
    ),
    verilog_case!(
        "Test_AlwaysEnabled.bsv",
        CompileExpectation::Pass,
        UNSAFE_ALWAYS_READY,
        [
            text!("mkSub.v", excludes "RDY__write"),
            text!("mkSub.v", contains "assign rg$EN = CLK_GATE ;"),
        ]
    ),
    verilog_case!(
        "ExportedGate.bsv",
        CompileExpectation::Pass,
        &[],
        [text!(
            "sysExportedGate.v",
            contains "assign RDY__write = g$CLK_GATE_OUT ;"
        )]
    ),
    pass!("AlwaysReady_OnInterface_Subinterface.bsv"),
    pass!("AlwaysReady_OnInterface_Top.bsv"),
    pass!("AlwaysReady_OnMethod.bsv"),
    pass!("AlwaysReady_OnModule_FullInterface.bsv"),
    pass!("AlwaysReady_OnModule_OneMethod.bsv"),
    pass!("AlwaysReady_OnSubinterface.bsv"),
];
