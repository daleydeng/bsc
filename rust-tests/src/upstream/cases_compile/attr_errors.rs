use super::CompileCase;

pub(super) const T1: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::T1.bsv",
    "testsuite/bsc.misc/attrErrors",
    "T1.bsv",
    "P0155"
);
pub(super) const T2: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::T2.bsv",
    "testsuite/bsc.misc/attrErrors",
    "T2.bsv",
    "P0063"
);
pub(super) const T3: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::T3.bsv",
    "testsuite/bsc.misc/attrErrors",
    "T3.bsv",
    "P0155"
);
pub(super) const T4: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::T4.bsv",
    "testsuite/bsc.misc/attrErrors",
    "T4.bsv",
    "P0155"
);
pub(super) const T5: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::T5.bsv",
    "testsuite/bsc.misc/attrErrors",
    "T5.bsv",
    "P0155"
);
pub(super) const T6: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::T6.bsv",
    "testsuite/bsc.misc/attrErrors",
    "T6.bsv",
    "P0005"
);
pub(super) const MULTIPLE_ATTRIB_MODULE: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::MultipleAttribModule.bsv",
    "testsuite/bsc.misc/attrErrors",
    "MultipleAttribModule.bsv",
    "P0156"
);
pub(super) const MULTIPLE_ATTRIB_FUNC: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::MultipleAttribFunc.bsv",
    "testsuite/bsc.misc/attrErrors",
    "MultipleAttribFunc.bsv",
    "P0156"
);
pub(super) const MULTIPLE_ATTRIB_RULE: CompileCase = compile_fail_error_case!(
    "bsc.misc/attrErrors::MultipleAttribRule.bsv",
    "testsuite/bsc.misc/attrErrors",
    "MultipleAttribRule.bsv",
    "P0156"
);
pub(super) const MULTIPLE_SAME_ATTRIB_MODULE: CompileCase = compile_pass_case!(
    "bsc.misc/attrErrors::MultipleSameAttribModule.bsv",
    "testsuite/bsc.misc/attrErrors",
    "MultipleSameAttribModule.bsv"
);
