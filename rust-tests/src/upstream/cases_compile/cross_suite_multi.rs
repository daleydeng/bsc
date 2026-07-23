//! Origins:
//! - `testsuite/bsc.interra/messages/EUnboundField/EUnboundField.exp`
//! - `testsuite/bsc.interra/messages/EUnboundTyCon/EUnboundTyCon.exp`
//! - `testsuite/bsc.interra/messages/EUnboundVar/EUnboundVar.exp`
//! - `testsuite/bsc.interra/messages/EUnknownSize/EUnknownSize.exp`
//! - `testsuite/bsc.interra/messages/EWeakContext/EWeakContext.exp`
//! - `testsuite/bsc.interra/messages/WMissingField/WMissingField.exp`
//! - `testsuite/bsc.interra/preprocessorTestcases/resetall/resetall.exp`
//! - `testsuite/bsc.lib/Stmt/Misc/Misc.exp`
//! - `testsuite/bsc.misc/ruledrop/ruledrop.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/prefixEnable/prefixEnable.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/prefixPort/prefixPort.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/prefixReady/prefixReady.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/resultPort/resultPort.exp`
//! - `testsuite/bsc.names/portRenaming/invalidAttrs/prefix/prefix.exp`
//! - `testsuite/bsc.scheduler/preempts/preempts.exp`
//! - `testsuite/bsc.typechecker/deriving/scope/deriving_scope.exp`
//! - `testsuite/bsc.typechecker/typeclasses/examples/pipeline/pipeline.exp`
//! - `testsuite/bsc.evaluator/arguments/arguments.exp`
//! - `testsuite/bsc.evaluator/intsize/intsize.exp`
//! - `testsuite/bsc.interra/messages/EContextReduction/EContextReduction.exp`
//! - `testsuite/bsc.interra/messages/EContextReductionVar/EContextReductionVar.exp`
//! - `testsuite/bsc.interra/messages/EDupField/EDupField.exp`
//! - `testsuite/bsc.interra/messages/EUnifyKind/EUnifyKind.exp`
//! - `testsuite/bsc.interra/preprocessorTestcases/include/include.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/clock/clock.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/enablePort/enablePort.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/enableResult/enableResult.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/readyPort/readyPort.exp`
//! - `testsuite/bsc.scheduler/urgency/methods/methods.exp`
//! - `testsuite/bsc.typechecker/registers/registers.exp`

use crate::upstream::{
    CompileCase, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

macro_rules! pass_golden_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $golden:literal) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: CompileExpectation::Pass,
            golden: Some(GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        }
    };
}

macro_rules! frontend_verilog_option_error_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $tag:literal) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &["-verilog"],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        }
    };
}

pub(super) const E_UNBOUND_FIELD: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundField::EUnboundField.bs",
    "testsuite/bsc.interra/messages/EUnboundField",
    "EUnboundField.bs",
    "EUnboundField.bs.bsc-out.expected"
);

pub(super) const E_UNBOUND_FIELD1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundField::EUnboundField1.bs",
    "testsuite/bsc.interra/messages/EUnboundField",
    "EUnboundField1.bs",
    "EUnboundField1.bs.bsc-out.expected"
);

pub(super) const E_UNBOUND_TY_CON: CompileCase = compile_fail_error_case!(
    "bsc.interra/messages/EUnboundTyCon::EUnboundTyCon.bs",
    "testsuite/bsc.interra/messages/EUnboundTyCon",
    "EUnboundTyCon.bs",
    "T0007"
);

pub(super) const E_UNBOUND_TY_CON1: CompileCase = compile_fail_error_case!(
    "bsc.interra/messages/EUnboundTyCon::EUnboundTyCon1.bs",
    "testsuite/bsc.interra/messages/EUnboundTyCon",
    "EUnboundTyCon1.bs",
    "T0007"
);

pub(super) const E_UNBOUND_VAR: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundVar::EUnboundVar.bs",
    "testsuite/bsc.interra/messages/EUnboundVar",
    "EUnboundVar.bs",
    "EUnboundVar.bs.bsc-out.expected"
);

pub(super) const E_UNBOUND_VAR1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnboundVar::EUnboundVar1.bs",
    "testsuite/bsc.interra/messages/EUnboundVar",
    "EUnboundVar1.bs",
    "EUnboundVar1.bs.bsc-out.expected"
);

pub(super) const E_UNKNOWN_SIZE: CompileCase = compile_fail_error_golden_case!(
    "bsc.interra/messages/EUnknownSize::EUnknownSize.bs",
    "testsuite/bsc.interra/messages/EUnknownSize",
    "EUnknownSize.bs",
    "T0035",
    "EUnknownSize.bs.bsc-out.expected"
);

pub(super) const E_UNKNOWN_SIZE2: CompileCase = compile_fail_error_golden_case!(
    "bsc.interra/messages/EUnknownSize::EUnknownSize2.bs",
    "testsuite/bsc.interra/messages/EUnknownSize",
    "EUnknownSize2.bs",
    "T0035",
    "EUnknownSize2.bs.bsc-out.expected"
);

pub(super) const E_WEAK_CONTEXT: CompileCase = compile_pass_case!(
    "bsc.interra/messages/EWeakContext::EWeakContext.bs",
    "testsuite/bsc.interra/messages/EWeakContext",
    "EWeakContext.bs"
);

pub(super) const E_WEAK_CONTEXT2: CompileCase = compile_fail_error_case!(
    "bsc.interra/messages/EWeakContext::EWeakContext2.bs",
    "testsuite/bsc.interra/messages/EWeakContext",
    "EWeakContext2.bs",
    "T0030"
);

pub(super) const W_MISSING_FIELD: CompileCase = pass_golden_case!(
    "bsc.interra/messages/WMissingField::WMissingField.bs",
    "testsuite/bsc.interra/messages/WMissingField",
    "WMissingField.bs",
    "WMissingField.bs.bsc-out.expected"
);

pub(super) const W_MISSING_FIELD1: CompileCase = pass_golden_case!(
    "bsc.interra/messages/WMissingField::WMissingField1.bs",
    "testsuite/bsc.interra/messages/WMissingField",
    "WMissingField1.bs",
    "WMissingField1.bs.bsc-out.expected"
);

pub(super) const RESETALL_NEG_PACKAGE_LEVEL: CompileCase = pass_golden_case!(
    "bsc.interra/preprocessorTestcases/resetall::Ifdef_NegPackageLevel.bsv",
    "testsuite/bsc.interra/preprocessorTestcases/resetall",
    "Ifdef_NegPackageLevel.bsv",
    "Ifdef_NegPackageLevel.bsv.bsc-out.expected"
);

pub(super) const RESETALL_PACKAGE_LEVEL: CompileCase = pass_golden_case!(
    "bsc.interra/preprocessorTestcases/resetall::Ifdef_PackageLevel.bsv",
    "testsuite/bsc.interra/preprocessorTestcases/resetall",
    "Ifdef_PackageLevel.bsv",
    "Ifdef_PackageLevel.bsv.bsc-out.expected"
);

pub(super) const STMT_MISC_CASE_STMT: CompileCase = compile_fail_error_case!(
    "bsc.lib/Stmt/Misc::CaseStmt.bsv",
    "testsuite/bsc.lib/Stmt/Misc",
    "CaseStmt.bsv",
    "P0218"
);

pub(super) const STMT_MISC_CASE_MATCHES_STMT: CompileCase = compile_fail_error_case!(
    "bsc.lib/Stmt/Misc::CaseMatchesStmt.bsv",
    "testsuite/bsc.lib/Stmt/Misc",
    "CaseMatchesStmt.bsv",
    "P0218"
);

pub(super) const RULEDROP_WARN_FALSE: CompileCase = compile_verilog_pass_warning_case!(
    "bsc.misc/ruledrop::WarnFalse.bsv",
    "testsuite/bsc.misc/ruledrop",
    "WarnFalse.bsv",
    "G0023"
);

pub(super) const RULEDROP_WARN_EMPTY: CompileCase = compile_verilog_pass_warning_case!(
    "bsc.misc/ruledrop::WarnEmpty.bsv",
    "testsuite/bsc.misc/ruledrop",
    "WarnEmpty.bsv",
    "G0023"
);

pub(super) const PREFIX_ENABLE_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/prefixEnable::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/prefixEnable",
    "Test01.bsv",
    "G0055"
);

pub(super) const PREFIX_ENABLE_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/prefixEnable::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/prefixEnable",
    "Test02.bsv",
    "G0055"
);

pub(super) const PREFIX_PORT_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/prefixPort::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/prefixPort",
    "Test01.bsv",
    "G0055"
);

pub(super) const PREFIX_PORT_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/prefixPort::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/prefixPort",
    "Test02.bsv",
    "G0055"
);

pub(super) const PREFIX_READY_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/prefixReady::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/prefixReady",
    "Test01.bsv",
    "G0055"
);

pub(super) const PREFIX_READY_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/prefixReady::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/prefixReady",
    "Test02.bsv",
    "G0055"
);

pub(super) const RESULT_PORT_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/resultPort::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/resultPort",
    "Test01.bsv",
    "G0055"
);

pub(super) const RESULT_PORT_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/resultPort::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/resultPort",
    "Test02.bsv",
    "G0055"
);

pub(super) const INVALID_PREFIX_DUPLICATE_ATTR: CompileCase = compile_verilog_fail_error_case!(
    "bsc.names/portRenaming/invalidAttrs/prefix::DuplicateAttr.bsv",
    "testsuite/bsc.names/portRenaming/invalidAttrs/prefix",
    "DuplicateAttr.bsv",
    "P0158"
);

pub(super) const INVALID_PREFIX_WRONG_LOC_INTERFACE: CompileCase = compile_verilog_fail_error_case!(
    "bsc.names/portRenaming/invalidAttrs/prefix::WrongLoc_Interface.bsv",
    "testsuite/bsc.names/portRenaming/invalidAttrs/prefix",
    "WrongLoc_Interface.bsv",
    "P0155"
);

pub(super) const PREEMPTS_WARNING: CompileCase = CompileCase {
    name: "bsc.scheduler/preempts::Preempts.bsv",
    fixture_dir: "testsuite/bsc.scheduler/preempts",
    source: "Preempts.bsv",
    fixtures: &["Preempts.bsv"],
    expectation: CompileExpectation::PassWithDiagnostic {
        kind: DiagnosticKind::Warning,
        tag: "G0021",
        count: 2,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const PREEMPTS_SINGLETON: CompileCase = compile_verilog_fail_error_case!(
    "bsc.scheduler/preempts::PreemptsSingleton.bsv",
    "testsuite/bsc.scheduler/preempts",
    "PreemptsSingleton.bsv",
    "P0005"
);

pub(super) const DERIVING_ABSTRACT_LIST: CompileCase = compile_pass_case!(
    "bsc.typechecker/deriving/scope::AbstractList.bs",
    "testsuite/bsc.typechecker/deriving/scope",
    "AbstractList.bs"
);

pub(super) const DERIVING_ABSTRACT_MAYBE: CompileCase = compile_pass_case!(
    "bsc.typechecker/deriving/scope::AbstractMaybe.bs",
    "testsuite/bsc.typechecker/deriving/scope",
    "AbstractMaybe.bs"
);

pub(super) const PIPELINE_TB1: CompileCase = CompileCase {
    name: "bsc.typechecker/typeclasses/examples/pipeline::Tb1.bsv",
    fixture_dir: "testsuite/bsc.typechecker/typeclasses/examples/pipeline",
    source: "Tb1.bsv",
    fixtures: &["Tb1.bsv", "Pipeline1.bsv"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const PIPELINE_TB2: CompileCase = CompileCase {
    name: "bsc.typechecker/typeclasses/examples/pipeline::Tb2.bsv",
    fixture_dir: "testsuite/bsc.typechecker/typeclasses/examples/pipeline",
    source: "Tb2.bsv",
    fixtures: &["Tb2.bsv", "Pipeline2.bsv"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const ARGUMENTS_LET_PORT_ARG_BH: CompileCase = compile_verilog_pass_case!(
    "bsc.evaluator/arguments::LetPortArg.bs",
    "testsuite/bsc.evaluator/arguments",
    "LetPortArg.bs"
);

pub(super) const ARGUMENTS_LET_PORT_ARG_BSV: CompileCase = compile_verilog_pass_case!(
    "bsc.evaluator/arguments::LetPortArg.bsv",
    "testsuite/bsc.evaluator/arguments",
    "LetPortArg.bsv"
);

pub(super) const ARGUMENTS_PORT_ARG_IMPL_COND: CompileCase = compile_verilog_fail_error_case!(
    "bsc.evaluator/arguments::PortArg_ImplCond.bsv",
    "testsuite/bsc.evaluator/arguments",
    "PortArg_ImplCond.bsv",
    "G0081"
);

pub(super) const INTSIZE_OK: CompileCase = compile_verilog_pass_case!(
    "bsc.evaluator/intsize::intok.bsv",
    "testsuite/bsc.evaluator/intsize",
    "intok.bsv"
);

pub(super) const INTSIZE_BAD1: CompileCase = compile_verilog_fail_error_case!(
    "bsc.evaluator/intsize::intbad1.bsv",
    "testsuite/bsc.evaluator/intsize",
    "intbad1.bsv",
    "T0051"
);

pub(super) const INTSIZE_BAD2: CompileCase = compile_verilog_fail_error_case!(
    "bsc.evaluator/intsize::intbad2.bsv",
    "testsuite/bsc.evaluator/intsize",
    "intbad2.bsv",
    "T0051"
);

pub(super) const E_CONTEXT_REDUCTION: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EContextReduction::EContextReduction.bs",
    "testsuite/bsc.interra/messages/EContextReduction",
    "EContextReduction.bs",
    "EContextReduction.bs.bsc-out.expected"
);

pub(super) const E_CONTEXT_REDUCTION1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EContextReduction::EContextReduction1.bs",
    "testsuite/bsc.interra/messages/EContextReduction",
    "EContextReduction1.bs",
    "EContextReduction1.bs.bsc-out.expected"
);

pub(super) const E_CONTEXT_REDUCTION2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EContextReduction::EContextReduction2.bs",
    "testsuite/bsc.interra/messages/EContextReduction",
    "EContextReduction2.bs",
    "EContextReduction2.bs.bsc-out.expected"
);

pub(super) const E_CONTEXT_REDUCTION_VAR: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EContextReductionVar::EContextReductionVar.bs",
    "testsuite/bsc.interra/messages/EContextReductionVar",
    "EContextReductionVar.bs",
    "EContextReductionVar.bs.bsc-out.expected"
);

pub(super) const E_CONTEXT_REDUCTION_VAR1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EContextReductionVar::EContextReductionVar1.bs",
    "testsuite/bsc.interra/messages/EContextReductionVar",
    "EContextReductionVar1.bs",
    "EContextReductionVar1.bs.bsc-out.expected"
);

pub(super) const E_CONTEXT_REDUCTION_VAR2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EContextReductionVar::EContextReductionVar2.bs",
    "testsuite/bsc.interra/messages/EContextReductionVar",
    "EContextReductionVar2.bs",
    "EContextReductionVar2.bs.bsc-out.expected"
);

pub(super) const E_DUP_FIELD1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EDupField::EDupField1.bs",
    "testsuite/bsc.interra/messages/EDupField",
    "EDupField1.bs",
    "EDupField1.bs.bsc-out.expected"
);

pub(super) const E_DUP_FIELD2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EDupField::EDupField2.bs",
    "testsuite/bsc.interra/messages/EDupField",
    "EDupField2.bs",
    "EDupField2.bs.bsc-out.expected"
);

pub(super) const E_DUP_FIELD3: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EDupField::EDupField3.bs",
    "testsuite/bsc.interra/messages/EDupField",
    "EDupField3.bs",
    "EDupField3.bs.bsc-out.expected"
);

pub(super) const E_UNIFY_KIND: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnifyKind::EUnifyKind.bs",
    "testsuite/bsc.interra/messages/EUnifyKind",
    "EUnifyKind.bs",
    "EUnifyKind.bs.bsc-out.expected"
);

pub(super) const E_UNIFY_KIND1: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnifyKind::EUnifyKind1.bs",
    "testsuite/bsc.interra/messages/EUnifyKind",
    "EUnifyKind1.bs",
    "EUnifyKind1.bs.bsc-out.expected"
);

pub(super) const E_UNIFY_KIND2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/messages/EUnifyKind::EUnifyKind2.bs",
    "testsuite/bsc.interra/messages/EUnifyKind",
    "EUnifyKind2.bs",
    "EUnifyKind2.bs.bsc-out.expected"
);

pub(super) const INCLUDE_TEST1: CompileCase = CompileCase {
    name: "bsc.interra/preprocessorTestcases/include::include_Test1.bsv",
    fixture_dir: "testsuite/bsc.interra/preprocessorTestcases/include",
    source: "include_Test1.bsv",
    fixtures: &[
        "include_Test1.bsv",
        "myDef",
        "include_Test1.bsv.bsc-out.expected",
    ],
    expectation: CompileExpectation::Fail,
    golden: Some(GoldenExpectation {
        expected: "include_Test1.bsv.bsc-out.expected",
    }),
    options: &[],
    nodeps: false,
    mode: CompileMode::Frontend,
    requirement: Requirement::Always,
};

pub(super) const INCLUDE_TEST2: CompileCase = compile_fail_golden_case!(
    "bsc.interra/preprocessorTestcases/include::include_Test2.bsv",
    "testsuite/bsc.interra/preprocessorTestcases/include",
    "include_Test2.bsv",
    "include_Test2.bsv.bsc-out.expected"
);

pub(super) const INCLUDE_TEST3: CompileCase = compile_fail_golden_case!(
    "bsc.interra/preprocessorTestcases/include::include_Test3.bsv",
    "testsuite/bsc.interra/preprocessorTestcases/include",
    "include_Test3.bsv",
    "include_Test3.bsv.bsc-out.expected"
);

pub(super) const CLOCK_ENABLE: CompileCase = compile_verilog_fail_error_case!(
    "bsc.names/portRenaming/conflicts/clock::ClockEnable.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/clock",
    "ClockEnable.bsv",
    "G0055"
);

pub(super) const CLOCK_RESULT: CompileCase = compile_verilog_fail_error_case!(
    "bsc.names/portRenaming/conflicts/clock::ClockResult.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/clock",
    "ClockResult.bsv",
    "G0055"
);

pub(super) const GATE_ENABLE: CompileCase = compile_verilog_fail_error_case!(
    "bsc.names/portRenaming/conflicts/clock::GateEnable.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/clock",
    "GateEnable.bsv",
    "G0055"
);

pub(super) const ENABLE_PORT_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/enablePort::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/enablePort",
    "Test01.bsv",
    "G0055"
);

pub(super) const ENABLE_PORT_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/enablePort::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/enablePort",
    "Test02.bsv",
    "G0055"
);

pub(super) const ENABLE_PORT_TEST03: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/enablePort::Test03.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/enablePort",
    "Test03.bsv",
    "G0055"
);

pub(super) const ENABLE_RESULT_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/enableResult::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/enableResult",
    "Test01.bsv",
    "G0055"
);

pub(super) const ENABLE_RESULT_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/enableResult::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/enableResult",
    "Test02.bsv",
    "G0055"
);

pub(super) const ENABLE_RESULT_TEST03: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/enableResult::Test03.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/enableResult",
    "Test03.bsv",
    "G0055"
);

pub(super) const READY_PORT_TEST01: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/readyPort::Test01.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/readyPort",
    "Test01.bsv",
    "G0055"
);

pub(super) const READY_PORT_TEST02: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/readyPort::Test02.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/readyPort",
    "Test02.bsv",
    "G0055"
);

pub(super) const READY_PORT_TEST03: CompileCase = frontend_verilog_option_error_case!(
    "bsc.names/portRenaming/conflicts/readyPort::Test03.bsv",
    "testsuite/bsc.names/portRenaming/conflicts/readyPort",
    "Test03.bsv",
    "G0055"
);

pub(super) const URGENCY_GET: CompileCase = CompileCase {
    name: "bsc.scheduler/urgency/methods::GetUrgency.bsv",
    fixture_dir: "testsuite/bsc.scheduler/urgency/methods",
    source: "GetUrgency.bsv",
    fixtures: &["GetUrgency.bsv", "BypassFIFO.bsv"],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const URGENCY_PUT: CompileCase = compile_verilog_pass_case!(
    "bsc.scheduler/urgency/methods::PutUrgency.bsv",
    "testsuite/bsc.scheduler/urgency/methods",
    "PutUrgency.bsv"
);

pub(super) const URGENCY_WARN_METHOD: CompileCase = compile_verilog_pass_warning_case!(
    "bsc.scheduler/urgency/methods::WarnMethodUrgency.bsv",
    "testsuite/bsc.scheduler/urgency/methods",
    "WarnMethodUrgency.bsv",
    "G0010"
);

pub(super) const REGISTERS_METHOD_CALL_WITH_READ: CompileCase = compile_pass_case!(
    "bsc.typechecker/registers::MethodCallOnIfcWithRead.bsv",
    "testsuite/bsc.typechecker/registers",
    "MethodCallOnIfcWithRead.bsv"
);

pub(super) const REGISTERS_IMPORT_WITH_READ_METHOD: CompileCase = compile_pass_case!(
    "bsc.typechecker/registers::ImportWithReadMethod.bsv",
    "testsuite/bsc.typechecker/registers",
    "ImportWithReadMethod.bsv"
);

pub(super) const REGISTERS_AS_IFC: CompileCase = compile_pass_case!(
    "bsc.typechecker/registers::AsIfc.bsv",
    "testsuite/bsc.typechecker/registers",
    "AsIfc.bsv"
);

pub(super) const CASES: &[CompileCase] = &[
    E_UNBOUND_FIELD,
    E_UNBOUND_FIELD1,
    E_UNBOUND_TY_CON,
    E_UNBOUND_TY_CON1,
    E_UNBOUND_VAR,
    E_UNBOUND_VAR1,
    E_UNKNOWN_SIZE,
    E_UNKNOWN_SIZE2,
    E_WEAK_CONTEXT,
    E_WEAK_CONTEXT2,
    W_MISSING_FIELD,
    W_MISSING_FIELD1,
    RESETALL_NEG_PACKAGE_LEVEL,
    RESETALL_PACKAGE_LEVEL,
    STMT_MISC_CASE_STMT,
    STMT_MISC_CASE_MATCHES_STMT,
    RULEDROP_WARN_FALSE,
    RULEDROP_WARN_EMPTY,
    PREFIX_ENABLE_TEST01,
    PREFIX_ENABLE_TEST02,
    PREFIX_PORT_TEST01,
    PREFIX_PORT_TEST02,
    PREFIX_READY_TEST01,
    PREFIX_READY_TEST02,
    RESULT_PORT_TEST01,
    RESULT_PORT_TEST02,
    INVALID_PREFIX_DUPLICATE_ATTR,
    INVALID_PREFIX_WRONG_LOC_INTERFACE,
    PREEMPTS_WARNING,
    PREEMPTS_SINGLETON,
    DERIVING_ABSTRACT_LIST,
    DERIVING_ABSTRACT_MAYBE,
    PIPELINE_TB1,
    PIPELINE_TB2,
    ARGUMENTS_LET_PORT_ARG_BH,
    ARGUMENTS_LET_PORT_ARG_BSV,
    ARGUMENTS_PORT_ARG_IMPL_COND,
    INTSIZE_OK,
    INTSIZE_BAD1,
    INTSIZE_BAD2,
    E_CONTEXT_REDUCTION,
    E_CONTEXT_REDUCTION1,
    E_CONTEXT_REDUCTION2,
    E_CONTEXT_REDUCTION_VAR,
    E_CONTEXT_REDUCTION_VAR1,
    E_CONTEXT_REDUCTION_VAR2,
    E_DUP_FIELD1,
    E_DUP_FIELD2,
    E_DUP_FIELD3,
    E_UNIFY_KIND,
    E_UNIFY_KIND1,
    E_UNIFY_KIND2,
    INCLUDE_TEST1,
    INCLUDE_TEST2,
    INCLUDE_TEST3,
    CLOCK_ENABLE,
    CLOCK_RESULT,
    GATE_ENABLE,
    ENABLE_PORT_TEST01,
    ENABLE_PORT_TEST02,
    ENABLE_PORT_TEST03,
    ENABLE_RESULT_TEST01,
    ENABLE_RESULT_TEST02,
    ENABLE_RESULT_TEST03,
    READY_PORT_TEST01,
    READY_PORT_TEST02,
    READY_PORT_TEST03,
    URGENCY_GET,
    URGENCY_PUT,
    URGENCY_WARN_METHOD,
    REGISTERS_METHOD_CALL_WITH_READ,
    REGISTERS_IMPORT_WITH_READ_METHOD,
    REGISTERS_AS_IFC,
];
