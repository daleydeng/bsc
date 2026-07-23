//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1328/b1328.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1497/b1497.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b578/b578.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b675/b675.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b737/b737.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b753/b753.exp`

use crate::upstream::{
    CompileCase, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

pub(super) const B1328_DIVIDER: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1328::Divider.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1328",
    "Divider.bsv"
);
pub(super) const B1328_TEST: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b1328::Test.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1328",
    "Test.bsv"
);

pub(super) const B1497_MPMC_TLM: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1497::Mpmc_TLM.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1497",
    source: "Mpmc_TLM.bsv",
    fixtures: &[
        "Mpmc_TLM.bsv",
        "Mpmc_NPI.bsv",
        "TLMDefines.bsv",
        "MPMC.defines",
        "TLM.defines",
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B1497_STATE: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1497::State.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1497",
    "State.bsv"
);
pub(super) const B1497_CACHE_CONTROLLER: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1497::Cache_Controller.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1497",
    source: "Cache_Controller.bsv",
    fixtures: &["Cache_Controller.bsv", "SRAM_Interfaces.bsv"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-let-gen"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B1497_TODD_BOGUS_BITS: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1497::ToddBogusBits.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1497",
    "ToddBogusBits.bsv"
);
pub(super) const B1497_BUFF_INDEX: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b1497::BuffIndex.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b1497",
    "BuffIndex.bsv"
);

pub(super) const B578_BUG578: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b578::Bug578.bs",
    "testsuite/bsc.bugs/bluespec_inc/b578",
    "Bug578.bs"
);
pub(super) const B578_BUG578_SIMPLE: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b578::Bug578_simple.bs",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b578",
    source: "Bug578_simple.bs",
    fixtures: &["Bug578_simple.bs", "Bug578_simple.bs.bsc-out.expected"],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0131",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Bug578_simple.bs.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const B675_BUG675_MODULE_COLLECT: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b675::Bug675_ModuleCollect.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b675",
    source: "Bug675_ModuleCollect.bsv",
    fixtures: &[
        "Bug675_ModuleCollect.bsv",
        "Bug675_ModuleCollect.bsv.bsc-out.expected",
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0030",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Bug675_ModuleCollect.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B675_BUG675_MODULE_COLLECT_CLASSIC: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b675::Bug675_ModuleCollect_Classic.bs",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b675",
    source: "Bug675_ModuleCollect_Classic.bs",
    fixtures: &[
        "Bug675_ModuleCollect_Classic.bs",
        "Bug675_ModuleCollect_Classic.bs.bsc-out.expected",
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0030",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Bug675_ModuleCollect_Classic.bs.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B675_SIMPLE: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b675::Simple.bs",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b675",
    source: "Simple.bs",
    fixtures: &["Simple.bs", "Simple.bs.bsc-out.expected"],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0030",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "Simple.bs.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B675_BUG675_PRIM_UPDATE_RANGE_FN: CompileCase = compile_pass_case!(
    "bsc.bugs/bluespec_inc/b675::Bug675_PrimUpdateRangeFn.bs",
    "testsuite/bsc.bugs/bluespec_inc/b675",
    "Bug675_PrimUpdateRangeFn.bs"
);

pub(super) const B737_METHOD_SELF_REFERENCE: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b737::MethodSelfReference.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b737",
    source: "MethodSelfReference.bsv",
    fixtures: &[
        "MethodSelfReference.bsv",
        "MethodSelfReference.bsv.bsc-out.expected",
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0004",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "MethodSelfReference.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B737_METHOD_TO_METHOD_REFERENCE: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b737::MethodToMethodReference.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b737",
    source: "MethodToMethodReference.bsv",
    fixtures: &[
        "MethodToMethodReference.bsv",
        "MethodToMethodReference.bsv.bsc-out.expected",
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0004",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "MethodToMethodReference.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B737_METHOD_INTERNAL_NAME_CLASH: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b737::MethodInternalNameClash.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b737",
    source: "MethodInternalNameClash.bsv",
    fixtures: &[
        "MethodInternalNameClash.bsv",
        "MethodInternalNameClash.bsv.bsc-out.expected",
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0011",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "MethodInternalNameClash.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B737_METHOD_EXTERNAL_NAME_CLASH: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b737::MethodExternalNameClash.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b737",
    source: "MethodExternalNameClash.bsv",
    fixtures: &[
        "MethodExternalNameClash.bsv",
        "MethodExternalNameClash.bsv.bsc-out.expected",
    ],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0011",
        count: 1,
    },
    golden: Some(GoldenExpectation {
        expected: "MethodExternalNameClash.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B737_ACTION_METHOD_REG_CLASH: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b737::ActionMethodRegClash.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b737",
    "ActionMethodRegClash.bsv",
    "T0011"
);
pub(super) const B737_ACTION_METHOD_REG_CLASH_SUB_IFC: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b737::ActionMethodRegClash_SubIfc.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b737",
    "ActionMethodRegClash_SubIfc.bsv"
);
pub(super) const B737_VALUE_METHOD_IN_ITS_CONDITION: CompileCase = compile_fail_error_case!(
    "bsc.bugs/bluespec_inc/b737::ValueMethodInItsCondition.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b737",
    "ValueMethodInItsCondition.bsv",
    "T0011"
);
pub(super) const B737_VALUE_METHOD_IN_ITS_CONDITION_SUB_IFC: CompileCase = compile_verilog_pass_case!(
    "bsc.bugs/bluespec_inc/b737::ValueMethodInItsCondition_SubIfc.bsv",
    "testsuite/bsc.bugs/bluespec_inc/b737",
    "ValueMethodInItsCondition_SubIfc.bsv"
);

pub(super) const B753_BUG753: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b753::Bug753.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b753",
    source: "Bug753.bsv",
    fixtures: &["Bug753.bsv", "SVA2.bs"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};
pub(super) const B753_BUG753_CLASSIC: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b753::Bug753_Classic.bs",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b753",
    source: "Bug753_Classic.bs",
    fixtures: &["Bug753_Classic.bs", "SVA2.bs"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const CASES: &[CompileCase] = &[
    B1328_DIVIDER,
    B1328_TEST,
    B1497_MPMC_TLM,
    B1497_STATE,
    B1497_CACHE_CONTROLLER,
    B1497_TODD_BOGUS_BITS,
    B1497_BUFF_INDEX,
    B578_BUG578,
    B578_BUG578_SIMPLE,
    B675_BUG675_MODULE_COLLECT,
    B675_BUG675_MODULE_COLLECT_CLASSIC,
    B675_SIMPLE,
    B675_BUG675_PRIM_UPDATE_RANGE_FN,
    B737_METHOD_SELF_REFERENCE,
    B737_METHOD_TO_METHOD_REFERENCE,
    B737_METHOD_INTERNAL_NAME_CLASH,
    B737_METHOD_EXTERNAL_NAME_CLASH,
    B737_ACTION_METHOD_REG_CLASH,
    B737_ACTION_METHOD_REG_CLASH_SUB_IFC,
    B737_VALUE_METHOD_IN_ITS_CONDITION,
    B737_VALUE_METHOD_IN_ITS_CONDITION_SUB_IFC,
    B753_BUG753,
    B753_BUG753_CLASSIC,
];
