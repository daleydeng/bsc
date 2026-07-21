//! Origin: `testsuite/bsc.syntax/bsv05/case/case.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bsv05/case";

pub(super) const MIXED_DEC: CompileCase = compile_fail_case!(
    "bsc.syntax/bsv05/case::CaseMixedDec.bsv",
    FIXTURE_DIR,
    "CaseMixedDec.bsv"
);

macro_rules! case_error {
    ($constant:ident, $source:literal, $tag:literal) => {
        pub(super) const $constant: CompileCase = compile_fail_error_case!(
            concat!("bsc.syntax/bsv05/case::", $source),
            FIXTURE_DIR,
            $source,
            $tag
        );
    };
}

case_error!(MIXED_LITERAL, "Case_MixedLit.bsv", "P0199");
case_error!(IF_DUMMY_1, "CaseIfDummy1.bsv", "P0005");
case_error!(IF_DUMMY_2, "CaseIfDummy2.bsv", "T0004");

macro_rules! case_pass {
    ($constant:ident, $source:literal) => {
        pub(super) const $constant: CompileCase = compile_pass_case!(
            concat!("bsc.syntax/bsv05/case::", $source),
            FIXTURE_DIR,
            $source
        );
    };
}

case_pass!(LITERAL_SIGNED, "CaseLiteralSigned.bsv");
case_pass!(STRING_LITERAL, "CaseStringLiteral.bsv");
case_pass!(MATCHES_STRING_LITERAL, "CaseMatchesStringLiteral.bsv");
