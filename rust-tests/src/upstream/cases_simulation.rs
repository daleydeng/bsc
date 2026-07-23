use super::{CaseModule, SimulationCase};
use std::sync::OnceLock;

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

macro_rules! case_modules {
    ($($module:ident),+ $(,)?) => {
        $(mod $module;)+

        pub(super) const MODULES: &[CaseModule<SimulationCase>] = &[
            $(CaseModule {
                name: stringify!($module),
                cases: $module::CASES,
            },)+
        ];
    };
}

case_modules!(
    b810,
    bounds_select,
    bounds_update,
    case_syntax,
    conflict_free,
    cross_suite_direct,
    dynamic,
    dynamic_strings,
    gearbox,
    read_desugaring,
    small_regressions,
    static_regressions,
);

pub(super) fn cases() -> &'static [SimulationCase] {
    static CASES: OnceLock<Vec<SimulationCase>> = OnceLock::new();
    CASES
        .get_or_init(|| {
            MODULES
                .iter()
                .flat_map(|module| module.cases.iter().copied())
                .collect()
        })
        .as_slice()
}
