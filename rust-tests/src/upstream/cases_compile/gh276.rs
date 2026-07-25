//! Origin: `testsuite/bsc.bugs/github/gh276/gh276.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/github/gh276";

macro_rules! diagnostic_golden {
    ($constant:ident, $source:literal) => {
        pub(super) const $constant: CompileCase = compile_fail_golden_case!(
            concat!("bsc.bugs/github/gh276::", $source),
            FIXTURE_DIR,
            $source,
            concat!($source, ".bsc-out.expected")
        );
    };
}

diagnostic_golden!(SUGGEST_VALUE_OF_NO_CON, "SuggestValueOf_NoCon.bsv");
diagnostic_golden!(SUGGEST_STRING_OF_NO_CON, "SuggestStringOf_NoCon.bsv");
diagnostic_golden!(SUGGEST_VALUE_OF_ONE_CON, "SuggestValueOf_OneCon.bsv");
diagnostic_golden!(
    SUGGEST_VALUE_OF_TWO_CON_INTEGER_CONTEXT,
    "SuggestValueOf_TwoCon_IntegerContext.bsv"
);
diagnostic_golden!(
    SUGGEST_STRING_OF_TWO_CON_INTEGER_CONTEXT,
    "SuggestStringOf_TwoCon_IntegerContext.bsv"
);
diagnostic_golden!(
    SUGGEST_VALUE_OF_TWO_CON_POLY_CONTEXT,
    "SuggestValueOf_TwoCon_PolyContext.bsv"
);

pub(super) const CASES: &[CompileCase] = &[
    SUGGEST_VALUE_OF_NO_CON,
    SUGGEST_STRING_OF_NO_CON,
    SUGGEST_VALUE_OF_ONE_CON,
    SUGGEST_VALUE_OF_TWO_CON_INTEGER_CONTEXT,
    SUGGEST_STRING_OF_TWO_CON_INTEGER_CONTEXT,
    SUGGEST_VALUE_OF_TWO_CON_POLY_CONTEXT,
];
