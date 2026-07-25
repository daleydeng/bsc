//! Origin: `testsuite/bsc.lib/Divide/divide.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/Divide";

macro_rules! backend_scenario {
    ($constant:ident, $module:literal, $backend:ident, $backend_name:literal, $vcd:expr, $requirement:ident) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.lib/Divide::",
                $module,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[
                concat!($module, ".bsv"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.lib/Divide::", $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass {
                    output: concat!("sys", $module, ".out.expected"),
                },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

macro_rules! divide_scenarios {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        backend_scenario!(
            $bluesim,
            $module,
            Bluesim,
            "bluesim",
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        );
        backend_scenario!(
            $icarus,
            $module,
            Icarus,
            "icarus",
            Some(VcdContract::parse()),
            VerilogEnabled
        );
    };
}

divide_scenarios!(DIVIDER_BLUESIM, DIVIDER_ICARUS, "Test_mkDivider");
divide_scenarios!(
    NON_PIPELINED_DIVIDER_BLUESIM,
    NON_PIPELINED_DIVIDER_ICARUS,
    "Test_mkNonPipelinedDivider"
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    DIVIDER_BLUESIM,
    DIVIDER_ICARUS,
    NON_PIPELINED_DIVIDER_BLUESIM,
    NON_PIPELINED_DIVIDER_ICARUS,
];
