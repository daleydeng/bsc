//! Origin: `testsuite/bsc.typechecker/constructors/constructors.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation,
    Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/constructors";

macro_rules! constructor_case {
    ($constant:ident, $source:literal, $fixtures:expr, $expectation:expr, $golden:expr, $assertions:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.typechecker/constructors::", $source),
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
        pass!($constant, $source, &[$source]);
    };
    ($constant:ident, $source:literal, $fixtures:expr) => {
        constructor_case!(
            $constant,
            $source,
            $fixtures,
            CompileExpectation::Pass,
            None,
            &[]
        );
    };
}

macro_rules! error {
    ($constant:ident, $source:literal, $tag:literal) => {
        error!($constant, $source, $tag, 1, &[$source], &[]);
    };
    ($constant:ident, $source:literal, $tag:literal, $count:expr) => {
        error!($constant, $source, $tag, $count, &[$source], &[]);
    };
    ($constant:ident, $source:literal, $tag:literal, $count:expr, $fixtures:expr, $assertions:expr) => {
        constructor_case!(
            $constant,
            $source,
            $fixtures,
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
        error_golden!(
            $constant,
            $source,
            $tag,
            1,
            &[$source, concat!($source, ".bsc-out.expected")]
        );
    };
    ($constant:ident, $source:literal, $tag:literal, $count:expr, $fixtures:expr) => {
        constructor_case!(
            $constant,
            $source,
            $fixtures,
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

macro_rules! regex_assertion {
    ($path:literal, $pattern:literal) => {
        &[ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Regex { pattern: $pattern },
        }]
    };
}

macro_rules! line_count_assertion {
    ($path:literal, $text:literal, $count:expr) => {
        &[ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $text,
                count: $count,
            },
        }]
    };
}

error_golden!(UNION_ONE_ARG_NONE, "UnionOneArgGivenNone.bsv", "T0144");
error_golden!(
    UNION_ONE_ARG_NONE_PATTERN,
    "UnionOneArgGivenNone_Pattern.bsv",
    "T0142"
);
error!(
    STRUCT_UPDATE_WRONG_FIELD,
    "StructUpd_WrongField.bsv", "T0016"
);
error!(
    STRUCT_SELECT_MULTIPLE_TYPES_WRONG_FIELD,
    "StructSelect_MultipleTypes_WrongField.bs", "T0016"
);
error!(
    STRUCT_LITERAL_WRONG_FIELD,
    "StructLit_WrongField.bsv", "T0016"
);
error_golden!(
    STRUCT_SELECT_NOT_IMPORTED,
    "StructSelect_NotImported.bsv",
    "T0140",
    1,
    &[
        "StructSelect_NotImported.bsv",
        "StructSelect_NotImported_Sub.bsv",
        "StructSelect_NotImported.bsv.bsc-out.expected",
    ]
);
error_golden!(
    STRUCT_SELECT_NOT_VISIBLE,
    "StructSelect_NotVisible.bsv",
    "T0139",
    4,
    &[
        "StructSelect_NotVisible.bsv",
        "StructSelect_NotVisible_Sub.bsv",
        "StructSelect_NotVisible.bsv.bsc-out.expected",
    ]
);
error_golden!(
    STRUCT_SELECT_NOT_STRUCT_FUNCTION,
    "StructSelect_NotStruct_Function.bsv",
    "T0138"
);
error!(
    FIELD_AMBIGUOUS_SELECT,
    "FieldAmb_Select.bsv",
    "T0018",
    1,
    &["FieldAmb_Select.bsv"],
    regex_assertion!(
        "FieldAmb_Select.bsv.bsc-out",
        "FieldAmb_Select::Bar, FieldAmb_Select::Foo"
    )
);
error!(
    FIELD_AMBIGUOUS_UPDATE,
    "FieldAmb_Update.bsv",
    "T0018",
    1,
    &["FieldAmb_Update.bsv"],
    regex_assertion!(
        "FieldAmb_Update.bsv.bsc-out",
        "FieldAmb_Update::Bar, FieldAmb_Update::Foo"
    )
);
pass!(
    FIELD_PATTERN_UNQUALIFIED,
    "FieldAmb_Pattern_Unqualified.bsv",
    &[
        "FieldAmb_Pattern_Unqualified.bsv",
        "FieldAmb_Pattern_Sub.bsv",
    ]
);
pass!(
    FIELD_PATTERN_QUALIFIED,
    "FieldAmb_Pattern_Qualified.bsv",
    &["FieldAmb_Pattern_Qualified.bsv", "FieldAmb_Pattern_Sub.bsv"]
);
pass!(
    STRUCT_LITERAL_QUALIFIED_FIELD,
    "StructLit_QualImp_QualField.bs"
);
pass!(
    STRUCT_PATTERN_QUALIFIED_FIELD,
    "StructPat_QualImp_QualField.bs"
);
error!(
    STRUCT_UPDATE_UNQUALIFIED_FIELD,
    "StructUpd_QualImp_UnqualField.bs", "T0140"
);
error!(
    STRUCT_LITERAL_UNQUALIFIED_FIELD,
    "StructLit_QualImp_UnqualField.bs", "T0140"
);
error!(
    STRUCT_PATTERN_UNQUALIFIED_FIELD,
    "StructPat_QualImp_UnqualField.bs", "T0140"
);
error!(
    STRUCT_UPDATE_BAD_QUALIFIER,
    "StructUpd_BadQualField.bs",
    "T0016",
    1,
    &["StructUpd_BadQualField.bs"],
    line_count_assertion!("StructUpd_BadQualField.bs.bsc-out", "Foo.exp", 1)
);
error!(
    STRUCT_LITERAL_BAD_QUALIFIER,
    "StructLit_BadQualField.bs",
    "T0016",
    1,
    &["StructLit_BadQualField.bs"],
    line_count_assertion!("StructLit_BadQualField.bs.bsc-out", "Foo.exp", 1)
);
error!(
    STRUCT_PATTERN_BAD_QUALIFIER,
    "StructPat_BadQualField.bs",
    "T0016",
    1,
    &["StructPat_BadQualField.bs"],
    line_count_assertion!("StructPat_BadQualField.bs.bsc-out", "Foo.exp", 1)
);
error!(
    STRUCT_UPDATE_DUPLICATE_BOTH_QUALIFIED,
    "StructUpd_DupField_BothQual.bs", "T0017"
);
error!(
    STRUCT_LITERAL_DUPLICATE_BOTH_QUALIFIED,
    "StructLit_DupField_BothQual.bs", "T0017"
);
error!(
    STRUCT_PATTERN_DUPLICATE_BOTH_QUALIFIED,
    "StructPat_DupField_BothQual.bs", "T0017"
);
error!(
    STRUCT_UPDATE_DUPLICATE_QUALIFIED_UNQUALIFIED,
    "StructUpd_DupField_QualAndUnqual.bs", "T0017"
);
error!(
    STRUCT_LITERAL_DUPLICATE_QUALIFIED_UNQUALIFIED,
    "StructLit_DupField_QualAndUnqual.bs", "T0017"
);
error!(
    STRUCT_PATTERN_DUPLICATE_QUALIFIED_UNQUALIFIED,
    "StructPat_DupField_QualAndUnqual.bs", "T0017"
);
pass!(PARTIAL_APPLICATION, "PartialApp.bs");
error_golden!(
    PARTIAL_APPLICATION_TOO_MANY_ARGUMENTS,
    "PartialAppTooManyArgs.bs",
    "T0144"
);
error!(UNBOUND_PATTERN, "Unbound_Pattern.bsv", "T0007");

pub(super) const CASES: &[CompileCase] = &[
    UNION_ONE_ARG_NONE,
    UNION_ONE_ARG_NONE_PATTERN,
    STRUCT_UPDATE_WRONG_FIELD,
    STRUCT_SELECT_MULTIPLE_TYPES_WRONG_FIELD,
    STRUCT_LITERAL_WRONG_FIELD,
    STRUCT_SELECT_NOT_IMPORTED,
    STRUCT_SELECT_NOT_VISIBLE,
    STRUCT_SELECT_NOT_STRUCT_FUNCTION,
    FIELD_AMBIGUOUS_SELECT,
    FIELD_AMBIGUOUS_UPDATE,
    FIELD_PATTERN_UNQUALIFIED,
    FIELD_PATTERN_QUALIFIED,
    STRUCT_LITERAL_QUALIFIED_FIELD,
    STRUCT_PATTERN_QUALIFIED_FIELD,
    STRUCT_UPDATE_UNQUALIFIED_FIELD,
    STRUCT_LITERAL_UNQUALIFIED_FIELD,
    STRUCT_PATTERN_UNQUALIFIED_FIELD,
    STRUCT_UPDATE_BAD_QUALIFIER,
    STRUCT_LITERAL_BAD_QUALIFIER,
    STRUCT_PATTERN_BAD_QUALIFIER,
    STRUCT_UPDATE_DUPLICATE_BOTH_QUALIFIED,
    STRUCT_LITERAL_DUPLICATE_BOTH_QUALIFIED,
    STRUCT_PATTERN_DUPLICATE_BOTH_QUALIFIED,
    STRUCT_UPDATE_DUPLICATE_QUALIFIED_UNQUALIFIED,
    STRUCT_LITERAL_DUPLICATE_QUALIFIED_UNQUALIFIED,
    STRUCT_PATTERN_DUPLICATE_QUALIFIED_UNQUALIFIED,
    PARTIAL_APPLICATION,
    PARTIAL_APPLICATION_TOO_MANY_ARGUMENTS,
    UNBOUND_PATTERN,
];
