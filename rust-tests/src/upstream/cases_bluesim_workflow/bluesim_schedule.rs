//! Origin: `testsuite/bsc.bluesim/schedule/schedule.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactAssertion, BluesimGeneration, BluesimLink, Requirement, ResourceClass,
    SimulationTimeouts, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.bluesim/schedule";

macro_rules! build_only_workflow {
    ($name:literal, $source:literal, $objects:expr, $options:expr, $assertions:expr) => {
        BluesimWorkflowScenario {
            name: concat!("bsc.bluesim/schedule::", $name, "::link"),
            fixture_dir: FIXTURE_DIR,
            fixtures: &[$source],
            generations: &[BluesimGeneration {
                source: $source,
                module: None,
                options: &[],
            }],
            link: BluesimLink {
                objects: $objects,
                top: $name,
                options: $options,
            },
            link_assertions: $assertions,
            runs: &[],
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            requirement: Requirement::BluesimEnabled,
        }
    };
}

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[
    build_only_workflow!(
        "sysMERulesInSubmod",
        "MERulesInSubmod.bsv",
        &["sysMERulesInSubmod", "mkMERulesInSubmod_Sub"],
        &["-keep-fires"],
        &[ArtifactAssertion::Text {
            path: "model_sysMERulesInSubmod.cxx",
            assertion: TextAssertion::Regex {
                pattern: r"INST_top\.INST_x\.DEF_CAN_FIRE_RL_r2 = (.*) && !\(INST_top\.INST_x\.DEF_CAN_FIRE_RL_r1\)",
            },
        }]
    ),
    build_only_workflow!(
        "sysMERuleAndMethodInSubmod",
        "MERuleAndMethodInSubmod.bsv",
        &[
            "sysMERuleAndMethodInSubmod",
            "mkMERuleAndMethodInSubmod_Sub1",
            "mkMERuleAndMethodInSubmod_Sub2",
        ],
        &["-keep-fires"],
        &[ArtifactAssertion::Text {
            path: "model_sysMERuleAndMethodInSubmod.cxx",
            assertion: TextAssertion::Regex {
                pattern: r"INST_top\.INST_x\.INST_s\.DEF_CAN_FIRE_RL_r2 = (.*) && !\(INST_top\.INST_x\.DEF_CAN_FIRE_RL_r1",
            },
        }]
    ),
    build_only_workflow!(
        "sysMutuallyExclusiveAssump_CombSched",
        "MutuallyExclusiveAssump_CombSched.bsv",
        &[
            "sysMutuallyExclusiveAssump_CombSched",
            "mkMutuallyExclusiveAssump_CombSched_Sub",
        ],
        &[],
        &[]
    ),
    build_only_workflow!(
        "sysMEValueMethod",
        "MEValueMethod.bsv",
        &["sysMEValueMethod"],
        &[],
        &[ArtifactAssertion::Text {
            path: "model_sysMEValueMethod.cxx",
            assertion: TextAssertion::LineCount {
                text: "DEF_CAN_FIRE_get",
                count: 0,
            },
        }]
    ),
];
