//! Origin: `testsuite/bsc.typechecker/kind/bound-vars/bound-vars.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/kind/bound-vars";

macro_rules! bound_var_error {
    ($constant:ident, $source:literal, $tag:literal) => {
        pub(super) const $constant: CompileCase = compile_fail_error_case!(
            concat!("bsc.typechecker/kind/bound-vars::", $source),
            FIXTURE_DIR,
            $source,
            $tag
        );
    };
}

bound_var_error!(C_HAS_TYPE, "CHasType.bs", "T0027");
bound_var_error!(C_DEFL, "CDefl.bs", "T0027");
bound_var_error!(C_DEFL_BSV, "CDeflBSV.bsv", "T0026");
bound_var_error!(C_BIND_T, "CBindT.bsv", "T0026");
bound_var_error!(
    KIND_MISMATCH_MISSING_ARG,
    "KindMismatchMissingArg.bsv",
    "T0025"
);
bound_var_error!(
    KIND_MISMATCH_ARG_TO_BOUND_VAR,
    "KindMismatchArgToBoundVar.bsv",
    "T0026"
);

pub(super) const WIDENING_PLUS: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/bound-vars::WideningPlus.bsv",
    FIXTURE_DIR,
    "WideningPlus.bsv"
);

pub(super) const ADJUST_SIZE: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/bound-vars::AdjustSize.bsv",
    FIXTURE_DIR,
    "AdjustSize.bsv"
);
