//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1439/b1439.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1796/b1796.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

macro_rules! b1439_workflow {
    ($source:literal) => {
        BluesimWorkflowScenario {
            name: concat!("bsc.bugs/bluespec_inc/b1439::", $source, "::link"),
            fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1439",
            fixtures: &[$source],
            generations: &[BluesimGeneration {
                source: $source,
                module: Some("mkBug1439"),
                options: &[],
            }],
            link: BluesimLink {
                objects: &["mkBug1439.ba"],
                top: "mkBug1439",
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
    b1439_workflow!("Bug1439.bs"),
    b1439_workflow!("Bug1439.bsv"),
    BluesimWorkflowScenario {
        name: "bsc.bugs/bluespec_inc/b1796::sysBug1796::link",
        fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1796",
        fixtures: &["Bug1796.bsv"],
        generations: &[BluesimGeneration {
            source: "Bug1796.bsv",
            module: None,
            options: &[],
        }],
        link: BluesimLink {
            objects: &[],
            top: "sysBug1796",
            options: &[],
        },
        link_assertions: &[],
        runs: &[],
        timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
        resource: ResourceClass::Normal,
        requirement: Requirement::BluesimEnabled,
    },
];
