use super::CompileCase;

macro_rules! compile_pass_case {
    ($name:literal, $fixture_dir:literal, $source:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_error_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $tag:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_golden_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $golden:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: $crate::upstream::CompileExpectation::Fail,
            golden: Some($crate::upstream::GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_fail_error_golden_case {
    ($name:literal, $fixture_dir:literal, $source:literal, $tag:literal, $golden:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: Some($crate::upstream::GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Frontend,
            requirement: $crate::upstream::Requirement::Always,
        }
    };
}

macro_rules! compile_verilog_pass_case {
    ($name:literal, $fixture_dir:literal, $source:literal) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! compile_verilog_fail_error_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $tag:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: $crate::upstream::CompileExpectation::FailWithDiagnostic {
                kind: $crate::upstream::DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: None,
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

macro_rules! compile_verilog_fail_golden_case {
    ($name:expr, $fixture_dir:expr, $source:expr, $golden:expr) => {
        $crate::upstream::CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source, $golden],
            expectation: $crate::upstream::CompileExpectation::Fail,
            golden: Some($crate::upstream::GoldenExpectation { expected: $golden }),
            options: &[],
            nodeps: false,
            mode: $crate::upstream::CompileMode::Verilog { module: None },
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

mod attr_errors;
mod bluespec_inc_fail;
mod bluespec_inc_golden_mixed;
mod bluespec_inc_pass;
mod bounds_select;
mod bounds_update;
mod dynamic;
mod enot_field;
mod infer_kinds;
mod other_directories;
mod underscore;

pub const CASES: &[CompileCase] = &[
    bluespec_inc_pass::B600,
    bluespec_inc_pass::B532,
    bluespec_inc_pass::B1470,
    bluespec_inc_pass::B271,
    bluespec_inc_pass::B1599,
    bluespec_inc_pass::B198,
    bluespec_inc_pass::B289,
    bluespec_inc_pass::B1294,
    bluespec_inc_pass::B267,
    bluespec_inc_pass::B547,
    bluespec_inc_pass::B41,
    bluespec_inc_pass::B542,
    bluespec_inc_pass::B384,
    bluespec_inc_pass::B436,
    bluespec_inc_pass::B394,
    other_directories::GH435,
    other_directories::GH309,
    bluespec_inc_pass::B927,
    bluespec_inc_fail::B1040,
    bluespec_inc_fail::B671,
    bluespec_inc_fail::B417,
    bluespec_inc_fail::B423,
    bluespec_inc_fail::B492,
    bluespec_inc_fail::B1044,
    bluespec_inc_fail::B340,
    bluespec_inc_fail::B460,
    bluespec_inc_fail::B459,
    bluespec_inc_fail::B580_1,
    bluespec_inc_fail::B580_2,
    bluespec_inc_golden_mixed::B1586,
    bluespec_inc_golden_mixed::B269,
    bluespec_inc_golden_mixed::B880,
    other_directories::BUG_ID_313,
    other_directories::BUG_ID_149,
    other_directories::BUG_ID_169,
    other_directories::E_MISSING_NL,
    other_directories::E_UNBOUND_CL_CON,
    other_directories::E_FIELD_AMB,
    bluespec_inc_golden_mixed::B1493_GOOD,
    bluespec_inc_golden_mixed::B1493_BAD,
    other_directories::MODULE_TYPE_GOOD,
    other_directories::MODULE_TYPE_MISSING_ARG,
    bluespec_inc_golden_mixed::B557_GOOD,
    bluespec_inc_golden_mixed::B557_BAD,
    enot_field::ENOT_FIELD_1,
    enot_field::ENOT_FIELD_2,
    enot_field::ENOT_FIELD_3,
    enot_field::ENOT_FIELD_4,
    attr_errors::T1,
    attr_errors::T2,
    attr_errors::T3,
    attr_errors::T4,
    attr_errors::T5,
    attr_errors::T6,
    attr_errors::MULTIPLE_ATTRIB_MODULE,
    attr_errors::MULTIPLE_ATTRIB_FUNC,
    attr_errors::MULTIPLE_ATTRIB_RULE,
    attr_errors::MULTIPLE_SAME_ATTRIB_MODULE,
    infer_kinds::INTERFACE_GROUNDED_INCORRECTLY,
    infer_kinds::INTERFACE_INFERED_FROM_TYPEDEF,
    infer_kinds::INTERFACE_PARTIAL_KIND,
    infer_kinds::INTERFACE_WRONG_PARTIAL_KIND,
    infer_kinds::INTERFACE_PARTIAL_KIND_MANY_PARAMS,
    infer_kinds::TYPEDEF_NUMERIC_RESULT,
    infer_kinds::SUB_UNION_SUB_STRUCT_PARTIAL_KIND,
    infer_kinds::CLASS_PARTIAL_KIND,
    infer_kinds::TYPE_ALIAS_SHADOW,
    underscore::TOP_DEF_VAR_TYPE,
    underscore::TOP_DEF_VAR_TYPE_BAD,
    underscore::TOP_DEF_VAR_NO_TYPE,
    underscore::TOP_DEF_FUNC_TYPE,
    underscore::TOP_DEF_FUNC_TYPE_BAD,
    underscore::TOP_DEF_FUNC_ARG,
    underscore::TOP_DEF_FUNC_ARG_BAD,
    underscore::MOD_DEF_PORT_ARG,
    underscore::MOD_DEF_PORT_ARG_BAD,
    underscore::IFC_DECL_METH_ARG,
    underscore::IFC_DEF_METH_ARG,
    underscore::IFC_DEF_METH_ARG_BAD,
    underscore::METH_ARG_SYNTH,
    underscore::TOP_DEF_FUNC_ARG_QMARK,
    underscore::MOD_DEF_PORT_ARG_QMARK,
    underscore::IFC_DECL_METH_ARG_QMARK,
    underscore::IFC_DEF_METH_ARG_QMARK,
    dynamic::DYNAMIC_INTEGER_FAIL,
    dynamic::E_RULES_MUX_1,
    dynamic::E_RULES_MUX_2,
    dynamic::E_RULES_MUX_2A,
    dynamic::E_RULES_MUX_3,
    dynamic::E_RULES_MUX_2A_CASE,
    dynamic::E_RULES_MUX_3_CASE,
    dynamic::E_RULES_MUX_2A_ARR_SEL,
    dynamic::E_RULES_MUX_3_ARR_SEL,
    dynamic::MOD_ARG_CLOCK,
    dynamic::MOD_ARG_RESET,
    dynamic::MOD_ARG_INOUT,
    dynamic::MOD_ARG_PARAM,
    dynamic::IFC_CLOCK_IF,
    dynamic::IFC_RESET_IF,
    dynamic::IFC_INOUT_IF,
    dynamic::IFC_CLOCK_CASE,
    dynamic::IFC_RESET_CASE,
    dynamic::IFC_INOUT_CASE,
    dynamic::IFC_CLOCK_ARR_SEL,
    dynamic::IFC_RESET_ARR_SEL,
    dynamic::IFC_INOUT_ARR_SEL,
    bounds_select::ARRAY_OUT_OF_BOUNDS_1,
    bounds_select::ARRAY_OUT_OF_BOUNDS_2,
    bounds_select::LIST_OUT_OF_BOUNDS_1,
    bounds_select::LIST_OUT_OF_BOUNDS_2,
    bounds_select::VECTOR_OUT_OF_BOUNDS_1,
    bounds_select::VECTOR_OUT_OF_BOUNDS_2,
    bounds_select::LIST_N_OUT_OF_BOUNDS_1,
    bounds_select::LIST_N_OUT_OF_BOUNDS_2,
    bounds_select::BIT_OUT_OF_BOUNDS_1,
    bounds_select::BIT_OUT_OF_BOUNDS_2,
    bounds_update::ARRAY_OUT_OF_BOUNDS_1,
    bounds_update::ARRAY_OUT_OF_BOUNDS_2,
    bounds_update::LIST_OUT_OF_BOUNDS_1,
    bounds_update::LIST_OUT_OF_BOUNDS_2,
    bounds_update::VECTOR_OUT_OF_BOUNDS_1,
    bounds_update::VECTOR_OUT_OF_BOUNDS_2,
    bounds_update::LIST_N_OUT_OF_BOUNDS_1,
    bounds_update::LIST_N_OUT_OF_BOUNDS_2,
    bounds_update::BIT_OUT_OF_BOUNDS_1,
    bounds_update::BIT_OUT_OF_BOUNDS_2,
];
