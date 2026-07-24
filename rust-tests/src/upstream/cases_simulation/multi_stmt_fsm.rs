//! Origins:
//! - `testsuite/bsc.interra/StmtFSM/Square1/square1.exp`
//! - `testsuite/bsc.interra/StmtFSM/Square2/square2.exp`
//! - `testsuite/bsc.interra/StmtFSM/Square3/square3.exp`
//! - `testsuite/bsc.interra/StmtFSM/Square4/square4.exp`
//! - `testsuite/bsc.interra/StmtFSM/Square5/square5.exp`
//! - `testsuite/bsc.interra/StmtFSM/Square6/square6.exp`
//! - `testsuite/bsc.interra/StmtFSM/clearOfOnce/clearOfOnce.exp`
//! - `testsuite/bsc.interra/StmtFSM/cycleUsage1/cycleUsage1.exp`
//! - `testsuite/bsc.interra/StmtFSM/cycleUsage2/cycleUsage2.exp`
//! - `testsuite/bsc.interra/StmtFSM/fifoTest/fifoTest.exp`
//! - `testsuite/bsc.interra/StmtFSM/forInRepeat/forInRepeat.exp`
//! - `testsuite/bsc.interra/StmtFSM/forInWhile/forInWhile.exp`
//! - `testsuite/bsc.interra/StmtFSM/nestedForLoop1/nestedForLoop1.exp`
//! - `testsuite/bsc.interra/StmtFSM/nestedRepeatLoop1/nestedRepeatLoop1.exp`
//! - `testsuite/bsc.interra/StmtFSM/nestedWhileLoop1/nestedWhileLoop1.exp`
//! - `testsuite/bsc.interra/StmtFSM/nestedWhileLoop2/nestedWhileLoop2.exp`
//! - `testsuite/bsc.interra/StmtFSM/parAuto/parAuto.exp`
//! - `testsuite/bsc.interra/StmtFSM/repeatInFor/repeatInFor.exp`
//! - `testsuite/bsc.interra/StmtFSM/repeatInWhile/repeatInWhile.exp`
//! - `testsuite/bsc.interra/StmtFSM/repeatTest/repeatTest.exp`
//! - `testsuite/bsc.interra/StmtFSM/whileInFor/whileInFor.exp`
//! - `testsuite/bsc.interra/StmtFSM/whileInRepeat/whileInRepeat.exp`
//! - `testsuite/bsc.interra/StmtFSM/whilePar/whilePar.exp`
//! - `testsuite/bsc.interra/StmtFSM/whileWithinForLoop/whileWithinForLoop.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

macro_rules! test_c_veri_bsv_multi_scenario {
    (
        $constant:ident,
        origin: $origin:literal,
        topbsv: $topbsv:literal,
        topmod: $topmod:literal,
        modules: $modules:expr,
        expected: $expected:literal,
        cbug: "",
        veribug: "",
        sort_output: 0,
        check_vcd: 1
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!("bsc.interra/StmtFSM/", $origin, "::", $topbsv),
            fixture_dir: concat!("testsuite/bsc.interra/StmtFSM/", $origin),
            source: concat!($topbsv, ".bsv"),
            fixtures: &[concat!($topbsv, ".bsv"), $expected],
            top: $topmod,
            link_inputs: $modules,
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.interra/StmtFSM/", $origin, "::", $topbsv, "::bluesim"),
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
                    name: concat!("bsc.interra/StmtFSM/", $origin, "::", $topbsv, "::icarus"),
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

macro_rules! stmt_fsm_backend_scenario {
    (
        $constant:ident,
        $origin:literal,
        $topbsv:literal,
        $topmod:literal,
        $modules:expr,
        $gen_options:expr,
        $expected:literal,
        $link_options:expr,
        $sim_options:expr,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.interra/StmtFSM/",
                $origin,
                "::",
                $topbsv,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: concat!("testsuite/bsc.interra/StmtFSM/", $origin),
            source: concat!($topbsv, ".bsv"),
            fixtures: &[concat!($topbsv, ".bsv"), $expected],
            top: $topmod,
            link_inputs: $modules,
            compile_options: $gen_options,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!(
                    "bsc.interra/StmtFSM/",
                    $origin,
                    "::",
                    $topbsv,
                    "::",
                    $backend_name
                ),
                assertions: &[],
                link_options: $link_options,
                simulation_options: $sim_options,
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

macro_rules! test_c_veri_bsv_multi_options_separately_scenarios {
    (
        bluesim: $bluesim_constant:ident,
        icarus: $icarus_constant:ident,
        origin: $origin:literal,
        topbsv: $topbsv:literal,
        topmod: $topmod:literal,
        modules: $modules:expr,
        gen_options: $gen_options:expr,
        expected: $expected:literal,
        cbug: "",
        veribug: "",
        do_c: 1,
        do_v: 1,
        link_options: $link_options:expr,
        sim_options: $sim_options:expr,
        sort_output: 0,
        check_vcd: 1
    ) => {
        stmt_fsm_backend_scenario!(
            $bluesim_constant,
            $origin,
            $topbsv,
            $topmod,
            $modules,
            $gen_options,
            $expected,
            $link_options,
            $sim_options,
            "bluesim",
            Bluesim,
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        );
        stmt_fsm_backend_scenario!(
            $icarus_constant,
            $origin,
            $topbsv,
            $topmod,
            $modules,
            $gen_options,
            $expected,
            $link_options,
            $sim_options,
            "icarus",
            Icarus,
            Some(VcdContract::parse()),
            VerilogEnabled
        );
    };
}

test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: SQUARE_1_BLUESIM,
    icarus: SQUARE_1_ICARUS,
    origin: "Square1",
    topbsv: "square1",
    topmod: "sysValidValue1",
    modules: &[SimulationLinkInput::GeneratedModule("mkSquare")],
    gen_options: &[],
    expected: "square1.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: SQUARE_2_BLUESIM,
    icarus: SQUARE_2_ICARUS,
    origin: "Square2",
    topbsv: "square2",
    topmod: "sysValidValue2",
    modules: &[SimulationLinkInput::GeneratedModule("mkSquare")],
    gen_options: &[],
    expected: "square2.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: SQUARE_3_BLUESIM,
    icarus: SQUARE_3_ICARUS,
    origin: "Square3",
    topbsv: "square3",
    topmod: "sysValidValue3",
    modules: &[SimulationLinkInput::GeneratedModule("mkSquare")],
    gen_options: &[],
    expected: "square3.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: SQUARE_4_BLUESIM,
    icarus: SQUARE_4_ICARUS,
    origin: "Square4",
    topbsv: "square4",
    topmod: "sysValidValue4",
    modules: &[SimulationLinkInput::GeneratedModule("mkSquare")],
    gen_options: &[],
    expected: "square4.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: SQUARE_5_BLUESIM,
    icarus: SQUARE_5_ICARUS,
    origin: "Square5",
    topbsv: "square5",
    topmod: "sysValidValue5",
    modules: &[SimulationLinkInput::GeneratedModule("mkSquare")],
    gen_options: &[],
    expected: "square5.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: SQUARE_6_BLUESIM,
    icarus: SQUARE_6_ICARUS,
    origin: "Square6",
    topbsv: "square6",
    topmod: "sysValidValue6",
    modules: &[SimulationLinkInput::GeneratedModule("mkSquare")],
    gen_options: &[],
    expected: "square6.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);

test_c_veri_bsv_multi_scenario!(
    CLEAR_OF_ONCE,
    origin: "clearOfOnce",
    topbsv: "clearOfOnce",
    topmod: "clearOfOnce",
    modules: &[],
    expected: "clearOfOnce.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    CYCLE_USAGE_1,
    origin: "cycleUsage1",
    topbsv: "cycleUsage1",
    topmod: "cycleUsage1",
    modules: &[],
    expected: "cycleUsage1.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    CYCLE_USAGE_2,
    origin: "cycleUsage2",
    topbsv: "cycleUsage2",
    topmod: "cycleUsage2",
    modules: &[],
    expected: "cycleUsage2.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    FIFO_TEST,
    origin: "fifoTest",
    topbsv: "fifoTest",
    topmod: "fifoTest",
    modules: &[],
    expected: "fifoTest.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    FOR_IN_REPEAT,
    origin: "forInRepeat",
    topbsv: "forInRepeat",
    topmod: "forInRepeat",
    modules: &[],
    expected: "forInRepeat.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    FOR_IN_WHILE,
    origin: "forInWhile",
    topbsv: "forInWhile",
    topmod: "forInWhile",
    modules: &[],
    expected: "forInWhile.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    NESTED_FOR_LOOP_1,
    origin: "nestedForLoop1",
    topbsv: "nestedForLoop1",
    topmod: "nestedForLoop1",
    modules: &[],
    expected: "nestedForLoop1.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    NESTED_REPEAT_LOOP_1,
    origin: "nestedRepeatLoop1",
    topbsv: "nestedRepeatLoop1",
    topmod: "nestedRepeatLoop1",
    modules: &[],
    expected: "nestedRepeatLoop1.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    NESTED_WHILE_LOOP_1,
    origin: "nestedWhileLoop1",
    topbsv: "nestedWhileLoop1",
    topmod: "nestedWhileLoop1",
    modules: &[],
    expected: "nestedWhileLoop1.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    NESTED_WHILE_LOOP_2,
    origin: "nestedWhileLoop2",
    topbsv: "nestedWhileLoop2",
    topmod: "nestedWhileLoop2",
    modules: &[],
    expected: "nestedWhileLoop2.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    PAR_AUTO,
    origin: "parAuto",
    topbsv: "parAuto",
    topmod: "parAuto",
    modules: &[],
    expected: "parAuto.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    REPEAT_IN_FOR,
    origin: "repeatInFor",
    topbsv: "repeatInFor",
    topmod: "repeatInFor",
    modules: &[],
    expected: "repeatInFor.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    REPEAT_IN_WHILE,
    origin: "repeatInWhile",
    topbsv: "repeatInWhile",
    topmod: "repeatInWhile",
    modules: &[],
    expected: "repeatInWhile.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_options_separately_scenarios!(
    bluesim: REPEAT_TEST_BLUESIM,
    icarus: REPEAT_TEST_ICARUS,
    origin: "repeatTest",
    topbsv: "repeatTest",
    topmod: "repeatTest",
    modules: &[],
    gen_options: &[],
    expected: "repeatTest.out.expected",
    cbug: "",
    veribug: "",
    do_c: 1,
    do_v: 1,
    link_options: &[],
    sim_options: &[],
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    WHILE_IN_FOR,
    origin: "whileInFor",
    topbsv: "whileInFor",
    topmod: "whileInFor",
    modules: &[],
    expected: "whileInFor.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    WHILE_IN_REPEAT,
    origin: "whileInRepeat",
    topbsv: "whileInRepeat",
    topmod: "whileInRepeat",
    modules: &[],
    expected: "whileInRepeat.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    WHILE_PAR,
    origin: "whilePar",
    topbsv: "whilePar",
    topmod: "whilePar",
    modules: &[],
    expected: "whilePar.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);
test_c_veri_bsv_multi_scenario!(
    WHILE_WITHIN_FOR_LOOP,
    origin: "whileWithinForLoop",
    topbsv: "whileWithinForLoop",
    topmod: "whileWithinForLoop",
    modules: &[],
    expected: "whileWithinForLoop.out.expected",
    cbug: "",
    veribug: "",
    sort_output: 0,
    check_vcd: 1
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    SQUARE_1_BLUESIM,
    SQUARE_1_ICARUS,
    SQUARE_2_BLUESIM,
    SQUARE_2_ICARUS,
    SQUARE_3_BLUESIM,
    SQUARE_3_ICARUS,
    SQUARE_4_BLUESIM,
    SQUARE_4_ICARUS,
    SQUARE_5_BLUESIM,
    SQUARE_5_ICARUS,
    SQUARE_6_BLUESIM,
    SQUARE_6_ICARUS,
    CLEAR_OF_ONCE,
    CYCLE_USAGE_1,
    CYCLE_USAGE_2,
    FIFO_TEST,
    FOR_IN_REPEAT,
    FOR_IN_WHILE,
    NESTED_FOR_LOOP_1,
    NESTED_REPEAT_LOOP_1,
    NESTED_WHILE_LOOP_1,
    NESTED_WHILE_LOOP_2,
    PAR_AUTO,
    REPEAT_IN_FOR,
    REPEAT_IN_WHILE,
    REPEAT_TEST_BLUESIM,
    REPEAT_TEST_ICARUS,
    WHILE_IN_FOR,
    WHILE_IN_REPEAT,
    WHILE_PAR,
    WHILE_WITHIN_FOR_LOOP,
];
