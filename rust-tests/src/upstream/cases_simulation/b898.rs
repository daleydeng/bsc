//! Origin: `testsuite/bsc.bugs/bluespec_inc/b898/b898.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b898";

macro_rules! backend_scenario {
    (
        $constant:ident,
        $module:literal,
        $expected:literal,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.bugs/bluespec_inc/b898::",
                $module,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[SimulationLinkInput::GeneratedModule(concat!("mk", $module))],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.bugs/bluespec_inc/b898::", $module, "::", $backend_name),
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

backend_scenario!(
    BUG_898_ICARUS,
    "Bug898",
    "sysBug898.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
backend_scenario!(
    BUG_898_BLUESIM,
    "Bug898",
    "sysBug898.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    BUG_898_2_ICARUS,
    "Bug898_2",
    "sysBug898_2.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
backend_scenario!(
    BUG_898_2_BLUESIM,
    "Bug898_2",
    "sysBug898_2.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    BUG_898_ICARUS,
    BUG_898_BLUESIM,
    BUG_898_2_ICARUS,
    BUG_898_2_BLUESIM,
];
