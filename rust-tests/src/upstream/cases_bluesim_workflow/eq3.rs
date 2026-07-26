//! Origin: `testsuite/bsc.misc/eq3/eq3.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.misc/eq3::mkEQ3::link",
    fixture_dir: "testsuite/bsc.misc/eq3",
    fixtures: &["EQ3.bs"],
    generations: &[BluesimGeneration {
        source: "EQ3.bs",
        module: None,
        options: &[],
    }],
    link: BluesimLink {
        objects: &[],
        top: "mkEQ3",
        options: &[],
    },
    link_assertions: &[],
    runs: &[],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
