//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b120/b120.exp`
//! - `testsuite/bsc.interra/messages/EAmbOper/EAmbOper.exp`
//! - `testsuite/bsc.assertions/properties/properties.exp`
//! - `testsuite/bsc.bsv_examples/Hamming/hamming.exp`

use super::CompileCase;

pub(super) const BUG_120_1: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b120::Bug120-1.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b120",
    "Bug120-1.bsv",
    "P0086"
);

pub(super) const BUG_120_2: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b120::Bug120-2.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b120",
    "Bug120-2.bsv",
    "P0086"
);

pub(super) const BUG_120_3: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b120::Bug120-3.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b120",
    "Bug120-3.bsv",
    "P0085"
);

pub(super) const E_AMB_OPER: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EAmbOper::EAmbOper.bs",
    "testsuite/bsc.interra/messages/EAmbOper",
    "EAmbOper.bs",
    "EAmbOper.bs.bsc-out.expected"
);

pub(super) const ASSERTION_SYNTAX: CompileCase = compile_pass_case!(
    "bsc.assertions/properties::SyntaxTest.bsv",
    "testsuite/bsc.assertions/properties",
    "SyntaxTest.bsv"
);

pub(super) const HAMMING_QUESTION: CompileCase = compile_pass_case!(
    "bsc.bsv_examples/Hamming::HammingQ.bsv",
    "testsuite/bsc.bsv_examples/Hamming",
    "HammingQ.bsv"
);

pub(super) const CASES: &[CompileCase] = &[
    BUG_120_1,
    BUG_120_2,
    BUG_120_3,
    E_AMB_OPER,
    ASSERTION_SYNTAX,
    HAMMING_QUESTION,
];
