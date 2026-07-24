//! Origins:
//! - `testsuite/bsc.verilog/schedule/schedule.exp`
//! - `testsuite/bsc.verilog/tasks/real/real_tasks.exp`
//! - `testsuite/bsc.verilog/tasks/time/time.exp`
//! - `testsuite/bsc.typechecker/display/display.exp`
//! - `testsuite/bsc.syntax/bsv05/stmt/stmt.exp`
//! - `testsuite/bsc.misc/bitextract/bitextract.exp`
//! - `testsuite/bsc.misc/format/format.exp`
//! - `testsuite/bsc.mcd/ClockMux/clockmux.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
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
                    name: concat!($prefix, "::", $module, "::icarus"),
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
        }
    };
}

macro_rules! backend_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal, $backend_name:literal, $backend:ident, $vcd:expr, $requirement:ident) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::", $backend_name),
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

macro_rules! bluesim_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "bluesim",
            Bluesim,
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        )
    };
}

macro_rules! icarus_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "icarus",
            Icarus,
            Some(VcdContract::parse()),
            VerilogEnabled
        )
    };
}

const SCHEDULE_DIR: &str = "testsuite/bsc.verilog/schedule";
const REAL_TASKS_DIR: &str = "testsuite/bsc.verilog/tasks/real";
const TIME_TASKS_DIR: &str = "testsuite/bsc.verilog/tasks/time";
const DISPLAY_DIR: &str = "testsuite/bsc.typechecker/display";
const STMT_DIR: &str = "testsuite/bsc.syntax/bsv05/stmt";
const BITEXTRACT_DIR: &str = "testsuite/bsc.misc/bitextract";
const FORMAT_DIR: &str = "testsuite/bsc.misc/format";
const CLOCK_MUX_DIR: &str = "testsuite/bsc.mcd/ClockMux";

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    // testsuite/bsc.verilog/schedule/schedule.exp
    shared_scenario!(
        "bsc.verilog/schedule",
        SCHEDULE_DIR,
        "EspositoPreempt",
        "sysEspositoPreempt.out.expected"
    ),
    // testsuite/bsc.verilog/tasks/real/real_tasks.exp
    shared_scenario!(
        "bsc.verilog/tasks/real",
        REAL_TASKS_DIR,
        "RealDisplay",
        "sysRealDisplay.out.expected"
    ),
    shared_scenario!(
        "bsc.verilog/tasks/real",
        REAL_TASKS_DIR,
        "RealDisplay2",
        "sysRealDisplay2.out.expected"
    ),
    bluesim_scenario!(
        "bsc.verilog/tasks/real",
        REAL_TASKS_DIR,
        "RealDisplayErr1",
        "sysRealDisplayErr1.out.expected"
    ),
    bluesim_scenario!(
        "bsc.verilog/tasks/real",
        REAL_TASKS_DIR,
        "RealDisplayErr2",
        "sysRealDisplayErr2.out.expected"
    ),
    // testsuite/bsc.verilog/tasks/time/time.exp
    icarus_scenario!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "PrintTime",
        "sysPrintTime.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "PrintTime",
        "sysPrintTime.out.expected"
    ),
    icarus_scenario!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "DisplayTime",
        "sysDisplayTime.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "DisplayTime",
        "sysDisplayTime.out.expected"
    ),
    icarus_scenario!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "RegTime",
        "sysRegTime.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "RegTime",
        "sysRegTime.out.expected"
    ),
    // testsuite/bsc.typechecker/display/display.exp
    shared_scenario!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplayBits",
        "sysDisplayBits.out.expected"
    ),
    shared_scenario!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplayLiteral",
        "sysDisplayLiteral.out.expected"
    ),
    shared_scenario!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplaySizedLiteral",
        "sysDisplaySizedLiteral.out.expected"
    ),
    icarus_scenario!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplayRealLiteral",
        "sysDisplayRealLiteral.out.expected"
    ),
    bluesim_scenario!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplayRealLiteral",
        "sysDisplayRealLiteral.c.out.expected"
    ),
    // testsuite/bsc.syntax/bsv05/stmt/stmt.exp
    shared_scenario!(
        "bsc.syntax/bsv05/stmt",
        STMT_DIR,
        "StmtFor",
        "sysStmtFor.out.expected"
    ),
    shared_scenario!(
        "bsc.syntax/bsv05/stmt",
        STMT_DIR,
        "StmtFor_ArrayUpd_Reg",
        "sysStmtFor_ArrayUpd_Reg.out.expected"
    ),
    shared_scenario!(
        "bsc.syntax/bsv05/stmt",
        STMT_DIR,
        "StmtFor_ArrayUpd_Elem",
        "sysStmtFor_ArrayUpd_Elem.out.expected"
    ),
    shared_scenario!(
        "bsc.syntax/bsv05/stmt",
        STMT_DIR,
        "StmtFor_RangeUpd",
        "sysStmtFor_RangeUpd.out.expected"
    ),
    shared_scenario!(
        "bsc.syntax/bsv05/stmt",
        STMT_DIR,
        "StmtFor_FieldUpd_Field",
        "sysStmtFor_FieldUpd_Field.out.expected"
    ),
    shared_scenario!(
        "bsc.syntax/bsv05/stmt",
        STMT_DIR,
        "StmtFor_FieldUpd_Reg",
        "sysStmtFor_FieldUpd_Reg.out.expected"
    ),
    // testsuite/bsc.misc/bitextract/bitextract.exp
    shared_scenario!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitExtractInRange",
        "sysBitExtractInRange.out.expected"
    ),
    shared_scenario!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitUpdateInRange",
        "sysBitUpdateInRange.out.expected"
    ),
    // testsuite/bsc.misc/format/format.exp
    shared_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format1",
        "sysFormat1.out.expected"
    ),
    shared_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format2",
        "sysFormat2.out.expected"
    ),
    shared_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format3",
        "sysFormat3.out.expected"
    ),
    bluesim_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format4",
        "sysFormat4.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format4",
        "sysFormat4.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format5",
        "sysFormat5.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format5",
        "sysFormat5.v.out.expected"
    ),
    shared_scenario!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Bug1572",
        "sysBug1572.out.expected"
    ),
    // testsuite/bsc.mcd/ClockMux/clockmux.exp
    bluesim_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "ClockMux",
        "sysClockMux.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "ClockMux",
        "sysClockMux.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "ClockSelect",
        "sysClockSelect.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "ClockSelect",
        "sysClockSelect.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "UngatedClockMux",
        "sysUngatedClockMux.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "UngatedClockMux",
        "sysUngatedClockMux.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "UngatedClockSelect",
        "sysUngatedClockSelect.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "UngatedClockSelect",
        "sysUngatedClockSelect.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "SlowSelectClock",
        "sysSlowSelectClock.out.expected"
    ),
    icarus_scenario!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "SlowSelectClock",
        "sysSlowSelectClock.v.out.expected"
    ),
];
