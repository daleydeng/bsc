//! Origin: `testsuite/bsc.codegen/rdy_en_pragmas/rdy_en_pragmas.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactAssertion, BluesimGeneration, BluesimLink, DiagnosticKind, Requirement, ResourceClass,
    SimulationTimeouts, TextAssertion,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[BluesimWorkflowScenario {
    name: "bsc.codegen/rdy_en_pragmas::sysTestEnableFail::link",
    fixture_dir: "testsuite/bsc.codegen/rdy_en_pragmas",
    fixtures: &["TestEnableFail.bsv"],
    generations: &[BluesimGeneration {
        source: "TestEnableFail.bsv",
        module: None,
        options: &[],
    }],
    link: BluesimLink {
        objects: &["sysTestEnableFail.ba", "mkSub.ba"],
        top: "sysTestEnableFail",
        options: &[],
    },
    link_assertions: &[ArtifactAssertion::Text {
        path: "TestEnableFail.bsv.bsc-ccomp-out",
        assertion: TextAssertion::DiagnosticCount {
            kind: DiagnosticKind::Warning,
            tag: "G0015",
            count: 2,
        },
    }],
    runs: &[],
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    requirement: Requirement::BluesimEnabled,
}];
