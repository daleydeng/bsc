//! Origin: `testsuite/bsc.mcd/Gearbox/Gearbox.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.mcd/Gearbox";

macro_rules! gearbox_scenario {
    (
        $constant:ident,
        $module:literal,
        $expected:literal,
        $backend:ident,
        $backend_name:literal,
        $vcd:expr,
        $requirement:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.mcd/Gearbox::",
                $module,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.mcd/Gearbox::", $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: $requirement,
            }],
        };
    };
}

gearbox_scenario!(
    FULL_SPEED_BLUESIM,
    "GearboxFullSpeedTest",
    "sysGearboxFullSpeedTest.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
gearbox_scenario!(
    FULL_SPEED_ICARUS,
    "GearboxFullSpeedTest",
    "sysGearboxFullSpeedTest.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);
gearbox_scenario!(
    BUBBLE_BLUESIM,
    "GearboxBubbleTest",
    "sysGearboxBubbleTest.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
gearbox_scenario!(
    BUBBLE_ICARUS,
    "GearboxBubbleTest",
    "sysGearboxBubbleTest.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);
gearbox_scenario!(
    ONE_TO_ONE_BLUESIM,
    "Gearbox1to1Test",
    "sysGearbox1to1Test.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
gearbox_scenario!(
    ONE_TO_ONE_ICARUS,
    "Gearbox1to1Test",
    "sysGearbox1to1Test.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);
gearbox_scenario!(
    SAME_CLOCK_BLUESIM,
    "GearboxSameClockTest",
    "sysGearboxSameClockTest.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
gearbox_scenario!(
    SAME_CLOCK_ICARUS,
    "GearboxSameClockTest",
    "sysGearboxSameClockTest.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    FULL_SPEED_BLUESIM,
    FULL_SPEED_ICARUS,
    BUBBLE_BLUESIM,
    BUBBLE_ICARUS,
    ONE_TO_ONE_BLUESIM,
    ONE_TO_ONE_ICARUS,
    SAME_CLOCK_BLUESIM,
    SAME_CLOCK_ICARUS,
];
