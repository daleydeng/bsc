//! Origins:
//! - `testsuite/bsc.codegen/vector_modargs/vector_modargs.exp`
//! - `testsuite/bsc.verilog/parameters/real/real_param.exp`
//! - `testsuite/bsc.verilog/positivereset/nameclash/nameclash.exp`
//! - `testsuite/bsc.verilog/splitports/splitports.exp`
//! - `testsuite/bsc.verilog/undet/undet.exp`

// Only origins with runtime helpers contribute `SCENARIOS`; compile-only origins are declared in
// the corresponding compile module.
use super::SimulationScenario;
use crate::upstream::{
    ArtifactAssertion, ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement,
    ResourceClass, SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts,
    TextAssertion, VcdContract,
};

macro_rules! text {
    ($path:expr, contains $value:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Contains { text: $value },
        }
    };
    ($path:expr, lines $value:expr, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $value,
                count: $count,
            },
        }
    };
    ($path:expr, regex $value:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Regex { pattern: $value },
        }
    };
    ($path:expr, regex_not $value:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::RegexDoesNotMatch { pattern: $value },
        }
    };
}

macro_rules! dual_scenario {
    (
        $constant:ident,
        origin: $origin:literal,
        dir: $dir:literal,
        stem: $stem:literal,
        extension: $extension:literal,
        fixtures: $fixtures:expr,
        links: $links:expr,
        compile: $compile:expr,
        icarus_assertions: $assertions:expr,
        link: $link:expr,
        simulation: $simulation:expr $(,)?
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($origin, "::", $stem),
            fixture_dir: $dir,
            source: concat!($stem, $extension),
            fixtures: $fixtures,
            top: concat!("sys", $stem),
            link_inputs: $links,
            compile_options: $compile,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($origin, "::", $stem, "::bluesim"),
                    assertions: &[],
                    link_options: $link,
                    simulation_options: $simulation,
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $stem, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($origin, "::", $stem, "::icarus"),
                    assertions: $assertions,
                    link_options: $link,
                    simulation_options: $simulation,
                    expectation: ExpectedOutcome::Pass {
                        output: concat!("sys", $stem, ".out.expected"),
                    },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

macro_rules! icarus_scenario {
    (
        $constant:ident,
        origin: $origin:literal,
        dir: $dir:literal,
        stem: $stem:literal,
        fixtures: $fixtures:expr,
        links: $links:expr,
        compile: $compile:expr,
        assertions: $assertions:expr,
        link: $link:expr,
        simulation: $simulation:expr $(,)?
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($origin, "::", $stem),
            fixture_dir: $dir,
            source: concat!($stem, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $stem),
            link_inputs: $links,
            compile_options: $compile,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::Icarus),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($origin, "::", $stem, "::icarus"),
                assertions: $assertions,
                link_options: $link,
                simulation_options: $simulation,
                expectation: ExpectedOutcome::Pass {
                    output: concat!("sys", $stem, ".out.expected"),
                },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::Icarus,
                vcd: Some(VcdContract::parse()),
                requirement: Requirement::VerilogEnabled,
            }],
        };
    };
}

dual_scenario!(
    SPLIT_SHALLOW,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "ShallowSplit",
    extension: ".bs",
    fixtures: &["ShallowSplit.bs", "sysShallowSplit.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkShallowSplitTest.v", regex r"input  \[7 : 0\] putFoo_1_x;"),
        text!("mkShallowSplitTest.v", regex r"input  \[15 : 0\] PUT_BAR_1_z;"),
        text!("mkShallowSplitTest.v", regex r"input  \[7 : 0\] putFooBar_fooIn_y;"),
        text!("mkShallowSplitTest.v", regex r"input  \[16 : 0\] putFooBar_barIn_w;"),
        text!("mkShallowSplitTest.v", regex r"input  \[15 : 0\] putFoos_1_0;"),
        text!("mkShallowSplitTest.v", regex r"input  \[15 : 0\] putFoos_1_49;"),
        text!("mkShallowSplitTest.v", regex r"input  \[16 : 0\] putBaz_1_a;"),
        text!("mkShallowSplitTest.v", regex r"input  \[491 : 0\] putBaz_1_c;"),
        text!("mkShallowSplitTest.v", regex r"output \[7 : 0\] getFoo_x;"),
        text!("mkShallowSplitTest.v", regex r"output \[7 : 0\] getFoo_y;"),
        text!("mkShallowSplitTest.v", regex r"output \[2 : 0\] GET_BAR_v;"),
        text!("mkShallowSplitTest.v", regex r"output \[15 : 0\] GET_BAR_z;"),
        text!("mkShallowSplitTest.v", regex r"input  \[7 : 0\] update_1_x;"),
        text!("mkShallowSplitTest.v", regex r"input  EN_update;"),
        text!("mkShallowSplitTest.v", regex r"output \[16 : 0\] update_w;"),
        text!("mkShallowSplitTest.v", regex r"output RDY_update;"),
        text!("mkShallowSplitTest.v", regex r"\(getBar_1_x, getBar_1_y\) -> GET_BAR_z"),
        text!("mkShallowSplitTest.v", regex r"update_1_x -> update_v"),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_DEEP,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "DeepSplit",
    extension: ".bs",
    fixtures: &["DeepSplit.bs", "sysDeepSplit.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] putFoo_1_x;"),
        text!("mkDeepSplitTest.v", regex r"input  PUT_BAR_1_v_2;"),
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] PUT_BAR_1_z_y;"),
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] putFooBar_fooIn_y;"),
        text!("mkDeepSplitTest.v", regex r"input  putFooBar_barIn_v_2;"),
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] putFoos_1_0_x;"),
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] putFoos_1_49_y;"),
        text!("mkDeepSplitTest.v", regex r"input  \[16 : 0\] putBaz_1_a;"),
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] putBaz_1_c_2_1_7_y;"),
        text!("mkDeepSplitTest.v", regex r"input  \[15 : 0\] putBaz_1_c_2_2_w_2;"),
        text!("mkDeepSplitTest.v", regex r"input  \[3 : 0\] putZug_1_qs_1;"),
        text!("mkDeepSplitTest.v", regex r"output \[7 : 0\] getFoo_x;"),
        text!("mkDeepSplitTest.v", regex r"output \[7 : 0\] getFoo_y;"),
        text!("mkDeepSplitTest.v", regex r"output GET_BAR_v_0;"),
        text!("mkDeepSplitTest.v", regex r"output GET_BAR_v_2;"),
        text!("mkDeepSplitTest.v", regex r"output \[15 : 0\] GET_BAR_w_2;"),
        text!("mkDeepSplitTest.v", regex r"output \[7 : 0\] GET_BAR_z_y;"),
        text!("mkDeepSplitTest.v", regex r"output \[3 : 0\] getZug_qs_0;"),
        text!("mkDeepSplitTest.v", regex r"output \[3 : 0\] getZug_qs_1;"),
        text!("mkDeepSplitTest.v", regex r"output getZug_blob;"),
        text!("mkDeepSplitTest.v", regex r"input  \[7 : 0\] update_1_x;"),
        text!("mkDeepSplitTest.v", regex r"input  EN_update;"),
        text!("mkDeepSplitTest.v", regex r"output \[15 : 0\] update_w_2;"),
        text!("mkDeepSplitTest.v", regex r"output RDY_update;"),
        text!("mkDeepSplitTest.v", regex r"getBar_1_x -> GET_BAR_z_x"),
        text!("mkDeepSplitTest.v", regex r"getBar_1_y -> GET_BAR_z_y"),
        text!("mkDeepSplitTest.v", regex r"update_1_x -> update_v_0"),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_INSTANCE,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "InstanceSplit",
    extension: ".bs",
    fixtures: &["InstanceSplit.bs", "sysInstanceSplit.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkInstanceSplitTest.v", regex r"input  \[7 : 0\] putFoo_1_x;"),
        text!("mkInstanceSplitTest.v", regex r"input  putFoo_1_ysign;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[6 : 0\] putFoo_1_yvalue;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[7 : 0\] PUT_BAR_1_z_x;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[6 : 0\] putFooBar_fooIn_yvalue;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[16 : 0\] putFooBar_barIn_w;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[799 : 0\] putFoos_1;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[16 : 0\] putBaz_1_a;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[491 : 0\] putBaz_1_c;"),
        text!("mkInstanceSplitTest.v", regex r"output \[7 : 0\] getFoo_x;"),
        text!("mkInstanceSplitTest.v", regex r"output getFoo_ysign;"),
        text!("mkInstanceSplitTest.v", regex r"output \[6 : 0\] getFoo_yvalue;"),
        text!("mkInstanceSplitTest.v", regex r"output \[2 : 0\] GET_BAR_v;"),
        text!("mkInstanceSplitTest.v", regex r"output GET_BAR_z_ysign;"),
        text!("mkInstanceSplitTest.v", regex r"input  \[7 : 0\] update_1_x;"),
        text!("mkInstanceSplitTest.v", regex r"input  EN_update;"),
        text!("mkInstanceSplitTest.v", regex r"output \[6 : 0\] update_z_yvalue;"),
        text!("mkInstanceSplitTest.v", regex r"output RDY_update;"),
        text!("mkInstanceSplitTest.v", regex r"getBar_1_x -> GET_BAR_z_x"),
        text!(
            "mkInstanceSplitTest.v",
            regex r"\(getBar_1_ysign, getBar_1_yvalue\) -> GET_BAR_z_ysign"
        ),
        text!(
            "mkInstanceSplitTest.v",
            regex r"\(getBar_1_ysign, getBar_1_yvalue\) -> GET_BAR_z_yvalue"
        ),
        text!("mkInstanceSplitTest.v", regex r"update_1_x -> update_v"),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_SOME_ARG_NAMES,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "SomeArgNames",
    extension: ".bs",
    fixtures: &["SomeArgNames.bs", "sysSomeArgNames.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkSomeArgNamesSplitTest.v", regex r"input  \[7 : 0\] putFooBar_fooIn_x;"),
        text!("mkSomeArgNamesSplitTest.v", regex r"input  \[7 : 0\] putFooBar_fooIn_y;"),
        text!("mkSomeArgNamesSplitTest.v", regex r"input  \[7 : 0\] putFooBar_2_f_x;"),
        text!("mkSomeArgNamesSplitTest.v", regex r"input  \[7 : 0\] putFooBar_2_f_y;"),
        text!("mkSomeArgNamesSplitTest.v", regex r"input  putFooBar_2_b;"),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_NOINLINE,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "NoinlineSplit",
    extension: ".bs",
    fixtures: &["NoinlineSplit.bs", "sysNoinlineSplit.out.expected"],
    links: &[SimulationLinkInput::GeneratedModule("module_swapFoo")],
    compile: &[],
    icarus_assertions: &[
        text!("module_swapFoo.v", regex r"input  \[7 : 0\] swapFoo_f_x;"),
        text!("module_swapFoo.v", regex r"input  \[7 : 0\] swapFoo_f_y;"),
        text!("module_swapFoo.v", regex r"output \[7 : 0\] swapFoo_x;"),
        text!("module_swapFoo.v", regex r"output \[7 : 0\] swapFoo_y;"),
        text!("sysNoinlineSplit.v", regex r"\.swapFoo_f_x\("),
        text!("sysNoinlineSplit.v", regex r"\.swapFoo_f_y\("),
        text!("sysNoinlineSplit.v", regex r"\.swapFoo_x\("),
        text!("sysNoinlineSplit.v", regex r"\.swapFoo_y\("),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_NOINLINE_MULTI,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "NoinlineSplitMulti",
    extension: ".bs",
    fixtures: &[
        "NoinlineSplitMulti.bs",
        "sysNoinlineSplitMulti.out.expected"
    ],
    links: &[SimulationLinkInput::GeneratedModule("module_combine")],
    compile: &[],
    icarus_assertions: &[
        text!("module_combine.v", regex r"input  \[7 : 0\] combine__a1000_x;"),
        text!("module_combine.v", regex r"input  \[7 : 0\] combine__a1000_y;"),
        text!("module_combine.v", regex r"input  \[7 : 0\] combine_k;"),
        text!("module_combine.v", regex r"input  \[7 : 0\] combine__a1002_x;"),
        text!("module_combine.v", regex r"input  \[7 : 0\] combine__a1002_y;"),
        text!("module_combine.v", regex r"output \[7 : 0\] combine_x;"),
        text!("module_combine.v", regex r"output \[7 : 0\] combine_y;"),
        text!("sysNoinlineSplitMulti.v", regex r"\.combine_k\(8'd10\)"),
        text!("sysNoinlineSplitMulti.v", regex r"\.combine__a1000_x\("),
        text!("sysNoinlineSplitMulti.v", regex r"\.combine__a1002_y\("),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_NOINLINE_DEEP,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "NoinlineDeepSplit",
    extension: ".bs",
    fixtures: &[
        "NoinlineDeepSplit.bs",
        "sysNoinlineDeepSplit.out.expected"
    ],
    links: &[SimulationLinkInput::GeneratedModule("module_tweak")],
    compile: &[],
    icarus_assertions: &[
        text!("module_tweak.v", regex r"input  tweak_b_v_0;"),
        text!("module_tweak.v", regex r"input  tweak_b_v_2;"),
        text!("module_tweak.v", regex r"input  tweak_b_w_1;"),
        text!("module_tweak.v", regex r"input  \[15 : 0\] tweak_b_w_2;"),
        text!("module_tweak.v", regex r"input  \[7 : 0\] tweak_b_z_x;"),
        text!("module_tweak.v", regex r"input  \[7 : 0\] tweak_b_z_y;"),
        text!("module_tweak.v", regex r"output tweak_v_0;"),
        text!("module_tweak.v", regex r"output \[15 : 0\] tweak_w_2;"),
        text!("module_tweak.v", regex r"output \[7 : 0\] tweak_z_x;"),
        text!("sysNoinlineDeepSplit.v", regex r"\.tweak_b_z_x\("),
        text!("sysNoinlineDeepSplit.v", regex r"\.tweak_z_y\("),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_NOINLINE_TUPLE,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "NoinlineSplitTuple",
    extension: ".bs",
    fixtures: &[
        "NoinlineSplitTuple.bs",
        "sysNoinlineSplitTuple.out.expected"
    ],
    links: &[SimulationLinkInput::GeneratedModule("module_divmod")],
    compile: &[],
    icarus_assertions: &[
        text!("module_divmod.v", regex r"output \[7 : 0\] divmod_fst;"),
        text!("module_divmod.v", regex r"output divmod_snd;"),
        text!("sysNoinlineSplitTuple.v", regex r"\.divmod_fst\("),
        text!("sysNoinlineSplitTuple.v", regex r"\.divmod_snd\("),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_NOINLINE_TUPLE_3,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "NoinlineSplitTuple3",
    extension: ".bs",
    fixtures: &[
        "NoinlineSplitTuple3.bs",
        "sysNoinlineSplitTuple3.out.expected"
    ],
    links: &[SimulationLinkInput::GeneratedModule("module_triple")],
    compile: &[],
    icarus_assertions: &[
        text!("module_triple.v", regex r"output \[7 : 0\] triple_fst;"),
        text!("module_triple.v", regex r"output \[8 : 0\] triple_snd;"),
        text!(
            "sysNoinlineSplitTuple3.v",
            regex r"\.triple_fst\(triple__f1_1\)"
        ),
        text!(
            "sysNoinlineSplitTuple3.v",
            regex r"\.triple_snd\(triple__f1_2\)"
        ),
        text!(
            "sysNoinlineSplitTuple3.v",
            regex r"triple__f1_2\[8:1\]"
        ),
        text!("sysNoinlineSplitTuple3.v", regex_not r"\]\["),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_INPUT_REGS,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "SplitInputRegs",
    extension: ".bs",
    fixtures: &["SplitInputRegs.bs", "sysSplitInputRegs.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkSplitInputRegs.v", regex r"input  \[7 : 0\] put_1_x;"),
        text!("mkSplitInputRegs.v", regex r"input  \[7 : 0\] put_1_y;"),
        text!("mkSplitInputRegs.v", regex r"rx[$]D_IN = put_1_x ;"),
        text!("mkSplitInputRegs.v", regex r"ry[$]D_IN = put_1_y ;"),
        text!("mkSplitInputRegs.v", regex_not r"put_1_[xy]\["),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_ARG_SLICE,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "SplitArgSlice",
    extension: ".bs",
    fixtures: &["SplitArgSlice.bs", "sysSplitArgSlice.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkSub.v", regex r"input  \[7 : 0\] put_1_a;"),
        text!("mkSub.v", regex r"input  \[15 : 0\] put_1_inner;"),
        text!(
            "sysSplitArgSlice.v",
            regex r"s[$]put_1_a = [A-Za-z0-9_]+_1 ;"
        ),
        text!(
            "sysSplitArgSlice.v",
            regex r"s[$]put_1_inner = [A-Za-z0-9_]+_2 ;"
        ),
        text!("sysSplitArgSlice.v", regex_not r"\[[0-9]+:[0-9]+\]"),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_TUPLE_SENSITIVITY,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "SplitTupleSensitivity",
    extension: ".bs",
    fixtures: &[
        "SplitTupleSensitivity.bs",
        "sysSplitTupleSensitivity.out.expected"
    ],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!(
            "sysSplitTupleSensitivity.v",
            regex r"pick__f1_1 or pick__f1_2"
        ),
        text!(
            "sysSplitTupleSensitivity.v",
            regex_not r"or pick__f1[ )]"
        ),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_VECTOR_PORTS,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "SplitVectorPorts",
    extension: ".bs",
    fixtures: &[
        "SplitVectorPorts.bs",
        "sysSplitVectorPorts.out.expected"
    ],
    links: &[],
    compile: &[],
    icarus_assertions: &[
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putInts_1_0;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putInts_1_3;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putFoos_1_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putFoos_1_2_y;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[2 : 0\] PUT_BARS_1_0_v;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[16 : 0\] PUT_BARS_1_1_w;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[15 : 0\] PUT_BARS_1_1_z;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putPairs_1_0_a;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[15 : 0\] putPairs_1_1_b;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putGrid_1_0_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putGrid_1_1_2_y;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putTwo_foos_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putTwo_ints_1;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] putWrap_1_items_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  putWrap_1_tag;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] getInts_0;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] getInts_3;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] GET_FOOS_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] GET_FOOS_2_y;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] bumpFoos_1_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] BUMP_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] BUMP_1_y;"),
        text!("mkSplitVectorPortsTest.v", regex r"bumpFoos_1_0_x -> BUMP_0_x"),
        text!("mkSplitVectorPortsTest.v", regex r"input  \[7 : 0\] updateVec_1_0;"),
        text!("mkSplitVectorPortsTest.v", regex r"input  EN_updateVec;"),
        text!("mkSplitVectorPortsTest.v", regex r"output \[7 : 0\] updateVec_0_x;"),
        text!("mkSplitVectorPortsTest.v", regex r"output RDY_updateVec;"),
    ],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    SPLIT_VECTOR_OPS,
    origin: "bsc.verilog/splitports",
    dir: "testsuite/bsc.verilog/splitports",
    stem: "SplitVectorOps",
    extension: ".bs",
    fixtures: &["SplitVectorOps.bs", "sysSplitVectorOps.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[],
    link: &[],
    simulation: &[],
);

macro_rules! undet_scenario {
    ($constant:ident, $stem:literal) => {
        dual_scenario!(
            $constant,
            origin: "bsc.verilog/undet",
            dir: "testsuite/bsc.verilog/undet",
            stem: $stem,
            extension: ".bs",
            fixtures: &[concat!($stem, ".bs"), concat!("sys", $stem, ".out.expected")],
            links: &[],
            compile: &[],
            icarus_assertions: &[],
            link: &[],
            simulation: &[],
        );
    };
}

// Upstream bug 138 marks only the output comparisons XFAIL. Generation, linking,
// simulation, VCD validation, and artifact assertions must still succeed.
pub(super) const UNDET_1: SimulationScenario = SimulationScenario {
    name: "bsc.verilog/undet::Undet1",
    fixture_dir: "testsuite/bsc.verilog/undet",
    source: "Undet1.bs",
    fixtures: &["Undet1.bs", "sysUndet1.out.expected"],
    top: "sysUndet1",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.verilog/undet::Undet1::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::XFailOutput {
                output: "sysUndet1.out.expected",
                reason: "upstream bug 138",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.verilog/undet::Undet1::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::XFailOutput {
                output: "sysUndet1.out.expected",
                reason: "upstream bug 138",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};
undet_scenario!(UNDET_2, "Undet2");
undet_scenario!(UNDET_3, "Undet3");

icarus_scenario!(
    REAL_SIMPLE_IMPORT,
    origin: "bsc.verilog/parameters/real",
    dir: "testsuite/bsc.verilog/parameters/real",
    stem: "SimpleRealImport",
    fixtures: &[
        "SimpleRealImport.bsv",
        "DisplayReal.v",
        "sysSimpleRealImport.out.expected"
    ],
    links: &[SimulationLinkInput::ExactFile("DisplayReal.v")],
    compile: &[],
    assertions: &[],
    link: &[],
    simulation: &[],
);
icarus_scenario!(
    REAL_TWO_LEVEL,
    origin: "bsc.verilog/parameters/real",
    dir: "testsuite/bsc.verilog/parameters/real",
    stem: "TwoLevelReal",
    fixtures: &[
        "TwoLevelReal.bsv",
        "DisplayReal.v",
        "sysTwoLevelReal.out.expected"
    ],
    links: &[
        SimulationLinkInput::ExactFile("DisplayReal.v"),
        SimulationLinkInput::GeneratedModule("mkRealPassThrough"),
    ],
    compile: &[],
    assertions: &[text!("mkRealPassThrough.v", contains "r = 0.0;")],
    link: &[],
    simulation: &[],
);
dual_scenario!(
    REAL_TWO_LEVEL_BSV,
    origin: "bsc.verilog/parameters/real",
    dir: "testsuite/bsc.verilog/parameters/real",
    stem: "TwoLevelReal2",
    extension: ".bsv",
    fixtures: &["TwoLevelReal2.bsv", "sysTwoLevelReal2.out.expected"],
    links: &[
        SimulationLinkInput::GeneratedModule("bsvDisplayReal"),
        SimulationLinkInput::GeneratedModule("mkRealPassThrough2"),
    ],
    compile: &[],
    icarus_assertions: &[],
    link: &[],
    simulation: &[],
);

dual_scenario!(
    POSITIVE_RESET_TEST_3,
    origin: "bsc.verilog/positivereset/nameclash",
    dir: "testsuite/bsc.verilog/positivereset/nameclash",
    stem: "Test3",
    extension: ".bsv",
    fixtures: &["Test3.bsv", "sysTest3.out.expected"],
    links: &[],
    compile: &["-reset-prefix", "RST_P"],
    icarus_assertions: &[text!("sysTest3.v", lines "input  RST_P", 1)],
    link: &["-reset-prefix", "RST_P"],
    simulation: &[],
);
dual_scenario!(
    POSITIVE_RESET_TEST_4,
    origin: "bsc.verilog/positivereset/nameclash",
    dir: "testsuite/bsc.verilog/positivereset/nameclash",
    stem: "Test4",
    extension: ".bsv",
    fixtures: &["Test4.bsv", "sysTest4.out.expected"],
    links: &[],
    compile: &["-reset-prefix", "RST_P"],
    icarus_assertions: &[],
    link: &[
        "-reset-prefix",
        "RESET_P",
        "-D",
        "BSV_POSITIVE_RESET",
    ],
    simulation: &[],
);

dual_scenario!(
    VECTOR_ORDER,
    origin: "bsc.codegen/vector_modargs",
    dir: "testsuite/bsc.codegen/vector_modargs",
    stem: "VecVecVecInt_Order",
    extension: ".bsv",
    fixtures: &[
        "VecVecVecInt_Order.bsv",
        "sysVecVecVecInt_Order.out.expected"
    ],
    links: &[],
    compile: &[],
    icarus_assertions: &[],
    link: &[],
    simulation: &[],
);
dual_scenario!(
    VECTOR_CLOCK_RESET_INTERFACE,
    origin: "bsc.codegen/vector_modargs",
    dir: "testsuite/bsc.codegen/vector_modargs",
    stem: "VecClockResetToRegIfc",
    extension: ".bsv",
    fixtures: &[
        "VecClockResetToRegIfc.bsv",
        "sysVecClockResetToRegIfc.out.expected"
    ],
    links: &[],
    compile: &[],
    icarus_assertions: &[],
    link: &[],
    simulation: &[],
);
icarus_scenario!(
    VECTOR_CLOCKED_BY_PORT,
    origin: "bsc.codegen/vector_modargs",
    dir: "testsuite/bsc.codegen/vector_modargs",
    stem: "ClockedByPort",
    fixtures: &["ClockedByPort.bsv", "sysClockedByPort.out.expected"],
    links: &[],
    compile: &[],
    assertions: &[],
    link: &[],
    simulation: &[],
);
dual_scenario!(
    VECTOR_CLOCKED_BY_RESET,
    origin: "bsc.codegen/vector_modargs",
    dir: "testsuite/bsc.codegen/vector_modargs",
    stem: "ClockedByReset",
    extension: ".bsv",
    fixtures: &["ClockedByReset.bsv", "sysClockedByReset.out.expected"],
    links: &[],
    compile: &[],
    icarus_assertions: &[],
    link: &[],
    simulation: &[],
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    SPLIT_SHALLOW,
    SPLIT_DEEP,
    SPLIT_INSTANCE,
    SPLIT_SOME_ARG_NAMES,
    SPLIT_NOINLINE,
    SPLIT_NOINLINE_MULTI,
    SPLIT_NOINLINE_DEEP,
    SPLIT_NOINLINE_TUPLE,
    SPLIT_NOINLINE_TUPLE_3,
    SPLIT_INPUT_REGS,
    SPLIT_ARG_SLICE,
    SPLIT_TUPLE_SENSITIVITY,
    SPLIT_VECTOR_PORTS,
    SPLIT_VECTOR_OPS,
    UNDET_1,
    UNDET_2,
    UNDET_3,
    REAL_SIMPLE_IMPORT,
    REAL_TWO_LEVEL,
    REAL_TWO_LEVEL_BSV,
    POSITIVE_RESET_TEST_3,
    POSITIVE_RESET_TEST_4,
    VECTOR_ORDER,
    VECTOR_CLOCK_RESET_INTERFACE,
    VECTOR_CLOCKED_BY_PORT,
    VECTOR_CLOCKED_BY_RESET,
];
