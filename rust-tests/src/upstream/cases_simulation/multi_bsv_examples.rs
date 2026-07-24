//! Origins:
//! - `testsuite/bsc.bsv_examples/AmbaLoadDemo/amba_load_demo.exp`
//! - `testsuite/bsc.bsv_examples/Maxtree/maxtree.exp`
//! - `testsuite/bsc.bsv_examples/RAMS/RAMS.exp`
//! - `testsuite/bsc.bsv_examples/SimpleIfcArgInvert/simple_ifc_arg_invert.exp`
//! - `testsuite/bsc.bsv_examples/gcd/gcd.exp`
//! - `testsuite/bsc.bsv_examples/mcd_Rand/rand.exp`
//! - `testsuite/bsc.bsv_examples/wallace/wallace.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario {
    (
        $constant:ident,
        $name:literal,
        $fixture_dir:expr,
        $source:literal,
        $fixtures:expr,
        $top:literal,
        $link_inputs:expr,
        $compile_options:expr,
        $expected:literal
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($name, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($name, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
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
        $name:literal,
        $fixture_dir:expr,
        $source:literal,
        $fixtures:expr,
        $top:literal,
        $link_inputs:expr,
        $compile_options:expr,
        $expected:literal,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($name, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: $compile_options,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($name, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

const AMBA_LOAD_DEMO_DIR: &str = "testsuite/bsc.bsv_examples/AmbaLoadDemo";
shared_scenario!(
    AMBA_LOAD_DEMO,
    "bsc.bsv_examples/AmbaLoadDemo::TBbaseline",
    AMBA_LOAD_DEMO_DIR,
    "TBbaseline.bsv",
    &[
        "TBbaseline.bsv",
        "Interfaces.bsv",
        "Buses.bsv",
        "Masters.bsv",
        "Slaves.bsv",
        "sysM1_25.out.expected",
    ],
    "sysM1_25",
    &[],
    &[],
    "sysM1_25.out.expected"
);

const MAXTREE_DIR: &str = "testsuite/bsc.bsv_examples/Maxtree";
backend_scenario!(
    MAXTREE_PUSH_ICARUS,
    "bsc.bsv_examples/Maxtree::TestPush::sys_fifo",
    MAXTREE_DIR,
    "TestPush.bsv",
    &["TestPush.bsv", "sys_fifo.v.out.expected"],
    "sys_fifo",
    &[SimulationLinkInput::GeneratedModule("mktestpush_fifo")],
    &[],
    "sys_fifo.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
backend_scenario!(
    MAXTREE_PUSH_BLUESIM,
    "bsc.bsv_examples/Maxtree::TestPush::sys_fifo",
    MAXTREE_DIR,
    "TestPush.bsv",
    &["TestPush.bsv", "sys_fifo.out.expected"],
    "sys_fifo",
    &[SimulationLinkInput::GeneratedModule("mktestpush_fifo")],
    &[],
    "sys_fifo.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    MAXTREE_TWO_QUEUES_ICARUS,
    "bsc.bsv_examples/Maxtree::TestMaxTree::sys_2q",
    MAXTREE_DIR,
    "TestMaxTree.bsv",
    &["TestMaxTree.bsv", "MaxTree.bsv", "sys_2q.v.out.expected"],
    "sys_2q",
    &[SimulationLinkInput::GeneratedModule("mkMaxTree8_2q")],
    &["-let-gen"],
    "sys_2q.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
backend_scenario!(
    MAXTREE_TWO_QUEUES_BLUESIM,
    "bsc.bsv_examples/Maxtree::TestMaxTree::sys_2q",
    MAXTREE_DIR,
    "TestMaxTree.bsv",
    &["TestMaxTree.bsv", "MaxTree.bsv", "sys_2q.out.expected"],
    "sys_2q",
    &[SimulationLinkInput::GeneratedModule("mkMaxTree8_2q")],
    &["-let-gen"],
    "sys_2q.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);

const RAMS_DIR: &str = "testsuite/bsc.bsv_examples/RAMS";
backend_scenario!(
    RAMS_ICARUS,
    "bsc.bsv_examples/RAMS::Test::mkTop",
    RAMS_DIR,
    "Test.bsv",
    &[
        "Test.bsv",
        "SRAM_wrapper.bsv",
        "Verilog_SRAM_model.v",
        "mem_init.data",
        "mkTop.out.expected",
    ],
    "mkTop",
    &[SimulationLinkInput::ExactFile("Verilog_SRAM_model.v")],
    &[],
    "mkTop.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

const SIMPLE_IFC_ARG_INVERT_DIR: &str = "testsuite/bsc.bsv_examples/SimpleIfcArgInvert";
backend_scenario!(
    SIMPLE_IFC_ARG_INVERT_PROC1_ICARUS,
    "bsc.bsv_examples/SimpleIfcArgInvert::Proc1::mkProc1_TB",
    SIMPLE_IFC_ARG_INVERT_DIR,
    "Proc1.bsv",
    &["Proc1.bsv", "Common.bsv", "proc.v.out.expected"],
    "mkProc1_TB",
    &[],
    &[],
    "proc.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
backend_scenario!(
    SIMPLE_IFC_ARG_INVERT_PROC1_BLUESIM,
    "bsc.bsv_examples/SimpleIfcArgInvert::Proc1::mkProc1_TB",
    SIMPLE_IFC_ARG_INVERT_DIR,
    "Proc1.bsv",
    &["Proc1.bsv", "Common.bsv", "proc.c.out.expected"],
    "mkProc1_TB",
    &[],
    &[],
    "proc.c.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    SIMPLE_IFC_ARG_INVERT_PROC2_ICARUS,
    "bsc.bsv_examples/SimpleIfcArgInvert::Proc2::mkProc2_TB",
    SIMPLE_IFC_ARG_INVERT_DIR,
    "Proc2.bsv",
    &["Proc2.bsv", "Common.bsv", "proc.v.out.expected"],
    "mkProc2_TB",
    &[SimulationLinkInput::GeneratedModule("mkProc2")],
    &[],
    "proc.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);
backend_scenario!(
    SIMPLE_IFC_ARG_INVERT_PROC2_BLUESIM,
    "bsc.bsv_examples/SimpleIfcArgInvert::Proc2::mkProc2_TB",
    SIMPLE_IFC_ARG_INVERT_DIR,
    "Proc2.bsv",
    &["Proc2.bsv", "Common.bsv", "proc.c.out.expected"],
    "mkProc2_TB",
    &[SimulationLinkInput::GeneratedModule("mkProc2")],
    &[],
    "proc.c.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);

const GCD_DIR: &str = "testsuite/bsc.bsv_examples/gcd";
shared_scenario!(
    GCD,
    "bsc.bsv_examples/gcd::TbGCD",
    GCD_DIR,
    "TbGCD.bsv",
    &["TbGCD.bsv", "GCD.bsv", "mkTbGCD.out.expected"],
    "mkTbGCD",
    &[SimulationLinkInput::GeneratedModule("mkGCD")],
    &[],
    "mkTbGCD.out.expected"
);
shared_scenario!(
    WIDE_GCD,
    "bsc.bsv_examples/gcd::TbWideGCD",
    GCD_DIR,
    "TbWideGCD.bsv",
    &["TbWideGCD.bsv", "WideGCD.bsv", "mkTbWideGCD.out.expected",],
    "mkTbWideGCD",
    &[SimulationLinkInput::GeneratedModule("mkWideGCD")],
    &[],
    "mkTbWideGCD.out.expected"
);
shared_scenario!(
    DIVISIBLE_BY_THREE,
    "bsc.bsv_examples/gcd::TbDIV3",
    GCD_DIR,
    "TbDIV3.bsv",
    &["TbDIV3.bsv", "DIV3.bsv", "mkTbDIV3.out.expected"],
    "mkTbDIV3",
    &[SimulationLinkInput::GeneratedModule("mkDIV3")],
    &[],
    "mkTbDIV3.out.expected"
);

const MCD_RAND_DIR: &str = "testsuite/bsc.bsv_examples/mcd_Rand";
const MCD_RAND_SOURCES: &[&str] = &[
    "Top.bsv",
    "RandTop5.bsv",
    "RandGen.bsv",
    "RandGlobal.bsv",
    "RandUser1.bsv",
    "RandUser2.bsv",
];
backend_scenario!(
    MCD_RAND_BLUESIM,
    "bsc.bsv_examples/mcd_Rand::Top::mkTop",
    MCD_RAND_DIR,
    "Top.bsv",
    &[
        MCD_RAND_SOURCES[0],
        MCD_RAND_SOURCES[1],
        MCD_RAND_SOURCES[2],
        MCD_RAND_SOURCES[3],
        MCD_RAND_SOURCES[4],
        MCD_RAND_SOURCES[5],
        "mkTop.out.expected",
    ],
    "mkTop",
    &[SimulationLinkInput::GeneratedModule("mkRandTop")],
    &["-aggressive-conditions"],
    "mkTop.out.expected",
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
backend_scenario!(
    MCD_RAND_ICARUS,
    "bsc.bsv_examples/mcd_Rand::Top::mkTop",
    MCD_RAND_DIR,
    "Top.bsv",
    &[
        MCD_RAND_SOURCES[0],
        MCD_RAND_SOURCES[1],
        MCD_RAND_SOURCES[2],
        MCD_RAND_SOURCES[3],
        MCD_RAND_SOURCES[4],
        MCD_RAND_SOURCES[5],
        "mkTop.v.out.expected",
    ],
    "mkTop",
    &[SimulationLinkInput::GeneratedModule("mkRandTop")],
    &["-aggressive-conditions"],
    "mkTop.v.out.expected",
    "icarus",
    Icarus,
    Some(VcdContract::parse()),
    VerilogEnabled
);

const WALLACE_DIR: &str = "testsuite/bsc.bsv_examples/wallace";
const WALLACE_FIXTURES: &[&str] = &[
    "WallaceTest.bsv",
    "CombWallace.bsv",
    "StatefulWallace.bsv",
    "WallaceLib.bsv",
    "WallaceServer.bsv",
    "wallace435.out.expected",
];
shared_scenario!(
    WALLACE_COMBINATIONAL,
    "bsc.bsv_examples/wallace::WallaceTest::testCombServer",
    WALLACE_DIR,
    "WallaceTest.bsv",
    WALLACE_FIXTURES,
    "testCombServer",
    &[SimulationLinkInput::GeneratedModule("sysCombServer")],
    &[],
    "wallace435.out.expected"
);
shared_scenario!(
    WALLACE_STATEFUL_1,
    "bsc.bsv_examples/wallace::WallaceTest::testStatefulServer1",
    WALLACE_DIR,
    "WallaceTest.bsv",
    WALLACE_FIXTURES,
    "testStatefulServer1",
    &[SimulationLinkInput::GeneratedModule("sysStatefulServer1")],
    &[],
    "wallace435.out.expected"
);
shared_scenario!(
    WALLACE_STATEFUL_2,
    "bsc.bsv_examples/wallace::WallaceTest::testStatefulServer2",
    WALLACE_DIR,
    "WallaceTest.bsv",
    WALLACE_FIXTURES,
    "testStatefulServer2",
    &[SimulationLinkInput::GeneratedModule("sysStatefulServer2")],
    &[],
    "wallace435.out.expected"
);
shared_scenario!(
    WALLACE_STATEFUL_3,
    "bsc.bsv_examples/wallace::WallaceTest::testStatefulServer3",
    WALLACE_DIR,
    "WallaceTest.bsv",
    WALLACE_FIXTURES,
    "testStatefulServer3",
    &[SimulationLinkInput::GeneratedModule("sysStatefulServer3")],
    &[],
    "wallace435.out.expected"
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    AMBA_LOAD_DEMO,
    MAXTREE_PUSH_ICARUS,
    MAXTREE_PUSH_BLUESIM,
    MAXTREE_TWO_QUEUES_ICARUS,
    MAXTREE_TWO_QUEUES_BLUESIM,
    RAMS_ICARUS,
    SIMPLE_IFC_ARG_INVERT_PROC1_ICARUS,
    SIMPLE_IFC_ARG_INVERT_PROC1_BLUESIM,
    SIMPLE_IFC_ARG_INVERT_PROC2_ICARUS,
    SIMPLE_IFC_ARG_INVERT_PROC2_BLUESIM,
    GCD,
    WIDE_GCD,
    DIVISIBLE_BY_THREE,
    MCD_RAND_BLUESIM,
    MCD_RAND_ICARUS,
    WALLACE_COMBINATIONAL,
    WALLACE_STATEFUL_1,
    WALLACE_STATEFUL_2,
    WALLACE_STATEFUL_3,
];
