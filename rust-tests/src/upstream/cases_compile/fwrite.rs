//! Origin: `testsuite/bsc.misc/fwrite/fwrite.exp`.

use super::CompileCase;
use crate::upstream::{CompileExpectation, CompileMode, DiagnosticKind, Requirement};

const FIXTURE_DIR: &str = "testsuite/bsc.misc/fwrite";

macro_rules! fail {
    ($constant:ident, $source:literal, $tag:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.misc/fwrite::", $source),
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
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        };
    };
}

fail!(FOPEN_3, "FOpen3.bsv", "T0020");
fail!(FILE_TYPE_ERR_1, "FileTypeErr1.bsv", "T0031");
fail!(FILE_TYPE_ERR_2, "FileTypeErr2.bsv", "T0092");
fail!(FILE_TYPE_ERR_3, "FileTypeErr3.bsv", "T0020");
fail!(GETC_ERR_1, "GetC_err1.bsv", "T0080");
fail!(GETC_ERR_2, "GetC_err2.bsv", "T0020");
fail!(GETC_ERR_3, "GetC_err3.bsv", "T0031");

pub(super) const CASES: &[CompileCase] = &[
    FOPEN_3,
    FILE_TYPE_ERR_1,
    FILE_TYPE_ERR_2,
    FILE_TYPE_ERR_3,
    GETC_ERR_1,
    GETC_ERR_2,
    GETC_ERR_3,
];
