//! Origins:
//! - `testsuite/bsc.evaluator/literal/literal.exp`
//! - `testsuite/bsc.evaluator/prims/module_fix/module_fix.exp`
//! - `testsuite/bsc.interra/libraries/PopCount/PopCount.exp`
//! - `testsuite/bsc.misc/mul/mul.exp`
//! - `testsuite/bsc.lib/PAClib/dft64/bsv/paclib_dft.exp`

use super::SimulationScenario;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, ExpectedOutcome, GenerationStrategy,
    OutputNormalization, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    SimulationLinkInput, SimulationTimeouts, TextAssertion, VcdContract,
};

macro_rules! line_count {
    ($path:literal, $text:literal, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $text,
                count: $count,
            },
        }
    };
}

macro_rules! dual_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $top:expr, $fixtures:expr, $compile_options:expr, $link_inputs:expr, $expected:literal, $bluesim_assertions:expr, $icarus_assertions:expr) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
                    assertions: $bluesim_assertions,
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::icarus"),
                    assertions: $icarus_assertions,
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        }
    };
}

macro_rules! simple_dual_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        dual_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            concat!("sys", $module),
            &[concat!($module, ".bsv"), $expected],
            &[],
            &[],
            $expected,
            &[],
            &[]
        )
    };
}

macro_rules! bluesim_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $top:expr, $fixtures:expr, $compile_options:expr, $link_inputs:expr, $expected:literal, $normalization:expr, $assertions:expr) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module, "::bluesim-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: $compile_options,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::Bluesim),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::bluesim"),
                assertions: $assertions,
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: $normalization,
                backend: SimulationBackend::Bluesim,
                vcd: Some(VcdContract::output_matches_normal()),
                requirement: Requirement::BluesimEnabled,
            }],
        }
    };
}

const EVALUATOR_LITERAL_DIR: &str = "testsuite/bsc.evaluator/literal";
const MODULE_FIX_DIR: &str = "testsuite/bsc.evaluator/prims/module_fix";
const POPCOUNT_DIR: &str = "testsuite/bsc.interra/libraries/PopCount";
const MUL_DIR: &str = "testsuite/bsc.misc/mul";
const DFT_DIR: &str = "testsuite/bsc.lib/PAClib/dft64/bsv";

const NEGATIVE_INT_OK: SimulationScenario = simple_dual_scenario!(
    "bsc.evaluator/literal",
    EVALUATOR_LITERAL_DIR,
    "NegativeIntOK",
    "sysNegativeIntOK.out.expected"
);

const POSITIVE_INT_OK: SimulationScenario = simple_dual_scenario!(
    "bsc.evaluator/literal",
    EVALUATOR_LITERAL_DIR,
    "PositiveIntOK",
    "sysPositiveIntOK.out.expected"
);

const INC_DEC_FIX: SimulationScenario = simple_dual_scenario!(
    "bsc.evaluator/prims/module_fix",
    MODULE_FIX_DIR,
    "IncDecFix",
    "sysIncDecFix.out.expected"
);

const INC_DEC_FIX_MC: SimulationScenario = dual_scenario!(
    "bsc.evaluator/prims/module_fix",
    MODULE_FIX_DIR,
    "IncDecFixMC",
    "sysIncDecFixMC",
    &["IncDecFixMC.bsv", "sysIncDecFixMC.out.expected"],
    &[],
    &[],
    "sysIncDecFixMC.out.expected",
    &[],
    &[
        line_count!("IncDecFixMC.bsv.bsc-out", "Compilation message", 3),
        line_count!("IncDecFixMC.bsv.bsc-out", ": 3", 1),
        line_count!("IncDecFixMC.bsv.bsc-out", ": 5", 1),
        line_count!("IncDecFixMC.bsv.bsc-out", ": 7", 1),
    ]
);

const INC_DEC_FIX_CONTEXT: SimulationScenario = dual_scenario!(
    "bsc.evaluator/prims/module_fix",
    MODULE_FIX_DIR,
    "IncDecFixContext",
    "sysIncDecFixContext",
    &["IncDecFixContext.bsv", "sysIncDecFixContext.out.expected"],
    &[],
    &[],
    "sysIncDecFixContext.out.expected",
    &[],
    &[
        line_count!("IncDecFixContext.bsv.bsc-out", "Compilation message", 1),
        line_count!("IncDecFixContext.bsv.bsc-out", ": 6", 1),
    ]
);

macro_rules! popcount_scenario {
    ($module:literal, $top:literal, $expected:literal, $icarus_assertions:expr) => {
        dual_scenario!(
            "bsc.interra/libraries/PopCount",
            POPCOUNT_DIR,
            $module,
            $top,
            &[concat!($module, ".bsv"), $expected],
            &[],
            &[],
            $expected,
            &[],
            $icarus_assertions
        )
    };
}

const POPCOUNT_NAIVE: SimulationScenario = popcount_scenario!(
    "PopCountNaive",
    "mkTestbench_PopCountNaive",
    "mkTestbench_PopCountNaive.v.out.expected",
    &[]
);

const POPCOUNT_TABLE: SimulationScenario = popcount_scenario!(
    "PopCountTable",
    "mkTestbench_PopCountTable",
    "mkTestbench_PopCountTable.v.out.expected",
    &[ArtifactAssertion::Text {
        path: "mkTestbench_PopCountTable.v",
        assertion: TextAssertion::Regex {
            pattern: concat!(
                "      8'd3,\n",
                "      8'd5,\n",
                "      8'd6,\n",
                "      8'd9,\n",
                "      8'd10,\n",
                "      8'd12,\n",
                "      8'd17,\n",
                "      8'd18,\n",
                "      8'd20,\n",
                "      8'd24,\n",
                "      8'd33,\n",
                "      8'd34,\n",
                "      8'd36,\n",
                "      8'd40,\n",
                "      8'd48,\n",
                "      8'd65,\n",
                "      8'd66,\n",
                "      8'd68,\n",
                "      8'd72,\n",
                "      8'd80,\n",
                "      8'd96,\n",
                "      8'd129,\n",
                "      8'd130,\n",
                "      8'd132,\n",
                "      8'd136,\n",
                "      8'd144,\n",
                "      8'd160,\n",
                "      8'd192:"
            ),
        },
    }]
);

const POPCOUNT_TABLE_TREE: SimulationScenario = popcount_scenario!(
    "PopCountTableTree",
    "mkTestbench_PopCountTableTree",
    "mkTestbench_PopCountTableTree.v.out.expected",
    &[]
);

const POPCOUNT_TABLE_WALLACE: SimulationScenario = popcount_scenario!(
    "PopCountTableWallace",
    "mkTestbench_PopCountTableWallace",
    "mkTestbench_PopCountTableWallace.v.out.expected",
    &[]
);

const POPCOUNT_TREE: SimulationScenario = popcount_scenario!(
    "PopCountTree",
    "mkTestbench_PopCountTree",
    "mkTestbench_PopCountTree.v.out.expected",
    &[]
);

const POPCOUNT_WALLACE: SimulationScenario = popcount_scenario!(
    "PopCountWallace",
    "mkTestbench_PopCountWallace",
    "mkTestbench_PopCountWallace.v.out.expected",
    &[]
);

const SIGNED_MUL: SimulationScenario = simple_dual_scenario!(
    "bsc.misc/mul",
    MUL_DIR,
    "SignedMul",
    "sysSignedMul.out.expected"
);

const COMPLEX_2: SimulationScenario = simple_dual_scenario!(
    "bsc.misc/mul",
    MUL_DIR,
    "Complex2",
    "sysComplex2.out.expected"
);

const MUL_TEST: SimulationScenario = dual_scenario!(
    "bsc.misc/mul",
    MUL_DIR,
    "Test",
    "sysTest",
    &["Test.bsv", "sysTest.out.expected"],
    &[],
    &[],
    "sysTest.out.expected",
    &[],
    &[line_count!("sysTest.v", "*", 1)]
);

macro_rules! dft_scenario {
    ($module:literal, $version:literal, $expected_output:literal, $actual_data:literal, $expected_data:literal) => {
        bluesim_scenario!(
            "bsc.lib/PAClib/dft64/bsv",
            DFT_DIR,
            $module,
            concat!("sys", $module),
            &[
                concat!($module, ".bsv"),
                concat!("DFT_", $version, ".bsv"),
                "DFT.bsv",
                "DFTCoef.bsv",
                "FixedPointIO.bsv",
                "FixedPointIO.c",
                "Utils.bsv",
                "Test.dat",
                $expected_output,
                $expected_data,
            ],
            &["-elab"],
            &[SimulationLinkInput::ExactFile("FixedPointIO.c")],
            $expected_output,
            // FixedPointIO and BSV write through separate streams; their merged status-line order
            // is platform-dependent. The numerical DFT output remains a strict artifact golden.
            OutputNormalization::SortedLines,
            &[ArtifactAssertion::Matches {
                actual: $actual_data,
                expected: $expected_data,
                normalization: ArtifactNormalization::DecimalTolerance {
                    fractional_digits: 6,
                    max_units: 1,
                },
            }]
        )
    };
}

const DFT_V1: SimulationScenario = dft_scenario!(
    "Tb_v1",
    "v1",
    "sysTb_v1.out.expected",
    "Test_out_v1.dat.out",
    "Test_out_v1.dat.out.expected"
);

const DFT_V2: SimulationScenario = dft_scenario!(
    "Tb_v2",
    "v2",
    "sysTb_v2.out.expected",
    "Test_out_v2.dat.out",
    "Test_out_v2.dat.out.expected"
);

const DFT_V5: SimulationScenario = dft_scenario!(
    "Tb_v5",
    "v5",
    "sysTb_v5.out.expected",
    "Test_out_v5.dat.out",
    "Test_out_v5.dat.out.expected"
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    NEGATIVE_INT_OK,
    POSITIVE_INT_OK,
    INC_DEC_FIX,
    INC_DEC_FIX_MC,
    INC_DEC_FIX_CONTEXT,
    POPCOUNT_NAIVE,
    POPCOUNT_TABLE,
    POPCOUNT_TABLE_TREE,
    POPCOUNT_TABLE_WALLACE,
    POPCOUNT_TREE,
    POPCOUNT_WALLACE,
    SIGNED_MUL,
    COMPLEX_2,
    MUL_TEST,
    DFT_V1,
    DFT_V2,
    DFT_V5,
];
