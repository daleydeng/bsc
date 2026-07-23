//! Origins:
//! - `testsuite/bsc.typechecker/elab_typeclasses/elab_typeclasses.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/always_ready/always_ready.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/osc/osc.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/prefixResult/prefixResult.exp`
//! - `testsuite/bsc.interra/StmtFSM/breakOutsideFSM/breakOutsideFSM.exp`
//! - `testsuite/bsc.interra/StmtFSM/continueOutsideFSM/continueOutsideFSM.exp`
//! - `testsuite/bsc.interra/StmtFSM/continueOutsideLoop/continueOutsideLoop.exp`
//! - `testsuite/bsc.interra/messages/ENoNF/ENoNF.exp`
//! - `testsuite/bsc.interra/messages/EStmtContext/EStmtContext.exp`
//! - `testsuite/bsc.bugs/github/gh678/gh678.exp`
//! - `testsuite/bsc.bsv_examples/ConditionWires/conditionWires.exp`
//! - `testsuite/bsc.interra/messages/EBadMatch/EBadMatch.exp`
//! - `testsuite/bsc.interra/messages/EConstrAmb/EConstrAmb.exp`
//! - `testsuite/bsc.interra/messages/EForeignNotBit/EForeignNotBit.exp`
//! - `testsuite/bsc.interra/messages/EKindArg/EKindArg.exp`
//! - `testsuite/bsc.interra/messages/ENotAnInterface/ENotAnInterface.exp`
//! - `testsuite/bsc.interra/messages/ENotExpr/ENotExpr.exp`
//! - `testsuite/bsc.interra/messages/ENotStruct/ENotStruct.exp`
//! - `testsuite/bsc.interra/messages/ENotStructId/ENotStructId.exp`
//! - `testsuite/bsc.interra/messages/ENoTypeSign/ENoTypeSign.exp`
//! - `testsuite/bsc.interra/messages/EPartialTypeApp/EPartialTypeApp.exp`
//! - `testsuite/bsc.interra/messages/ESyntax/ESyntax.exp`
//! - `testsuite/bsc.interra/messages/EUnboundTyVar/EUnboundTyVar.exp`
//! - `testsuite/bsc.interra/messages/EUntermComm/EUntermComm.exp`
//! - `testsuite/bsc.interra/messages/EValueOf/EValueOf.exp`
//! - `testsuite/bsc.interra/messages/EWrongArity/EWrongArity.exp`
//! - `testsuite/bsc.interra/bugs/bugID133/bugID133.exp`
//! - `testsuite/bsc.interra/bugs/bugID153/bugID153.exp`
//! - `testsuite/bsc.interra/bugs/bugID154/bugID154.exp`
//! - `testsuite/bsc.interra/bugs/bugID161/bugID161.exp`

use crate::upstream::{
    CompileCase, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

pub(super) const ELAB_TYPECLASSES: CompileCase = compile_pass_case!(
    "bsc.typechecker/elab_typeclasses::ElabTypeclasses.bs",
    "testsuite/bsc.typechecker/elab_typeclasses",
    "ElabTypeclasses.bs"
);

pub(super) const ALWAYS_READY_WRONG_LOC_INTERFACE_WITH_ARG: CompileCase = CompileCase {
    name: "bsc.names/portRenaming/invalidAttrs/always_ready::WrongLoc_InterfaceWithArg.bsv",
    fixture_dir: "testsuite/bsc.names/portRenaming/invalidAttrs/always_ready",
    source: "WrongLoc_InterfaceWithArg.bsv",
    fixtures: &["WrongLoc_InterfaceWithArg.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "P0159",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const OSC_WRONG_LOC_INOUT_ARG: CompileCase = CompileCase {
    name: "bsc.names/portRenaming/invalidAttrs/osc::WrongLoc_InoutArg.bsv",
    fixture_dir: "testsuite/bsc.names/portRenaming/invalidAttrs/osc",
    source: "WrongLoc_InoutArg.bsv",
    fixtures: &["WrongLoc_InoutArg.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "P0181",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const PREFIX_RESULT_TEST01: CompileCase = CompileCase {
    name: "bsc.names/portRenaming/conflicts/prefixResult::Test01.bsv",
    fixture_dir: "testsuite/bsc.names/portRenaming/conflicts/prefixResult",
    source: "Test01.bsv",
    fixtures: &["Test01.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0055",
        count: 1,
    },
    golden: None,
    options: &["-verilog"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BREAK_OUTSIDE_FSM: CompileCase = CompileCase {
    name: "bsc.interra/StmtFSM/breakOutsideFSM::breakOutsideFSM.bsv",
    fixture_dir: "testsuite/bsc.interra/StmtFSM/breakOutsideFSM",
    source: "breakOutsideFSM.bsv",
    fixtures: &["breakOutsideFSM.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "P0163",
        count: 1,
    },
    golden: None,
    options: &["-verilog"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const CONTINUE_OUTSIDE_FSM: CompileCase = CompileCase {
    name: "bsc.interra/StmtFSM/continueOutsideFSM::continueOutsideFSM.bsv",
    fixture_dir: "testsuite/bsc.interra/StmtFSM/continueOutsideFSM",
    source: "continueOutsideFSM.bsv",
    fixtures: &["continueOutsideFSM.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "P0164",
        count: 1,
    },
    golden: None,
    options: &["-verilog"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const CONTINUE_OUTSIDE_LOOP: CompileCase = CompileCase {
    name: "bsc.interra/StmtFSM/continueOutsideLoop::continueOutsideLoop.bsv",
    fixture_dir: "testsuite/bsc.interra/StmtFSM/continueOutsideLoop",
    source: "continueOutsideLoop.bsv",
    fixtures: &["continueOutsideLoop.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "S0015",
        count: 1,
    },
    golden: None,
    options: &["-verilog"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const ENO_NF3: CompileCase = CompileCase {
    name: "bsc.interra/messages/ENoNF::ENoNF3.bs",
    fixture_dir: "testsuite/bsc.interra/messages/ENoNF",
    source: "ENoNF3.bs",
    fixtures: &["ENoNF3.bs"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0070",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const E_STMT_CONTEXT1: CompileCase = CompileCase {
    name: "bsc.interra/messages/EStmtContext::EStmtContext1.bs",
    fixture_dir: "testsuite/bsc.interra/messages/EStmtContext",
    source: "EStmtContext1.bs",
    fixtures: &["EStmtContext1.bs"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0045",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const GH678: CompileCase = CompileCase {
    name: "bsc.bugs/github/gh678::Test.bs",
    fixture_dir: "testsuite/bsc.bugs/github/gh678",
    source: "Test.bs",
    fixtures: &["Test.bs", "GenCRepr.bs", "SizedVector.bs", "State.bs"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const CONDITION_WIRES: CompileCase = CompileCase {
    name: "bsc.bsv_examples/ConditionWires::DemonstrateConditions.bsv",
    fixture_dir: "testsuite/bsc.bsv_examples/ConditionWires",
    source: "DemonstrateConditions.bsv",
    fixtures: &["DemonstrateConditions.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["+RTS", "-K10M", "-RTS"],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("mkTopLevel"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const E_BAD_MATCH1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadMatch::EBadMatch1.bs",
    "testsuite/bsc.interra/messages/EBadMatch",
    "EBadMatch1.bs",
    "EBadMatch1.bs.bsc-out.expected"
);

pub(super) const E_CONSTR_AMB: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EConstrAmb::EConstrAmb.bs",
    "testsuite/bsc.interra/messages/EConstrAmb",
    "EConstrAmb.bs",
    "EConstrAmb.bs.bsc-out.expected"
);

pub(super) const E_FOREIGN_NOT_BIT: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EForeignNotBit::EForeignNotBit.bs",
    "testsuite/bsc.interra/messages/EForeignNotBit",
    "EForeignNotBit.bs",
    "EForeignNotBit.bs.bsc-out.expected"
);

pub(super) const E_KIND_ARG: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EKindArg::EKindArg.bs",
    "testsuite/bsc.interra/messages/EKindArg",
    "EKindArg.bs",
    "EKindArg.bs.bsc-out.expected"
);

pub(super) const E_NOT_AN_INTERFACE: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotAnInterface::ENotAnInterface.bs",
    "testsuite/bsc.interra/messages/ENotAnInterface",
    "ENotAnInterface.bs",
    "ENotAnInterface.bs.bsc-out.expected"
);

pub(super) const E_NOT_EXPR: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotExpr::ENotExpr.bs",
    "testsuite/bsc.interra/messages/ENotExpr",
    "ENotExpr.bs",
    "ENotExpr.bs.bsc-out.expected"
);

pub(super) const E_NOT_STRUCT1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotStruct::ENotStruct1.bs",
    "testsuite/bsc.interra/messages/ENotStruct",
    "ENotStruct1.bs",
    "ENotStruct1.bs.bsc-out.expected"
);

pub(super) const E_NOT_STRUCT_ID: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENotStructId::ENotStructId.bs",
    "testsuite/bsc.interra/messages/ENotStructId",
    "ENotStructId.bs",
    "ENotStructId.bs.bsc-out.expected"
);

pub(super) const E_NO_TYPE_SIGN: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ENoTypeSign::ENoTypeSign.bs",
    "testsuite/bsc.interra/messages/ENoTypeSign",
    "ENoTypeSign.bs",
    "ENoTypeSign.bs.bsc-out.expected"
);

pub(super) const E_PARTIAL_TYPE_APP1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EPartialTypeApp::EPartialTypeApp1.bs",
    "testsuite/bsc.interra/messages/EPartialTypeApp",
    "EPartialTypeApp1.bs",
    "EPartialTypeApp1.bs.bsc-out.expected"
);

pub(super) const E_SYNTAX1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ESyntax::ESyntax1.bs",
    "testsuite/bsc.interra/messages/ESyntax",
    "ESyntax1.bs",
    "ESyntax1.bs.bsc-out.expected"
);

pub(super) const E_UNBOUND_TY_VAR1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundTyVar::EUnboundTyVar1.bs",
    "testsuite/bsc.interra/messages/EUnboundTyVar",
    "EUnboundTyVar1.bs",
    "EUnboundTyVar1.bs.bsc-out.expected"
);

pub(super) const E_UNTERM_COMM1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUntermComm::EUntermComm1.bs",
    "testsuite/bsc.interra/messages/EUntermComm",
    "EUntermComm1.bs",
    "EUntermComm1.bs.bsc-out.expected"
);

pub(super) const E_VALUE_OF: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EValueOf::EValueOf.bs",
    "testsuite/bsc.interra/messages/EValueOf",
    "EValueOf.bs",
    "EValueOf.bs.bsc-out.expected"
);

pub(super) const E_WRONG_ARITY: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EWrongArity::EWrongArity.bs",
    "testsuite/bsc.interra/messages/EWrongArity",
    "EWrongArity.bs",
    "EWrongArity.bs.bsc-out.expected"
);

pub(super) const BUG_ID_133: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID133::Ambg.bs",
    "testsuite/bsc.interra/bugs/bugID133",
    "Ambg.bs",
    "Ambg.bs.bsc-out.expected"
);

pub(super) const BUG_ID_153: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID153::BsTop.bs",
    "testsuite/bsc.interra/bugs/bugID153",
    "BsTop.bs",
    "BsTop.bs.bsc-out.expected"
);

pub(super) const BUG_ID_154: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID154::BsTop.bs",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID154",
    source: "BsTop.bs",
    fixtures: &["BsTop.bs", "BsTop.bs.bsc-out.expected"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "BsTop.bs.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_161: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID161::Test.bs",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID161",
    source: "Test.bs",
    fixtures: &["Test.bs", "Test.bs.bsc-out.expected"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "Test.bs.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const CASES: &[CompileCase] = &[
    ELAB_TYPECLASSES,
    ALWAYS_READY_WRONG_LOC_INTERFACE_WITH_ARG,
    OSC_WRONG_LOC_INOUT_ARG,
    PREFIX_RESULT_TEST01,
    BREAK_OUTSIDE_FSM,
    CONTINUE_OUTSIDE_FSM,
    CONTINUE_OUTSIDE_LOOP,
    ENO_NF3,
    E_STMT_CONTEXT1,
    GH678,
    CONDITION_WIRES,
    E_BAD_MATCH1,
    E_CONSTR_AMB,
    E_FOREIGN_NOT_BIT,
    E_KIND_ARG,
    E_NOT_AN_INTERFACE,
    E_NOT_EXPR,
    E_NOT_STRUCT1,
    E_NOT_STRUCT_ID,
    E_NO_TYPE_SIGN,
    E_PARTIAL_TYPE_APP1,
    E_SYNTAX1,
    E_UNBOUND_TY_VAR1,
    E_UNTERM_COMM1,
    E_VALUE_OF,
    E_WRONG_ARITY,
    BUG_ID_133,
    BUG_ID_153,
    BUG_ID_154,
    BUG_ID_161,
];
