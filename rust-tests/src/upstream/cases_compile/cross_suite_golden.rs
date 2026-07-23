//! Origins:
//! - `testsuite/bsc.interra/bugs/bugID198/bugID198.exp`
//! - `testsuite/bsc.interra/bugs/bugID235/bugID235.exp`
//! - `testsuite/bsc.interra/bugs/bugID238/bugID238.exp`
//! - `testsuite/bsc.interra/bugs/bugID263/bugID263.exp`
//! - `testsuite/bsc.interra/bugs/bugID277/bugID277.exp`
//! - `testsuite/bsc.interra/bugs/bugID278/bugID278.exp`
//! - `testsuite/bsc.interra/bugs/bugID279/bugID279.exp`
//! - `testsuite/bsc.interra/bugs/bugID299/bugID299.exp`
//! - `testsuite/bsc.interra/bugs/bugID340/bugID340.exp`
//! - `testsuite/bsc.bsc_examples/pong/bscpong.exp`
//! - `testsuite/bsc.bsv_examples/PortReplicator/PortReplicator.exp`
//! - `testsuite/bsc.bsv_examples/rwire/rwire.exp`
//! - `testsuite/bsc.interra/bugs/bugID159/bugID159.exp`
//! - `testsuite/bsc.interra/bugs/bugID298/bugID298.exp`
//! - `testsuite/bsc.interra/bugs/bugID334/bugID334.exp`
//! - `testsuite/bsc.interra/bugs/bugID355/bugID355.exp`
//! - `testsuite/bsc.interra/messages/EBadExport/EBadExport.exp`
//! - `testsuite/bsc.interra/messages/EBadLexChar/EBadLexChar.exp`
//! - `testsuite/bsc.interra/messages/EBadModuleInterface/EBadModuleInterface.exp`
//! - `testsuite/bsc.interra/messages/EBadStringLit/EBadStringLit.exp`
//! - `testsuite/bsc.interra/messages/ECannotDerive/ECannotDerive.exp`
//! - `testsuite/bsc.interra/messages/ELocalRec/ELocalRec.exp`
//! - `testsuite/bsc.interra/messages/EMultipleDef/EMultipleDef.exp`
//! - `testsuite/bsc.interra/messages/ENotAlwaysReady/ENotAlwaysReady.exp`
//! - `testsuite/bsc.interra/messages/EUnboundCon/EUnboundCon.exp`

use crate::upstream::{
    CompileCase, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

pub(super) const BUG_ID_198: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID198::bug.bsv",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID198",
    source: "bug.bsv",
    fixtures: &["bug.bsv", "bug.bsv.bsc-out.expected"],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "bug.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_235: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID235::TcheckFail.bsv",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID235",
    source: "TcheckFail.bsv",
    fixtures: &["TcheckFail.bsv", "TcheckFail.bsv.bsc-out.expected"],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "TcheckFail.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_238: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID238::IfElseIf.bsv",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID238",
    source: "IfElseIf.bsv",
    fixtures: &["IfElseIf.bsv", "IfElseIf.bsv.bsc-out.expected"],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "IfElseIf.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_263: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID263::Testbench.bsv",
    "testsuite/bsc.interra/bugs/bugID263",
    "Testbench.bsv",
    "Testbench.bsv.bsc-out.expected"
);

pub(super) const BUG_ID_277: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID277::bug.bsv",
    "testsuite/bsc.interra/bugs/bugID277",
    "bug.bsv",
    "bug.bsv.bsc-out.expected"
);

pub(super) const BUG_ID_278: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID278::bug.bsv",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID278",
    source: "bug.bsv",
    fixtures: &["bug.bsv", "bug.bsv.bsc-out.expected"],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "bug.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_279: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID279::bug.bsv",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID279",
    source: "bug.bsv",
    fixtures: &["bug.bsv", "bug.bsv.bsc-out.expected"],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "bug.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_299: CompileCase = CompileCase {
    name: "bsc.interra/bugs/bugID299::bug.bsv",
    fixture_dir: "testsuite/bsc.interra/bugs/bugID299",
    source: "bug.bsv",
    fixtures: &["bug.bsv", "bug.bsv.bsc-out.expected"],
    expectation: CompileExpectation::Pass,
    golden: Some(GoldenExpectation {
        expected: "bug.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const BUG_ID_340: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID340::Design_return.bsv",
    "testsuite/bsc.interra/bugs/bugID340",
    "Design_return.bsv",
    "Design_return.bsv.bsc-out.expected"
);

pub(super) const PONG_TB_TOP_LEVEL_FRONTEND: CompileCase = CompileCase {
    name: "bsc.bsc_examples/pong::TbTopLevel.bs::frontend",
    fixture_dir: "testsuite/bsc.bsc_examples/pong",
    source: "TbTopLevel.bs",
    fixtures: &[
        "TbTopLevel.bs",
        "TopLevel.bs",
        "VGACore.bs",
        "Global.bs",
        "LedDecoder.bs",
        "Controller.bs",
        "Kbd.bs",
        "Paddle.bs",
        "Counter.bs",
        "Shape.bs",
        "Color.bs",
        "Ball.bs",
        "Border.bs",
        "Switch.bs",
        "Score.bs",
        "Decimal.bs",
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-let-gen"],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const PONG_TB_TOP_LEVEL_VERILOG: CompileCase = CompileCase {
    name: "bsc.bsc_examples/pong::TbTopLevel.bs::verilog-mkTbTopLevel",
    fixture_dir: "testsuite/bsc.bsc_examples/pong",
    source: "TbTopLevel.bs",
    fixtures: &[
        "TbTopLevel.bs",
        "TopLevel.bs",
        "VGACore.bs",
        "Global.bs",
        "LedDecoder.bs",
        "Controller.bs",
        "Kbd.bs",
        "Paddle.bs",
        "Counter.bs",
        "Shape.bs",
        "Color.bs",
        "Ball.bs",
        "Border.bs",
        "Switch.bs",
        "Score.bs",
        "Decimal.bs",
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-let-gen"],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("mkTbTopLevel"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const PORT_REPLICATOR: CompileCase = CompileCase {
    name: "bsc.bsv_examples/PortReplicator::PortReplicator.bsv",
    fixture_dir: "testsuite/bsc.bsv_examples/PortReplicator",
    source: "PortReplicator.bsv",
    fixtures: &["PortReplicator.bsv", "AsyncROM.bs"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const PORT_REPLICATOR_2: CompileCase = CompileCase {
    name: "bsc.bsv_examples/PortReplicator::PortReplicator2.bsv",
    fixture_dir: "testsuite/bsc.bsv_examples/PortReplicator",
    source: "PortReplicator2.bsv",
    fixtures: &["PortReplicator2.bsv", "AsyncROM.bs"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const RWIRE_REGISTER_Q: CompileCase = compile_pass_case!(
    "bsc.bsv_examples/rwire::RegisterQ.bsv",
    "testsuite/bsc.bsv_examples/rwire",
    "RegisterQ.bsv"
);

pub(super) const RWIRE_REGISTER: CompileCase = compile_verilog_fail_error_case!(
    "bsc.bsv_examples/rwire::Register.bsv",
    "testsuite/bsc.bsv_examples/rwire",
    "Register.bsv",
    "G0032"
);

pub(super) const BUG_ID_159_FIND_FIELDS: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID159::FindFields.bs",
    "testsuite/bsc.interra/bugs/bugID159",
    "FindFields.bs",
    "FindFields.bs.bsc-out.expected"
);

pub(super) const BUG_ID_159_TUPLE_CHK1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID159::TupleChk1.bs",
    "testsuite/bsc.interra/bugs/bugID159",
    "TupleChk1.bs",
    "TupleChk1.bs.bsc-out.expected"
);

pub(super) const BUG_ID_298_BUG: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID298::bug.bsv",
    "testsuite/bsc.interra/bugs/bugID298",
    "bug.bsv",
    "bug.bsv.bsc-out.expected"
);

pub(super) const BUG_ID_298_BUG1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID298::bug1.bsv",
    "testsuite/bsc.interra/bugs/bugID298",
    "bug1.bsv",
    "bug1.bsv.bsc-out.expected"
);

pub(super) const BUG_ID_334_DESIGN_SEQ: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID334::Design_seq.bsv",
    "testsuite/bsc.interra/bugs/bugID334",
    "Design_seq.bsv",
    "Design_seq.bsv.bsc-out.expected"
);

pub(super) const BUG_ID_334_DESIGN_CASE: CompileCase = compile_fail_golden_case!(
    "bsc.interra/bugs/bugID334::Design_case.bsv",
    "testsuite/bsc.interra/bugs/bugID334",
    "Design_case.bsv",
    "Design_case.bsv.bsc-out.expected"
);

pub(super) const BUG_ID_355_TEST: CompileCase = compile_pass_case!(
    "bsc.interra/bugs/bugID355::Test.bsv",
    "testsuite/bsc.interra/bugs/bugID355",
    "Test.bsv"
);

pub(super) const BUG_ID_355_DESIGN: CompileCase = compile_pass_case!(
    "bsc.interra/bugs/bugID355::Design.bsv",
    "testsuite/bsc.interra/bugs/bugID355",
    "Design.bsv"
);

pub(super) const E_BAD_EXPORT1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadExport::EBadExport1.bs",
    "testsuite/bsc.interra/messages/EBadExport",
    "EBadExport1.bs",
    "EBadExport1.bs.bsc-out.expected"
);

pub(super) const E_BAD_EXPORT2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadExport::EBadExport2.bs",
    "testsuite/bsc.interra/messages/EBadExport",
    "EBadExport2.bs",
    "EBadExport2.bs.bsc-out.expected"
);

pub(super) const E_BAD_LEX_CHAR1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadLexChar::EBadLexChar1.bs",
    "testsuite/bsc.interra/messages/EBadLexChar",
    "EBadLexChar1.bs",
    "EBadLexChar1.bs.bsc-out.expected"
);

pub(super) const E_BAD_LEX_CHAR2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadLexChar::EBadLexChar2.bs",
    "testsuite/bsc.interra/messages/EBadLexChar",
    "EBadLexChar2.bs",
    "EBadLexChar2.bs.bsc-out.expected"
);

pub(super) const E_BAD_MODULE_INTERFACE: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadModuleInterface::EBadModuleInterface.bs",
    "testsuite/bsc.interra/messages/EBadModuleInterface",
    "EBadModuleInterface.bs",
    "EBadModuleInterface.bs.bsc-out.expected"
);

pub(super) const E_BAD_MODULE_INTERFACE1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadModuleInterface::EBadModuleInterface1.bs",
    "testsuite/bsc.interra/messages/EBadModuleInterface",
    "EBadModuleInterface1.bs",
    "EBadModuleInterface1.bs.bsc-out.expected"
);

pub(super) const E_BAD_STRING_LIT: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadStringLit::EBadStringLit.bs",
    "testsuite/bsc.interra/messages/EBadStringLit",
    "EBadStringLit.bs",
    "EBadStringLit.bs.bsc-out.expected"
);

pub(super) const E_BAD_STRING_LIT2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EBadStringLit::EBadStringLit2.bs",
    "testsuite/bsc.interra/messages/EBadStringLit",
    "EBadStringLit2.bs",
    "EBadStringLit2.bs.bsc-out.expected"
);

pub(super) const E_CANNOT_DERIVE1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ECannotDerive::ECannotDerive1.bs",
    "testsuite/bsc.interra/messages/ECannotDerive",
    "ECannotDerive1.bs",
    "ECannotDerive1.bs.bsc-out.expected"
);

pub(super) const E_CANNOT_DERIVE2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ECannotDerive::ECannotDerive2.bs",
    "testsuite/bsc.interra/messages/ECannotDerive",
    "ECannotDerive2.bs",
    "ECannotDerive2.bs.bsc-out.expected"
);

pub(super) const E_LOCAL_REC: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ELocalRec::ELocalRec.bs",
    "testsuite/bsc.interra/messages/ELocalRec",
    "ELocalRec.bs",
    "ELocalRec.bs.bsc-out.expected"
);

pub(super) const E_LOCAL_REC1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/ELocalRec::ELocalRec1.bs",
    "testsuite/bsc.interra/messages/ELocalRec",
    "ELocalRec1.bs",
    "ELocalRec1.bs.bsc-out.expected"
);

pub(super) const E_MULTIPLE_DEF: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EMultipleDef::EMultipleDef.bs",
    "testsuite/bsc.interra/messages/EMultipleDef",
    "EMultipleDef.bs",
    "EMultipleDef.bs.bsc-out.expected"
);

pub(super) const E_MULTIPLE_DEF1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EMultipleDef::EMultipleDef1.bs",
    "testsuite/bsc.interra/messages/EMultipleDef",
    "EMultipleDef1.bs",
    "EMultipleDef1.bs.bsc-out.expected"
);

pub(super) const E_NOT_ALWAYS_READY_STRICT: CompileCase = CompileCase {
    name: "bsc.interra/messages/ENotAlwaysReady::ENotAlwaysReady.bs::strict",
    fixture_dir: "testsuite/bsc.interra/messages/ENotAlwaysReady",
    source: "ENotAlwaysReady.bs",
    fixtures: &["ENotAlwaysReady.bs"],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0006",
        count: 2,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("mkGCD"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const E_NOT_ALWAYS_READY_UNSAFE: CompileCase = CompileCase {
    name: "bsc.interra/messages/ENotAlwaysReady::ENotAlwaysReady.bs::unsafe-always-ready",
    fixture_dir: "testsuite/bsc.interra/messages/ENotAlwaysReady",
    source: "ENotAlwaysReady.bs",
    fixtures: &["ENotAlwaysReady.bs"],
    expectation: CompileExpectation::PassWithDiagnostic {
        kind: DiagnosticKind::Warning,
        tag: "G0006",
        count: 2,
    },
    golden: None,
    options: &["-unsafe-always-ready"],
    nodeps: false,
    mode: CompileMode::Verilog {
        module: Some("mkGCD"),
    },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const E_UNBOUND_CON: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundCon::EUnboundCon.bs",
    "testsuite/bsc.interra/messages/EUnboundCon",
    "EUnboundCon.bs",
    "EUnboundCon.bs.bsc-out.expected"
);

pub(super) const E_UNBOUND_CON1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundCon::EUnboundCon1.bs",
    "testsuite/bsc.interra/messages/EUnboundCon",
    "EUnboundCon1.bs",
    "EUnboundCon1.bs.bsc-out.expected"
);

pub(super) const CASES: &[CompileCase] = &[
    BUG_ID_198,
    BUG_ID_235,
    BUG_ID_238,
    BUG_ID_263,
    BUG_ID_277,
    BUG_ID_278,
    BUG_ID_279,
    BUG_ID_299,
    BUG_ID_340,
    PONG_TB_TOP_LEVEL_FRONTEND,
    PONG_TB_TOP_LEVEL_VERILOG,
    PORT_REPLICATOR,
    PORT_REPLICATOR_2,
    RWIRE_REGISTER_Q,
    RWIRE_REGISTER,
    BUG_ID_159_FIND_FIELDS,
    BUG_ID_159_TUPLE_CHK1,
    BUG_ID_298_BUG,
    BUG_ID_298_BUG1,
    BUG_ID_334_DESIGN_SEQ,
    BUG_ID_334_DESIGN_CASE,
    BUG_ID_355_TEST,
    BUG_ID_355_DESIGN,
    E_BAD_EXPORT1,
    E_BAD_EXPORT2,
    E_BAD_LEX_CHAR1,
    E_BAD_LEX_CHAR2,
    E_BAD_MODULE_INTERFACE,
    E_BAD_MODULE_INTERFACE1,
    E_BAD_STRING_LIT,
    E_BAD_STRING_LIT2,
    E_CANNOT_DERIVE1,
    E_CANNOT_DERIVE2,
    E_LOCAL_REC,
    E_LOCAL_REC1,
    E_MULTIPLE_DEF,
    E_MULTIPLE_DEF1,
    E_NOT_ALWAYS_READY_STRICT,
    E_NOT_ALWAYS_READY_UNSAFE,
    E_UNBOUND_CON,
    E_UNBOUND_CON1,
];
