//! Origin: `testsuite/bsc.bugs/bluespec_inc/b1243/b1243.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.bugs/bluespec_inc/b1243::link",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1243",
    fixtures: &["Bug1243.bsv"],
    generations: &[BluesimGeneration {
        source: "Bug1243.bsv",
        module: None,
        options: &[],
    }],
    link: BluesimLink {
        objects: &[],
        top: "sysBug1243",
        options: &[],
    },
    link_assertions: &[],
    runs: &[],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
