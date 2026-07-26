//! Origins:
//! - `testsuite/bsc.interra/Library_latency/BGetPut/BGetPut.exp`
//! - `testsuite/bsc.interra/Library_latency/CGetPut/CGetPut.exp`
//! - `testsuite/bsc.interra/Library_latency/ClientServer/ClientServer.exp`
//! - `testsuite/bsc.interra/Library_latency/GetPut/GetPut.exp`
//! - `testsuite/bsc.interra/Library_latency/RAM/RAM.exp`
//! - `testsuite/bsc.interra/Library_latency/SRAM/SRAM.exp`
//! - `testsuite/bsc.interra/Library_latency/SyncRAM/SyncRAM.exp`

use super::BluesimWorkflowScenario;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, BluesimGeneration, BluesimLink, BluesimWorkflowRun,
    Requirement, ResourceClass, SimulationTimeouts,
};

macro_rules! latency_workflow {
    ($prefix:literal, $fixture_dir:literal, $source:literal, $top:literal, $fixtures:expr, $options:expr, $run:literal) => {
        BluesimWorkflowScenario {
            name: concat!($prefix, "::", $top, "::workflow"),
            fixture_dir: $fixture_dir,
            fixtures: $fixtures,
            generations: &[BluesimGeneration {
                source: $source,
                module: Some($top),
                options: &[],
            }],
            link: BluesimLink {
                objects: &[$top],
                top: $top,
                options: &[],
            },
            link_assertions: &[],
            runs: &[BluesimWorkflowRun {
                name: concat!($prefix, "::", $top, "::", $run),
                options: $options,
                stdout: concat!($top, ".out"),
                transfers: &[],
                assertions: &[ArtifactAssertion::Matches {
                    actual: concat!($top, ".out"),
                    expected: concat!($top, ".out.expected"),
                    normalization: ArtifactNormalization::GoldenOutput,
                }],
            }],
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            requirement: Requirement::BluesimEnabled,
        }
    };
}

const M10010: &[&str] = &["-m", "10010"];
const DEFAULT: &[&str] = &[];

pub(super) const SCENARIOS: &[BluesimWorkflowScenario] = &[
    latency_workflow!(
        "bsc.interra/Library_latency/BGetPut",
        "testsuite/bsc.interra/Library_latency/BGetPut",
        "MkBClientServer.bsv",
        "mkTestbench_MkBClientServer",
        &[
            "MkBClientServer.bsv",
            "mkTestbench_MkBClientServer.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/BGetPut",
        "testsuite/bsc.interra/Library_latency/BGetPut",
        "MkBGetPut.bsv",
        "mkTestbench_MkBGetPut",
        &["MkBGetPut.bsv", "mkTestbench_MkBGetPut.out.expected"],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/CGetPut",
        "testsuite/bsc.interra/Library_latency/CGetPut",
        "MkCClientServer.bsv",
        "mkTestbench_MkCClientServer",
        &[
            "MkCClientServer.bsv",
            "mkTestbench_MkCClientServer.out.expected",
        ],
        DEFAULT,
        "default"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/CGetPut",
        "testsuite/bsc.interra/Library_latency/CGetPut",
        "MkCGetCPut.bsv",
        "mkTestbench_MkCGetCPut",
        &["MkCGetCPut.bsv", "mkTestbench_MkCGetCPut.out.expected"],
        DEFAULT,
        "default"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/CGetPut",
        "testsuite/bsc.interra/Library_latency/CGetPut",
        "MkCGetPut.bsv",
        "mkTestbench_MkCGetPut",
        &["MkCGetPut.bsv", "mkTestbench_MkCGetPut.out.expected"],
        DEFAULT,
        "default"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/CGetPut",
        "testsuite/bsc.interra/Library_latency/CGetPut",
        "MkCGetCPut_extra_buffer.bsv",
        "mkTestbench_Mk_extrabuffer",
        &[
            "MkCGetCPut_extra_buffer.bsv",
            "mkTestbench_Mk_extrabuffer.out.expected",
        ],
        DEFAULT,
        "default"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "JoinServers1.bsv",
        "mkTestbench_JoinServers",
        &["JoinServers1.bsv", "mkTestbench_JoinServers.out.expected",],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "JoinServers_twoserver.bsv",
        "mkTestbench_TwoJoinServers",
        &[
            "JoinServers_twoserver.bsv",
            "mkTestbench_TwoJoinServers.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "MkRequestBuffer.bsv",
        "mkTestbench_MkRequestBuffer",
        &[
            "MkRequestBuffer.bsv",
            "mkTestbench_MkRequestBuffer.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "MkRequestResponseBuffer1.bsv",
        "mkTestbench_MkRequestResponseBuffer1",
        &[
            "MkRequestResponseBuffer1.bsv",
            "mkTestbench_MkRequestResponseBuffer1.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "MkRequestResponseBuffer_1.bsv",
        "mkTestbench_MkRequestResponseBuffer_1",
        &[
            "MkRequestResponseBuffer_1.bsv",
            "mkTestbench_MkRequestResponseBuffer_1.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "MkResponseBuffer.bsv",
        "mkTestbench_MkResponseBuffer",
        &[
            "MkResponseBuffer.bsv",
            "mkTestbench_MkResponseBuffer.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/ClientServer",
        "testsuite/bsc.interra/Library_latency/ClientServer",
        "MkSizedRequestResponseBuffer.bsv",
        "mkTestbench_MkSizedRequestResponseBuffer",
        &[
            "MkSizedRequestResponseBuffer.bsv",
            "mkTestbench_MkSizedRequestResponseBuffer.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/GetPut",
        "testsuite/bsc.interra/Library_latency/GetPut",
        "MkGPFIFO.bsv",
        "mkTestbench_MkGPFIFO",
        &["MkGPFIFO.bsv", "mkTestbench_MkGPFIFO.out.expected"],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/GetPut",
        "testsuite/bsc.interra/Library_latency/GetPut",
        "MkGPSizedFIFO.bsv",
        "mkTestbench_MkGPSizedFIFO",
        &[
            "MkGPSizedFIFO.bsv",
            "mkTestbench_MkGPSizedFIFO.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/GetPut",
        "testsuite/bsc.interra/Library_latency/GetPut",
        "MkGetPut.bsv",
        "mkTestbench_MkGetPut",
        &["MkGetPut.bsv", "mkTestbench_MkGetPut.out.expected"],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/GetPut",
        "testsuite/bsc.interra/Library_latency/GetPut",
        "MkGPFIFO_alt_rw.bsv",
        "mkTestbench_MkGPFIFO_alt_rw",
        &[
            "MkGPFIFO_alt_rw.bsv",
            "mkTestbench_MkGPFIFO_alt_rw.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/GetPut",
        "testsuite/bsc.interra/Library_latency/GetPut",
        "MkGPFIFO_non_alt_rw.bsv",
        "mkTestbench_MkGPFIFO_non_alt_rw",
        &[
            "MkGPFIFO_non_alt_rw.bsv",
            "mkTestbench_MkGPFIFO_non_alt_rw.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/RAM",
        "testsuite/bsc.interra/Library_latency/RAM",
        "TestRAM.bsv",
        "mkTestbench_Ram",
        &["TestRAM.bsv", "mkTestbench_Ram.out.expected"],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/RAM",
        "testsuite/bsc.interra/Library_latency/RAM",
        "TagRam.bsv",
        "mkTestbench_TagRam",
        &["TagRam.bsv", "mkTestbench_TagRam.out.expected"],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/SRAM",
        "testsuite/bsc.interra/Library_latency/SRAM",
        "MkWrapSRAM.bsv",
        "mkTestbench_MkWrapSRAM",
        &[
            "MkWrapSRAM.bsv",
            "Precedence.bs",
            "mkTestbench_MkWrapSRAM.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/SRAM",
        "testsuite/bsc.interra/Library_latency/SRAM",
        "MkWrapSTRAM.bsv",
        "mkTestbench_MkWrapSTRAM",
        &[
            "MkWrapSTRAM.bsv",
            "Precedence.bs",
            "mkTestbench_MkWrapSTRAM.out.expected",
        ],
        M10010,
        "m10010"
    ),
    latency_workflow!(
        "bsc.interra/Library_latency/SyncRAM",
        "testsuite/bsc.interra/Library_latency/SyncRAM",
        "TestSPSRAM.bsv",
        "mkTestbench_SPSRam",
        &[
            "TestSPSRAM.bsv",
            "Precedence.bs",
            "mkTestbench_SPSRam.out.expected",
        ],
        M10010,
        "m10010"
    ),
];
