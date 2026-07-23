//! Origin: `testsuite/bsc.typechecker/mismatch/mismatch.exp`.

use super::CompileCase;
use crate::upstream::{ArtifactAssertion, TextAssertion};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/mismatch";

macro_rules! mismatch_error {
    ($constant:ident, $source:literal, $tag:literal) => {
        mismatch_error!($constant, $source, $tag, &[]);
    };
    ($constant:ident, $source:literal, $tag:literal, $assertions:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.typechecker/mismatch::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: $assertions,
            expectation: crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: crate::upstream::CompileMode::Frontend,
            requirement: crate::upstream::Requirement::Always,
        };
    };
}

mismatch_error!(TOO_MANY_ARGS, "FuncMismatchTooManyArgs.bsv", "T0081");
mismatch_error!(TOO_FEW_ARGS, "FuncMismatchTooFewArgs.bsv", "T0081");
mismatch_error!(
    NUM_ARGS_MAYBE_ARG,
    "FuncMismatchNumArgsMaybeArgN.bsv",
    "T0081"
);
mismatch_error!(
    METHOD_NUM_ARGS_MAYBE_ARG,
    "MethMismatchNumArgsMaybeArgN.bsv",
    "T0081"
);
mismatch_error!(RESULT, "FuncMismatchResult.bsv", "T0080");
mismatch_error!(ARG, "FuncMismatchArgN.bsv", "T0080");
mismatch_error!(NO_ARGS_DEFINITION, "FuncMismatchNoArgsDef.bsv", "T0083");
mismatch_error!(NO_ARGS_USE, "FuncMismatchNoArgsUse.bsv", "T0084");
mismatch_error!(
    ASSIGN_TOO_MANY_ARGS,
    "FuncMismatchAssignTooManyArgs.bsv",
    "T0081"
);
mismatch_error!(ASSIGN_ARG, "FuncMismatchAssignArgN.bsv", "T0082");
mismatch_error!(
    ASSIGN_NUM_ARGS_MAYBE_ARG,
    "FuncMismatchAssignNumArgsMaybeArgN.bsv",
    "T0081",
    &[ArtifactAssertion::Text {
        path: "FuncMismatchAssignNumArgsMaybeArgN.bsv.bsc-out",
        assertion: TextAssertion::LineCount {
            text: "argument 2",
            count: 1,
        },
    }]
);
mismatch_error!(CANNOT_UNIFY, "FuncMismatchCannotUnify.bsv", "T0020");

pub(super) const CASES: &[CompileCase] = &[
    TOO_MANY_ARGS,
    TOO_FEW_ARGS,
    NUM_ARGS_MAYBE_ARG,
    METHOD_NUM_ARGS_MAYBE_ARG,
    RESULT,
    ARG,
    NO_ARGS_DEFINITION,
    NO_ARGS_USE,
    ASSIGN_TOO_MANY_ARGS,
    ASSIGN_ARG,
    ASSIGN_NUM_ARGS_MAYBE_ARG,
    CANNOT_UNIFY,
];
