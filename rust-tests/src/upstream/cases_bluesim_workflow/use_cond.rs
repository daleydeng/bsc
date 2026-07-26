//! Origin: `testsuite/bsc.scheduler/use_cond/use_cond.exp`.

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

const FIXTURE_DIR: &str = "testsuite/bsc.scheduler/use_cond";

macro_rules! bug_1741_workflow {
    ($source:literal, $top:literal) => {
        BluesimWorkflowScenario {
            name: concat!("bsc.scheduler/use_cond::", $top, "::link"),
            fixture_dir: FIXTURE_DIR,
            fixtures: &[$source],
            generations: &[BluesimGeneration {
                source: $source,
                module: None,
                options: &[],
            }],
            link: BluesimLink {
                objects: &[],
                top: $top,
                options: &[],
            },
            link_assertions: &[],
            runs: &[],
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            requirement: Requirement::BluesimEnabled,
        }
    };
}

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[
    bug_1741_workflow!("Bug1741.bsv", "mkBug1741"),
    bug_1741_workflow!("Bug1741_And.bsv", "mkBug1741_And"),
    bug_1741_workflow!("Bug1741_Not.bsv", "mkBug1741_Not"),
];
