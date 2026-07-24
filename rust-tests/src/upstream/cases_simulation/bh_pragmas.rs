//! Origin: `testsuite/bsc.syntax/bh/bh_pragmas/bh_pragmas.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, ExpectedOutcome, GenerationStrategy,
    OutputNormalization, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bh/bh_pragmas";

const MEMORY_FIXTURES: &[&str] = &[
    "mkPragmas_0_file.txt",
    "mkPragmas_1_file.txt",
    "mkPragmas_2_file.txt",
    "mkPragmas_3_file.txt",
];

macro_rules! pragma_scenario {
    (
        $constant:ident,
        $module:literal,
        $source:literal,
        $top:literal,
        $expected:literal,
        $verilog:literal,
        $verilog_expected:literal,
        $compile_options:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.syntax/bh/bh_pragmas::", $module),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[
                $source,
                $expected,
                $verilog_expected,
                MEMORY_FIXTURES[0],
                MEMORY_FIXTURES[1],
                MEMORY_FIXTURES[2],
                MEMORY_FIXTURES[3],
            ],
            top: $top,
            link_inputs: &[],
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.syntax/bh/bh_pragmas::", $module, "::bluesim"),
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
                    name: concat!("bsc.syntax/bh/bh_pragmas::", $module, "::icarus"),
                    assertions: &[ArtifactAssertion::Matches {
                        actual: $verilog,
                        expected: $verilog_expected,
                        normalization: ArtifactNormalization::Verilog,
                    }],
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

pragma_scenario!(
    PRAGMAS,
    "Pragmas",
    "Pragmas.bs",
    "sysPragmas",
    "sysPragmas.out.expected",
    "mkPragmas.v",
    "mkPragmas.v.expected",
    &[]
);
pragma_scenario!(
    PROPERTIES,
    "Properties",
    "Properties.bs",
    "sysProperties",
    "sysProperties.out.expected",
    "mkProperties.v",
    "mkProperties.v.expected",
    &["-g", "mkProperties"]
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[PRAGMAS, PROPERTIES];
