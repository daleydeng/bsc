//! Origins:
//! - `testsuite/bsc.interra/MCD_library/AsyncRAM/asyncRAM.exp`
//! - `testsuite/bsc.interra/MCD_library/BitSync/bitsync.exp`
//! - `testsuite/bsc.interra/MCD_library/BitSync1/bitsync1.exp`
//! - `testsuite/bsc.interra/MCD_library/FIFOSync/SyncFIFO.exp`
//! - `testsuite/bsc.interra/MCD_library/PulseHandShakeSync/PulseHandShake.exp`
//! - `testsuite/bsc.interra/MCD_library/RegSync/SyncReg.exp`
//! - `testsuite/bsc.interra/MCD_library/SpecialSyncFIFO/SpecialSyncFIFO.exp`
//! - `testsuite/bsc.interra/MCD_library/SpecialSyncReg/SpecialSyncReg.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

macro_rules! backend_scenario {
    (
        $prefix:literal,
        $fixture_dir:expr,
        $topbsv:literal,
        $topmod:literal,
        [$($fixture:literal),* $(,)?],
        [$($module:literal),* $(,)?],
        $expected:literal,
        $backend_name:literal,
        $backend:ident,
        $vcd:expr,
        $requirement:ident
    ) => {
        SimulationScenario {
            name: concat!(
                $prefix,
                "::",
                $topbsv,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: $fixture_dir,
            source: concat!($topbsv, ".bsv"),
            fixtures: &[
                concat!($topbsv, ".bsv"),
                $($fixture,)*
                $expected,
            ],
            top: $topmod,
            link_inputs: &[$(SimulationLinkInput::GeneratedModule($module),)*],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $topbsv, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        }
    };
}

macro_rules! mcd_scenario_pair {
    (
        $icarus_constant:ident,
        $bluesim_constant:ident,
        $prefix:literal,
        $fixture_dir:expr,
        $topbsv:literal,
        $topmod:literal,
        fixtures: [$($fixture:literal),* $(,)?],
        modules: [$($module:literal),* $(,)?],
        icarus_expected: $icarus_expected:literal,
        bluesim_expected: $bluesim_expected:literal $(,)?
    ) => {
        pub(super) const $icarus_constant: SimulationScenario = backend_scenario!(
            $prefix,
            $fixture_dir,
            $topbsv,
            $topmod,
            [$($fixture),*],
            [$($module),*],
            $icarus_expected,
            "icarus",
            Icarus,
            Some(VcdContract::parse()),
            VerilogEnabled
        );
        pub(super) const $bluesim_constant: SimulationScenario = backend_scenario!(
            $prefix,
            $fixture_dir,
            $topbsv,
            $topmod,
            [$($fixture),*],
            [$($module),*],
            $bluesim_expected,
            "bluesim",
            Bluesim,
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        );
    };
}

const ASYNC_RAM_DIR: &str = "testsuite/bsc.interra/MCD_library/AsyncRAM";
const BIT_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/BitSync";
const BIT_SYNC_1_DIR: &str = "testsuite/bsc.interra/MCD_library/BitSync1";
const FIFO_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/FIFOSync";
const PULSE_HANDSHAKE_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/PulseHandShakeSync";
const REG_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/RegSync";
const SPECIAL_SYNC_FIFO_DIR: &str = "testsuite/bsc.interra/MCD_library/SpecialSyncFIFO";
const SPECIAL_SYNC_REG_DIR: &str = "testsuite/bsc.interra/MCD_library/SpecialSyncReg";

mcd_scenario_pair!(
    ASYNC_RAM_WRITE_FAST_READ_SLOW_ICARUS,
    ASYNC_RAM_WRITE_FAST_READ_SLOW_BLUESIM,
    "bsc.interra/MCD_library/AsyncRAM",
    ASYNC_RAM_DIR,
    "Testbench_write_fast_read_slow",
    "mkTestbench_write_fast_read_slow",
    fixtures: ["Design.bsv"],
    modules: [],
    icarus_expected: "mkTestbench_write_fast_read_slow.out.expected",
    bluesim_expected: "mkTestbench_write_fast_read_slow.c.out.expected",
);
mcd_scenario_pair!(
    ASYNC_RAM_WRITE_SLOW_READ_FAST_ICARUS,
    ASYNC_RAM_WRITE_SLOW_READ_FAST_BLUESIM,
    "bsc.interra/MCD_library/AsyncRAM",
    ASYNC_RAM_DIR,
    "Testbench_write_slow_read_fast",
    "mkTestbench_write_slow_read_fast",
    fixtures: ["Design.bsv"],
    modules: [],
    icarus_expected: "mkTestbench_write_slow_read_fast.out.expected",
    bluesim_expected: "mkTestbench_write_slow_read_fast.c.out.expected",
);
mcd_scenario_pair!(
    ASYNC_RAM_SAME_WITH_PHASE_DIFF_ICARUS,
    ASYNC_RAM_SAME_WITH_PHASE_DIFF_BLUESIM,
    "bsc.interra/MCD_library/AsyncRAM",
    ASYNC_RAM_DIR,
    "Testbench_same_with_phase_diff",
    "mkTestbench_same_with_phase_diff",
    fixtures: ["Design.bsv"],
    modules: [],
    icarus_expected: "mkTestbench_same_with_phase_diff.out.expected",
    bluesim_expected: "mkTestbench_same_with_phase_diff.c.out.expected",
);

mcd_scenario_pair!(
    BIT_SYNC_FAST_TO_SLOW_ICARUS,
    BIT_SYNC_FAST_TO_SLOW_BLUESIM,
    "bsc.interra/MCD_library/BitSync",
    BIT_SYNC_DIR,
    "Testbench_fast_to_slow",
    "mkTestbench_fast_to_slow",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_fast_to_slow.out.expected",
    bluesim_expected: "mkTestbench_fast_to_slow.c.out.expected",
);
mcd_scenario_pair!(
    BIT_SYNC_SLOW_TO_FAST_ICARUS,
    BIT_SYNC_SLOW_TO_FAST_BLUESIM,
    "bsc.interra/MCD_library/BitSync",
    BIT_SYNC_DIR,
    "Testbench_slow_to_fast",
    "mkTestbench_slow_to_fast",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_slow_to_fast.out.expected",
    bluesim_expected: "mkTestbench_slow_to_fast.c.out.expected",
);
mcd_scenario_pair!(
    BIT_SYNC_SAME_WITH_PHASE_DIFF_ICARUS,
    BIT_SYNC_SAME_WITH_PHASE_DIFF_BLUESIM,
    "bsc.interra/MCD_library/BitSync",
    BIT_SYNC_DIR,
    "Testbench_same_with_phase_diff",
    "mkTestbench_same_with_phase_diff",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_same_with_phase_diff.out.expected",
    bluesim_expected: "mkTestbench_same_with_phase_diff.c.out.expected",
);

mcd_scenario_pair!(
    BIT_SYNC_1_FAST_TO_SLOW_ICARUS,
    BIT_SYNC_1_FAST_TO_SLOW_BLUESIM,
    "bsc.interra/MCD_library/BitSync1",
    BIT_SYNC_1_DIR,
    "Testbench_fast_to_slow",
    "mkTestbench_fast_to_slow",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_fast_to_slow.out.expected",
    bluesim_expected: "mkTestbench_fast_to_slow.c.out.expected",
);
mcd_scenario_pair!(
    BIT_SYNC_1_SLOW_TO_FAST_ICARUS,
    BIT_SYNC_1_SLOW_TO_FAST_BLUESIM,
    "bsc.interra/MCD_library/BitSync1",
    BIT_SYNC_1_DIR,
    "Testbench_slow_to_fast",
    "mkTestbench_slow_to_fast",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_slow_to_fast.out.expected",
    bluesim_expected: "mkTestbench_slow_to_fast.c.out.expected",
);
mcd_scenario_pair!(
    BIT_SYNC_1_SAME_WITH_PHASE_DIFF_ICARUS,
    BIT_SYNC_1_SAME_WITH_PHASE_DIFF_BLUESIM,
    "bsc.interra/MCD_library/BitSync1",
    BIT_SYNC_1_DIR,
    "Testbench_same_with_phase_diff",
    "mkTestbench_same_with_phase_diff",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_same_with_phase_diff.out.expected",
    bluesim_expected: "mkTestbench_same_with_phase_diff.c.out.expected",
);

mcd_scenario_pair!(
    FIFO_SYNC_WRITE_FAST_READ_SLOW_ICARUS,
    FIFO_SYNC_WRITE_FAST_READ_SLOW_BLUESIM,
    "bsc.interra/MCD_library/FIFOSync",
    FIFO_SYNC_DIR,
    "Testbench_write_fast_read_slow",
    "mkTestbench_write_fast_read_slow",
    fixtures: ["Design.bsv", "Disp.bsv"],
    modules: [],
    icarus_expected: "mkTestbench_write_fast_read_slow.out.expected",
    bluesim_expected: "mkTestbench_write_fast_read_slow.out.expected",
);
mcd_scenario_pair!(
    FIFO_SYNC_WRITE_SLOW_READ_FAST_ICARUS,
    FIFO_SYNC_WRITE_SLOW_READ_FAST_BLUESIM,
    "bsc.interra/MCD_library/FIFOSync",
    FIFO_SYNC_DIR,
    "Testbench_write_slow_read_fast",
    "mkTestbench_write_slow_read_fast",
    fixtures: ["Design.bsv", "Disp.bsv"],
    modules: [],
    icarus_expected: "mkTestbench_write_slow_read_fast.out.expected",
    bluesim_expected: "mkTestbench_write_slow_read_fast.out.expected",
);
mcd_scenario_pair!(
    FIFO_SYNC_SAME_WITH_PHASE_DIFF_ICARUS,
    FIFO_SYNC_SAME_WITH_PHASE_DIFF_BLUESIM,
    "bsc.interra/MCD_library/FIFOSync",
    FIFO_SYNC_DIR,
    "Testbench_same_with_phase_diff",
    "mkTestbench_same_with_phase_diff",
    fixtures: ["Design.bsv", "Disp.bsv"],
    modules: [],
    icarus_expected: "mkTestbench_same_with_phase_diff.out.expected",
    bluesim_expected: "mkTestbench_same_with_phase_diff.out.expected",
);

mcd_scenario_pair!(
    PULSE_HANDSHAKE_FAST_TO_SLOW_ICARUS,
    PULSE_HANDSHAKE_FAST_TO_SLOW_BLUESIM,
    "bsc.interra/MCD_library/PulseHandShakeSync",
    PULSE_HANDSHAKE_SYNC_DIR,
    "Testbench_fast_to_slow",
    "mkTestbench_fast_to_slow",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_fast_to_slow.out.expected",
    bluesim_expected: "mkTestbench_fast_to_slow.c.out.expected",
);
mcd_scenario_pair!(
    PULSE_HANDSHAKE_SLOW_TO_FAST_ICARUS,
    PULSE_HANDSHAKE_SLOW_TO_FAST_BLUESIM,
    "bsc.interra/MCD_library/PulseHandShakeSync",
    PULSE_HANDSHAKE_SYNC_DIR,
    "Testbench_slow_to_fast",
    "mkTestbench_slow_to_fast",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_slow_to_fast.out.expected",
    bluesim_expected: "mkTestbench_slow_to_fast.c.out.expected",
);
mcd_scenario_pair!(
    PULSE_HANDSHAKE_SAME_WITH_PHASE_DIFF_ICARUS,
    PULSE_HANDSHAKE_SAME_WITH_PHASE_DIFF_BLUESIM,
    "bsc.interra/MCD_library/PulseHandShakeSync",
    PULSE_HANDSHAKE_SYNC_DIR,
    "Testbench_same_with_phase_diff",
    "mkTestbench_same_with_phase_diff",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_same_with_phase_diff.out.expected",
    bluesim_expected: "mkTestbench_same_with_phase_diff.c.out.expected",
);

mcd_scenario_pair!(
    REG_SYNC_FAST_TO_SLOW_ICARUS,
    REG_SYNC_FAST_TO_SLOW_BLUESIM,
    "bsc.interra/MCD_library/RegSync",
    REG_SYNC_DIR,
    "Testbench_fast_to_slow",
    "mkTestbench_fast_to_slow",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_fast_to_slow.out.expected",
    bluesim_expected: "mkTestbench_fast_to_slow.c.out.expected",
);
mcd_scenario_pair!(
    REG_SYNC_SLOW_TO_FAST_ICARUS,
    REG_SYNC_SLOW_TO_FAST_BLUESIM,
    "bsc.interra/MCD_library/RegSync",
    REG_SYNC_DIR,
    "Testbench_slow_to_fast",
    "mkTestbench_slow_to_fast",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_slow_to_fast.out.expected",
    bluesim_expected: "mkTestbench_slow_to_fast.c.out.expected",
);
mcd_scenario_pair!(
    REG_SYNC_SAME_WITH_PHASE_DIFF_ICARUS,
    REG_SYNC_SAME_WITH_PHASE_DIFF_BLUESIM,
    "bsc.interra/MCD_library/RegSync",
    REG_SYNC_DIR,
    "Testbench_same_with_phase_diff",
    "mkTestbench_same_with_phase_diff",
    fixtures: ["Design.bsv"],
    modules: ["mkDesign"],
    icarus_expected: "mkTestbench_same_with_phase_diff.out.expected",
    bluesim_expected: "mkTestbench_same_with_phase_diff.c.out.expected",
);

mcd_scenario_pair!(
    SPECIAL_SYNC_FIFO_WRITE_SLOW_READ_FAST_ICARUS,
    SPECIAL_SYNC_FIFO_WRITE_SLOW_READ_FAST_BLUESIM,
    "bsc.interra/MCD_library/SpecialSyncFIFO",
    SPECIAL_SYNC_FIFO_DIR,
    "Testbench_write_slow_read_fast",
    "mkTestbench_write_slow_read_fast",
    fixtures: ["SyncFIFOSlow.bsv"],
    modules: ["mkSyncFIFOSlow"],
    icarus_expected: "mkTestbench_write_slow_read_fast.out.expected",
    bluesim_expected: "mkTestbench_write_slow_read_fast.out.expected",
);

mcd_scenario_pair!(
    SPECIAL_SYNC_REG_FAST_TO_SLOW_ICARUS,
    SPECIAL_SYNC_REG_FAST_TO_SLOW_BLUESIM,
    "bsc.interra/MCD_library/SpecialSyncReg",
    SPECIAL_SYNC_REG_DIR,
    "Testbench_fast_to_slow",
    "mkTestbench_fast_to_slow",
    fixtures: ["SyncRegToSlow.bsv"],
    modules: ["mkSyncRegSlow"],
    icarus_expected: "mkTestbench_fast_to_slow.out.expected",
    bluesim_expected: "mkTestbench_fast_to_slow.out.expected",
);
mcd_scenario_pair!(
    SPECIAL_SYNC_REG_SLOW_TO_FAST_ICARUS,
    SPECIAL_SYNC_REG_SLOW_TO_FAST_BLUESIM,
    "bsc.interra/MCD_library/SpecialSyncReg",
    SPECIAL_SYNC_REG_DIR,
    "Testbench_slow_to_fast",
    "mkTestbench_slow_to_fast",
    fixtures: ["SyncRegToFast.bsv"],
    modules: ["mkSyncRegFast"],
    icarus_expected: "mkTestbench_slow_to_fast.out.expected",
    bluesim_expected: "mkTestbench_slow_to_fast.out.expected",
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    ASYNC_RAM_WRITE_FAST_READ_SLOW_ICARUS,
    ASYNC_RAM_WRITE_FAST_READ_SLOW_BLUESIM,
    ASYNC_RAM_WRITE_SLOW_READ_FAST_ICARUS,
    ASYNC_RAM_WRITE_SLOW_READ_FAST_BLUESIM,
    ASYNC_RAM_SAME_WITH_PHASE_DIFF_ICARUS,
    ASYNC_RAM_SAME_WITH_PHASE_DIFF_BLUESIM,
    BIT_SYNC_FAST_TO_SLOW_ICARUS,
    BIT_SYNC_FAST_TO_SLOW_BLUESIM,
    BIT_SYNC_SLOW_TO_FAST_ICARUS,
    BIT_SYNC_SLOW_TO_FAST_BLUESIM,
    BIT_SYNC_SAME_WITH_PHASE_DIFF_ICARUS,
    BIT_SYNC_SAME_WITH_PHASE_DIFF_BLUESIM,
    BIT_SYNC_1_FAST_TO_SLOW_ICARUS,
    BIT_SYNC_1_FAST_TO_SLOW_BLUESIM,
    BIT_SYNC_1_SLOW_TO_FAST_ICARUS,
    BIT_SYNC_1_SLOW_TO_FAST_BLUESIM,
    BIT_SYNC_1_SAME_WITH_PHASE_DIFF_ICARUS,
    BIT_SYNC_1_SAME_WITH_PHASE_DIFF_BLUESIM,
    FIFO_SYNC_WRITE_FAST_READ_SLOW_ICARUS,
    FIFO_SYNC_WRITE_FAST_READ_SLOW_BLUESIM,
    FIFO_SYNC_WRITE_SLOW_READ_FAST_ICARUS,
    FIFO_SYNC_WRITE_SLOW_READ_FAST_BLUESIM,
    FIFO_SYNC_SAME_WITH_PHASE_DIFF_ICARUS,
    FIFO_SYNC_SAME_WITH_PHASE_DIFF_BLUESIM,
    PULSE_HANDSHAKE_FAST_TO_SLOW_ICARUS,
    PULSE_HANDSHAKE_FAST_TO_SLOW_BLUESIM,
    PULSE_HANDSHAKE_SLOW_TO_FAST_ICARUS,
    PULSE_HANDSHAKE_SLOW_TO_FAST_BLUESIM,
    PULSE_HANDSHAKE_SAME_WITH_PHASE_DIFF_ICARUS,
    PULSE_HANDSHAKE_SAME_WITH_PHASE_DIFF_BLUESIM,
    REG_SYNC_FAST_TO_SLOW_ICARUS,
    REG_SYNC_FAST_TO_SLOW_BLUESIM,
    REG_SYNC_SLOW_TO_FAST_ICARUS,
    REG_SYNC_SLOW_TO_FAST_BLUESIM,
    REG_SYNC_SAME_WITH_PHASE_DIFF_ICARUS,
    REG_SYNC_SAME_WITH_PHASE_DIFF_BLUESIM,
    SPECIAL_SYNC_FIFO_WRITE_SLOW_READ_FAST_ICARUS,
    SPECIAL_SYNC_FIFO_WRITE_SLOW_READ_FAST_BLUESIM,
    SPECIAL_SYNC_REG_FAST_TO_SLOW_ICARUS,
    SPECIAL_SYNC_REG_FAST_TO_SLOW_BLUESIM,
    SPECIAL_SYNC_REG_SLOW_TO_FAST_ICARUS,
    SPECIAL_SYNC_REG_SLOW_TO_FAST_BLUESIM,
];
