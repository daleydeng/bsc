//! Origin: `testsuite/bsc.scheduler/conflict_free/conflict_free.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.scheduler/conflict_free";

macro_rules! shared_scenario {
    ($constant:ident, $module:literal, $expected:literal, $compile_options:expr) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.scheduler/conflict_free::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.scheduler/conflict_free::", $module, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.scheduler/conflict_free::", $module, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

macro_rules! backend_scenario {
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
                "bsc.scheduler/conflict_free::",
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
                name: concat!(
                    "bsc.scheduler/conflict_free::",
                    $module,
                    "::",
                    $backend_name
                ),
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

shared_scenario!(OK, "ConflictFreeOK", "sysConflictFreeOK.out.expected", &[]);
shared_scenario!(
    OK_2,
    "ConflictFreeOK2",
    "sysConflictFreeOK2.out.expected",
    &[]
);
shared_scenario!(
    OK_3,
    "ConflictFreeOK3",
    "sysConflictFreeOK3.out.expected",
    &["-aggressive-conditions"]
);
backend_scenario!(
    NOT_OK_BLUESIM,
    "ConflictFreeNotOK",
    "sysConflictFreeNotOK.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
backend_scenario!(
    NOT_OK_ICARUS,
    "ConflictFreeNotOK",
    "sysConflictFreeNotOK.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);
shared_scenario!(
    RESOURCE,
    "ConflictFreeResource",
    "sysConflictFreeResource.out.expected",
    &[]
);
backend_scenario!(
    EXEC_ORDER_1_BLUESIM,
    "CFExecOrder1",
    "sysCFExecOrder1.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
backend_scenario!(
    EXEC_ORDER_1_ICARUS,
    "CFExecOrder1",
    "sysCFExecOrder1.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);
shared_scenario!(
    EXEC_ORDER_2,
    "CFExecOrder2",
    "sysCFExecOrder2.out.expected",
    &[]
);
shared_scenario!(
    EXEC_ORDER_3,
    "CFExecOrder3",
    "sysCFExecOrder3.out.expected",
    &[]
);
backend_scenario!(
    SWITCH_BLUESIM,
    "CFSwitch",
    "sysCFSwitch.c.out.expected",
    Bluesim,
    "bluesim",
    Some(VcdContract::output_matches_normal()),
    Requirement::BluesimEnabled
);
backend_scenario!(
    SWITCH_ICARUS,
    "CFSwitch",
    "sysCFSwitch.v.out.expected",
    Icarus,
    "icarus",
    Some(VcdContract::parse()),
    Requirement::VerilogEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    OK,
    OK_2,
    OK_3,
    NOT_OK_BLUESIM,
    NOT_OK_ICARUS,
    RESOURCE,
    EXEC_ORDER_1_BLUESIM,
    EXEC_ORDER_1_ICARUS,
    EXEC_ORDER_2,
    EXEC_ORDER_3,
    SWITCH_BLUESIM,
    SWITCH_ICARUS,
];
