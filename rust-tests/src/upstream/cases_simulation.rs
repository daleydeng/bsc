use super::{CaseModule, SimulationScenario};
use std::sync::OnceLock;

macro_rules! case_modules {
    ($($module:ident),+ $(,)?) => {
        $(mod $module;)+

        pub(super) const MODULES: &[CaseModule<SimulationScenario>] = &[
            $(CaseModule {
                name: stringify!($module),
                cases: $module::SCENARIOS,
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
    cross_suite_examples_wrappers,
    cross_suite_options_memory,
    cross_suite_static_examples,
    cross_suite_static_language,
    cross_suite_static_library,
    dynamic,
    dynamic_strings,
    gearbox,
    read_desugaring,
    small_regressions,
    static_regressions,
    vcd_smoke,
);

pub(super) fn scenarios() -> &'static [SimulationScenario] {
    static SCENARIOS: OnceLock<Vec<SimulationScenario>> = OnceLock::new();
    SCENARIOS
        .get_or_init(|| {
            MODULES
                .iter()
                .flat_map(|module| module.cases.iter().copied())
                .collect()
        })
        .as_slice()
}
