//! Origin: `testsuite/bsc.typechecker/generics/generics.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/generics";

macro_rules! generic_scenario {
    ($constant:ident, $module:literal, $fixtures:expr) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.typechecker/generics::", $module),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bs"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.typechecker/generics::", $module, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $module, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.typechecker/generics::", $module, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $module, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
    ($constant:ident, $module:literal) => {
        generic_scenario!(
            $constant,
            $module,
            &[
                concat!($module, ".bs"),
                concat!("sys", $module, ".out.expected"),
            ]
        );
    };
}

generic_scenario!(
    GENERIC_TESTS,
    "GenericTests",
    &[
        "GenericTests.bs",
        "GenericTestsBSV.bsv",
        "sysGenericTests.out.expected",
    ]
);
generic_scenario!(CUSTOM_BITS, "CustomBits");
generic_scenario!(C_PRINT_TYPE, "CPrintType");

pub(super) const SCENARIOS: &[SimulationScenario] = &[GENERIC_TESTS, CUSTOM_BITS, C_PRINT_TYPE];
