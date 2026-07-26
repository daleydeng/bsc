//! Origin: `testsuite/bsc.syntax/bsv05/strings/parse_strings.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.syntax/bsv05/strings::sysOctalChars::link",
    fixture_dir: "testsuite/bsc.syntax/bsv05/strings",
    fixtures: &["OctalChars.bsv"],
    generations: &[BluesimGeneration {
        source: "OctalChars.bsv",
        module: None,
        options: &[],
    }],
    link: BluesimLink {
        objects: &["sysOctalChars"],
        top: "sysOctalChars",
        options: &[],
    },
    link_assertions: &[],
    runs: &[],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
