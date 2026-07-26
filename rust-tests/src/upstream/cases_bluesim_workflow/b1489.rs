//! Origin: `testsuite/bsc.bugs/bluespec_inc/b1489/b1489.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactAssertion, BluesimGeneration, BluesimLink, BluesimWorkflowRun, Requirement,
    ResourceClass, SimulationTimeouts, TextAssertion,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.bugs/bluespec_inc/b1489::workflow",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1489",
    fixtures: &["Bug1489.bsv"],
    generations: &[BluesimGeneration {
        source: "Bug1489.bsv",
        module: None,
        options: &[],
    }],
    link: BluesimLink {
        objects: &["sysBug1489.ba"],
        top: "sysBug1489",
        options: &[],
    },
    link_assertions: &[],
    runs: &[BluesimWorkflowRun {
        name: "bsc.bugs/bluespec_inc/b1489::sysBug1489",
        options: &[],
        stdout: "sysBug1489.out",
        transfers: &[],
        assertions: &[
            ArtifactAssertion::Text {
                path: "sysBug1489.out",
                assertion: TextAssertion::LineCount {
                    text: "file11.hex",
                    count: 1,
                },
            },
            ArtifactAssertion::Text {
                path: "sysBug1489.out",
                assertion: TextAssertion::LineCount {
                    text: "file100.hex",
                    count: 1,
                },
            },
        ],
    }],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
