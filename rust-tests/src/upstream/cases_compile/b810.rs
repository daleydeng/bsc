//! Origin: `testsuite/bsc.bugs/bluespec_inc/b810/b810.exp`.

use super::CompileCase;

pub(super) const BUG_810_2: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b810::Bug810_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b810",
    "Bug810_2.bsv",
    "T0031"
);
