//! Origins:
//! - `testsuite/bsc.typechecker/literals/literals.exp`
//! - `testsuite/bsc.real/evaluator/errors/errors.exp`
//! - `testsuite/bsc.real/evaluator/evaluator.exp`
//! - `testsuite/bsc.real/parser/parser.exp`
//! - `testsuite/bsc.evaluator/literal/literal.exp`
//! - `testsuite/bsc.evaluator/prims/module_fix/module_fix.exp`
//! - `testsuite/bsc.evaluator/prims/name/name.exp`
//! - `testsuite/bsc.evaluator/messages/message.exp`
//! - `testsuite/bsc.interra/Path_Analysis/Input_Output_Path/Input_Output_Path.exp`
//! - `testsuite/bsc.misc/mul/mul.exp`
//! - `testsuite/bsc.syntax/bsv05/statename/statename.exp`
//! - `testsuite/bsc.lib/IsModule/is_module.exp`
//! - `testsuite/bsc.bugs/perf-creg-blowup/perf-creg-blowup.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, DiagnosticKind,
    GoldenExpectation, Requirement, TextAssertion,
};

macro_rules! compile_case {
    ($name:expr, $fixture_dir:expr, $source:literal, $fixtures:expr, $assertions:expr, $expectation:expr, $golden:expr, $options:expr, $mode:expr, $requirement:expr) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            assertions: $assertions,
            expectation: $expectation,
            golden: $golden,
            options: $options,
            nodeps: false,
            mode: $mode,
            requirement: $requirement,
        }
    };
}

macro_rules! frontend_pass {
    ($prefix:literal, $fixture_dir:expr, $source:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            &[],
            CompileExpectation::Pass,
            None,
            &[],
            CompileMode::Frontend,
            Requirement::Always
        )
    };
}

macro_rules! frontend_error {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            &[],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            None,
            &[],
            CompileMode::Frontend,
            Requirement::Always
        )
    };
}

macro_rules! verilog_pass {
    ($prefix:literal, $fixture_dir:expr, $source:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            &[],
            CompileExpectation::Pass,
            None,
            &[],
            CompileMode::Verilog { module: None },
            Requirement::VerilogEnabled
        )
    };
}

macro_rules! verilog_pass_golden {
    ($prefix:literal, $fixture_dir:expr, $source:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source, concat!($source, ".bsc-vcomp-out.expected")],
            &[],
            CompileExpectation::Pass,
            Some(GoldenExpectation {
                expected: concat!($source, ".bsc-vcomp-out.expected"),
            }),
            &[],
            CompileMode::Verilog { module: None },
            Requirement::VerilogEnabled
        )
    };
}

macro_rules! verilog_error {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal) => {
        verilog_error!($prefix, $fixture_dir, $source, $tag, 1)
    };
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal, $count:expr) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            &[],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            None,
            &[],
            CompileMode::Verilog { module: None },
            Requirement::VerilogEnabled
        )
    };
}

macro_rules! verilog_error_golden {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source, concat!($source, ".bsc-vcomp-out.expected")],
            &[],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            Some(GoldenExpectation {
                expected: concat!($source, ".bsc-vcomp-out.expected"),
            }),
            &[],
            CompileMode::Verilog { module: None },
            Requirement::VerilogEnabled
        )
    };
}

macro_rules! line_count {
    ($path:literal, $text:literal, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $text,
                count: $count,
            },
        }
    };
}

macro_rules! golden_match {
    ($actual:literal, $expected:literal) => {
        ArtifactAssertion::Matches {
            actual: $actual,
            expected: $expected,
            normalization: ArtifactNormalization::GoldenOutput,
        }
    };
}

const LITERALS_DIR: &str = "testsuite/bsc.typechecker/literals";
const REAL_ERRORS_DIR: &str = "testsuite/bsc.real/evaluator/errors";
const REAL_EVALUATOR_DIR: &str = "testsuite/bsc.real/evaluator";
const REAL_PARSER_DIR: &str = "testsuite/bsc.real/parser";
const EVALUATOR_LITERAL_DIR: &str = "testsuite/bsc.evaluator/literal";
const MODULE_FIX_DIR: &str = "testsuite/bsc.evaluator/prims/module_fix";
const NAME_DIR: &str = "testsuite/bsc.evaluator/prims/name";
const MESSAGES_DIR: &str = "testsuite/bsc.evaluator/messages";
const PATH_DIR: &str = "testsuite/bsc.interra/Path_Analysis/Input_Output_Path";
const MUL_DIR: &str = "testsuite/bsc.misc/mul";
const STATENAME_DIR: &str = "testsuite/bsc.syntax/bsv05/statename";
const IS_MODULE_DIR: &str = "testsuite/bsc.lib/IsModule";
const CREG_BLOWUP_DIR: &str = "testsuite/bsc.bugs/perf-creg-blowup";

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.typechecker/literals/literals.exp
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "LiteralInTuple.bs"
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "BinaryLiterals.bs"
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "BinaryLiteral.bsv"
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "DefaultingLiteral.bs"
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "DefaultingRealLiteral.bs"
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "DefaultingSizedLiteral.bsv"
    ),
    frontend_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_TooLarge.bsv",
        "T0132"
    ),
    frontend_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_TooLarge_CaseMatches.bsv",
        "T0132"
    ),
    frontend_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_TooLarge_Case.bsv",
        "T0132"
    ),
    frontend_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_TooLargeNeg.bsv",
        "T0132"
    ),
    verilog_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "Literal_TooLarge.bsv",
        "T0051"
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_Bounds.bsv"
    ),
    compile_case!(
        "bsc.typechecker/literals::SizedLiteral_Neg.bsv",
        LITERALS_DIR,
        "SizedLiteral_Neg.bsv",
        &["SizedLiteral_Neg.bsv"],
        &[line_count!("sysSizedLiteral_Neg.v", "11'h7FF", 1)],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.typechecker/literals::Literal_Neg.bsv",
        LITERALS_DIR,
        "Literal_Neg.bsv",
        &["Literal_Neg.bsv"],
        &[line_count!("sysLiteral_Neg.v", "11'h7FF", 1)],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    frontend_pass!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_ZeroSize.bsv"
    ),
    frontend_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_ZeroSize_TooLarge.bsv",
        "T0132"
    ),
    frontend_error!(
        "bsc.typechecker/literals",
        LITERALS_DIR,
        "SizedLiteral_ZeroSize_TypeMismatch.bsv",
        "T0060"
    ),
    frontend_pass!("bsc.typechecker/literals", LITERALS_DIR, "LeadingMinus.bsv"),
    // testsuite/bsc.real/evaluator/errors/errors.exp
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "DivZero.bsv",
        "G0059"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "LogBaseZero.bsv",
        "G0110"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "LogBaseNegative.bsv",
        "G0110"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "LogBaseOneOne.bsv",
        "G0111"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "SqrtNegative.bsv",
        "G0112"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "AddPosInfNegInf.bsv",
        "G0113"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "AddNegInfPosInf.bsv",
        "G0113"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "SubPosInf.bsv",
        "G0113"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "SubNegInf.bsv",
        "G0113"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "MulPosZeroPosInf.bsv",
        "G0114"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "MulNegZeroPosInf.bsv",
        "G0114"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "MulPosZeroNegInf.bsv",
        "G0114"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "MulNegZeroNegInf.bsv",
        "G0114"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "BitsToNaN1.bsv",
        "G0115"
    ),
    verilog_error_golden!(
        "bsc.real/evaluator/errors",
        REAL_ERRORS_DIR,
        "BitsToNaN2.bsv",
        "G0115"
    ),
    // testsuite/bsc.real/evaluator/evaluator.exp
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "LiteralEqOrd.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Arith.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Logs.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Exps.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Bits.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Sqrt.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Rounds.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "IsInfinite.bsv"),
    verilog_pass_golden!(
        "bsc.real/evaluator",
        REAL_EVALUATOR_DIR,
        "IsNegativeZero.bsv"
    ),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Introspect.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Constants.bsv"),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "AddSubInf.bsv"),
    verilog_pass_golden!(
        "bsc.real/evaluator",
        REAL_EVALUATOR_DIR,
        "IntrospectInfinite.bsv"
    ),
    verilog_pass_golden!(
        "bsc.real/evaluator",
        REAL_EVALUATOR_DIR,
        "RoundInfinite.bsv"
    ),
    verilog_pass_golden!("bsc.real/evaluator", REAL_EVALUATOR_DIR, "Zero.bsv"),
    // testsuite/bsc.real/parser/parser.exp
    verilog_pass_golden!(
        "bsc.real/parser",
        REAL_PARSER_DIR,
        "FractionalLeadingZeros.bsv"
    ),
    verilog_pass_golden!("bsc.real/parser", REAL_PARSER_DIR, "LargeReal.bsv"),
    verilog_pass_golden!("bsc.real/parser", REAL_PARSER_DIR, "Classic.bs"),
    // testsuite/bsc.evaluator/literal/literal.exp
    verilog_error!(
        "bsc.evaluator/literal",
        EVALUATOR_LITERAL_DIR,
        "NegativeUInt.bsv",
        "T0051"
    ),
    verilog_error!(
        "bsc.evaluator/literal",
        EVALUATOR_LITERAL_DIR,
        "NegativeIntErr.bsv",
        "T0051"
    ),
    verilog_error!(
        "bsc.evaluator/literal",
        EVALUATOR_LITERAL_DIR,
        "PositiveIntErr.bsv",
        "T0051"
    ),
    verilog_error!(
        "bsc.evaluator/literal",
        EVALUATOR_LITERAL_DIR,
        "ConcatTestOK.bsv",
        "T0035"
    ),
    verilog_error!(
        "bsc.evaluator/literal",
        EVALUATOR_LITERAL_DIR,
        "ConcatTestFail.bsv",
        "T0035"
    ),
    compile_case!(
        "bsc.evaluator/literal::Invalid_Bit_Bin.bsv",
        EVALUATOR_LITERAL_DIR,
        "Invalid_Bit_Bin.bsv",
        &["Invalid_Bit_Bin.bsv"],
        &[line_count!("Invalid_Bit_Bin.bsv.bsc-out", "'b10101", 1)],
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "T0051",
            count: 1
        },
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.evaluator/literal::Invalid_Bit_Hex.bsv",
        EVALUATOR_LITERAL_DIR,
        "Invalid_Bit_Hex.bsv",
        &["Invalid_Bit_Hex.bsv"],
        &[line_count!("Invalid_Bit_Hex.bsv.bsc-out", "'hFF", 1)],
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "T0051",
            count: 1
        },
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.evaluator/literal::Invalid_Int_Oct.bsv",
        EVALUATOR_LITERAL_DIR,
        "Invalid_Int_Oct.bsv",
        &["Invalid_Int_Oct.bsv"],
        &[line_count!("Invalid_Int_Oct.bsv.bsc-out", "'o777", 1)],
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "T0051",
            count: 1
        },
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.evaluator/literal::Invalid_UInt_Dec.bsv",
        EVALUATOR_LITERAL_DIR,
        "Invalid_UInt_Dec.bsv",
        &["Invalid_UInt_Dec.bsv"],
        &[line_count!("Invalid_UInt_Dec.bsv.bsc-out", "256", 1)],
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "T0051",
            count: 1
        },
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.evaluator/prims/module_fix/module_fix.exp
    verilog_error!(
        "bsc.evaluator/prims/module_fix",
        MODULE_FIX_DIR,
        "ModLoop.bsv",
        "G0104"
    ),
    // testsuite/bsc.evaluator/prims/name/name.exp
    compile_case!(
        "bsc.evaluator/prims/name::MakeName.bsv",
        NAME_DIR,
        "MakeName.bsv",
        &["MakeName.bsv"],
        &[line_count!("sysMakeName.v", "assign bp$PROBE = !rg ;", 1)],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.evaluator/prims/name::MakeNameBadName.bsv",
        NAME_DIR,
        "MakeNameBadName.bsv",
        &["MakeNameBadName.bsv"],
        &[line_count!(
            "sysMakeNameBadName.v",
            "assign thisisabadname$PROBE = !rg ;",
            1
        )],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    verilog_error!(
        "bsc.evaluator/prims/name",
        NAME_DIR,
        "MakeNameDynamicString.bsv",
        "G0012"
    ),
    compile_case!(
        "bsc.evaluator/prims/name::ReaderModuleTest.bs",
        NAME_DIR,
        "ReaderModuleTest.bs",
        &["ReaderModuleTest.bs", "ReaderModule.bs"],
        &[
            line_count!("mkReaderModuleTestInner.v", "reg [15 : 0] count;", 1),
            line_count!("mkReaderModuleTestOuter.v", "reg [15 : 0] count;", 1),
            line_count!(
                "mkReaderModuleTestOuter.v",
                "mkReaderModuleTestInner inner(",
                1
            ),
        ],
        CompileExpectation::Pass,
        None,
        &["-dATS"],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.evaluator/messages/message.exp
    compile_case!(
        "bsc.evaluator/messages::BasicMessage.bsv",
        MESSAGES_DIR,
        "BasicMessage.bsv",
        &["BasicMessage.bsv"],
        &[line_count!(
            "BasicMessage.bsv.bsc-out",
            "Testing sysBasicMessage...",
            1
        )],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.evaluator/messages::LoopMessage.bsv",
        MESSAGES_DIR,
        "LoopMessage.bsv",
        &["LoopMessage.bsv"],
        &[line_count!(
            "LoopMessage.bsv.bsc-out",
            "Testing sysLoopMessage...",
            8
        )],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.interra/Path_Analysis/Input_Output_Path/Input_Output_Path.exp
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Argument2Rdy2.bsv::G0034",
        PATH_DIR,
        "Argument2Rdy2.bsv",
        &["Argument2Rdy2.bsv"],
        &[],
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "G0034",
            count: 1
        },
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Argument2Rdy2.bsv::G0035",
        PATH_DIR,
        "Argument2Rdy2.bsv",
        &["Argument2Rdy2.bsv"],
        &[],
        CompileExpectation::FailWithDiagnostic {
            kind: DiagnosticKind::Error,
            tag: "G0035",
            count: 1
        },
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Argument2Rdy.bsv",
        PATH_DIR,
        "Argument2Rdy.bsv",
        &[
            "Argument2Rdy.bsv",
            "Argument2Rdy.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "Argument2Rdy.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkArgument2Rdy")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Argument2ReturnValue2.bsv",
        PATH_DIR,
        "Argument2ReturnValue2.bsv",
        &[
            "Argument2ReturnValue2.bsv",
            "Argument2ReturnValue2.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "Argument2ReturnValue2.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Argument2ReturnValue3.bsv",
        PATH_DIR,
        "Argument2ReturnValue3.bsv",
        &[
            "Argument2ReturnValue3.bsv",
            "Argument2ReturnValue3.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "Argument2ReturnValue3.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkArgument2ReturnValue3")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Argument2ReturnValue.bsv",
        PATH_DIR,
        "Argument2ReturnValue.bsv",
        &[
            "Argument2ReturnValue.bsv",
            "Argument2ReturnValue.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "Argument2ReturnValue.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkArgument2ReturnValue")
        },
        Requirement::VerilogEnabled
    ),
    verilog_error!(
        "bsc.interra/Path_Analysis/Input_Output_Path",
        PATH_DIR,
        "En2Rdy2.bsv",
        "G0030"
    ),
    verilog_error!(
        "bsc.interra/Path_Analysis/Input_Output_Path",
        PATH_DIR,
        "En2Rdy3.bsv",
        "G0035"
    ),
    verilog_error!(
        "bsc.interra/Path_Analysis/Input_Output_Path",
        PATH_DIR,
        "En2Rdy4.bsv",
        "G0033",
        2
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::En2Rdy5.bsv",
        PATH_DIR,
        "En2Rdy5.bsv",
        &["En2Rdy5.bsv", "En2Rdy5.bsv.bsc-vcomp-out.expected"],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "En2Rdy5.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkEn2Rdy5")
        },
        Requirement::VerilogEnabled
    ),
    verilog_error!(
        "bsc.interra/Path_Analysis/Input_Output_Path",
        PATH_DIR,
        "En2Rdy.bsv",
        "G0033"
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::En2ReturnValue.bsv",
        PATH_DIR,
        "En2ReturnValue.bsv",
        &[
            "En2ReturnValue.bsv",
            "En2ReturnValue.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "En2ReturnValue.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkEn2ReturnValue")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::MuxMethods1.bsv",
        PATH_DIR,
        "MuxMethods1.bsv",
        &["MuxMethods1.bsv", "MuxMethods1.bsv.bsc-vcomp-out.expected"],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "MuxMethods1.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkMuxMethods1")
        },
        Requirement::VerilogEnabled
    ),
    verilog_pass!(
        "bsc.interra/Path_Analysis/Input_Output_Path",
        PATH_DIR,
        "MuxMethods2.bsv"
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Parameter2Rdy.bsv",
        PATH_DIR,
        "Parameter2Rdy.bsv",
        &[
            "Parameter2Rdy.bsv",
            "Parameter2Rdy.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "Parameter2Rdy.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkParameter2Rdy")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.interra/Path_Analysis/Input_Output_Path::Parameter2ReturnValue.bsv",
        PATH_DIR,
        "Parameter2ReturnValue.bsv",
        &[
            "Parameter2ReturnValue.bsv",
            "Parameter2ReturnValue.bsv.bsc-vcomp-out.expected"
        ],
        &[],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "Parameter2ReturnValue.bsv.bsc-vcomp-out.expected"
        }),
        &["-dpathsPostSched"],
        CompileMode::Verilog {
            module: Some("mkParameter2ReturnValue")
        },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.misc/mul/mul.exp
    compile_case!(
        "bsc.misc/mul::FP.bsv",
        MUL_DIR,
        "FP.bsv",
        &["FP.bsv"],
        &[line_count!("module_testMult15.v", "*", 1)],
        CompileExpectation::Pass,
        None,
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.syntax/bsv05/statename/statename.exp
    compile_case!(
        "bsc.syntax/bsv05/statename::StateNameTest.bsv",
        STATENAME_DIR,
        "StateNameTest.bsv",
        &["StateNameTest.bsv", "sysStateNameTest.atsexpand.expected"],
        &[golden_match!(
            "sysStateNameTest.atsexpand",
            "sysStateNameTest.atsexpand.expected"
        )],
        CompileExpectation::Pass,
        None,
        &["-dATSexpand=%m.atsexpand"],
        CompileMode::Verilog {
            module: Some("sysStateNameTest")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.syntax/bsv05/statename::StateNameTest2.bsv",
        STATENAME_DIR,
        "StateNameTest2.bsv",
        &["StateNameTest2.bsv", "sysStateNameTest2.atsexpand.expected"],
        &[golden_match!(
            "sysStateNameTest2.atsexpand",
            "sysStateNameTest2.atsexpand.expected"
        )],
        CompileExpectation::Pass,
        None,
        &["-dATSexpand=%m.atsexpand"],
        CompileMode::Verilog {
            module: Some("sysStateNameTest2")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.syntax/bsv05/statename::UseMod2.bsv",
        STATENAME_DIR,
        "UseMod2.bsv",
        &["UseMod2.bsv", "sysUseMod2.atsexpand.expected"],
        &[golden_match!(
            "sysUseMod2.atsexpand",
            "sysUseMod2.atsexpand.expected"
        )],
        CompileExpectation::Pass,
        None,
        &["-dATSexpand=%m.atsexpand"],
        CompileMode::Verilog {
            module: Some("sysUseMod2")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.syntax/bsv05/statename::UseMod2Arrow.bsv",
        STATENAME_DIR,
        "UseMod2Arrow.bsv",
        &["UseMod2Arrow.bsv", "sysUseMod2Arrow.atsexpand.expected"],
        &[golden_match!(
            "sysUseMod2Arrow.atsexpand",
            "sysUseMod2Arrow.atsexpand.expected"
        )],
        CompileExpectation::Pass,
        None,
        &["-dATSexpand=%m.atsexpand"],
        CompileMode::Verilog {
            module: Some("sysUseMod2Arrow")
        },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.lib/IsModule/is_module.exp
    compile_case!(
        "bsc.lib/IsModule::ModuleCollectClock.bsv",
        IS_MODULE_DIR,
        "ModuleCollectClock.bsv",
        &[
            "ModuleCollectClock.bsv",
            "ModuleCollectClock.bsv.bsc-vcomp-out.expected"
        ],
        &[line_count!("mkTestWrapper.v", "posedge CLK_c", 1)],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "ModuleCollectClock.bsv.bsc-vcomp-out.expected"
        }),
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.lib/IsModule::ModuleContextClock.bsv",
        IS_MODULE_DIR,
        "ModuleContextClock.bsv",
        &[
            "ModuleContextClock.bsv",
            "ModuleContextClock.bsv.bsc-vcomp-out.expected"
        ],
        &[line_count!("mkTestWrapper2.v", "posedge CLK_c", 1)],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "ModuleContextClock.bsv.bsc-vcomp-out.expected"
        }),
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.lib/IsModule::FIFOContextTest.bsv",
        IS_MODULE_DIR,
        "FIFOContextTest.bsv",
        &[
            "FIFOContextTest.bsv",
            "FIFOContextTest.bsv.bsc-vcomp-out.expected",
            "mkFIFOContextTest.v.expected",
        ],
        &[ArtifactAssertion::Matches {
            actual: "mkFIFOContextTest.v",
            expected: "mkFIFOContextTest.v.expected",
            normalization: ArtifactNormalization::Verilog,
        }],
        CompileExpectation::Pass,
        Some(GoldenExpectation {
            expected: "FIFOContextTest.bsv.bsc-vcomp-out.expected"
        }),
        &[],
        CompileMode::Verilog { module: None },
        Requirement::VerilogEnabled
    ),
    // testsuite/bsc.bugs/perf-creg-blowup/perf-creg-blowup.exp
    compile_case!(
        "bsc.bugs/perf-creg-blowup::CregInlineBlowup.bsv",
        CREG_BLOWUP_DIR,
        "CregInlineBlowup.bsv",
        &["CregInlineBlowup.bsv"],
        &[],
        CompileExpectation::Pass,
        None,
        &["-elab", "+RTS", "-K16m", "-RTS"],
        CompileMode::Verilog {
            module: Some("mkCregInlineBlowup")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.bugs/perf-creg-blowup::CregMemBlowup.bsv",
        CREG_BLOWUP_DIR,
        "CregMemBlowup.bsv",
        &["CregMemBlowup.bsv"],
        &[],
        CompileExpectation::Pass,
        None,
        &["-elab", "+RTS", "-K16m", "-RTS"],
        CompileMode::Verilog {
            module: Some("mkCregMemBlowup")
        },
        Requirement::VerilogEnabled
    ),
    compile_case!(
        "bsc.bugs/perf-creg-blowup::AoptScanBlowup.bsv",
        CREG_BLOWUP_DIR,
        "AoptScanBlowup.bsv",
        &["AoptScanBlowup.bsv"],
        &[],
        CompileExpectation::Pass,
        None,
        &["-elab", "+RTS", "-K40m", "-RTS"],
        CompileMode::Verilog {
            module: Some("mkAoptScanBlowup")
        },
        Requirement::VerilogEnabled
    ),
];
