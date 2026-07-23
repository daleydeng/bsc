//! Origins:
//! - `testsuite/bsc.interra/StmtFSM/CycleTest/cycletest.exp`
//! - `testsuite/bsc.interra/StmtFSM/ServerInServer/serverinserver.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

macro_rules! stmt_fsm_backend_scenario {
    (
        $constant:ident,
        $prefix:literal,
        $fixture_dir:literal,
        $module:literal,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($prefix, "::", $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[
                concat!($module, ".bsv"),
                concat!("sys", $module, ".out.expected"),
            ],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::", $backend_name),
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

stmt_fsm_backend_scenario!(
    CYCLE_TEST_BLUESIM,
    "bsc.interra/StmtFSM/CycleTest",
    "testsuite/bsc.interra/StmtFSM/CycleTest",
    "CycleTest",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
stmt_fsm_backend_scenario!(
    CYCLE_TEST_ICARUS,
    "bsc.interra/StmtFSM/CycleTest",
    "testsuite/bsc.interra/StmtFSM/CycleTest",
    "CycleTest",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
stmt_fsm_backend_scenario!(
    SERVER_IN_SERVER_BLUESIM,
    "bsc.interra/StmtFSM/ServerInServer",
    "testsuite/bsc.interra/StmtFSM/ServerInServer",
    "ServerInServer",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
stmt_fsm_backend_scenario!(
    SERVER_IN_SERVER_ICARUS,
    "bsc.interra/StmtFSM/ServerInServer",
    "testsuite/bsc.interra/StmtFSM/ServerInServer",
    "ServerInServer",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    CYCLE_TEST_BLUESIM,
    CYCLE_TEST_ICARUS,
    SERVER_IN_SERVER_BLUESIM,
    SERVER_IN_SERVER_ICARUS,
];
