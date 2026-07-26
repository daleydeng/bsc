use super::{BluesimWorkflowScenario, CaseModule};
use std::sync::OnceLock;

macro_rules! case_modules {
    ($($module:ident),+ $(,)?) => {
        $(mod $module;)+

        pub(super) const MODULES: &[CaseModule<BluesimWorkflowScenario>] = &[
            $(CaseModule {
                name: stringify!($module),
                cases: $module::SCENARIOS,
            },)+
        ];
    };
}

case_modules!(
    b1243,
    b1489,
    bluesim_debugging,
    bluesim_schedule,
    bluespec_inc_build_only,
    commandline_vcd,
    eq3,
    interactive_examples,
    library_latency,
    library_sram,
    parse_strings,
    rdy_en_pragmas,
    traffic_light_controller_separate,
    use_cond,
);

pub(super) fn scenarios() -> &'static [BluesimWorkflowScenario] {
    static SCENARIOS: OnceLock<Vec<BluesimWorkflowScenario>> = OnceLock::new();
    SCENARIOS
        .get_or_init(|| {
            MODULES
                .iter()
                .flat_map(|module| module.cases.iter().copied())
                .collect()
        })
        .as_slice()
}
