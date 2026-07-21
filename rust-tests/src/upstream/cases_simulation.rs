use super::SimulationCase;

macro_rules! bluesim_case {
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr) => {
        $crate::upstream::SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            expected: $expected,
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: $crate::upstream::SimulationBackend::Bluesim,
            requirement: $crate::upstream::Requirement::BluesimEnabled,
        }
    };
}

macro_rules! icarus_case {
    ($name:expr, $fixture_dir:expr, $module:expr, $expected:expr) => {
        $crate::upstream::SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            expected: $expected,
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: $crate::upstream::SimulationBackend::Icarus,
            requirement: $crate::upstream::Requirement::VerilogEnabled,
        }
    };
}

mod bounds_select;
mod bounds_update;
mod dynamic;
mod gearbox;

pub const SIMULATION_CASES: &[SimulationCase] = &[
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
];
