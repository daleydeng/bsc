//! Origin: `testsuite/bsc.bsv_examples/vending/vending.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bsv_examples/vending";

macro_rules! vending_backend_scenario {
    (
        $constant:ident,
        $expected:literal,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.bsv_examples/vending::TestVending::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: FIXTURE_DIR,
            source: "TestVending.bsv",
            fixtures: &[
                "TestVending.bsv",
                "Vending.bsv",
                "VendingIfc.bsv",
                $expected,
            ],
            top: "sysTestVending",
            link_inputs: &[SimulationLinkInput::GeneratedModule("mkVending")],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.bsv_examples/vending::TestVending::", $backend_name),
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

vending_backend_scenario!(
    TEST_VENDING_ICARUS,
    "sysTestVending.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
vending_backend_scenario!(
    TEST_VENDING_BLUESIM,
    "sysTestVending.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[TEST_VENDING_ICARUS, TEST_VENDING_BLUESIM];
