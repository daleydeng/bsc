use super::CompileCase;

const DYNAMIC_DIR: &str = "testsuite/bsc.evaluator/dynamic";
const ERRORS_DIR: &str = "testsuite/bsc.evaluator/dynamic/errors";

pub(super) const DYNAMIC_INTEGER_FAIL: CompileCase = compile_verilog_fail_error_case!(
    "bsc.evaluator/dynamic::DynamicIntegerFail.bsv",
    DYNAMIC_DIR,
    "DynamicIntegerFail.bsv",
    "T0051"
);

macro_rules! dynamic_error_case {
    ($constant:ident, $source:literal) => {
        pub(super) const $constant: CompileCase = compile_verilog_fail_golden_case!(
            concat!("bsc.evaluator/dynamic/errors::", $source),
            ERRORS_DIR,
            $source,
            concat!($source, ".bsc-vcomp-out.expected")
        );
    };
}

dynamic_error_case!(E_RULES_MUX_1, "ERulesMux1.bsv");
dynamic_error_case!(E_RULES_MUX_2, "ERulesMux2.bsv");
dynamic_error_case!(E_RULES_MUX_2A, "ERulesMux2a.bsv");
dynamic_error_case!(E_RULES_MUX_3, "ERulesMux3.bsv");
dynamic_error_case!(E_RULES_MUX_2A_CASE, "ERulesMux2a_Case.bsv");
dynamic_error_case!(E_RULES_MUX_3_CASE, "ERulesMux3_Case.bsv");
dynamic_error_case!(E_RULES_MUX_2A_ARR_SEL, "ERulesMux2a_ArrSel.bsv");
dynamic_error_case!(E_RULES_MUX_3_ARR_SEL, "ERulesMux3_ArrSel.bsv");
dynamic_error_case!(MOD_ARG_CLOCK, "ModArg_Clock.bsv");
dynamic_error_case!(MOD_ARG_RESET, "ModArg_Reset.bsv");
dynamic_error_case!(MOD_ARG_INOUT, "ModArg_Inout.bsv");
dynamic_error_case!(MOD_ARG_PARAM, "ModArg_Param.bsv");
dynamic_error_case!(IFC_CLOCK_IF, "Ifc_Clock_If.bsv");
dynamic_error_case!(IFC_RESET_IF, "Ifc_Reset_If.bsv");
dynamic_error_case!(IFC_INOUT_IF, "Ifc_Inout_If.bsv");
dynamic_error_case!(IFC_CLOCK_CASE, "Ifc_Clock_Case.bsv");
dynamic_error_case!(IFC_RESET_CASE, "Ifc_Reset_Case.bsv");
dynamic_error_case!(IFC_INOUT_CASE, "Ifc_Inout_Case.bsv");
dynamic_error_case!(IFC_CLOCK_ARR_SEL, "Ifc_Clock_ArrSel.bsv");
dynamic_error_case!(IFC_RESET_ARR_SEL, "Ifc_Reset_ArrSel.bsv");
dynamic_error_case!(IFC_INOUT_ARR_SEL, "Ifc_Inout_ArrSel.bsv");
