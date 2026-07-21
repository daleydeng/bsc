use super::CompileCase;

pub(super) const B1586: CompileCase = compile_fail_golden_case!(
    "b1586::Bug1586.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1586",
    "Bug1586.bsv",
    "Bug1586.bsv.bsc-out.expected"
);
pub(super) const B269: CompileCase = compile_fail_error_golden_case!(
    "b269::Bug269.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b269",
    "Bug269.bsv",
    "P0070",
    "Bug269.bsv.bsc-out.expected"
);
pub(super) const B880: CompileCase = compile_fail_error_golden_case!(
    "b880::FieldSelectError.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b880",
    "FieldSelectError.bsv",
    "T0138",
    "FieldSelectError.bsv.bsc-out.expected"
);
pub(super) const B1493_GOOD: CompileCase = compile_pass_case!(
    "b1493::Bug1493.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1493",
    "Bug1493.bsv"
);
pub(super) const B1493_BAD: CompileCase = compile_fail_error_case!(
    "b1493::Bug1493_Bad.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1493",
    "Bug1493_Bad.bsv",
    "T0020"
);
pub(super) const B557_GOOD: CompileCase = compile_pass_case!(
    "b557::Bug557_1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b557",
    "Bug557_1.bsv"
);
pub(super) const B557_BAD: CompileCase = compile_fail_error_case!(
    "b557::Bug557_2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b557",
    "Bug557_2.bsv",
    "P0109"
);
