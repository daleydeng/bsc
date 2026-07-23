//! Origin: `testsuite/bsc.names/portRenaming/conflicts/enableReady/enableReady.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.names/portRenaming/conflicts/enableReady";

macro_rules! enable_ready_error {
    ($constant:ident, $source:literal) => {
        enable_ready_error!($constant, $source, &[]);
    };
    ($constant:ident, $source:literal, $assertions:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.names/portRenaming/conflicts/enableReady::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: $assertions,
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: "G0055",
                count: 1,
            },
            golden: None,
            options: &["-verilog"],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        };
    };
}

enable_ready_error!(TEST_01, "Test01.bsv");
enable_ready_error!(TEST_02, "Test02.bsv");
enable_ready_error!(TEST_03, "Test03.bsv");
enable_ready_error!(TEST_04, "Test04.bsv");
enable_ready_error!(TEST_05, "Test05.bsv");
enable_ready_error!(
    TEST_06,
    "Test06.bsv",
    &[ArtifactAssertion::Text {
        path: "Test06.bsv.bsc-out",
        assertion: TextAssertion::DiagnosticCount {
            kind: DiagnosticKind::Error,
            tag: "G0055",
            count: 1,
        },
    }]
);

pub(super) const CASES: &[CompileCase] = &[TEST_01, TEST_02, TEST_03, TEST_04, TEST_05, TEST_06];
