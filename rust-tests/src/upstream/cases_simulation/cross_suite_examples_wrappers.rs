//! Origins:
//! - `testsuite/bsc.bsv_examples/AES/aes.exp`
//! - `testsuite/bsc.bsv_examples/FP/FP.exp`
//! - `testsuite/bsc.bsv_examples/GlibcRandom/GlibcRandom.exp`
//! - `testsuite/bsc.bsv_examples/mimo/mimo.exp`
//! - `testsuite/bsc.verilog/positivereset/SyncReset/SyncReset.exp`
//! - `testsuite/bsc.real/evaluator/undef/undef.exp`

use super::SimulationCase;
use crate::upstream::{GenerationStrategy, Requirement, ResourceClass, SimulationBackend};

macro_rules! simulation_case {
    ($constant:ident, $name:expr, $fixture_dir:expr, $module:literal, $expected:expr, $fixtures:expr, $compile_options:expr, $generated_modules:expr, $backend:expr, $requirement:expr, $heavy:expr) => {
        simulation_case!(
            $constant,
            $name,
            $fixture_dir,
            $module,
            $expected,
            $fixtures,
            $compile_options,
            $generated_modules,
            &[],
            GenerationStrategy::BackendSpecific,
            $backend,
            $requirement,
            $heavy
        );
    };
    ($constant:ident, $name:expr, $fixture_dir:expr, $module:literal, $expected:expr, $fixtures:expr, $compile_options:expr, $generated_modules:expr, $link_options:expr, $backend:expr, $requirement:expr, $heavy:expr) => {
        simulation_case!(
            $constant,
            $name,
            $fixture_dir,
            $module,
            $expected,
            $fixtures,
            $compile_options,
            $generated_modules,
            $link_options,
            GenerationStrategy::BackendSpecific,
            $backend,
            $requirement,
            $heavy
        );
    };
    ($constant:ident, $name:expr, $fixture_dir:expr, $module:literal, $expected:expr, $fixtures:expr, $compile_options:expr, $generated_modules:expr, $link_options:expr, $generation:expr, $backend:expr, $requirement:expr, $heavy:expr) => {
        pub(super) const $constant: SimulationCase = SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            generated_modules: $generated_modules,
            expected: $expected,
            compile_options: $compile_options,
            link_options: $link_options,
            simulation_options: &[],
            sort_output: false,
            backend: $backend,
            generation: $generation,
            vcd: $crate::upstream::VcdExpectation::None,
            requirement: $requirement,
            timeout: if $heavy {
                $crate::BSC_HEAVY_TIMEOUT
            } else {
                $crate::BSC_TIMEOUT
            },
            resource: if $heavy {
                ResourceClass::Heavy
            } else {
                ResourceClass::Normal
            },
        };
    };
}

macro_rules! backend_pair {
    ($bluesim:ident, $icarus:ident, $prefix:literal, $fixture_dir:expr, $module:literal, $expected:expr, $fixtures:expr, $compile_options:expr, $generated_modules:expr, $heavy:expr) => {
        simulation_case!(
            $bluesim,
            concat!($prefix, "::", $module, "::bluesim"),
            $fixture_dir,
            $module,
            $expected,
            $fixtures,
            $compile_options,
            $generated_modules,
            &[],
            GenerationStrategy::SharedElaboration,
            SimulationBackend::Bluesim,
            Requirement::BluesimEnabled,
            $heavy
        );
        simulation_case!(
            $icarus,
            concat!($prefix, "::", $module, "::icarus"),
            $fixture_dir,
            $module,
            $expected,
            $fixtures,
            $compile_options,
            $generated_modules,
            &[],
            GenerationStrategy::SharedElaboration,
            SimulationBackend::Icarus,
            Requirement::VerilogEnabled,
            $heavy
        );
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
backend_pair!(
    AES_BLUESIM,
    AES_ICARUS,
    "bsc.bsv_examples/AES",
    AES_DIR,
    "Aes_TB",
    AES_EXPECTED,
    AES_FIXTURES,
    &["-steps", "500000", "-elab"],
    AES_GENERATED_MODULES,
    true
);

const FP_DIR: &str = "testsuite/bsc.bsv_examples/FP";
backend_pair!(
    FP_BASIC_BLUESIM,
    FP_BASIC_ICARUS,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "Basic",
    "sysBasic.expected",
    &["Basic.bsv", "FloatingPoint.bsv", "sysBasic.expected"],
    &[],
    &[],
    false
);
backend_pair!(
    FP_ARITH_BLUESIM,
    FP_ARITH_ICARUS,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "Arith",
    "sysArith.expected",
    &["Arith.bsv", "FloatingPoint.bsv", "sysArith.expected"],
    &[],
    &[],
    false
);
backend_pair!(
    FP_SYNTH_BLUESIM,
    FP_SYNTH_ICARUS,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "Synth",
    "sysSynth.expected",
    &["Synth.bsv", "FloatingPoint.bsv", "sysSynth.expected"],
    &[],
    &[],
    false
);
backend_pair!(
    FP_ARITH_PIPE_BLUESIM,
    FP_ARITH_PIPE_ICARUS,
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
    false
);
backend_pair!(
    FP_PIPE_MULT_BLUESIM,
    FP_PIPE_MULT_ICARUS,
    "bsc.bsv_examples/FP",
    FP_DIR,
    "PipeMult",
    "sysPipeMult.expected",
    &["PipeMult.bsv", "FloatingPoint.bsv", "sysPipeMult.expected"],
    &[],
    &[],
    false
);

const GLIBC_RANDOM_DIR: &str = "testsuite/bsc.bsv_examples/GlibcRandom";
backend_pair!(
    GLIBC_RANDOM_FAST_BLUESIM,
    GLIBC_RANDOM_FAST_ICARUS,
    "bsc.bsv_examples/GlibcRandom",
    GLIBC_RANDOM_DIR,
    "tbFast",
    "systbFast.out.expected",
    &["tbFast.bsv", "GlibcRandom.bsv", "systbFast.out.expected"],
    &[],
    &[],
    false
);
backend_pair!(
    GLIBC_RANDOM_SLOW_BLUESIM,
    GLIBC_RANDOM_SLOW_ICARUS,
    "bsc.bsv_examples/GlibcRandom",
    GLIBC_RANDOM_DIR,
    "tbSlow",
    "systbSlow.out.expected",
    &["tbSlow.bsv", "GlibcRandom.bsv", "systbSlow.out.expected"],
    &[],
    &[],
    false
);

const MIMO_DIR: &str = "testsuite/bsc.bsv_examples/mimo";
backend_pair!(
    MIMO_BASIC_BLUESIM,
    MIMO_BASIC_ICARUS,
    "bsc.bsv_examples/mimo",
    MIMO_DIR,
    "Basic",
    "sysBasic.expected",
    &["Basic.bsv", "sysBasic.expected"],
    &[],
    &[],
    false
);
backend_pair!(
    MIMO_TRAFFIC_REG_BLUESIM,
    MIMO_TRAFFIC_REG_ICARUS,
    "bsc.bsv_examples/mimo",
    MIMO_DIR,
    "TrafficREG",
    "sysTrafficREG.out.expected",
    &["TrafficREG.bsv", "sysTrafficREG.out.expected"],
    &["-no-aggressive-conditions"],
    &[],
    false
);
backend_pair!(
    MIMO_TRAFFIC_BRAM_BLUESIM,
    MIMO_TRAFFIC_BRAM_ICARUS,
    "bsc.bsv_examples/mimo",
    MIMO_DIR,
    "TrafficBRAM",
    "sysTrafficBRAM.out.expected",
    &["TrafficBRAM.bsv", "sysTrafficBRAM.out.expected"],
    &["-no-aggressive-conditions"],
    &[],
    false
);

const POSITIVE_RESET_DIR: &str = "testsuite/bsc.verilog/positivereset/SyncReset";
const POSITIVE_RESET_OPTIONS: &[&str] = &["-reset-prefix", "RESET_P", "-D", "BSV_POSITIVE_RESET"];
macro_rules! positive_reset_pair {
    ($bluesim:ident, $icarus:ident, $module:literal, $bluesim_expected:literal, $icarus_expected:literal) => {
        simulation_case!(
            $bluesim,
            concat!(
                "bsc.verilog/positivereset/SyncReset::",
                $module,
                "::bluesim"
            ),
            POSITIVE_RESET_DIR,
            $module,
            $bluesim_expected,
            &[concat!($module, ".bsv"), $bluesim_expected],
            POSITIVE_RESET_OPTIONS,
            &[],
            POSITIVE_RESET_OPTIONS,
            SimulationBackend::Bluesim,
            Requirement::BluesimEnabled,
            false
        );
        simulation_case!(
            $icarus,
            concat!("bsc.verilog/positivereset/SyncReset::", $module, "::icarus"),
            POSITIVE_RESET_DIR,
            $module,
            $icarus_expected,
            &[concat!($module, ".bsv"), $icarus_expected],
            POSITIVE_RESET_OPTIONS,
            &[],
            POSITIVE_RESET_OPTIONS,
            SimulationBackend::Icarus,
            Requirement::VerilogEnabled,
            false
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

simulation_case!(
    UNDEF_DYNAMIC_SELECT_ICARUS,
    "bsc.real/evaluator/undef::DontCareDynSelectStaticArrayReal::icarus",
    "testsuite/bsc.real/evaluator/undef",
    "DontCareDynSelectStaticArrayReal",
    "sysDontCareDynSelectStaticArrayReal.out.expected",
    &[
        "DontCareDynSelectStaticArrayReal.bsv",
        "sysDontCareDynSelectStaticArrayReal.out.expected"
    ],
    &[],
    &[],
    SimulationBackend::Icarus,
    Requirement::VerilogEnabled,
    false
);

pub(super) const CASES: &[SimulationCase] = &[
    AES_BLUESIM,
    AES_ICARUS,
    FP_BASIC_BLUESIM,
    FP_BASIC_ICARUS,
    FP_ARITH_BLUESIM,
    FP_ARITH_ICARUS,
    FP_SYNTH_BLUESIM,
    FP_SYNTH_ICARUS,
    FP_ARITH_PIPE_BLUESIM,
    FP_ARITH_PIPE_ICARUS,
    FP_PIPE_MULT_BLUESIM,
    FP_PIPE_MULT_ICARUS,
    GLIBC_RANDOM_FAST_BLUESIM,
    GLIBC_RANDOM_FAST_ICARUS,
    GLIBC_RANDOM_SLOW_BLUESIM,
    GLIBC_RANDOM_SLOW_ICARUS,
    MIMO_BASIC_BLUESIM,
    MIMO_BASIC_ICARUS,
    MIMO_TRAFFIC_REG_BLUESIM,
    MIMO_TRAFFIC_REG_ICARUS,
    MIMO_TRAFFIC_BRAM_BLUESIM,
    MIMO_TRAFFIC_BRAM_ICARUS,
    POSITIVE_RESET_BLUESIM,
    POSITIVE_RESET_ICARUS,
    POSITIVE_RESET_V1_BLUESIM,
    POSITIVE_RESET_V1_ICARUS,
    POSITIVE_RESET_V2_BLUESIM,
    POSITIVE_RESET_V2_ICARUS,
    UNDEF_DYNAMIC_SELECT_ICARUS,
];
