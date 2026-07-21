//! Origin: `testsuite/bsc.syntax/bsv05/underscore/underscore.exp`.

use super::CompileCase;

pub(super) const TOP_DEF_VAR_TYPE: CompileCase = compile_pass_case!(
    "bsc.syntax/bsv05/underscore::TopDef_Var_Type.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_Var_Type.bsv"
);
pub(super) const TOP_DEF_VAR_TYPE_BAD: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::TopDef_Var_Type_Bad.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_Var_Type_Bad.bsv",
    "T0020"
);
pub(super) const TOP_DEF_VAR_NO_TYPE: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::TopDef_Var_NoType.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_Var_NoType.bsv",
    "P0127"
);
pub(super) const TOP_DEF_FUNC_TYPE: CompileCase = compile_pass_case!(
    "bsc.syntax/bsv05/underscore::TopDef_Func_Type.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_Func_Type.bsv"
);
pub(super) const TOP_DEF_FUNC_TYPE_BAD: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::TopDef_Func_Type_Bad.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_Func_Type_Bad.bsv",
    "T0084"
);
pub(super) const TOP_DEF_FUNC_ARG: CompileCase = compile_pass_case!(
    "bsc.syntax/bsv05/underscore::TopDef_FuncArg.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_FuncArg.bsv"
);
pub(super) const TOP_DEF_FUNC_ARG_BAD: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::TopDef_FuncArg_Bad.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_FuncArg_Bad.bsv",
    "T0020"
);
pub(super) const MOD_DEF_PORT_ARG: CompileCase = compile_pass_case!(
    "bsc.syntax/bsv05/underscore::ModDef_PortArg.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "ModDef_PortArg.bsv"
);
pub(super) const MOD_DEF_PORT_ARG_BAD: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::ModDef_PortArg_Bad.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "ModDef_PortArg_Bad.bsv",
    "T0020"
);
pub(super) const IFC_DECL_METH_ARG: CompileCase = compile_verilog_pass_case!(
    "bsc.syntax/bsv05/underscore::IfcDecl_MethArg.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "IfcDecl_MethArg.bsv"
);
pub(super) const IFC_DEF_METH_ARG: CompileCase = compile_verilog_pass_case!(
    "bsc.syntax/bsv05/underscore::IfcDef_MethArg.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "IfcDef_MethArg.bsv"
);
pub(super) const IFC_DEF_METH_ARG_BAD: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::IfcDef_MethArg_Bad.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "IfcDef_MethArg_Bad.bsv",
    "T0020"
);
pub(super) const METH_ARG_SYNTH: CompileCase = compile_verilog_pass_case!(
    "bsc.syntax/bsv05/underscore::MethArg_Synth.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "MethArg_Synth.bsv"
);
pub(super) const TOP_DEF_FUNC_ARG_QMARK: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::TopDef_FuncArg_Qmark.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "TopDef_FuncArg_Qmark.bsv",
    "P0005"
);
pub(super) const MOD_DEF_PORT_ARG_QMARK: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::ModDef_PortArg_Qmark.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "ModDef_PortArg_Qmark.bsv",
    "P0005"
);
pub(super) const IFC_DECL_METH_ARG_QMARK: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::IfcDecl_MethArg_Qmark.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "IfcDecl_MethArg_Qmark.bsv",
    "P0005"
);
pub(super) const IFC_DEF_METH_ARG_QMARK: CompileCase = compile_fail_error_case!(
    "bsc.syntax/bsv05/underscore::IfcDef_MethArg_Qmark.bsv",
    "testsuite/bsc.syntax/bsv05/underscore",
    "IfcDef_MethArg_Qmark.bsv",
    "P0005"
);
