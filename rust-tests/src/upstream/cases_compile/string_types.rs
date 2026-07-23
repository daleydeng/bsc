//! Origin: `testsuite/bsc.typechecker/string/string.exp`.

use super::CompileCase;
use crate::upstream::{
    CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/string";

macro_rules! error_with_golden {
    ($constant:ident, $source:literal, $tag:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.typechecker/string::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, concat!($source, ".bsc-out.expected")],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            options: &[],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        };
    };
}

pub(super) const STRING_KIND_POLY_TO_SPECIFIC: CompileCase = compile_pass_case!(
    "bsc.typechecker/string::StringKindPolyToSpecific.bs",
    FIXTURE_DIR,
    "StringKindPolyToSpecific.bs"
);
error_with_golden!(
    STRING_KIND_SPECIFIC_TO_POLY,
    "StringKindSpecificToPoly.bs",
    "T0029"
);
error_with_golden!(
    STRING_KIND_PHANTOM_MISMATCH,
    "StringKindPhantomMismatch.bs",
    "T0020"
);
pub(super) const KIND_MISMATCH: CompileCase = compile_fail_golden_case!(
    "bsc.typechecker/string::KindMismatch.bs",
    FIXTURE_DIR,
    "KindMismatch.bs",
    "KindMismatch.bs.bsc-out.expected"
);

pub(super) const CASES: &[CompileCase] = &[
    STRING_KIND_POLY_TO_SPECIFIC,
    STRING_KIND_SPECIFIC_TO_POLY,
    STRING_KIND_PHANTOM_MISMATCH,
    KIND_MISMATCH,
];
