//! Origin: `testsuite/bsc.syntax/bsv05/import-foreign/import-foreign.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, DiagnosticKind,
    GoldenExpectation, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bsv05/import-foreign";

macro_rules! import_foreign_case {
    (
        $constant:ident,
        $source:literal,
        fixtures: $fixtures:expr,
        assertions: $assertions:expr,
        expectation: $expectation:expr,
        golden: $golden:expr,
        options: $options:expr,
        mode: $mode:expr,
        requirement: $requirement:expr $(,)?
    ) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.syntax/bsv05/import-foreign::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: $fixtures,
            assertions: $assertions,
            expectation: $expectation,
            golden: $golden,
            options: $options,
            nodeps: false,
            mode: $mode,
            requirement: $requirement,
        };
    };
}

macro_rules! frontend_pass {
    ($constant:ident, $source:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! frontend_fail {
    ($constant:ident, $source:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Fail,
            golden: None,
            options: &[],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
    ($constant:ident, $source:literal, options: [$($option:literal),+ $(,)?]) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Fail,
            golden: None,
            options: &[$($option),+],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! frontend_error {
    ($constant:ident, $source:literal, $tag:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! frontend_fail_golden {
    ($constant:ident, $source:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source, concat!($source, ".bsc-out.expected")],
            assertions: &[],
            expectation: CompileExpectation::Fail,
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            options: &[],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! frontend_error_golden {
    ($constant:ident, $source:literal, $tag:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source, concat!($source, ".bsc-out.expected")],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            options: &[],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! frontend_pass_golden {
    ($constant:ident, $source:literal, $option:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source, concat!($source, ".bsc-out.expected")],
            assertions: &[],
            expectation: CompileExpectation::Pass,
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            options: &[$option],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! frontend_pass_regex {
    ($constant:ident, $source:literal, $option:literal, $pattern:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source],
            assertions: &[ArtifactAssertion::Text {
                path: concat!($source, ".bsc-out"),
                assertion: TextAssertion::Regex { pattern: $pattern },
            }],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[$option],
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        );
    };
}

macro_rules! verilog_pass_matches {
    ($constant:ident, $source:literal, $actual:literal) => {
        import_foreign_case!(
            $constant,
            $source,
            fixtures: &[$source, concat!($actual, ".expected")],
            assertions: &[
                ArtifactAssertion::ParsesAsSystemVerilog { path: $actual },
                ArtifactAssertion::Matches {
                    actual: $actual,
                    expected: concat!($actual, ".expected"),
                    normalization: ArtifactNormalization::Verilog,
                },
            ],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        );
    };
}

verilog_pass_matches!(
    IMPORT_FOREIGN_MODULE,
    "ImportForeignModule.bsv",
    "sysImportForeignModule.v"
);
verilog_pass_matches!(
    IMPORT_FOREIGN_MODULE_CLK_GATE,
    "ImportForeignModuleClkGate.bsv",
    "sysImportForeignModuleClkGate.v"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_BUG,
    "ImportForeignModuleBug.bsv",
    "P0005"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_GEN_1,
    "ImportForeignModuleGen1.bsv",
    "P0080"
);
import_foreign_case!(
    IMPORT_FOREIGN_MODULE_GEN_2,
    "ImportForeignModuleGen2.bsv",
    fixtures: &["ImportForeignModuleGen2.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "S0074",
        count: 1,
    },
    golden: None,
    options: &[],
    mode: CompileMode::Verilog {
        module: Some("vMkReg"),
    },
    requirement: Requirement::VerilogEnabled,
);
frontend_fail_golden!(
    IMPORT_FOREIGN_MODULE_PORT_ERRORS,
    "ImportForeignModule_PortErrors.bsv"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_CLOCK,
    "ImportForeignModule_DupDefaultClock.bsv",
    "P0139"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_CLOCK,
    "ImportForeignModule_DupDefaultInputClock.bsv",
    "P0152"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_CLOCK_2,
    "ImportForeignModule_DupDefaultInputClock2.bsv",
    "P0152"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_INPUT_CLOCK,
    "ImportForeignModule_DupInputClock.bsv",
    "P0152"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_OUTPUT_CLOCK,
    "ImportForeignModule_DupOutputClock.bsv",
    "P0152"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_INPUT_OUTPUT_CLOCK,
    "ImportForeignModule_DupInputOutputClock.bsv",
    "P0152"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_RESET,
    "ImportForeignModule_DupDefaultReset.bsv",
    "P0140"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_RESET,
    "ImportForeignModule_DupDefaultInputReset.bsv",
    "P0173"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_RESET_2,
    "ImportForeignModule_DupDefaultInputReset2.bsv",
    "P0173"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_INPUT_RESET,
    "ImportForeignModule_DupInputReset.bsv",
    "P0173"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_OUTPUT_RESET,
    "ImportForeignModule_DupOutputReset.bsv",
    "P0173"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_INPUT_OUTPUT_RESET,
    "ImportForeignModule_DupInputOutputReset.bsv",
    "P0173"
);
frontend_pass!(
    IMPORT_FOREIGN_MODULE_TWO_STMT_DEFAULT_CLOCK,
    "ImportForeignModule_TwoStmtDefaultClock.bsv"
);
frontend_pass!(
    IMPORT_FOREIGN_MODULE_TWO_STMT_DEFAULT_RESET,
    "ImportForeignModule_TwoStmtDefaultReset.bsv"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DECLARE_DEFAULT_NO_CLOCK,
    "ImportForeignModule_DeclareDefaultNoClock.bsv",
    "P0174"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DECLARE_INPUT_NO_CLOCK,
    "ImportForeignModule_DeclareInputNoClock.bsv",
    "P0174"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DECLARE_OUTPUT_NO_CLOCK,
    "ImportForeignModule_DeclareOutputNoClock.bsv",
    "P0174"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DECLARE_DEFAULT_NO_RESET,
    "ImportForeignModule_DeclareDefaultNoReset.bsv",
    "P0175"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DECLARE_INPUT_NO_RESET,
    "ImportForeignModule_DeclareInputNoReset.bsv",
    "P0175"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DECLARE_OUTPUT_NO_RESET,
    "ImportForeignModule_DeclareOutputNoReset.bsv",
    "P0175"
);
frontend_pass!(
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_NO_RESET,
    "ImportForeignModule_DefaultResetNoReset.bsv"
);
frontend_fail!(
    IMPORT_FOREIGN_MODULE_EMPTY_INPUT_CLOCK,
    "ImportForeignModule_EmptyInputClock.bsv"
);
frontend_fail!(
    IMPORT_FOREIGN_MODULE_EMPTY_INPUT_RESET,
    "ImportForeignModule_EmptyInputReset.bsv"
);
frontend_pass!(
    IMPORT_FOREIGN_MODULE_SUB_IFC,
    "ImportForeignModule_SubIfc.bsv"
);
import_foreign_case!(
    WBVI_ACTION_METHOD_NO_CLOCK,
    "WBVIActionMethodNoClock.bsv",
    fixtures: &["WBVIActionMethodNoClock.bsv"],
    assertions: &[],
    expectation: CompileExpectation::PassWithDiagnostic {
        kind: DiagnosticKind::Warning,
        tag: "P0172",
        count: 2,
    },
    golden: None,
    options: &[],
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_UNDECLARED_CLOCK_PORT,
    "ImportForeignModule_UndeclaredClock_Port.bsv",
    "P0134"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_UNDECLARED_RESET_PORT,
    "ImportForeignModule_UndeclaredReset_Port.bsv",
    "P0137"
);
frontend_pass_regex!(
    IMPORT_FOREIGN_MODULE_PORT_WITH_CLOCK,
    "ImportForeignModule_PortWithClock.bsv",
    "-disimplify",
    r"port \(D_IN, \[\]\) clocked_by \(aClk\) reset_by \(no_reset\);"
);
frontend_pass_regex!(
    IMPORT_FOREIGN_MODULE_PORT_WITH_CLOCK_RESET,
    "ImportForeignModule_PortWithClockReset.bsv",
    "-disimplify",
    r"port \(D_IN, \[\]\) clocked_by \(aClk\) reset_by \(aRst\);"
);
frontend_pass_regex!(
    IMPORT_FOREIGN_MODULE_PORT_WITH_PROP,
    "ImportForeignModule_PortWithProp.bsv",
    "-disimplify",
    r"port \(D_IN, \[reg\]\) clocked_by \(no_clock\) reset_by \(no_reset\);"
);
import_foreign_case!(
    IMPORT_FOREIGN_MODULE_PORT_WITH_BIND,
    "ImportForeignModule_PortWithBind.bsv",
    fixtures: &["ImportForeignModule_PortWithBind.bsv"],
    assertions: &[
        ArtifactAssertion::Text {
            path: "ImportForeignModule_PortWithBind.bsv.bsc-out",
            assertion: TextAssertion::Regex {
                pattern: r"Instantiating mkMod with argument 17",
            },
        },
        ArtifactAssertion::Text {
            path: "sysImportForeignModule_PortWithBind.v",
            assertion: TextAssertion::Regex {
                pattern: r"\.IN\(32\'d17\)",
            },
        },
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_PORT_DUP_CLOCKED_BY,
    "ImportForeignModule_PortDupClockedBy.bsv",
    "P0197"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_PORT_DUP_RESET_BY,
    "ImportForeignModule_PortDupResetBy.bsv",
    "P0197"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_METHOD_DUP_RESET_BY,
    "ImportForeignModule_MethodDupResetBy.bsv",
    "P0197"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_METHOD_DUP_ENABLE,
    "ImportForeignModule_MethodDupEnable.bsv",
    "P0197"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_METHOD_DUP_READY,
    "ImportForeignModule_MethodDupReady.bsv",
    "P0197"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_METHOD_DUP_CLOCKED_BY,
    "ImportForeignModule_MethodDupClockedBy.bsv",
    "P0197"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_SCHEDULE,
    "ImportForeignModule_DupSchedule.bsv",
    "P0201"
);
frontend_pass!(
    IMPORT_FOREIGN_MODULE_DUP_SCHEDULE_OK,
    "ImportForeignModule_DupScheduleOK.bsv"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_SCHEDULE_UNRELATED_CLOCKS,
    "ImportForeignModule_ScheduleUnrelatedClocks.bsv",
    "P0153"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_SCHEDULE_NO_CLOCK,
    "ImportForeignModule_ScheduleNoClock.bsv",
    "P0213"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_METHOD,
    "ImportForeignModule_DupMethod.bsv",
    "P0214"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_DUP_OUTPUT_INOUT,
    "ImportForeignModule_DupOutputInout.bsv",
    "P0214"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_BODY_END,
    "ImportForeignModule_BodyEnd.bsv",
    "P0005"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_VAR_EXPR,
    "ImportForeignModule_InputClock_NoName_VarExpr.bsv",
    "-dparsed"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_VAR_EXPR_DEFAULT_CLOCK,
    "ImportForeignModule_InputClock_NoName_VarExpr_DefaultClock.bsv",
    "P0174"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_VAR_EXPR_NO_CLOCK,
    "ImportForeignModule_InputClock_NoName_VarExpr_NoClock.bsv",
    "P0174"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_NON_VAR_EXPR_DUP,
    "ImportForeignModule_InputClock_NoName_NonVarExpr_Dup.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_EXPOSE_CUR_CLK,
    "ImportForeignModule_InputClock_NoName_ExposeCurClk.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_CLASH,
    "ImportForeignModule_InputClock_NoName_Clash.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_UNDERSCORE,
    "ImportForeignModule_InputClock_Underscore.bsv",
    "-dparsed"
);
frontend_fail!(
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_NOT_IN_SCOPE,
    "ImportForeignModule_InputClock_NoName_NotInScope.bsv",
    options: ["P0134"]
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_DEFAULT_CLOCK_NO_NAME_NON_VAR_EXPR,
    "ImportForeignModule_DefaultClock_NoName_NonVarExpr.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_DEFAULT_CLOCK_NO_NAME_INPUT_CLOCK_NO_NAME,
    "ImportForeignModule_DefaultClock_NoName_InputClock_NoName.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_DEFAULT_CLOCK_UNDERSCORE,
    "ImportForeignModule_DefaultClock_Underscore.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_VAR_EXPR,
    "ImportForeignModule_InputReset_NoName_VarExpr.bsv",
    "-dparsed"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_VAR_EXPR_DEFAULT_RESET,
    "ImportForeignModule_InputReset_NoName_VarExpr_DefaultReset.bsv",
    "P0175"
);
frontend_error!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_VAR_EXPR_NO_RESET,
    "ImportForeignModule_InputReset_NoName_VarExpr_NoReset.bsv",
    "P0175"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_NON_VAR_EXPR_DUP,
    "ImportForeignModule_InputReset_NoName_NonVarExpr_Dup.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_EXPOSE_CUR_RST,
    "ImportForeignModule_InputReset_NoName_ExposeCurRst.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_CLASH,
    "ImportForeignModule_InputReset_NoName_Clash.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_UNDERSCORE,
    "ImportForeignModule_InputReset_Underscore.bsv",
    "-dparsed"
);
frontend_fail!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_NOT_IN_SCOPE,
    "ImportForeignModule_InputReset_NoName_NotInScope.bsv",
    options: ["P0137"]
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_NO_NAME_NON_VAR_EXPR,
    "ImportForeignModule_DefaultReset_NoName_NonVarExpr.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_NO_NAME_INPUT_RESET_NO_NAME,
    "ImportForeignModule_DefaultReset_NoName_InputReset_NoName.bsv",
    "-dparsed"
);
frontend_pass_golden!(
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_UNDERSCORE,
    "ImportForeignModule_DefaultReset_Underscore.bsv",
    "-dparsed"
);
frontend_error_golden!(
    IMPORT_FOREIGN_MODULE_METHOD_CLOCKED_BY_UNDECLARED_CLK,
    "ImportForeignModule_Method_ClockedBy_UndeclaredClk.bsv",
    "P0134"
);
frontend_error_golden!(
    IMPORT_FOREIGN_MODULE_INPUT_RESET_CLOCKED_BY_UNDECLARED_CLK,
    "ImportForeignModule_InputReset_ClockedBy_UndeclaredClk.bsv",
    "P0134"
);

pub(super) const CASES: &[CompileCase] = &[
    IMPORT_FOREIGN_MODULE,
    IMPORT_FOREIGN_MODULE_CLK_GATE,
    IMPORT_FOREIGN_MODULE_BUG,
    IMPORT_FOREIGN_MODULE_GEN_1,
    IMPORT_FOREIGN_MODULE_GEN_2,
    IMPORT_FOREIGN_MODULE_PORT_ERRORS,
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_CLOCK,
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_CLOCK,
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_CLOCK_2,
    IMPORT_FOREIGN_MODULE_DUP_INPUT_CLOCK,
    IMPORT_FOREIGN_MODULE_DUP_OUTPUT_CLOCK,
    IMPORT_FOREIGN_MODULE_DUP_INPUT_OUTPUT_CLOCK,
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_RESET,
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_RESET,
    IMPORT_FOREIGN_MODULE_DUP_DEFAULT_INPUT_RESET_2,
    IMPORT_FOREIGN_MODULE_DUP_INPUT_RESET,
    IMPORT_FOREIGN_MODULE_DUP_OUTPUT_RESET,
    IMPORT_FOREIGN_MODULE_DUP_INPUT_OUTPUT_RESET,
    IMPORT_FOREIGN_MODULE_TWO_STMT_DEFAULT_CLOCK,
    IMPORT_FOREIGN_MODULE_TWO_STMT_DEFAULT_RESET,
    IMPORT_FOREIGN_MODULE_DECLARE_DEFAULT_NO_CLOCK,
    IMPORT_FOREIGN_MODULE_DECLARE_INPUT_NO_CLOCK,
    IMPORT_FOREIGN_MODULE_DECLARE_OUTPUT_NO_CLOCK,
    IMPORT_FOREIGN_MODULE_DECLARE_DEFAULT_NO_RESET,
    IMPORT_FOREIGN_MODULE_DECLARE_INPUT_NO_RESET,
    IMPORT_FOREIGN_MODULE_DECLARE_OUTPUT_NO_RESET,
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_NO_RESET,
    IMPORT_FOREIGN_MODULE_EMPTY_INPUT_CLOCK,
    IMPORT_FOREIGN_MODULE_EMPTY_INPUT_RESET,
    IMPORT_FOREIGN_MODULE_SUB_IFC,
    WBVI_ACTION_METHOD_NO_CLOCK,
    IMPORT_FOREIGN_MODULE_UNDECLARED_CLOCK_PORT,
    IMPORT_FOREIGN_MODULE_UNDECLARED_RESET_PORT,
    IMPORT_FOREIGN_MODULE_PORT_WITH_CLOCK,
    IMPORT_FOREIGN_MODULE_PORT_WITH_CLOCK_RESET,
    IMPORT_FOREIGN_MODULE_PORT_WITH_PROP,
    IMPORT_FOREIGN_MODULE_PORT_WITH_BIND,
    IMPORT_FOREIGN_MODULE_PORT_DUP_CLOCKED_BY,
    IMPORT_FOREIGN_MODULE_PORT_DUP_RESET_BY,
    IMPORT_FOREIGN_MODULE_METHOD_DUP_RESET_BY,
    IMPORT_FOREIGN_MODULE_METHOD_DUP_ENABLE,
    IMPORT_FOREIGN_MODULE_METHOD_DUP_READY,
    IMPORT_FOREIGN_MODULE_METHOD_DUP_CLOCKED_BY,
    IMPORT_FOREIGN_MODULE_DUP_SCHEDULE,
    IMPORT_FOREIGN_MODULE_DUP_SCHEDULE_OK,
    IMPORT_FOREIGN_MODULE_SCHEDULE_UNRELATED_CLOCKS,
    IMPORT_FOREIGN_MODULE_SCHEDULE_NO_CLOCK,
    IMPORT_FOREIGN_MODULE_DUP_METHOD,
    IMPORT_FOREIGN_MODULE_DUP_OUTPUT_INOUT,
    IMPORT_FOREIGN_MODULE_BODY_END,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_VAR_EXPR,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_VAR_EXPR_DEFAULT_CLOCK,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_VAR_EXPR_NO_CLOCK,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_NON_VAR_EXPR_DUP,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_EXPOSE_CUR_CLK,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_CLASH,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_UNDERSCORE,
    IMPORT_FOREIGN_MODULE_INPUT_CLOCK_NO_NAME_NOT_IN_SCOPE,
    IMPORT_FOREIGN_MODULE_DEFAULT_CLOCK_NO_NAME_NON_VAR_EXPR,
    IMPORT_FOREIGN_MODULE_DEFAULT_CLOCK_NO_NAME_INPUT_CLOCK_NO_NAME,
    IMPORT_FOREIGN_MODULE_DEFAULT_CLOCK_UNDERSCORE,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_VAR_EXPR,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_VAR_EXPR_DEFAULT_RESET,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_VAR_EXPR_NO_RESET,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_NON_VAR_EXPR_DUP,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_EXPOSE_CUR_RST,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_CLASH,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_UNDERSCORE,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_NO_NAME_NOT_IN_SCOPE,
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_NO_NAME_NON_VAR_EXPR,
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_NO_NAME_INPUT_RESET_NO_NAME,
    IMPORT_FOREIGN_MODULE_DEFAULT_RESET_UNDERSCORE,
    IMPORT_FOREIGN_MODULE_METHOD_CLOCKED_BY_UNDECLARED_CLK,
    IMPORT_FOREIGN_MODULE_INPUT_RESET_CLOCKED_BY_UNDECLARED_CLK,
];
