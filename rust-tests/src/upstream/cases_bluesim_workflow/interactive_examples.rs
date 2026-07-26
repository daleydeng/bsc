//! Origins:
//! - `testsuite/bsc.interra/bluesim/interactive/handshake_protocol/handshake_protocol.exp`
//! - `testsuite/bsc.interra/bluesim/interactive/parity_checker/parity_checker.exp`
//! - `testsuite/bsc.interra/bluesim/interactive/traffic_light_controller_hierar/traffic_light_controller_hier.exp`
//! - `testsuite/bsc.interra/bluesim/interactive/traffic_light_controller_separate/traffic_light_controller.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    BluesimGeneration, BluesimLink, Requirement, ResourceClass, SimulationTimeouts,
};

macro_rules! separate_design_workflow {
    ($origin:literal, $fixture_dir:literal) => {
        BluesimWorkflowScenario {
            name: concat!($origin, "::mkTestbench::link"),
            fixture_dir: $fixture_dir,
            fixtures: &["Design.bsv", "Testbench.bsv"],
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
            runs: &[],
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            requirement: Requirement::BluesimEnabled,
        }
    };
}

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[
    separate_design_workflow!(
        "bsc.interra/bluesim/interactive/handshake_protocol",
        "testsuite/bsc.interra/bluesim/interactive/handshake_protocol"
    ),
    separate_design_workflow!(
        "bsc.interra/bluesim/interactive/parity_checker",
        "testsuite/bsc.interra/bluesim/interactive/parity_checker"
    ),
    BluesimWorkflowScenario {
        name: "bsc.interra/bluesim/interactive/traffic_light_controller_hierar::mkTestbench::link",
        fixture_dir: "testsuite/bsc.interra/bluesim/interactive/traffic_light_controller_hierar",
        fixtures: &["Design.bsv", "Testbench.bsv"],
        generations: &[BluesimGeneration {
            source: "Testbench.bsv",
            module: Some("mkTestbench"),
            options: &[],
        }],
        link: BluesimLink {
            objects: &["mkTestbench"],
            top: "mkTestbench",
            options: &[],
        },
        link_assertions: &[],
        runs: &[],
        timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
        resource: ResourceClass::Normal,
        requirement: Requirement::BluesimEnabled,
    },
    separate_design_workflow!(
        "bsc.interra/bluesim/interactive/traffic_light_controller_separate",
        "testsuite/bsc.interra/bluesim/interactive/traffic_light_controller_separate"
    ),
];
