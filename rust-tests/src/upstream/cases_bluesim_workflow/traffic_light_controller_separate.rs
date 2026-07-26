//! Origin: `testsuite/bsc.interra/bluesim/commandline_options/traffic_light_controller_separate/traffic_light_controller_separate.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, ArtifactTransfer, ArtifactTransferOperation,
    BluesimGeneration, BluesimLink, BluesimWorkflowRun, Requirement, ResourceClass,
    SimulationTimeouts,
};

const FIXTURE_DIR: &str =
    "testsuite/bsc.interra/bluesim/commandline_options/traffic_light_controller_separate";

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.interra/bluesim/commandline_options/traffic_light_controller_separate::workflow",
    fixture_dir: FIXTURE_DIR,
    fixtures: &[
        "Design.bsv",
        "Testbench.bsv",
        "command_m_1.out.expected",
        "command_m_2.out.expected",
    ],
    generations: &[
        BluesimGeneration {
            source: "Design.bsv",
            module: Some("mkDesign"),
            options: &[],
        },
        BluesimGeneration {
            source: "Testbench.bsv",
            module: Some("mkTestbench"),
            options: &[],
        },
    ],
    link: BluesimLink {
        objects: &["mkTestbench", "mkDesign"],
        top: "mkTestbench",
        options: &[],
    },
    link_assertions: &[],
    runs: &[
        BluesimWorkflowRun {
            name: "bsc.interra/bluesim/commandline_options/traffic_light_controller_separate::m17",
            options: &["-m", "17"],
            stdout: "mkTestbench.out",
            transfers: &[ArtifactTransfer {
                operation: ArtifactTransferOperation::Copy,
                source: "mkTestbench.out",
                destination: "command_m_1.out",
            }],
            assertions: &[ArtifactAssertion::Matches {
                actual: "command_m_1.out",
                expected: "command_m_1.out.expected",
                normalization: ArtifactNormalization::GoldenOutput,
            }],
        },
        BluesimWorkflowRun {
            name: "bsc.interra/bluesim/commandline_options/traffic_light_controller_separate::m5",
            options: &["-m", "5"],
            stdout: "mkTestbench.out",
            transfers: &[ArtifactTransfer {
                operation: ArtifactTransferOperation::Copy,
                source: "mkTestbench.out",
                destination: "command_m_2.out",
            }],
            assertions: &[ArtifactAssertion::Matches {
                actual: "command_m_2.out",
                expected: "command_m_2.out.expected",
                normalization: ArtifactNormalization::GoldenOutput,
            }],
        },
    ],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
