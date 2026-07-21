use super::CompileCase;

pub(super) const INTERFACE_GROUNDED_INCORRECTLY: CompileCase = compile_fail_error_case!(
    "bsc.typechecker/kind/inferkinds::InterfaceGroundedIncorrectly.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "InterfaceGroundedIncorrectly.bsv",
    "T0027"
);
pub(super) const INTERFACE_INFERED_FROM_TYPEDEF: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::InterfaceInferedFromTypedef.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "InterfaceInferedFromTypedef.bsv"
);
pub(super) const INTERFACE_PARTIAL_KIND: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::InterfacePartialKind.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "InterfacePartialKind.bsv"
);
pub(super) const INTERFACE_WRONG_PARTIAL_KIND: CompileCase = compile_fail_error_case!(
    "bsc.typechecker/kind/inferkinds::InterfaceWrongPartialKind.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "InterfaceWrongPartialKind.bsv",
    "T0026"
);
pub(super) const INTERFACE_PARTIAL_KIND_MANY_PARAMS: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::InterfacePartialKindManyParams.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "InterfacePartialKindManyParams.bsv"
);
pub(super) const TYPEDEF_NUMERIC_RESULT: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::TypedefNumericResult.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "TypedefNumericResult.bsv"
);
pub(super) const SUB_UNION_SUB_STRUCT_PARTIAL_KIND: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::SubUnionSubStructPartialKind.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "SubUnionSubStructPartialKind.bsv"
);
pub(super) const CLASS_PARTIAL_KIND: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::ClassPartialKind.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "ClassPartialKind.bsv"
);
pub(super) const TYPE_ALIAS_SHADOW: CompileCase = compile_pass_case!(
    "bsc.typechecker/kind/inferkinds::TypeAliasShadow.bsv",
    "testsuite/bsc.typechecker/kind/inferkinds",
    "TypeAliasShadow.bsv"
);
