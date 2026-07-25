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
    amba_trans_model,
    b402,
    b621,
    b810,
    b898,
    backend_runtime_regressions,
    bh_pragmas,
    bounds_select,
    bounds_update,
    capability_language,
    capability_verilog,
    case_syntax,
    cntrs,
    completion_buffer,
    conflict_free,
    constant_set_examples,
    constructors,
    cross_suite_direct,
    cross_suite_examples_wrappers,
    cross_suite_options_memory,
    cross_suite_static_examples,
    cross_suite_static_language,
    cross_suite_static_library,
    cshow,
    dynamic,
    dynamic_strings,
    foldable,
    fwrite,
    gating,
    gearbox,
    generics,
    gh836,
    library_runtime,
    life,
    memq,
    multi_bsv_examples,
    multi_mcd_library,
    multi_misc,
    multi_stmt_fsm,
    read_desugaring,
    small_regressions,
    static_regressions,
    stmt_fsm,
    string_types,
    vcd_smoke,
    vector,
    vending,
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
