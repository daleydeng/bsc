//! Origins:
//! - `testsuite/bsc.verilog/schedule/schedule.exp`
//! - `testsuite/bsc.verilog/tasks/real/real_tasks.exp`
//! - `testsuite/bsc.verilog/tasks/time/time.exp`
//! - `testsuite/bsc.typechecker/display/display.exp`
//! - `testsuite/bsc.syntax/bsv05/stmt/stmt.exp`
//! - `testsuite/bsc.misc/bitextract/bitextract.exp`
//! - `testsuite/bsc.misc/format/format.exp`
//! - `testsuite/bsc.mcd/ClockMux/clockmux.exp`

use super::SimulationCase;
use crate::upstream::{Requirement, SimulationBackend};

macro_rules! simulation_case {
    ($name:expr, $fixture_dir:expr, $module:literal, $expected:expr, $backend:expr, $requirement:expr) => {
        SimulationCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            expected: $expected,
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: $backend,
            requirement: $requirement,
            timeout: $crate::BSC_TIMEOUT,
            heavy: false,
        }
    };
}

macro_rules! bluesim {
    ($prefix:literal, $fixture_dir:expr, $module:literal) => {
        bluesim!(
            $prefix,
            $fixture_dir,
            $module,
            concat!("sys", $module, ".out.expected")
        )
    };
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:expr) => {
        simulation_case!(
            concat!($prefix, "::", $module, "::bluesim"),
            $fixture_dir,
            $module,
            $expected,
            SimulationBackend::Bluesim,
            Requirement::BluesimEnabled
        )
    };
}

macro_rules! icarus {
    ($prefix:literal, $fixture_dir:expr, $module:literal) => {
        icarus!(
            $prefix,
            $fixture_dir,
            $module,
            concat!("sys", $module, ".out.expected")
        )
    };
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:expr) => {
        simulation_case!(
            concat!($prefix, "::", $module, "::icarus"),
            $fixture_dir,
            $module,
            $expected,
            SimulationBackend::Icarus,
            Requirement::VerilogEnabled
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

pub(super) const CASES: &[SimulationCase] = &[
    // testsuite/bsc.verilog/schedule/schedule.exp
    bluesim!("bsc.verilog/schedule", SCHEDULE_DIR, "EspositoPreempt"),
    icarus!("bsc.verilog/schedule", SCHEDULE_DIR, "EspositoPreempt"),
    // testsuite/bsc.verilog/tasks/real/real_tasks.exp
    bluesim!("bsc.verilog/tasks/real", REAL_TASKS_DIR, "RealDisplay"),
    icarus!("bsc.verilog/tasks/real", REAL_TASKS_DIR, "RealDisplay"),
    bluesim!("bsc.verilog/tasks/real", REAL_TASKS_DIR, "RealDisplay2"),
    icarus!("bsc.verilog/tasks/real", REAL_TASKS_DIR, "RealDisplay2"),
    bluesim!("bsc.verilog/tasks/real", REAL_TASKS_DIR, "RealDisplayErr1"),
    bluesim!("bsc.verilog/tasks/real", REAL_TASKS_DIR, "RealDisplayErr2"),
    // testsuite/bsc.verilog/tasks/time/time.exp
    icarus!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "PrintTime",
        "sysPrintTime.v.out.expected"
    ),
    bluesim!("bsc.verilog/tasks/time", TIME_TASKS_DIR, "PrintTime"),
    icarus!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "DisplayTime",
        "sysDisplayTime.v.out.expected"
    ),
    bluesim!("bsc.verilog/tasks/time", TIME_TASKS_DIR, "DisplayTime"),
    icarus!(
        "bsc.verilog/tasks/time",
        TIME_TASKS_DIR,
        "RegTime",
        "sysRegTime.v.out.expected"
    ),
    bluesim!("bsc.verilog/tasks/time", TIME_TASKS_DIR, "RegTime"),
    // testsuite/bsc.typechecker/display/display.exp
    bluesim!("bsc.typechecker/display", DISPLAY_DIR, "DisplayBits"),
    icarus!("bsc.typechecker/display", DISPLAY_DIR, "DisplayBits"),
    bluesim!("bsc.typechecker/display", DISPLAY_DIR, "DisplayLiteral"),
    icarus!("bsc.typechecker/display", DISPLAY_DIR, "DisplayLiteral"),
    bluesim!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplaySizedLiteral"
    ),
    icarus!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplaySizedLiteral"
    ),
    icarus!("bsc.typechecker/display", DISPLAY_DIR, "DisplayRealLiteral"),
    bluesim!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplayRealLiteral",
        "sysDisplayRealLiteral.c.out.expected"
    ),
    // testsuite/bsc.syntax/bsv05/stmt/stmt.exp
    bluesim!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor"),
    icarus!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor"),
    bluesim!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_ArrayUpd_Reg"),
    icarus!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_ArrayUpd_Reg"),
    bluesim!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_ArrayUpd_Elem"),
    icarus!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_ArrayUpd_Elem"),
    bluesim!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_RangeUpd"),
    icarus!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_RangeUpd"),
    bluesim!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_FieldUpd_Field"),
    icarus!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_FieldUpd_Field"),
    bluesim!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_FieldUpd_Reg"),
    icarus!("bsc.syntax/bsv05/stmt", STMT_DIR, "StmtFor_FieldUpd_Reg"),
    // testsuite/bsc.misc/bitextract/bitextract.exp
    bluesim!("bsc.misc/bitextract", BITEXTRACT_DIR, "BitExtractInRange"),
    icarus!("bsc.misc/bitextract", BITEXTRACT_DIR, "BitExtractInRange"),
    bluesim!("bsc.misc/bitextract", BITEXTRACT_DIR, "BitUpdateInRange"),
    icarus!("bsc.misc/bitextract", BITEXTRACT_DIR, "BitUpdateInRange"),
    // testsuite/bsc.misc/format/format.exp
    bluesim!("bsc.misc/format", FORMAT_DIR, "Format1"),
    icarus!("bsc.misc/format", FORMAT_DIR, "Format1"),
    bluesim!("bsc.misc/format", FORMAT_DIR, "Format2"),
    icarus!("bsc.misc/format", FORMAT_DIR, "Format2"),
    bluesim!("bsc.misc/format", FORMAT_DIR, "Format3"),
    icarus!("bsc.misc/format", FORMAT_DIR, "Format3"),
    bluesim!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format4",
        "sysFormat4.c.out.expected"
    ),
    icarus!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format4",
        "sysFormat4.v.out.expected"
    ),
    bluesim!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format5",
        "sysFormat5.c.out.expected"
    ),
    icarus!(
        "bsc.misc/format",
        FORMAT_DIR,
        "Format5",
        "sysFormat5.v.out.expected"
    ),
    bluesim!("bsc.misc/format", FORMAT_DIR, "Bug1572"),
    icarus!("bsc.misc/format", FORMAT_DIR, "Bug1572"),
    // testsuite/bsc.mcd/ClockMux/clockmux.exp
    bluesim!("bsc.mcd/ClockMux", CLOCK_MUX_DIR, "ClockMux"),
    icarus!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "ClockMux",
        "sysClockMux.v.out.expected"
    ),
    bluesim!("bsc.mcd/ClockMux", CLOCK_MUX_DIR, "ClockSelect"),
    icarus!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "ClockSelect",
        "sysClockSelect.v.out.expected"
    ),
    bluesim!("bsc.mcd/ClockMux", CLOCK_MUX_DIR, "UngatedClockMux"),
    icarus!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "UngatedClockMux",
        "sysUngatedClockMux.v.out.expected"
    ),
    bluesim!("bsc.mcd/ClockMux", CLOCK_MUX_DIR, "UngatedClockSelect"),
    icarus!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "UngatedClockSelect",
        "sysUngatedClockSelect.v.out.expected"
    ),
    bluesim!("bsc.mcd/ClockMux", CLOCK_MUX_DIR, "SlowSelectClock"),
    icarus!(
        "bsc.mcd/ClockMux",
        CLOCK_MUX_DIR,
        "SlowSelectClock",
        "sysSlowSelectClock.v.out.expected"
    ),
];
