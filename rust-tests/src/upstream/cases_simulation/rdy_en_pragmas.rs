//! Origin: `testsuite/bsc.codegen/rdy_en_pragmas/rdy_en_pragmas.exp`

use super::SimulationScenario;
use crate::upstream::{
    ArtifactAssertion, DiagnosticKind, ExpectedOutcome, GenerationStrategy, OutputNormalization,
    Requirement, ResourceClass, SimulationBackend, SimulationContract, SimulationLinkInput,
    SimulationTimeouts, TextAssertion, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.codegen/rdy_en_pragmas";
const UNSAFE_ALWAYS_READY: &[&str] = &["-unsafe-always-ready"];

macro_rules! pragma_scenario {
    ($source:literal, $generated_module:literal, $icarus_assertions:expr) => {
        SimulationScenario {
            name: concat!("bsc.codegen/rdy_en_pragmas::", $source),
            fixture_dir: FIXTURE_DIR,
            source: concat!($source, ".bsv"),
            fixtures: &[
                concat!($source, ".bsv"),
                concat!("sys", $source, ".out.expected"),
            ],
            top: concat!("sys", $source),
            link_inputs: &[SimulationLinkInput::GeneratedModule($generated_module)],
            compile_options: UNSAFE_ALWAYS_READY,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.codegen/rdy_en_pragmas::", $source, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $source, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.codegen/rdy_en_pragmas::", $source, "::icarus"),
                    assertions: $icarus_assertions,
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $source, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        }
    };
}

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    pragma_scenario!(
        "AlwaysEnabledNotOK",
        "mkTest",
        &[ArtifactAssertion::Text {
            path: "AlwaysEnabledNotOK.bsv.bsc-out",
            assertion: TextAssertion::DiagnosticCount {
                kind: DiagnosticKind::Warning,
                tag: "G0006",
                count: 1,
            },
        }]
    ),
    pragma_scenario!("AlwaysReadyNotOK", "mkTestReady", &[]),
    pragma_scenario!("AlwaysEnabledGated1", "mkTestGated1", &[]),
    pragma_scenario!("AlwaysEnabledGated2", "mkTestGated2", &[]),
];
