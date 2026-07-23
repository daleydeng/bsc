//! Origin: `testsuite/bsc.evaluator/prims/when/when.exp`.

use super::CompileCase;

pub(super) const CASES: &[CompileCase] = &[
    compile_verilog_fail_error_case!(
        "bsc.evaluator/prims/when::WhenActionValue.bsv",
        "testsuite/bsc.evaluator/prims/when",
        "WhenActionValue.bsv",
        "G0122"
    ),
    compile_verilog_fail_error_case!(
        "bsc.evaluator/prims/when::WhenMethodArg.bsv",
        "testsuite/bsc.evaluator/prims/when",
        "WhenMethodArg.bsv",
        "G0122"
    ),
];
