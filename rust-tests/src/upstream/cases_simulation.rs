use super::SimulationCase;

macro_rules! bluesim_case {
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr) => {
        bluesim_case!($name, $fixture_dir, $module, $expected, &[])
    };
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr, $compile_options:expr) => {
        $crate::upstream::SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            expected: $expected,
            compile_options: $compile_options,
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: $crate::upstream::SimulationBackend::Bluesim,
            requirement: $crate::upstream::Requirement::BluesimEnabled,
            timeout: $crate::BSC_TIMEOUT,
            heavy: false,
        }
    };
}

macro_rules! icarus_case {
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr) => {
        icarus_case!($name, $fixture_dir, $module, $expected, &[])
    };
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr, $compile_options:expr) => {
        icarus_case!(
            $name,
            $fixture_dir,
            $module,
            $expected,
            $compile_options,
            $crate::upstream::Requirement::VerilogEnabled
        )
    };
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr, $compile_options:expr, $requirement:expr) => {
        $crate::upstream::SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            expected: $expected,
            compile_options: $compile_options,
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: $crate::upstream::SimulationBackend::Icarus,
            requirement: $requirement,
            timeout: $crate::BSC_TIMEOUT,
            heavy: false,
        }
    };
}

mod b810;
mod bounds_select;
mod bounds_update;
mod case_syntax;
mod conflict_free;
mod direct_batch;
mod dynamic;
mod dynamic_strings;
mod gearbox;
mod read_desugaring;
mod small_regressions;

pub const SIMULATION_CASES: &[SimulationCase] = &[
    small_regressions::B1037_BLUESIM,
    small_regressions::B1037_ICARUS,
    small_regressions::B1045_BLUESIM,
    small_regressions::B1045_ICARUS,
    direct_batch::FIR_BLUESIM,
    direct_batch::FIR_ICARUS,
    direct_batch::PROPERTIES_BLUESIM,
    direct_batch::PROPERTIES_ICARUS,
    direct_batch::HAMMING_BLUESIM,
    direct_batch::HAMMING_ICARUS,
    direct_batch::BRAM_BLUESIM,
    direct_batch::BRAM_ICARUS,
    direct_batch::BRAM_1_BLUESIM,
    direct_batch::BRAM_1_ICARUS,
    direct_batch::BRAM_PIPELINED_BLUESIM,
    direct_batch::BRAM_PIPELINED_ICARUS,
    b810::BUG_810_1_BLUESIM,
    b810::BUG_810_1_ICARUS,
    b810::BUG_810_3_BLUESIM,
    b810::BUG_810_3_ICARUS,
    b810::OPT_BUG_BLUESIM,
    b810::OPT_BUG_ICARUS,
    read_desugaring::LIST_DESUGAR_BLUESIM,
    read_desugaring::LIST_DESUGAR_ICARUS,
    read_desugaring::STRUCT_REG_BLUESIM,
    read_desugaring::STRUCT_REG_ICARUS,
    read_desugaring::TWO_D_UPDATE_TEST_BLUESIM,
    read_desugaring::TWO_D_UPDATE_TEST_ICARUS,
    case_syntax::MATCHES_MIXED_LIT_BLUESIM,
    case_syntax::MATCHES_MIXED_LIT_ICARUS,
    case_syntax::MIXED_HEX_BLUESIM,
    case_syntax::MIXED_HEX_ICARUS,
    case_syntax::MIXED_OCT_BLUESIM,
    case_syntax::MIXED_OCT_ICARUS,
    gearbox::FULL_SPEED_BLUESIM,
    gearbox::FULL_SPEED_ICARUS,
    gearbox::BUBBLE_BLUESIM,
    gearbox::BUBBLE_ICARUS,
    gearbox::ONE_TO_ONE_BLUESIM,
    gearbox::ONE_TO_ONE_ICARUS,
    gearbox::SAME_CLOCK_BLUESIM,
    gearbox::SAME_CLOCK_ICARUS,
    dynamic::INTEGER_BLUESIM,
    dynamic::INTEGER_ICARUS,
    dynamic::INTEGER_NESTED_BLUESIM,
    dynamic::INTEGER_NESTED_ICARUS,
    dynamic::DIV_BLUESIM,
    dynamic::DIV_ICARUS,
    dynamic::NEG_BLUESIM,
    dynamic::NEG_ICARUS,
    dynamic::NEG_2_BLUESIM,
    dynamic::NEG_2_ICARUS,
    dynamic::LT_BLUESIM,
    dynamic::LT_ICARUS,
    dynamic::ADD_BLUESIM,
    dynamic::ADD_ICARUS,
    dynamic_strings::MUX_BLUESIM,
    dynamic_strings::MUX_ICARUS,
    dynamic_strings::CONCAT_BLUESIM,
    dynamic_strings::CONCAT_ICARUS,
    dynamic_strings::INTEGER_BLUESIM,
    dynamic_strings::INTEGER_ICARUS,
    dynamic_strings::INTEGER_WITH_NULL_BLUESIM,
    dynamic_strings::INTEGER_WITH_NULL_ICARUS,
    dynamic_strings::EQ_BLUESIM,
    dynamic_strings::EQ_ICARUS,
    dynamic_strings::LT_BLUESIM,
    dynamic_strings::LT_ICARUS,
    dynamic_strings::FORMAT_BLUESIM,
    dynamic_strings::FORMAT_ICARUS,
    bounds_select::ARRAY_1_BLUESIM,
    bounds_select::ARRAY_1_ICARUS,
    bounds_select::ARRAY_2_BLUESIM,
    bounds_select::ARRAY_2_ICARUS,
    bounds_select::LIST_1_BLUESIM,
    bounds_select::LIST_1_ICARUS,
    bounds_select::LIST_2_BLUESIM,
    bounds_select::LIST_2_ICARUS,
    bounds_select::VECTOR_1_BLUESIM,
    bounds_select::VECTOR_1_ICARUS,
    bounds_select::VECTOR_2_BLUESIM,
    bounds_select::VECTOR_2_ICARUS,
    bounds_select::LIST_N_1_BLUESIM,
    bounds_select::LIST_N_1_ICARUS,
    bounds_select::LIST_N_2_BLUESIM,
    bounds_select::LIST_N_2_ICARUS,
    bounds_select::BIT_1_BLUESIM,
    bounds_select::BIT_1_ICARUS,
    bounds_select::BIT_2_BLUESIM,
    bounds_select::BIT_2_ICARUS,
    bounds_update::ARRAY_1_BLUESIM,
    bounds_update::ARRAY_1_ICARUS,
    bounds_update::ARRAY_2_BLUESIM,
    bounds_update::ARRAY_2_ICARUS,
    bounds_update::LIST_1_BLUESIM,
    bounds_update::LIST_1_ICARUS,
    bounds_update::LIST_2_BLUESIM,
    bounds_update::LIST_2_ICARUS,
    bounds_update::VECTOR_1_BLUESIM,
    bounds_update::VECTOR_1_ICARUS,
    bounds_update::VECTOR_2_BLUESIM,
    bounds_update::VECTOR_2_ICARUS,
    bounds_update::LIST_N_1_BLUESIM,
    bounds_update::LIST_N_1_ICARUS,
    bounds_update::LIST_N_2_BLUESIM,
    bounds_update::LIST_N_2_ICARUS,
    bounds_update::BIT_1_BLUESIM,
    bounds_update::BIT_1_ICARUS,
    bounds_update::BIT_2_BLUESIM,
    bounds_update::BIT_2_ICARUS,
    conflict_free::OK_BLUESIM,
    conflict_free::OK_ICARUS,
    conflict_free::OK_2_BLUESIM,
    conflict_free::OK_2_ICARUS,
    conflict_free::OK_3_BLUESIM,
    conflict_free::OK_3_ICARUS,
    conflict_free::NOT_OK_BLUESIM,
    conflict_free::NOT_OK_ICARUS,
    conflict_free::RESOURCE_BLUESIM,
    conflict_free::RESOURCE_ICARUS,
    conflict_free::EXEC_ORDER_1_BLUESIM,
    conflict_free::EXEC_ORDER_1_ICARUS,
    conflict_free::EXEC_ORDER_2_BLUESIM,
    conflict_free::EXEC_ORDER_2_ICARUS,
    conflict_free::EXEC_ORDER_3_BLUESIM,
    conflict_free::EXEC_ORDER_3_ICARUS,
    conflict_free::SWITCH_BLUESIM,
    conflict_free::SWITCH_ICARUS,
];
