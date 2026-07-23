//! Origins:
//! - `testsuite/bsc.bsv_examples/AES/aes.exp`
//! - `testsuite/bsc.bsv_examples/FP/FP.exp`
//! - `testsuite/bsc.bsv_examples/GlibcRandom/GlibcRandom.exp`
//! - `testsuite/bsc.bsv_examples/mimo/mimo.exp`
//! - `testsuite/bsc.verilog/positivereset/SyncReset/SyncReset.exp`
//! - `testsuite/bsc.real/evaluator/undef/undef.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
};

macro_rules! shared_scenario {
    (
        $constant:ident,
        $prefix:literal,
        $fixture_dir:expr,
        $module:literal,
        $expected:expr,
        $fixtures:expr,
        $compile_options:expr,
        $generated_modules:expr,
        $timeout:expr,
        $resource:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            generated_modules: $generated_modules,
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeout: $timeout,
            resource: $resource,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
                    expected: $expected,
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::icarus"),
                    expected: $expected,
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Icarus,
                    vcd: VcdExpectation::IcarusSmoke,
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
        $module:literal,
        $expected:expr,
        $fixtures:expr,
        $compile_options:expr,
        $generated_modules:expr,
        $link_options:expr,
        $backend_name:literal,
        $backend:ident,
        $vcd:ident,
        $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($prefix, "::", $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            generated_modules: $generated_modules,
            compile_options: $compile_options,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::", $backend_name),
                expected: $expected,
                link_options: $link_options,
                simulation_options: &[],
                sort_output: false,
                backend: SimulationBackend::$backend,
                vcd: VcdExpectation::$vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

const AES_DIR: &str = "testsuite/bsc.bsv_examples/AES";
const AES_EXPECTED: &str = "sysAes_TB.out.expected";
const AES_FIXTURES: &[&str] = &[
    "Aes_TB.bsv",
    "Aes.bsv",
    "Defines.bsv",
    "InvSboxComb.bsv",
    "ProbeWire.bsv",
    "RconComb.bsv",
    "SboxComb.bsv",
    AES_EXPECTED,
    "dat.vectors",
    "key128.vectors",
    "key192.vectors",
    "key256.vectors",
];
const AES_GENERATED_MODULES: &[&str] = &["mkRconRom", "mkSboxRom", "mkInvSboxRom", "mkAes"];
shared_scenario!(
    AES,
    "bsc.bsv_examples/AES",
    AES_DIR,
    "Aes_TB",
    AES_EXPECTED,
    AES_FIXTURES,
    &["-steps", "500000"],
    AES_GENERATED_MODULES,
    crate::BSC_HEAVY_TIMEOUT,
    ResourceClass::Heavy
);

const FP_DIR: &str = "testsuite/bsc.bsv_examples/FP";
shared_scenario!(
    FP_BASIC,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "Basic",
    "sysBasic.expected",
    &["Basic.bsv", "FloatingPoint.bsv", "sysBasic.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    FP_ARITH,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "Arith",
    "sysArith.expected",
    &["Arith.bsv", "FloatingPoint.bsv", "sysArith.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    FP_SYNTH,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "Synth",
    "sysSynth.expected",
    &["Synth.bsv", "FloatingPoint.bsv", "sysSynth.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    FP_ARITH_PIPE,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "ArithPipe",
    "sysArithPipe.expected",
    &[
        "ArithPipe.bsv",
        "FloatingPoint.bsv",
        "sysArithPipe.expected"
    ],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    FP_PIPE_MULT,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "PipeMult",
    "sysPipeMult.expected",
    &["PipeMult.bsv", "FloatingPoint.bsv", "sysPipeMult.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);

const GLIBC_RANDOM_DIR: &str = "testsuite/bsc.bsv_examples/GlibcRandom";
shared_scenario!(
    GLIBC_RANDOM_FAST,
    "bsc.bsv_examples/GlibcRandom",
    GLIBC_RANDOM_DIR,
    "tbFast",
    "systbFast.out.expected",
    &["tbFast.bsv", "GlibcRandom.bsv", "systbFast.out.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    GLIBC_RANDOM_SLOW,
    "bsc.bsv_examples/GlibcRandom",
    GLIBC_RANDOM_DIR,
    "tbSlow",
    "systbSlow.out.expected",
    &["tbSlow.bsv", "GlibcRandom.bsv", "systbSlow.out.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);

const MIMO_DIR: &str = "testsuite/bsc.bsv_examples/mimo";
shared_scenario!(
    MIMO_BASIC,
    "bsc.bsv_examples/mimo",
    MIMO_DIR,
    "Basic",
    "sysBasic.expected",
    &["Basic.bsv", "sysBasic.expected"],
    &[],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    MIMO_TRAFFIC_REG,
    "bsc.bsv_examples/mimo",
    MIMO_DIR,
    "TrafficREG",
    "sysTrafficREG.out.expected",
    &["TrafficREG.bsv", "sysTrafficREG.out.expected"],
    &["-no-aggressive-conditions"],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);
shared_scenario!(
    MIMO_TRAFFIC_BRAM,
    "bsc.bsv_examples/mimo",
    MIMO_DIR,
    "TrafficBRAM",
    "sysTrafficBRAM.out.expected",
    &["TrafficBRAM.bsv", "sysTrafficBRAM.out.expected"],
    &["-no-aggressive-conditions"],
    &[],
    crate::BSC_TIMEOUT,
    ResourceClass::Normal
);

const POSITIVE_RESET_DIR: &str = "testsuite/bsc.verilog/positivereset/SyncReset";
const POSITIVE_RESET_OPTIONS: &[&str] = &["-reset-prefix", "RESET_P", "-D", "BSV_POSITIVE_RESET"];
macro_rules! positive_reset_pair {
    ($bluesim:ident, $icarus:ident, $module:literal, $bluesim_expected:literal, $icarus_expected:literal) => {
        backend_scenario!(
            $bluesim,
            "bsc.verilog/positivereset/SyncReset",
            POSITIVE_RESET_DIR,
            $module,
            $bluesim_expected,
            &[concat!($module, ".bsv"), $bluesim_expected],
            POSITIVE_RESET_OPTIONS,
            &[],
            POSITIVE_RESET_OPTIONS,
            "bluesim",
            Bluesim,
            BluesimOutputMatchesNormal,
            BluesimEnabled
        );
        backend_scenario!(
            $icarus,
            "bsc.verilog/positivereset/SyncReset",
            POSITIVE_RESET_DIR,
            $module,
            $icarus_expected,
            &[concat!($module, ".bsv"), $icarus_expected],
            POSITIVE_RESET_OPTIONS,
            &[],
            POSITIVE_RESET_OPTIONS,
            "icarus",
            Icarus,
            IcarusSmoke,
            VerilogEnabled
        );
    };
}

positive_reset_pair!(
    POSITIVE_RESET_BLUESIM,
    POSITIVE_RESET_ICARUS,
    "RstTest",
    "sysRstTest.out.expected",
    "sysRstTest.v.out.expected"
);
positive_reset_pair!(
    POSITIVE_RESET_V1_BLUESIM,
    POSITIVE_RESET_V1_ICARUS,
    "RstTest_V1",
    "sysRstTest_V1.out.expected",
    "sysRstTest_V1.v.out.expected"
);
positive_reset_pair!(
    POSITIVE_RESET_V2_BLUESIM,
    POSITIVE_RESET_V2_ICARUS,
    "RstTest_V2",
    "sysRstTest_V2.out.expected",
    "sysRstTest_V2.v.out.expected"
);

backend_scenario!(
    UNDEF_DYNAMIC_SELECT_ICARUS,
    "bsc.real/evaluator/undef",
    "testsuite/bsc.real/evaluator/undef",
    "DontCareDynSelectStaticArrayReal",
    "sysDontCareDynSelectStaticArrayReal.out.expected",
    &[
        "DontCareDynSelectStaticArrayReal.bsv",
        "sysDontCareDynSelectStaticArrayReal.out.expected"
    ],
    &[],
    &[],
    &[],
    "icarus",
    Icarus,
    IcarusSmoke,
    VerilogEnabled
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    AES,
    FP_BASIC,
    FP_ARITH,
    FP_SYNTH,
    FP_ARITH_PIPE,
    FP_PIPE_MULT,
    GLIBC_RANDOM_FAST,
    GLIBC_RANDOM_SLOW,
    MIMO_BASIC,
    MIMO_TRAFFIC_REG,
    MIMO_TRAFFIC_BRAM,
    POSITIVE_RESET_BLUESIM,
    POSITIVE_RESET_ICARUS,
    POSITIVE_RESET_V1_BLUESIM,
    POSITIVE_RESET_V1_ICARUS,
    POSITIVE_RESET_V2_BLUESIM,
    POSITIVE_RESET_V2_ICARUS,
    UNDEF_DYNAMIC_SELECT_ICARUS,
];
