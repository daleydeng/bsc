//! Origin: `testsuite/bsc.typechecker/context-errors/context-errors.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation,
    Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/context-errors";

macro_rules! context_case {
    ($constant:ident, $source:literal, $fixtures:expr, $expectation:expr, $golden:expr, $assertions:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.typechecker/context-errors::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: $fixtures,
            assertions: $assertions,
            expectation: $expectation,
            golden: $golden,
            options: &[],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        };
    };
}

macro_rules! pass {
    ($constant:ident, $source:literal) => {
        context_case!(
            $constant,
            $source,
            &[$source],
            CompileExpectation::Pass,
            None,
            &[]
        );
    };
}

macro_rules! fail_golden {
    ($constant:ident, $source:literal) => {
        context_case!(
            $constant,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            CompileExpectation::Fail,
            Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        );
    };
}

macro_rules! error {
    ($constant:ident, $source:literal, $tag:literal) => {
        error!($constant, $source, $tag, 1, &[]);
    };
    ($constant:ident, $source:literal, $tag:literal, $count:expr) => {
        error!($constant, $source, $tag, $count, &[]);
    };
    ($constant:ident, $source:literal, $tag:literal, $count:expr, $assertions:expr) => {
        context_case!(
            $constant,
            $source,
            &[$source],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            None,
            $assertions
        );
    };
}

macro_rules! error_golden {
    ($constant:ident, $source:literal, $tag:literal) => {
        error_golden!($constant, $source, $tag, 1);
    };
    ($constant:ident, $source:literal, $tag:literal, $count:expr) => {
        context_case!(
            $constant,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[]
        );
    };
}

fail_golden!(CONTEXT_TOO_WEAK, "ContextTooWeak.bs");
fail_golden!(CONTEXT_TOO_WEAK_2, "ContextTooWeak2.bsv");
fail_golden!(
    DUPLICATE_NICE_NAME_DEFAULT,
    "DuplicateNiceName_WeakCtx_Default.bsv"
);
fail_golden!(
    DUPLICATE_NICE_NAME_BIT_EXTEND,
    "DuplicateNiceName_WeakCtx_BitExtend.bsv"
);
fail_golden!(CONTEXT_REDUCTION_METHOD, "ContextReductionMethod.bsv");
fail_golden!(
    CONTEXT_REDUCTION_IMPL_FUNCTION,
    "ContextReductionImplFunction.bsv"
);
fail_golden!(
    CONTEXT_REDUCTION_IMPL_FUNCTION_2,
    "ContextReductionImplFunction2.bsv"
);
fail_golden!(
    CONTEXT_REDUCTION_EXPL_FUNCTION,
    "ContextReductionExplFunction.bsv"
);
fail_golden!(AMBIG_EXPL_PACK_UNPACK, "AmbigCtxExplPackUnpack.bsv");
fail_golden!(AMBIG_IMPL_PACK_UNPACK, "AmbigCtxImplPackUnpack.bsv");
error_golden!(
    AMBIG_EXPL_TRUNCATE_EXTEND,
    "AmbigCtxExplTruncateExtend.bsv",
    "T0035"
);
error_golden!(
    AMBIG_IMPL_TRUNCATE_EXTEND,
    "AmbigCtxImplTruncateExtend.bsv",
    "T0035",
    2
);
error!(
    AMBIG_EXPL_WITH_PROVISO,
    "AmbigCtxExplWithProviso.bsv", "T0079"
);
error!(AMBIG_INSTANCE, "AmbigCtxInstance.bsv", "T0079");
fail_golden!(AMBIG_REMOVE_FUN_DEPS, "AmbigCtx_RemoveFunDeps.bsv");
error!(
    REDUCTION_REMOVE_IMPLIED,
    "ContextReductionRemoveImplied.bs", "T0031"
);
error_golden!(
    REDUCTION_REMOVE_IMPLIED_2,
    "ContextReductionRemoveImplied2.bsv",
    "T0032"
);
fail_golden!(TOO_WEAK_REMOVE_IMPLIED, "ContextTooWeakRemoveImplied.bs");
error_golden!(
    REDUCTION_REMOVE_IMPLIED_CLOSE_FD,
    "ContextReductionRemoveImpliedCloseFD.bsv",
    "T0032"
);
error_golden!(TOO_WEAK_DEFERRED, "ContextTooWeakDeferred.bsv", "T0030");
pass!(TOO_WEAK_RESOLVED, "ContextTooWeakResolved.bsv");
error!(WRONG_BIT_SIZE, "ECtxRedWrongBitSize.bsv", "T0060");
error!(WRONG_BIT_SIZE_2, "ECtxRedWrongBitSize2.bsv", "T0060");
error!(BITWISE_BOOL, "ECtxRedBitwiseBool.bsv", "T0061");
error!(BITWISE, "ECtxRedBitwise.bsv", "T0062");
error!(
    BITWISE_WITH_TYPE_VARIABLES,
    "ContextReductionBitwiseWithTVars.bsv", "T0031"
);
error!(
    BIT_EXTEND_NEEDS_ADD_CONTEXT,
    "EWeakCtxBitExtendNeedsAddCtx.bsv", "T0065"
);
error!(
    BIT_EXTEND_BAD_SIZES,
    "ECtxRedBitExtendBadSizes.bsv", "T0063"
);
error!(BIT_EXTEND_BAD_TYPE, "ECtxRedBitExtendBadType.bsv", "T0064");
error!(NOT_SELECTABLE, "ECtxRedNotSelectable.bsv", "T0070");
error!(
    WRONG_SELECTION_RESULT,
    "ECtxRedWrongSelectionResult.bsv", "T0020"
);
error!(BAD_SELECTION_INDEX, "ECtxRedBadSelectionIndex.bsv", "T0072");
error!(NOT_UPDATEABLE, "ECtxRedNotUpdateable.bsv", "T0095");
error!(WRONG_UPDATE_ARGUMENT, "ECtxRedWrongUpdateArg.bsv", "T0020");
error!(NOT_WRITEABLE, "ECtxRedNotWriteable.bsv", "T0097");
error!(WRONG_WRITE_ARGUMENT, "ECtxRedWrongWriteArg.bsv", "T0020");
error!(
    SELECTABLE_NEEDS_INDEX_CONTEXT,
    "EWeakCtxPrimSelectableNeedsPrimIndexCtx.bsv", "T0030"
);
error!(
    WRONG_SELECTION_VIA_WEAK_CONTEXT,
    "ECtxRedWrongSelectionResult_ViaWeakCtx.bsv", "T0020"
);
error!(
    AMBIG_SELECTABLE_INDEX,
    "AmbigCtxPrimSelectableIndex.bsv", "T0035"
);
error!(
    WRONG_SELECTION_VIA_AMBIG_CONTEXT,
    "ECtxRedWrongSelectionResult_ViaAmbigCtx.bsv", "T0020"
);
error!(
    WRONG_UPDATE_VIA_WEAK_CONTEXT,
    "ECtxRedWrongUpdateArg_ViaWeakCtx.bsv", "T0020"
);
error!(
    WRONG_UPDATE_VIA_AMBIG_CONTEXT,
    "ECtxRedWrongUpdateArg_ViaAmbigCtx.bsv", "T0020"
);
error!(
    WRONG_WRITE_VIA_WEAK_CONTEXT,
    "ECtxRedWrongWriteArg_ViaWeakCtx.bsv", "T0080"
);
error!(
    WRONG_WRITE_VIA_AMBIG_CONTEXT,
    "ECtxRedWrongWriteArg_ViaAmbigCtx.bsv", "T0080"
);
pass!(SELECTION_INDEX_TOO_LONG, "ECtxRedSelectionIndexTooLong.bsv");
pass!(INDEX_NEEDS_ADD_CONTEXT, "EWeakCtxPrimIndexNeedsAddCtx.bsv");
fail_golden!(
    PRIM_INDEX_WRONG_SIZE,
    "ContextReductionPrimIndexWrongSize.bsv"
);
error!(BIT_REDUCE, "ECtxRedBitReduce.bsv", "T0074");
error!(IS_MODULE, "ECtxRedIsModule.bsv", "T0107");
error!(MOD_TOO_MANY_1, "EModInstWrongArgs_TooMany1.bsv", "T0108");
error!(MOD_TOO_MANY_2, "EModInstWrongArgs_TooMany2.bsv", "T0108");
error!(
    MOD_TOO_MANY_VIA_MAP,
    "EModInstWrongArgs_TooManyViaMap.bsv", "T0108"
);
error!(MOD_TOO_FEW_1, "EModInstWrongArgs_TooFew1.bsv", "T0108");
error!(MOD_TOO_FEW_2, "EModInstWrongArgs_TooFew2.bsv", "T0084");
error!(
    MOD_TOO_FEW_VIA_MAP,
    "EModInstWrongArgs_TooFewViaMap.bsv", "T0107"
);
error_golden!(
    NICE_TYPES_AFTER_SIMPLIFY,
    "NiceTypesAfterSimplify.bsv",
    "T0031"
);
error!(
    ACTION_VALUE_BIND_IN_MODULE,
    "ECtxRedIsModuleActionValue_AVBindInModBlock.bsv",
    "T0113",
    1,
    &[ArtifactAssertion::Text {
        path: "ECtxRedIsModuleActionValue_AVBindInModBlock.bsv.bsc-out",
        assertion: TextAssertion::Regex {
            pattern: r#"AVBindInModBlock\.bsv", line 2, column 8:"#,
        },
    }]
);
error!(
    ACTION_VALUE_EXPR_IN_MODULE,
    "ECtxRedIsModuleActionValue_AVExprInModBlock.bsv",
    "T0113",
    1,
    &[ArtifactAssertion::Text {
        path: "ECtxRedIsModuleActionValue_AVExprInModBlock.bsv.bsc-out",
        assertion: TextAssertion::Regex {
            pattern: r#"AVExprInModBlock\.bsv", line 2, column 4:"#,
        },
    }]
);
error!(
    MODULE_BIND_IN_ACTION_VALUE,
    "ECtxRedIsModuleActionValue_ModBindInAVBlock.bsv",
    "T0113",
    1,
    &[ArtifactAssertion::Text {
        path: "ECtxRedIsModuleActionValue_ModBindInAVBlock.bsv.bsc-out",
        assertion: TextAssertion::Regex {
            pattern: r#"ModBindInAVBlock\.bsv", line 3, column 17:"#,
        },
    }]
);

pub(super) const CASES: &[CompileCase] = &[
    CONTEXT_TOO_WEAK,
    CONTEXT_TOO_WEAK_2,
    DUPLICATE_NICE_NAME_DEFAULT,
    DUPLICATE_NICE_NAME_BIT_EXTEND,
    CONTEXT_REDUCTION_METHOD,
    CONTEXT_REDUCTION_IMPL_FUNCTION,
    CONTEXT_REDUCTION_IMPL_FUNCTION_2,
    CONTEXT_REDUCTION_EXPL_FUNCTION,
    AMBIG_EXPL_PACK_UNPACK,
    AMBIG_IMPL_PACK_UNPACK,
    AMBIG_EXPL_TRUNCATE_EXTEND,
    AMBIG_IMPL_TRUNCATE_EXTEND,
    AMBIG_EXPL_WITH_PROVISO,
    AMBIG_INSTANCE,
    AMBIG_REMOVE_FUN_DEPS,
    REDUCTION_REMOVE_IMPLIED,
    REDUCTION_REMOVE_IMPLIED_2,
    TOO_WEAK_REMOVE_IMPLIED,
    REDUCTION_REMOVE_IMPLIED_CLOSE_FD,
    TOO_WEAK_DEFERRED,
    TOO_WEAK_RESOLVED,
    WRONG_BIT_SIZE,
    WRONG_BIT_SIZE_2,
    BITWISE_BOOL,
    BITWISE,
    BITWISE_WITH_TYPE_VARIABLES,
    BIT_EXTEND_NEEDS_ADD_CONTEXT,
    BIT_EXTEND_BAD_SIZES,
    BIT_EXTEND_BAD_TYPE,
    NOT_SELECTABLE,
    WRONG_SELECTION_RESULT,
    BAD_SELECTION_INDEX,
    NOT_UPDATEABLE,
    WRONG_UPDATE_ARGUMENT,
    NOT_WRITEABLE,
    WRONG_WRITE_ARGUMENT,
    SELECTABLE_NEEDS_INDEX_CONTEXT,
    WRONG_SELECTION_VIA_WEAK_CONTEXT,
    AMBIG_SELECTABLE_INDEX,
    WRONG_SELECTION_VIA_AMBIG_CONTEXT,
    WRONG_UPDATE_VIA_WEAK_CONTEXT,
    WRONG_UPDATE_VIA_AMBIG_CONTEXT,
    WRONG_WRITE_VIA_WEAK_CONTEXT,
    WRONG_WRITE_VIA_AMBIG_CONTEXT,
    SELECTION_INDEX_TOO_LONG,
    INDEX_NEEDS_ADD_CONTEXT,
    PRIM_INDEX_WRONG_SIZE,
    BIT_REDUCE,
    IS_MODULE,
    MOD_TOO_MANY_1,
    MOD_TOO_MANY_2,
    MOD_TOO_MANY_VIA_MAP,
    MOD_TOO_FEW_1,
    MOD_TOO_FEW_2,
    MOD_TOO_FEW_VIA_MAP,
    NICE_TYPES_AFTER_SIMPLIFY,
    ACTION_VALUE_BIND_IN_MODULE,
    ACTION_VALUE_EXPR_IN_MODULE,
    MODULE_BIND_IN_ACTION_VALUE,
];
