//! Origin: `testsuite/bsc.typechecker/read_desugaring/read_desugaring.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/read_desugaring";

pub(super) const LIST_DESUGAR_FAIL: CompileCase = compile_fail_error_case!(
    "bsc.typechecker/read_desugaring::ListDesugarFail.bsv",
    FIXTURE_DIR,
    "ListDesugarFail.bsv",
    "T0060"
);

pub(super) const LIST_DESUGAR_FAIL_2: CompileCase = compile_fail_error_case!(
    "bsc.typechecker/read_desugaring::ListDesugarFail2.bsv",
    FIXTURE_DIR,
    "ListDesugarFail2.bsv",
    "T0020"
);

pub(super) const STRUCT_REG_FAIL: CompileCase = compile_fail_error_case!(
    "bsc.typechecker/read_desugaring::StructRegFail.bsv",
    FIXTURE_DIR,
    "StructRegFail.bsv",
    "T0020"
);
