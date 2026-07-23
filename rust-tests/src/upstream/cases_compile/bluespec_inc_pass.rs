//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b600/b600.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b532/b532.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1470/b1470.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b271/b271.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1599/b1599.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b198/b198.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b289/b289.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1294/b1294.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b267/b267.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b547/b547.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b41/b41.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b542/b542.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b384/b384.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b436/b436.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b394/b394.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b927/b927.exp`

use super::CompileCase;

pub(super) const B600: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b600::Bug600.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b600",
    "Bug600.bsv"
);
pub(super) const B532: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b532::Bug532.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b532",
    "Bug532.bsv"
);
pub(super) const B1470: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1470::Bug1470.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1470",
    "Bug1470.bsv"
);
pub(super) const B271: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b271::Bug271.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b271",
    "Bug271.bsv"
);
pub(super) const B1599: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1599::Bug1599.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1599",
    "Bug1599.bsv"
);
pub(super) const B198: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b198::Bug198.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b198",
    "Bug198.bsv"
);
pub(super) const B289: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b289::Bug289.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b289",
    "Bug289.bsv"
);
pub(super) const B1294: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1294::HasTupleFailFastBug.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1294",
    "HasTupleFailFastBug.bsv"
);
pub(super) const B267: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b267::Bug267.bs",
    "testsuite/bsc.bugs/bluespec_inc/b267",
    "Bug267.bs"
);
pub(super) const B547: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b547::Bug547.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b547",
    "Bug547.bsv"
);
pub(super) const B41: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b41::Bug41.bs",
    "testsuite/bsc.bugs/bluespec_inc/b41",
    "Bug41.bs"
);
pub(super) const B542: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b542::Bug542.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b542",
    "Bug542.bsv"
);
pub(super) const B384: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b384::Bug384_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b384",
    "Bug384_1.bsv"
);
pub(super) const B436: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b436::ArrayReg.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b436",
    "ArrayReg.bsv"
);
pub(super) const B394: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b394::Bug394.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b394",
    "Bug394.bsv"
);
pub(super) const B927: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b927::Bug927.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b927",
    "Bug927.bsv"
);

pub(super) const CASES: &[CompileCase] = &[
    B600, B532, B1470, B271, B1599, B198, B289, B1294, B267, B547, B41, B542, B384, B436, B394,
    B927,
];
