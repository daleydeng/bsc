//! Origin: `testsuite/bsc.lib/sram/sram.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, BluesimGeneration, BluesimLink, BluesimWorkflowRun,
    Requirement, ResourceClass, SimulationTimeouts,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.lib/sram::throughput::workflow",
    fixture_dir: "testsuite/bsc.lib/sram",
    fixtures: &["Throughput.bs", "throughput.out.expected"],
    generations: &[BluesimGeneration {
        source: "Throughput.bs",
        module: Some("throughput"),
        options: &[],
    }],
    link: BluesimLink {
        objects: &["throughput"],
        top: "throughput",
        options: &[],
    },
    link_assertions: &[],
    runs: &[BluesimWorkflowRun {
        name: "bsc.lib/sram::throughput::default",
        options: &[],
        stdout: "throughput.out",
        transfers: &[],
        assertions: &[ArtifactAssertion::Matches {
            actual: "throughput.out",
            expected: "throughput.out.expected",
            normalization: ArtifactNormalization::GoldenOutput,
        }],
    }],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
