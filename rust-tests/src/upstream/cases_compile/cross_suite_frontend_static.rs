//! Origins:
//! - `testsuite/bsc.syntax/bh/underscore/underscore.exp`
//! - `testsuite/bsc.syntax/bsv05/attribs/attribs.exp`
//! - `testsuite/bsc.bsc_examples/trafficlight/trafficlight.exp`
//! - `testsuite/bsc.typechecker/ctxreduce/ctxreduce.exp`

use super::CompileCase;

macro_rules! frontend_pass {
    ($prefix:literal, $fixture_dir:literal, $source:literal) => {
        compile_pass_case!(concat!($prefix, "::", $source), $fixture_dir, $source)
    };
}

macro_rules! frontend_error {
    ($prefix:literal, $fixture_dir:literal, $source:literal, $tag:literal) => {
        compile_fail_error_case!(concat!($prefix, "::", $source), $fixture_dir, $source, $tag)
    };
}

macro_rules! underscore_pass {
    ($source:literal) => {
        frontend_pass!(
            "bsc.syntax/bh/underscore",
            "testsuite/bsc.syntax/bh/underscore",
            $source
        )
    };
}

macro_rules! underscore_error {
    ($source:literal, $tag:literal) => {
        frontend_error!(
            "bsc.syntax/bh/underscore",
            "testsuite/bsc.syntax/bh/underscore",
            $source,
            $tag
        )
    };
}

macro_rules! attribs_error {
    ($source:literal, $tag:literal) => {
        frontend_error!(
            "bsc.syntax/bsv05/attribs",
            "testsuite/bsc.syntax/bsv05/attribs",
            $source,
            $tag
        )
    };
}

macro_rules! traffic_light_pass {
    ($source:literal) => {
        frontend_pass!(
            "bsc.bsc_examples/trafficlight",
            "testsuite/bsc.bsc_examples/trafficlight",
            $source
        )
    };
}

macro_rules! ctx_reduce_pass {
    ($source:literal) => {
        frontend_pass!(
            "bsc.typechecker/ctxreduce",
            "testsuite/bsc.typechecker/ctxreduce",
            $source
        )
    };
}

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.syntax/bh/underscore/underscore.exp
    underscore_error!("VarDefn_NoType.bs", "P0005"),
    underscore_error!("VarDefn_Type.bs", "P0005"),
    underscore_error!("VarDefn_Clauses_NoType.bs", "P0005"),
    underscore_pass!("Defl_NoType.bs"),
    underscore_error!("Defl_Type.bs", "P0005"),
    underscore_error!("Defl_Clauses_NoType.bs", "P0005"),
    underscore_error!("StructDefn_Field.bs", "P0005"),
    underscore_error!("StructDefn_Field_WithDefault.bs", "P0005"),
    underscore_pass!("StmtBind_NoType.bs"),
    underscore_pass!("StmtBind_Type_OneLine.bs"),
    underscore_pass!("StmtBind_Type_TwoLines.bs"),
    underscore_error!("Export.bs", "P0005"),
    underscore_pass!("Lambda_Arg.bs"),
    underscore_error!("Foreign.bs", "P0005"),
    underscore_error!("Primitive.bs", "P0005"),
    underscore_pass!("Expr.bs"),
    underscore_pass!("Expr_Where.bs"),
    underscore_error!("Expr_Range.bs", "T0035"),
    underscore_pass!("Expr_HasType.bs"),
    underscore_pass!("Expr_FieldUpd.bs"),
    underscore_pass!("Expr_Ap.bs"),
    underscore_pass!("Pattern.bs"),
    underscore_pass!("Pattern_As.bs"),
    // testsuite/bsc.syntax/bsv05/attribs/attribs.exp
    attribs_error!("AttribsBVI.bsv", "P0202"),
    attribs_error!("AttribsCase.bsv", "P0203"),
    attribs_error!("AttribsNestedAction.bsv", "P0016"),
    attribs_error!("AttribsActionNakedExpr.bsv", "P0027"),
    attribs_error!("AttribsIfcNaked.bsv", "P0027"),
    attribs_error!("AttribsSubIfcEq.bsv", "P0027"),
    attribs_error!("AttribsSubIfcSemi.bsv", "P0027"),
    // testsuite/bsc.bsc_examples/trafficlight/trafficlight.exp
    traffic_light_pass!("TL0.bs"),
    traffic_light_pass!("TL1.bs"),
    traffic_light_pass!("TL2.bs"),
    traffic_light_pass!("TL3.bs"),
    traffic_light_pass!("TL4.bs"),
    traffic_light_pass!("TL5.bs"),
    traffic_light_pass!("TL6.bs"),
    traffic_light_pass!("TL7.bs"),
    traffic_light_pass!("TL8.bs"),
    traffic_light_pass!("TL9.bs"),
    // testsuite/bsc.typechecker/ctxreduce/ctxreduce.exp
    ctx_reduce_pass!("SatisfyFV.bsv"),
    ctx_reduce_pass!("AliasSizeOf.bsv"),
    ctx_reduce_pass!("AliasSizeOf_Instance.bsv"),
    ctx_reduce_pass!("SiblingFundepSubst.bs"),
];
