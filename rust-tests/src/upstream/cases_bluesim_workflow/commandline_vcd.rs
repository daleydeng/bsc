//! Origins:
//! - `testsuite/bsc.interra/bluesim/commandline_options/array/array.exp`
//! - `testsuite/bsc.interra/bluesim/commandline_options/handshake_protocol/handshake_protocol_cl.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactTransfer, ArtifactTransferOperation, BluesimGeneration, BluesimLink,
    BluesimWorkflowRun, Requirement, ResourceClass, SimulationTimeouts,
};

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[
    BluesimWorkflowScenario {
        name: "bsc.interra/bluesim/commandline_options/array::mkTestbench::workflow",
        fixture_dir: "testsuite/bsc.interra/bluesim/commandline_options/array",
        fixtures: &["Testbench.bsv"],
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
        runs: &[
            BluesimWorkflowRun {
                name: "bsc.interra/bluesim/commandline_options/array::vcd",
                options: &["-V", "dump.vcd"],
                stdout: "mkTestbench.out",
                transfers: &[ArtifactTransfer {
                    operation: ArtifactTransferOperation::Copy,
                    source: "dump.vcd",
                    destination: "dump_V.out",
                }],
                assertions: &[],
            },
            BluesimWorkflowRun {
                name: "bsc.interra/bluesim/commandline_options/array::vcd-m5",
                options: &["-V", "dump.vcd", "-m", "5"],
                stdout: "mkTestbench.out",
                transfers: &[ArtifactTransfer {
                    operation: ArtifactTransferOperation::Copy,
                    source: "dump.vcd",
                    destination: "dump_V_m.out",
                }],
                assertions: &[],
            },
        ],
        timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
        resource: ResourceClass::Normal,
        requirement: Requirement::BluesimEnabled,
    },
    BluesimWorkflowScenario {
        name: "bsc.interra/bluesim/commandline_options/handshake_protocol::mkTestbench::workflow",
        fixture_dir: "testsuite/bsc.interra/bluesim/commandline_options/handshake_protocol",
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
        runs: &[BluesimWorkflowRun {
            name: "bsc.interra/bluesim/commandline_options/handshake_protocol::vcd-default",
            options: &["-V"],
            stdout: "mkTestbench.out",
            transfers: &[ArtifactTransfer {
                operation: ArtifactTransferOperation::Copy,
                source: "dump.vcd",
                destination: "dump.out",
            }],
            assertions: &[],
        }],
        timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
        resource: ResourceClass::Normal,
        requirement: Requirement::BluesimEnabled,
    },
];
