//! Origin: `testsuite/bsc.interra/messages/ENotField/ENotField.exp`.

use super::CompileCase;

pub(super) const ENOT_FIELD_1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotField::ENotField1.bs",
    "testsuite/bsc.interra/messages/ENotField",
    "ENotField1.bs",
    "ENotField1.bs.bsc-out.expected"
);
pub(super) const ENOT_FIELD_2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotField::ENotField2.bs",
    "testsuite/bsc.interra/messages/ENotField",
    "ENotField2.bs",
    "ENotField2.bs.bsc-out.expected"
);
pub(super) const ENOT_FIELD_3: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotField::ENotField3.bs",
    "testsuite/bsc.interra/messages/ENotField",
    "ENotField3.bs",
    "ENotField3.bs.bsc-out.expected"
);
pub(super) const ENOT_FIELD_4: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotField::ENotField4.bs",
    "testsuite/bsc.interra/messages/ENotField",
    "ENotField4.bs",
    "ENotField4.bs.bsc-out.expected"
);

pub(super) const CASES: &[CompileCase] = &[
    ENOT_FIELD_1,
    ENOT_FIELD_2,
    ENOT_FIELD_3,
    ENOT_FIELD_4,
];
