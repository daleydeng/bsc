//! Origin: `testsuite/bsc.bluesim/debugging/debugging.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

macro_rules! build_only_workflow {
    ($name:literal, $source:literal, $module:literal, $objects:expr, $fixtures:expr) => {
        BluesimWorkflowScenario {
            name: concat!("bsc.bluesim/debugging::", $name, "::link"),
            fixture_dir: "testsuite/bsc.bluesim/debugging",
            fixtures: $fixtures,
            generations: &[BluesimGeneration {
                source: $source,
                module: Some($module),
                options: &[],
            }],
            link: BluesimLink {
                objects: $objects,
                top: $module,
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
    build_only_workflow!(
        "mkTbGCD",
        "TbGCD.bsv",
        "mkTbGCD",
        &["mkTbGCD", "mkGCD"],
        &["TbGCD.bsv", "GCD.bsv"]
    ),
    build_only_workflow!("mkRF", "RF.bsv", "mkRF", &["mkRF"], &["RF.bsv"]),
    build_only_workflow!(
        "mkMCDTest",
        "MCDTest.bsv",
        "mkMCDTest",
        &["mkMCDTest"],
        &["MCDTest.bsv"]
    ),
];
