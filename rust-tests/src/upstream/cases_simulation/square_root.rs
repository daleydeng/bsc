//! Origin: `testsuite/bsc.lib/SquareRoot/squareroot.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/SquareRoot";

macro_rules! backend_scenario {
    (
        $constant:ident,
        $module:literal,
        $expected:expr,
        $backend:ident,
        $backend_name:literal,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.lib/SquareRoot::",
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
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.lib/SquareRoot::", $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: if matches!(SimulationBackend::$backend, SimulationBackend::Bluesim) {
                    OutputNormalization::MaskedLines { prefix: "sqrt (" }
                } else {
                    OutputNormalization::Preserve
                },
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

macro_rules! square_root_scenarios {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        backend_scenario!(
            $bluesim,
            $module,
            concat!("sys", $module, ".c.out.expected"),
            Bluesim,
            "bluesim",
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        );
        backend_scenario!(
            $icarus,
            $module,
            concat!("sys", $module, ".v.out.expected"),
            Icarus,
            "icarus",
            Some(VcdContract::parse()),
            VerilogEnabled
        );
    };
}

square_root_scenarios!(
    SQUARE_ROOTER_BLUESIM,
    SQUARE_ROOTER_ICARUS,
    "Test_mkSquareRooter"
);
square_root_scenarios!(
    FIXED_POINT_SQUARE_ROOTER_BLUESIM,
    FIXED_POINT_SQUARE_ROOTER_ICARUS,
    "Test_mkFixedPointSquareRooter"
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    SQUARE_ROOTER_BLUESIM,
    SQUARE_ROOTER_ICARUS,
    FIXED_POINT_SQUARE_ROOTER_BLUESIM,
    FIXED_POINT_SQUARE_ROOTER_ICARUS,
];
