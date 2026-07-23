//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1048/b1048.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1163/b1163.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1198/b1198.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1229/b1229.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1318/b1318.exp`
//! - `testsuite/bsc.bugs/github/gh894/gh894.exp`

use super::CompileCase;

pub(super) const B1048: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1048::Bug.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1048",
    "Bug.bsv"
);

pub(super) const B1163: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1163::g.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1163",
    "g.bsv"
);

pub(super) const B1198: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1198::Bug1198.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1198",
    "Bug1198.bsv"
);

pub(super) const B1229: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1229::ActionValueStructBind.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1229",
    "ActionValueStructBind.bsv"
);

pub(super) const B1318: CompileCase = compile_verilog_fail_error_case!(
    "bsc.bugs/bluespec_inc/b1318::Test.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1318",
    "Test.bsv",
    "G0008"
);

pub(super) const GH894: CompileCase = compile_fail_golden_case!(
    "bsc.bugs/github/gh894::Test.bs",
    "testsuite/bsc.bugs/github/gh894",
    "Test.bs",
    "Test.bs.bsc-out.expected"
);

pub(super) const CASES: &[CompileCase] = &[
    B1048,
    B1163,
    B1198,
    B1229,
    B1318,
    GH894,
];
