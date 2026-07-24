//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1118/b1118.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1121/b1121.exp`
//! - `testsuite/bsc.interra/libraries/Push/Push.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/BypassFIFO/BypassFIFO.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/LoopyFIFO/LoopyFIFO.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/RWire_mult/RWire_mult.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/RegFile/RegFile.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/byte_en/byte_en.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/demux/demux.exp`
//! - `testsuite/bsc.interra/relax_method_urgency/prod_con/prod_con.exp`
//! - `testsuite/bsc.lib/Stmt/FacTest/FacTest.exp`
//! - `testsuite/bsc.lib/Stmt/RepeatTest/RepeatTest.exp`
//! - `testsuite/bsc.lib/Stmt/Server/Server.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

macro_rules! dual_backend_scenario {
    (
        $constant:ident,
        $prefix:literal,
        $fixture_dir:expr,
        $case:literal,
        $source:expr,
        $top:literal,
        $fixtures:expr,
        $link_inputs:expr,
        $bluesim_expected:expr,
        $icarus_expected:expr,
        $output:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($prefix, "::", $case),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $case, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: $bluesim_expected,
                    },
                    output: $output,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $case, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass {
                        output: $icarus_expected,
                    },
                    output: $output,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

macro_rules! backend_scenario {
    (
        $constant:ident,
        $prefix:literal,
        $fixture_dir:expr,
        $case:literal,
        $source:expr,
        $top:literal,
        $fixtures:expr,
        $link_inputs:expr,
        $expected:expr,
        $output:expr,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($prefix, "::", $case, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $case, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: $output,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

dual_backend_scenario!(
    B1118_CONTINUE_IN_FOR_LOOP,
    "bsc.bugs/bluespec_inc/b1118",
    "testsuite/bsc.bugs/bluespec_inc/b1118",
    "continueInForLoop",
    "continueInForLoop.bsv",
    "forTest",
    &["continueInForLoop.bsv", "forTest.out.expected"],
    &[],
    "forTest.out.expected",
    "forTest.out.expected",
    OutputNormalization::Preserve
);

dual_backend_scenario!(
    B1121_SIZED_FIFO,
    "bsc.bugs/bluespec_inc/b1121",
    "testsuite/bsc.bugs/bluespec_inc/b1121",
    "Bug1121",
    "Bug1121.bsv",
    "testSizedFIFO",
    &["Bug1121.bsv", "testSizedFIFO.out.expected"],
    &[],
    "testSizedFIFO.out.expected",
    "testSizedFIFO.out.expected",
    OutputNormalization::Preserve
);

const PUSH_DIR: &str = "testsuite/bsc.interra/libraries/Push";

macro_rules! push_scenario {
    ($constant:ident, $module:literal, $top:literal, $expected:literal) => {
        dual_backend_scenario!(
            $constant,
            "bsc.interra/libraries/Push",
            PUSH_DIR,
            $module,
            concat!($module, ".bsv"),
            $top,
            &[concat!($module, ".bsv"), $expected],
            &[],
            $expected,
            $expected,
            OutputNormalization::Preserve
        );
    };
}

push_scenario!(
    PUSH_APPLY,
    "Apply",
    "mkTestbench_Apply",
    "mkTestbench_Apply.out.expected"
);
push_scenario!(
    PUSH_TEE,
    "Tee",
    "mkTestbench_Tee",
    "mkTestbench_Tee.out.expected"
);
push_scenario!(
    PUSH_PASS,
    "Pass",
    "mkTestbench_Pass",
    "mkTestbench_Pass.out.expected"
);
push_scenario!(
    PUSH_PASSED,
    "Passed",
    "mkTestbench_Passed",
    "mkTestbench_Passed.out.expected"
);
push_scenario!(
    PUSH_BUFFER,
    "Buffer",
    "mkTestbench_Buffer",
    "mkTestbench_Buffer.out.expected"
);
push_scenario!(
    PUSH_QBUFFER,
    "Qbuffer",
    "mkTestbench_Qbuffer",
    "mkTestbench_Qbuffer.out.expected"
);
push_scenario!(
    PUSH_BUFFERED,
    "Buffered",
    "mkTestbench_Buffered",
    "mkTestbench_Buffered.out.expected"
);
push_scenario!(
    PUSH_QBUFFERED,
    "Qbuffered",
    "mkTestbench_Qbuffered",
    "mkTestbench_Qbuffered.out.expected"
);
push_scenario!(
    PUSH_SINK,
    "Sink",
    "mkTestbench_Sink",
    "mkTestbench_Sink.out.expected"
);
push_scenario!(
    PUSH_SPEW,
    "Spew",
    "mkTestbench_Spew",
    "mkTestbench_Spew.out.expected"
);
push_scenario!(
    PUSH_PIPE,
    "Pipe",
    "mkTestbench_Pipe",
    "mkTestbench_Pipe.out.expected"
);
push_scenario!(
    PUSH_REG_TO_PUSH,
    "RegToPush",
    "mkTestbench_RegToPush",
    "mkTestbench_RegToPush.out.expected"
);
push_scenario!(
    PUSH_FIFO_TO_PUSH,
    "FifoToPush",
    "mkTestbench_FifoToPush",
    "mkTestbench_FifoToPush.out.expected"
);

backend_scenario!(
    BYPASS_FIFO_BLUESIM,
    "bsc.interra/relax_method_urgency/BypassFIFO",
    "testsuite/bsc.interra/relax_method_urgency/BypassFIFO",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &[
        "Testbench.bsv",
        "Top.bsv",
        "Design.bsv",
        "mkTestbench.c.out.expected",
    ],
    &[
        SimulationLinkInput::GeneratedModule("mkTop"),
        SimulationLinkInput::GeneratedModule("mkDesign"),
    ],
    "mkTestbench.c.out.expected",
    OutputNormalization::Preserve,
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    BYPASS_FIFO_ICARUS,
    "bsc.interra/relax_method_urgency/BypassFIFO",
    "testsuite/bsc.interra/relax_method_urgency/BypassFIFO",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &[
        "Testbench.bsv",
        "Top.bsv",
        "Design.bsv",
        "mkTestbench.v.out.expected",
    ],
    &[
        SimulationLinkInput::GeneratedModule("mkTop"),
        SimulationLinkInput::GeneratedModule("mkDesign"),
    ],
    "mkTestbench.v.out.expected",
    OutputNormalization::Preserve,
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

dual_backend_scenario!(
    LOOPY_FIFO,
    "bsc.interra/relax_method_urgency/LoopyFIFO",
    "testsuite/bsc.interra/relax_method_urgency/LoopyFIFO",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Loopy.bsv", "mkTestbench.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.out.expected",
    "mkTestbench.out.expected",
    OutputNormalization::Preserve
);

backend_scenario!(
    RWIRE_MULT_BLUESIM,
    "bsc.interra/relax_method_urgency/RWire_mult",
    "testsuite/bsc.interra/relax_method_urgency/RWire_mult",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.c.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.c.out.expected",
    OutputNormalization::Preserve,
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    RWIRE_MULT_ICARUS,
    "bsc.interra/relax_method_urgency/RWire_mult",
    "testsuite/bsc.interra/relax_method_urgency/RWire_mult",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.v.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.v.out.expected",
    OutputNormalization::Preserve,
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

backend_scenario!(
    REG_FILE_BLUESIM,
    "bsc.interra/relax_method_urgency/RegFile",
    "testsuite/bsc.interra/relax_method_urgency/RegFile",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.c.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.c.out.expected",
    OutputNormalization::SortedLines,
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    REG_FILE_ICARUS,
    "bsc.interra/relax_method_urgency/RegFile",
    "testsuite/bsc.interra/relax_method_urgency/RegFile",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.v.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.v.out.expected",
    OutputNormalization::SortedLines,
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

dual_backend_scenario!(
    BYTE_ENABLE,
    "bsc.interra/relax_method_urgency/byte_en",
    "testsuite/bsc.interra/relax_method_urgency/byte_en",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.out.expected",
    "mkTestbench.out.expected",
    OutputNormalization::Preserve
);

backend_scenario!(
    DEMUX_BLUESIM,
    "bsc.interra/relax_method_urgency/demux",
    "testsuite/bsc.interra/relax_method_urgency/demux",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.c.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.c.out.expected",
    OutputNormalization::Preserve,
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    DEMUX_ICARUS,
    "bsc.interra/relax_method_urgency/demux",
    "testsuite/bsc.interra/relax_method_urgency/demux",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &["Testbench.bsv", "Design.bsv", "mkTestbench.v.out.expected"],
    &[SimulationLinkInput::GeneratedModule("mkDesign")],
    "mkTestbench.v.out.expected",
    OutputNormalization::Preserve,
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

backend_scenario!(
    PRODUCER_CONSUMER_BLUESIM,
    "bsc.interra/relax_method_urgency/prod_con",
    "testsuite/bsc.interra/relax_method_urgency/prod_con",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &[
        "Testbench.bsv",
        "producer.bsv",
        "consumer.bsv",
        "mkTestbench.c.out.expected",
    ],
    &[
        SimulationLinkInput::GeneratedModule("producer"),
        SimulationLinkInput::GeneratedModule("consumer"),
    ],
    "mkTestbench.c.out.expected",
    OutputNormalization::SortedLines,
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    PRODUCER_CONSUMER_ICARUS,
    "bsc.interra/relax_method_urgency/prod_con",
    "testsuite/bsc.interra/relax_method_urgency/prod_con",
    "Testbench",
    "Testbench.bsv",
    "mkTestbench",
    &[
        "Testbench.bsv",
        "producer.bsv",
        "consumer.bsv",
        "mkTestbench.v.out.expected",
    ],
    &[
        SimulationLinkInput::GeneratedModule("producer"),
        SimulationLinkInput::GeneratedModule("consumer"),
    ],
    "mkTestbench.v.out.expected",
    OutputNormalization::SortedLines,
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

dual_backend_scenario!(
    FAC_TEST,
    "bsc.lib/Stmt/FacTest",
    "testsuite/bsc.lib/Stmt/FacTest",
    "FacTest",
    "FacTest.bsv",
    "mkFacTest",
    &["FacTest.bsv", "mkFacTest.out.expected"],
    &[],
    "mkFacTest.out.expected",
    "mkFacTest.out.expected",
    OutputNormalization::Preserve
);

backend_scenario!(
    REPEAT_TEST_BLUESIM,
    "bsc.lib/Stmt/RepeatTest",
    "testsuite/bsc.lib/Stmt/RepeatTest",
    "RepeatTest",
    "RepeatTest.bsv",
    "mkRepeatTest",
    &["RepeatTest.bsv", "mkRepeatTest.out.expected"],
    &[],
    "mkRepeatTest.out.expected",
    OutputNormalization::Preserve,
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    REPEAT_TEST_ICARUS,
    "bsc.lib/Stmt/RepeatTest",
    "testsuite/bsc.lib/Stmt/RepeatTest",
    "RepeatTest",
    "RepeatTest.bsv",
    "mkRepeatTest",
    &["RepeatTest.bsv", "mkRepeatTest.v.out.expected"],
    &[],
    "mkRepeatTest.v.out.expected",
    OutputNormalization::Preserve,
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

dual_backend_scenario!(
    SERVER_TEST,
    "bsc.lib/Stmt/Server",
    "testsuite/bsc.lib/Stmt/Server",
    "ServerTest",
    "ServerTest.bsv",
    "sysServerTest",
    &["ServerTest.bsv", "sysServerTest.out.expected"],
    &[],
    "sysServerTest.out.expected",
    "sysServerTest.out.expected",
    OutputNormalization::Preserve
);

dual_backend_scenario!(
    SERVER_TEST_UPDATE,
    "bsc.lib/Stmt/Server",
    "testsuite/bsc.lib/Stmt/Server",
    "ServerTestUpdate",
    "ServerTestUpdate.bsv",
    "sysServerTestUpdate",
    &["ServerTestUpdate.bsv", "sysServerTestUpdate.out.expected"],
    &[],
    "sysServerTestUpdate.out.expected",
    "sysServerTestUpdate.out.expected",
    OutputNormalization::Preserve
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    B1118_CONTINUE_IN_FOR_LOOP,
    B1121_SIZED_FIFO,
    PUSH_APPLY,
    PUSH_TEE,
    PUSH_PASS,
    PUSH_PASSED,
    PUSH_BUFFER,
    PUSH_QBUFFER,
    PUSH_BUFFERED,
    PUSH_QBUFFERED,
    PUSH_SINK,
    PUSH_SPEW,
    PUSH_PIPE,
    PUSH_REG_TO_PUSH,
    PUSH_FIFO_TO_PUSH,
    BYPASS_FIFO_BLUESIM,
    BYPASS_FIFO_ICARUS,
    LOOPY_FIFO,
    RWIRE_MULT_BLUESIM,
    RWIRE_MULT_ICARUS,
    REG_FILE_BLUESIM,
    REG_FILE_ICARUS,
    BYTE_ENABLE,
    DEMUX_BLUESIM,
    DEMUX_ICARUS,
    PRODUCER_CONSUMER_BLUESIM,
    PRODUCER_CONSUMER_ICARUS,
    FAC_TEST,
    REPEAT_TEST_BLUESIM,
    REPEAT_TEST_ICARUS,
    SERVER_TEST,
    SERVER_TEST_UPDATE,
];
