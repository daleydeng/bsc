//! Origin: `testsuite/bsc.mcd/Gating/Gating.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.mcd/Gating";
const FLAGS: &[&str] = &[];

macro_rules! backend_scenario {
    (
        $constant:ident,
        $module:literal,
        $expected:expr,
        [$($generated_module:literal),* $(,)?],
        $compile_options:expr,
        $backend:ident,
        $backend_name:literal,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.mcd/Gating::",
                $module,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[
                $(SimulationLinkInput::GeneratedModule($generated_module),)*
            ],
            compile_options: $compile_options,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.mcd/Gating::", $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

macro_rules! separate_scenarios {
    (
        $bluesim_constant:ident,
        $icarus_constant:ident,
        $module:literal,
        [$($generated_module:literal),* $(,)?],
        $compile_options:expr
    ) => {
        separate_scenarios!(
            $bluesim_constant,
            $icarus_constant,
            $module,
            concat!("sys", $module, ".out.expected"),
            [$($generated_module),*],
            $compile_options
        );
    };
    (
        $bluesim_constant:ident,
        $icarus_constant:ident,
        $module:literal,
        $expected:expr,
        [$($generated_module:literal),* $(,)?],
        $compile_options:expr
    ) => {
        backend_scenario!(
            $bluesim_constant,
            $module,
            $expected,
            [$($generated_module),*],
            $compile_options,
            Bluesim,
            "bluesim",
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        );
        backend_scenario!(
            $icarus_constant,
            $module,
            $expected,
            [$($generated_module),*],
            $compile_options,
            Icarus,
            "icarus",
            Some(VcdContract::parse()),
            VerilogEnabled
        );
    };
}

separate_scenarios!(
    GATED_CLOCK_ONE_MOD_BLUESIM,
    GATED_CLOCK_ONE_MOD_ICARUS,
    "GatedClock_OneMod",
    [],
    FLAGS
);
separate_scenarios!(
    GATED_CLOCK_TWO_MOD_ONE_SYN_BLUESIM,
    GATED_CLOCK_TWO_MOD_ONE_SYN_ICARUS,
    "GatedClock_TwoModOneSyn",
    "sysGatedClock_OneMod.out.expected",
    [],
    FLAGS
);
separate_scenarios!(
    GATED_CLOCK_TWO_MOD_TWO_SYN_BLUESIM,
    GATED_CLOCK_TWO_MOD_TWO_SYN_ICARUS,
    "GatedClock_TwoModTwoSyn",
    "sysGatedClock_OneMod.out.expected",
    ["mkGatedClock_TwoModTwoSyn_Sub"],
    FLAGS
);

pub(super) const GATED_CLOCK_CYCLE: SimulationScenario = SimulationScenario {
    name: "bsc.mcd/Gating::GatedClockCycle",
    fixture_dir: FIXTURE_DIR,
    source: "GatedClockCycle.bsv",
    fixtures: &["GatedClockCycle.bsv", "sysGatedClockCycle.out.expected"],
    top: "sysGatedClockCycle",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.mcd/Gating::GatedClockCycle::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysGatedClockCycle.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.mcd/Gating::GatedClockCycle::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysGatedClockCycle.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

separate_scenarios!(
    SUB_METHOD_BLUESIM,
    SUB_METHOD_ICARUS,
    "SubMethod",
    "sysGatedClock_OneMod.out.expected",
    [],
    FLAGS
);
separate_scenarios!(
    SUB_RULE_BLUESIM,
    SUB_RULE_ICARUS,
    "SubRule",
    "sysGatedClock_OneMod.out.expected",
    [],
    FLAGS
);
separate_scenarios!(
    METHOD_TB_BLUESIM,
    METHOD_TB_ICARUS,
    "MethodTb",
    "sysGatedClock_OneMod.out.expected",
    [],
    FLAGS
);
separate_scenarios!(
    RULE_TB_BLUESIM,
    RULE_TB_ICARUS,
    "RuleTb",
    "sysGatedClock_OneMod.out.expected",
    [],
    FLAGS
);
separate_scenarios!(
    METHOD_TB_2_BLUESIM,
    METHOD_TB_2_ICARUS,
    "MethodTb2",
    "sysGatedClock_OneMod.out.expected",
    [],
    FLAGS
);

separate_scenarios!(
    METHOD_TRUE_BLUESIM,
    METHOD_TRUE_ICARUS,
    "MethodTrue",
    [],
    &[]
);
separate_scenarios!(
    METHOD_FALSE_BLUESIM,
    METHOD_FALSE_ICARUS,
    "MethodFalse",
    [],
    &[]
);

backend_scenario!(
    DEFAULT_CLOCK_METHOD_BLUESIM,
    "DefaultClockMethod",
    "sysDefaultClockMethod.out.expected",
    [],
    &[],
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    INPUT_CLOCK_METHOD_BLUESIM,
    "InputClockMethod",
    "sysInputClockMethod.out.expected",
    [],
    &[],
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    GATED_CLOCK_ONE_MOD_BLUESIM,
    GATED_CLOCK_ONE_MOD_ICARUS,
    GATED_CLOCK_TWO_MOD_ONE_SYN_BLUESIM,
    GATED_CLOCK_TWO_MOD_ONE_SYN_ICARUS,
    GATED_CLOCK_TWO_MOD_TWO_SYN_BLUESIM,
    GATED_CLOCK_TWO_MOD_TWO_SYN_ICARUS,
    GATED_CLOCK_CYCLE,
    SUB_METHOD_BLUESIM,
    SUB_METHOD_ICARUS,
    SUB_RULE_BLUESIM,
    SUB_RULE_ICARUS,
    METHOD_TB_BLUESIM,
    METHOD_TB_ICARUS,
    RULE_TB_BLUESIM,
    RULE_TB_ICARUS,
    METHOD_TB_2_BLUESIM,
    METHOD_TB_2_ICARUS,
    METHOD_TRUE_BLUESIM,
    METHOD_TRUE_ICARUS,
    METHOD_FALSE_BLUESIM,
    METHOD_FALSE_ICARUS,
    DEFAULT_CLOCK_METHOD_BLUESIM,
    INPUT_CLOCK_METHOD_BLUESIM,
];
