//! Origins:
//! - `testsuite/bsc.typechecker/kind/kind.exp`
//! - `testsuite/bsc.interra/preprocessorTestcases/define/define.exp`
//! - `testsuite/bsc.typechecker/primtcons/primtcons.exp`
//! - `testsuite/bsc.bugs/github/gh353/gh353.exp`
//! - `testsuite/bsc.typechecker/fundeps/fundeps.exp`
//! - `testsuite/bsc.typechecker/foreignmodule/parameters/parameters.exp`
//! - `testsuite/bsc.typechecker/foreignmodule/ports/ports.exp`
//! - `testsuite/bsc.typechecker/kind/mismatch/mismatch.exp`
//! - `testsuite/bsc.bugs/github/gh221/gh221.exp`
//! - `testsuite/bsc.interra/preprocessorTestcases/ifdef/ifdef.exp`
//! - `testsuite/bsc.interra/preprocessorTestcases/undef/undef.exp`
//! - `testsuite/bsc.syntax/bsv05/dups/dups.exp`

use super::CompileCase;

macro_rules! frontend_case {
    ($name:expr, $dir:expr, $source:expr, $fixtures:expr, $expectation:expr, $golden:expr, $options:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $dir,
            source: $source,
            fixtures: $fixtures,
            assertions: &[],
            expectation: $expectation,
            golden: $golden,
            options: $options,
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! verilog_case {
    ($name:expr, $dir:expr, $source:expr, $expectation:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: $expectation,
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! pass {
    ($prefix:literal, $dir:expr, $source:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source],
            $crate::upstream::CompileExpectation::Pass,
            None,
            &[]
        )
    };
}

macro_rules! pass_as {
    ($name:literal, $dir:expr, $source:literal) => {
        frontend_case!(
            $name,
            $dir,
            $source,
            &[$source],
            $crate::upstream::CompileExpectation::Pass,
            None,
            &[]
        )
    };
}

macro_rules! pass_golden {
    ($prefix:literal, $dir:expr, $source:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            $crate::upstream::CompileExpectation::Pass,
            Some($crate::upstream::GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        )
    };
}

macro_rules! pass_options {
    ($prefix:literal, $dir:expr, $source:literal, $options:expr) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source],
            $crate::upstream::CompileExpectation::Pass,
            None,
            $options
        )
    };
}

macro_rules! pass_dep {
    ($prefix:literal, $dir:expr, $source:literal, $dep:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source, $dep],
            $crate::upstream::CompileExpectation::Pass,
            None,
            &[]
        )
    };
}

macro_rules! fail_golden {
    ($prefix:literal, $dir:expr, $source:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            $crate::upstream::CompileExpectation::Fail,
            Some($crate::upstream::GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        )
    };
}

macro_rules! fail_golden_dep {
    ($prefix:literal, $dir:expr, $source:literal, $dep:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source, $dep, concat!($source, ".bsc-out.expected")],
            $crate::upstream::CompileExpectation::Fail,
            Some($crate::upstream::GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        )
    };
}

macro_rules! error {
    ($prefix:literal, $dir:expr, $source:literal, $tag:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source],
            $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            None,
            &[]
        )
    };
}

macro_rules! error_count {
    ($prefix:literal, $dir:expr, $source:literal, $tag:literal, $count:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source],
            $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            None,
            &[]
        )
    };
}

macro_rules! error_golden {
    ($prefix:literal, $dir:expr, $source:literal, $tag:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            Some($crate::upstream::GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        )
    };
}

macro_rules! error_count_golden {
    ($prefix:literal, $dir:expr, $source:literal, $tag:literal, $count:literal) => {
        frontend_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            Some($crate::upstream::GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        )
    };
}

macro_rules! verilog_pass {
    ($prefix:literal, $dir:expr, $source:literal) => {
        verilog_case!(
            concat!($prefix, "::", $source),
            $dir,
            $source,
            $crate::upstream::CompileExpectation::Pass
        )
    };
}

macro_rules! verilog_error_as {
    ($name:literal, $dir:expr, $source:literal, $tag:literal) => {
        verilog_case!(
            $name,
            $dir,
            $source,
            $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            }
        )
    };
}

const KIND_DIR: &str = "testsuite/bsc.typechecker/kind";
const DEFINE_DIR: &str = "testsuite/bsc.interra/preprocessorTestcases/define";
const PRIM_TCONS_DIR: &str = "testsuite/bsc.typechecker/primtcons";
const GH353_DIR: &str = "testsuite/bsc.bugs/github/gh353";
const FUNDEPS_DIR: &str = "testsuite/bsc.typechecker/fundeps";
const PARAMETERS_DIR: &str = "testsuite/bsc.typechecker/foreignmodule/parameters";
const PORTS_DIR: &str = "testsuite/bsc.typechecker/foreignmodule/ports";
const KIND_MISMATCH_DIR: &str = "testsuite/bsc.typechecker/kind/mismatch";
const GH221_DIR: &str = "testsuite/bsc.bugs/github/gh221";
const IFDEF_DIR: &str = "testsuite/bsc.interra/preprocessorTestcases/ifdef";
const UNDEF_DIR: &str = "testsuite/bsc.interra/preprocessorTestcases/undef";
const DUPS_DIR: &str = "testsuite/bsc.syntax/bsv05/dups";

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.typechecker/kind/kind.exp
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "OneArgMissingOne.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "TwoArgMissingTwo.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "TwoArgMissingOne.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "NoArgPlusOne.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "OneArgPlusOne.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "NumPlusOne.bs"),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "NonNumWhereNumExpected.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "NumWhereNonNumExpected.bs"
    ),
    error!(
        "bsc.typechecker/kind",
        KIND_DIR, "ModIfc_TooFewArgs_Local.bsv", "T0025"
    ),
    error!(
        "bsc.typechecker/kind",
        KIND_DIR, "ModIfc_TooFewArgs_TopLevel.bsv", "T0025"
    ),
    error!(
        "bsc.typechecker/kind",
        KIND_DIR, "ModIfc_TooManyArgs_TopLevel.bsv", "T0024"
    ),
    error!(
        "bsc.typechecker/kind",
        KIND_DIR, "ImportModIfc_TooFewArgs.bsv", "T0025"
    ),
    error!(
        "bsc.typechecker/kind",
        KIND_DIR, "ImportModIfc_TooManyArgs.bsv", "T0024"
    ),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "ClassDefFieldIsNum.bs"),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "ClassDefParamConflictingUses.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "ClassDefParamGivenNonNumUsedNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "ClassDefParamGivenNumUsedNonNum.bs"
    ),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "ClassDefResGivenNum.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "DataDefFieldIsNum.bs"),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "DataDefParamConflictingUses.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "DataDefParamGivenNonNumUsedNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "DataDefParamGivenNumUsedNonNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "DataDefParamGivenNonNumUsedFunc.bs"
    ),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "DataDefResGivenNum.bs"),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "StructDefFieldIsNum.bs"),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "StructDefParamConflictingUses.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "StructDefParamGivenNonNumUsedNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "StructDefParamGivenNumUsedNonNum.bs"
    ),
    fail_golden!("bsc.typechecker/kind", KIND_DIR, "StructDefResGivenNum.bs"),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamConflictingUses.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenNonNumUsedNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenNumUsedNonNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasPartialAppWithConflictingKindSig.bs"
    ),
    pass!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasPartialAppWithoutKindSig.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasResGivenNonNumIsNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasResGivenNonNumIsNumParam.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasResGivenNumIsNonNum.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasResGivenNumIsNonNumParam.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenTooFew.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenTooFew_ToNone.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenTooMany.bs"
    ),
    fail_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenTooMany_FromNone.bs"
    ),
    pass!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "TypeAliasParamGivenTooMany_OK.bs"
    ),
    error_golden!(
        "bsc.typechecker/kind",
        KIND_DIR,
        "DataConstrOfNoArgsAppliedToMultipleArgs.bsv",
        "T0143"
    ),
    // testsuite/bsc.interra/preprocessorTestcases/define/define.exp
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_BITVECTOR.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_BOOLVALUE.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_CaseSen.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_diffMacroNames.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_HavingMacroValueBool.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_IN_BITVECTOR_DECL.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_IN_FOR.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_IN_IF_STMT.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_InsideFunction.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_InsideMethod.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_InsideModule.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_InsideRule.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_IN_TERNARY_STMT.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_IN_WHILE.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_MACROTAKESPARAM.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_MACROVALTAKESCOMMENT.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_MacroValueHavingBlockComment.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_MacroValueHavingISingleLineComment.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_MORETHANONELINE.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_PackageLevel.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_RecursiveDefine.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_redefiningMacroValue.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINDINTERFACEKEYWORD.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINDMETHODKEYWORD.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINGBITKEYWORD.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINGFUNCTIONKEYWORD.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINGMODULEKEYWORD.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINGOTHERDEFINE.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINGPACKAGEKEYWORD.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_USINGRULEKEYWORD.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_WithBlackslashQuote.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_WithMacroValuewithSemiColonAtEnd.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/define",
        DEFINE_DIR,
        "Define_WithSingleQuotesInMacroValue.bsv"
    ),
    // testsuite/bsc.typechecker/primtcons/primtcons.exp
    pass!("bsc.typechecker/primtcons", PRIM_TCONS_DIR, "ExpSizeOf.bsv"),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_Instances.bsv"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_InstancesBase.bsv"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_InstancesBaseSyn.bsv"
    ),
    verilog_pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_Field.bsv"
    ),
    verilog_pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn.bsv"
    ),
    verilog_pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn_BS.bs"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn_Minimal.bs"
    ),
    error_golden!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn_Use_NoCtx.bs",
        "T0030"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn_Use_WithCtx.bs"
    ),
    error_count_golden!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn_Unbound_NoCtx.bs",
        "T0030",
        2
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_FieldSyn_Unbound_WithCtx.bs"
    ),
    error_golden!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_ValueOf_NoCtx.bs",
        "T0030"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_ValueOf_WithCtx.bs"
    ),
    error_golden!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_StringOf_NoCtx.bs",
        "T0030"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_StringOf_WithCtx.bs"
    ),
    error_golden!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_TypedExpr_NoCtx.bs",
        "T0030"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_TypedExpr_WithCtx.bs"
    ),
    error_golden!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_Let_NoCtx.bs",
        "T0030"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "ExpSizeOf_Let_WithCtx.bs"
    ),
    error!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR, "ValueOf_KindMismatch_Arity.bs", "T0025"
    ),
    error!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR, "ValueOf_KindMismatch_Res.bs", "T0026"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "CExtendSynonym.bsv"
    ),
    error!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR, "CExtendATF.bsv", "T0035"
    ),
    pass!(
        "bsc.typechecker/primtcons",
        PRIM_TCONS_DIR,
        "CExtendATFExplicit.bsv"
    ),
    pass_as!(
        "bsc.typechecker/primtcons::ExpSizeOf_VectorIfc.bsv::frontend",
        PRIM_TCONS_DIR,
        "ExpSizeOf_VectorIfc.bsv"
    ),
    verilog_error_as!(
        "bsc.typechecker/primtcons::ExpSizeOf_VectorIfc.bsv::verilog",
        PRIM_TCONS_DIR,
        "ExpSizeOf_VectorIfc.bsv",
        "T0043"
    ),
    // testsuite/bsc.bugs/github/gh353/gh353.exp
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_BH.bs",
        "Bug353_Type.bs"
    ),
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_BSV_Struct.bsv",
        "Bug353_Type.bs"
    ),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_BSV_Constr.bsv",
        "Bug353_Type.bs"
    ),
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_Pat_BH.bs",
        "Bug353_Type.bs"
    ),
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_Pat_BSV_Struct.bsv",
        "Bug353_Type.bs"
    ),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_Pat_BSV_Constr.bsv",
        "Bug353_Type.bs"
    ),
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_Match_BH.bs",
        "Bug353_Type.bs"
    ),
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_Match_BSV_Struct.bsv",
        "Bug353_Type.bs"
    ),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Bug353_Match_BSV_Constr.bsv",
        "Bug353_Type.bs"
    ),
    pass_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Disambig_Cons_NoStruct_NotAmbig.bs",
        "Types.bs"
    ),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "Disambig_Cons_NoStruct_Ambig.bs",
        "Types.bs"
    ),
    pass!("bsc.bugs/github/gh353", GH353_DIR, "BH_Cons_NamedFields.bs"),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "BH_Cons_NonNamedFields.bs",
        "Types_NonNamed.bs"
    ),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "BSV_Cons_NonNamedFields.bsv",
        "Types_NonNamed.bs"
    ),
    fail_golden_dep!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "BSV_SDataCon_Match.bsv",
        "Types.bs"
    ),
    fail_golden!("bsc.bugs/github/gh353", GH353_DIR, "BSV_WrongSyntax.bsv"),
    fail_golden!(
        "bsc.bugs/github/gh353",
        GH353_DIR,
        "BSV_WrongSyntax_Pat.bsv"
    ),
    pass!("bsc.bugs/github/gh353", GH353_DIR, "BSV_Cons_NoFields.bsv"),
    // testsuite/bsc.typechecker/fundeps/fundeps.exp
    pass!("bsc.typechecker/fundeps", FUNDEPS_DIR, "ThreeDimArray.bsv"),
    pass!("bsc.typechecker/fundeps", FUNDEPS_DIR, "FourDimArray.bsv"),
    pass!("bsc.typechecker/fundeps", FUNDEPS_DIR, "TenDimArray.bsv"),
    pass!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "StructSelectOneDimArray.bsv"
    ),
    pass!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "StructSelectFiveDimArray.bsv"
    ),
    pass_options!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "StructUpdateOneDimArray.bs",
        &["-let-gen"]
    ),
    pass!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "StructUpdateFourDimArray.bsv"
    ),
    pass!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "ConstructorFourDimArrayUpdate.bsv"
    ),
    pass!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "ConstructorFourDimArrayEquality.bsv"
    ),
    pass!("bsc.typechecker/fundeps", FUNDEPS_DIR, "Bug355.bsv"),
    error_golden!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "NonDepParam.bs",
        "T0124"
    ),
    error_golden!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "NonMergedFunDeps.bs",
        "T0124"
    ),
    error!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR, "OverlapDepParam.bsv", "T0135"
    ),
    error!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR, "ExtraDepParam.bsv", "T0136"
    ),
    error_count!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR,
        "EmptyDepParam.bsv",
        "T0137",
        2
    ),
    pass!("bsc.typechecker/fundeps", FUNDEPS_DIR, "FunDepUnify.bs"),
    error!(
        "bsc.typechecker/fundeps",
        FUNDEPS_DIR, "FunDepUnify2.bs", "T0060"
    ),
    // testsuite/bsc.typechecker/foreignmodule/parameters/parameters.exp
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_Integer.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_Literal.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_Bit32.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_SizedLiteral.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_String.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_StringLiteral.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_Real.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_RealLiteral.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_InBits_NoPack.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_InBits_Pack.bsv"
    ),
    error_golden!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_NotInBits.bsv",
        "T0133"
    ),
    error_golden!(
        "bsc.typechecker/foreignmodule/parameters",
        PARAMETERS_DIR,
        "BVI_Param_NeedsProviso.bsv",
        "T0133"
    ),
    // testsuite/bsc.typechecker/foreignmodule/ports/ports.exp
    pass!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_Bit32.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_SizedLiteral.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_Literal.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_InBits_NoPack.bsv"
    ),
    pass!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_InBits_Pack.bsv"
    ),
    error_golden!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_NotInBits.bsv",
        "T0134"
    ),
    error_golden!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_NeedsProviso.bsv",
        "T0134"
    ),
    error!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR, "BVI_Port_Integer.bsv", "T0134"
    ),
    error!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR, "BVI_Port_String.bsv", "T0134"
    ),
    error!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR, "BVI_Port_StringLiteral.bsv", "T0134"
    ),
    error!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR, "BVI_Port_Real.bsv", "T0134"
    ),
    error_golden!(
        "bsc.typechecker/foreignmodule/ports",
        PORTS_DIR,
        "BVI_Port_RealLiteral.bsv",
        "T0033"
    ),
    // testsuite/bsc.typechecker/kind/mismatch/mismatch.exp
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "ProvisoBaseMismatch_TopLevel.bsv",
        "T0027"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "ProvisoBaseMismatch_Local.bsv",
        "T0027"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "UnionDefValueIsNum_ByParam.bsv",
        "T0027"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "StructDefFieldIsNum_ByParam.bsv",
        "T0027"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "Proviso_NonNumUsedNum.bsv",
        "T0026"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "TyConParamTooFew.bsv",
        "T0025"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "TyConParamTooFew_InProviso.bsv",
        "T0025"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "DefaultProvisoMismatch_TyCon.bsv",
        "T0027"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "DefaultBaseMismatch_TyConParam.bsv",
        "T0027"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "ProvisoProvisoMismatch_TopLevel.bsv",
        "T0026"
    ),
    error_golden!(
        "bsc.typechecker/kind/mismatch",
        KIND_MISMATCH_DIR,
        "ProvisoProvisoMismatch_Local.bsv",
        "T0026"
    ),
    // testsuite/bsc.bugs/github/gh221/gh221.exp
    error_golden!("bsc.bugs/github/gh221", GH221_DIR, "Test.bs", "T0013"),
    error_golden!("bsc.bugs/github/gh221", GH221_DIR, "ZipCrash.bs", "T0013"),
    pass!("bsc.bugs/github/gh221", GH221_DIR, "Example1.bs"),
    error_golden!("bsc.bugs/github/gh221", GH221_DIR, "Example2.bs", "T0032"),
    pass!("bsc.bugs/github/gh221", GH221_DIR, "Example3.bs"),
    error_golden!("bsc.bugs/github/gh221", GH221_DIR, "Example4.bs", "T0031"),
    pass!("bsc.bugs/github/gh221", GH221_DIR, "ShouldCompile.bs"),
    error_golden!("bsc.bugs/github/gh221", GH221_DIR, "ZipCrash2.bs", "T0013"),
    pass!("bsc.bugs/github/gh221", GH221_DIR, "ZipNoCrash3.bs"),
    // testsuite/bsc.interra/preprocessorTestcases/ifdef/ifdef.exp
    fail_golden!(
        "bsc.interra/preprocessorTestcases/ifdef",
        IFDEF_DIR,
        "Ifdef_endifWithoutPreceedingIfdef.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/ifdef",
        IFDEF_DIR,
        "Ifdef_ExpInIfdef.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/ifdef",
        IFDEF_DIR,
        "IFdef_InsideMethod.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/ifdef",
        IFDEF_DIR,
        "Ifdef_InsideModule.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/ifdef",
        IFDEF_DIR,
        "Ifdef_InsideRule.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/ifdef",
        IFDEF_DIR,
        "Ifdef_PackageLevel.bsv"
    ),
    // testsuite/bsc.interra/preprocessorTestcases/undef/undef.exp
    pass_golden!(
        "bsc.interra/preprocessorTestcases/undef",
        UNDEF_DIR,
        "Undef_InsideMethod.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/undef",
        UNDEF_DIR,
        "Undef_InsideModule.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/undef",
        UNDEF_DIR,
        "Undef_InsideRule.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/undef",
        UNDEF_DIR,
        "Undef_MacroUndef.bsv"
    ),
    fail_golden!(
        "bsc.interra/preprocessorTestcases/undef",
        UNDEF_DIR,
        "Undef_NoMacroName.bsv"
    ),
    pass_golden!(
        "bsc.interra/preprocessorTestcases/undef",
        UNDEF_DIR,
        "Undef_PackageLevel.bsv"
    ),
    // testsuite/bsc.syntax/bsv05/dups/dups.exp
    fail_golden!("bsc.syntax/bsv05/dups", DUPS_DIR, "InterfaceDupFields.bsv"),
    fail_golden!("bsc.syntax/bsv05/dups", DUPS_DIR, "StructDupFields.bsv"),
    fail_golden!("bsc.syntax/bsv05/dups", DUPS_DIR, "SubstructDupFields.bsv"),
    fail_golden!("bsc.syntax/bsv05/dups", DUPS_DIR, "UnionDupCons.bsv"),
    fail_golden!(
        "bsc.syntax/bsv05/dups",
        DUPS_DIR,
        "DupTypeclassFunction.bsv"
    ),
    fail_golden!(
        "bsc.syntax/bsv05/dups",
        DUPS_DIR,
        "DupTopLevelVariableDef.bsv"
    ),
];
