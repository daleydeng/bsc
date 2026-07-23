//! Origins:
//! - `testsuite/bsc.names/portRenaming/conflicts/miscellaneous/conflicts.exp`
//! - `testsuite/bsc.bsv_examples/fsm/fsm.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1490/b1490.exp`
//! - `testsuite/bsc.names/portRenaming/conflicts/readyResult/readyResult.exp`

use crate::upstream::{CompileCase, CompileExpectation, CompileMode, DiagnosticKind, Requirement};

macro_rules! frontend_error {
    ($name:expr, $fixture_dir:expr, $source:expr, $tag:expr, $count:expr, $options:expr) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            golden: None,
            options: $options,
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        }
    };
}

macro_rules! frontend_pass {
    ($name:expr, $fixture_dir:expr, $source:expr) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Frontend,
            requirement: Requirement::Always,
        }
    };
}

macro_rules! verilog_pass {
    ($name:expr, $fixture_dir:expr, $source:expr, $module:expr, $options:expr) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: $options,
            nodeps: false,
            mode: CompileMode::Verilog { module: $module },
            requirement: Requirement::VerilogEnabled,
        }
    };
}

const MISCELLANEOUS_DIR: &str = "testsuite/bsc.names/portRenaming/conflicts/miscellaneous";
const FSM_DIR: &str = "testsuite/bsc.bsv_examples/fsm";
const B1490_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b1490";
const READY_RESULT_DIR: &str = "testsuite/bsc.names/portRenaming/conflicts/readyResult";

const VERILOG_OPTION: &[&str] = &["-verilog"];
const B1490_RTS_OPTIONS: &[&str] = &["+RTS", "-M288M", "-Sstderr", "-RTS"];

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.names/portRenaming/conflicts/miscellaneous/conflicts.exp
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test01.bsv",
        MISCELLANEOUS_DIR,
        "Test01.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test02.bsv",
        MISCELLANEOUS_DIR,
        "Test02.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test03.bsv",
        MISCELLANEOUS_DIR,
        "Test03.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test04.bsv",
        MISCELLANEOUS_DIR,
        "Test04.bsv",
        "P0086",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test05.bsv",
        MISCELLANEOUS_DIR,
        "Test05.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test06.bsv",
        MISCELLANEOUS_DIR,
        "Test06.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test07.bsv",
        MISCELLANEOUS_DIR,
        "Test07.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test08.bsv",
        MISCELLANEOUS_DIR,
        "Test08.bsv",
        "G0055",
        7,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test09.bsv",
        MISCELLANEOUS_DIR,
        "Test09.bsv",
        "G0055",
        2,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test10.bsv",
        MISCELLANEOUS_DIR,
        "Test10.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test11.bsv",
        MISCELLANEOUS_DIR,
        "Test11.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test12.bsv",
        MISCELLANEOUS_DIR,
        "Test12.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test13.bsv",
        MISCELLANEOUS_DIR,
        "Test13.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test14.bsv",
        MISCELLANEOUS_DIR,
        "Test14.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test15.bsv",
        MISCELLANEOUS_DIR,
        "Test15.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test16.bsv",
        MISCELLANEOUS_DIR,
        "Test16.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test17.bsv",
        MISCELLANEOUS_DIR,
        "Test17.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test18.bsv",
        MISCELLANEOUS_DIR,
        "Test18.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test19.bsv",
        MISCELLANEOUS_DIR,
        "Test19.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test20.bsv",
        MISCELLANEOUS_DIR,
        "Test20.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/miscellaneous::Test21.bsv",
        MISCELLANEOUS_DIR,
        "Test21.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    // testsuite/bsc.bsv_examples/fsm/fsm.exp
    frontend_pass!(
        "bsc.bsv_examples/fsm::FSM1.bsv::frontend",
        FSM_DIR,
        "FSM1.bsv"
    ),
    verilog_pass!(
        "bsc.bsv_examples/fsm::FSM1.bsv::verilog",
        FSM_DIR,
        "FSM1.bsv",
        Some("mkFSM"),
        &[]
    ),
    frontend_pass!(
        "bsc.bsv_examples/fsm::FSM2.bsv::frontend",
        FSM_DIR,
        "FSM2.bsv"
    ),
    verilog_pass!(
        "bsc.bsv_examples/fsm::FSM2.bsv::verilog",
        FSM_DIR,
        "FSM2.bsv",
        Some("mkFSM"),
        &[]
    ),
    frontend_pass!(
        "bsc.bsv_examples/fsm::FSM3.bsv::frontend",
        FSM_DIR,
        "FSM3.bsv"
    ),
    verilog_pass!(
        "bsc.bsv_examples/fsm::FSM3.bsv::verilog",
        FSM_DIR,
        "FSM3.bsv",
        Some("mkFSM"),
        &[]
    ),
    frontend_pass!(
        "bsc.bsv_examples/fsm::FSM4.bsv::frontend",
        FSM_DIR,
        "FSM4.bsv"
    ),
    verilog_pass!(
        "bsc.bsv_examples/fsm::FSM4.bsv::verilog",
        FSM_DIR,
        "FSM4.bsv",
        Some("mkFSM"),
        &[]
    ),
    frontend_error!(
        "bsc.bsv_examples/fsm::FSMbug1.bsv",
        FSM_DIR,
        "FSMbug1.bsv",
        "P0005",
        1,
        &[]
    ),
    // testsuite/bsc.bugs/bluespec_inc/b1490/b1490.exp
    verilog_pass!(
        "bsc.bugs/bluespec_inc/b1490::Bug1490Bool.bsv",
        B1490_DIR,
        "Bug1490Bool.bsv",
        None,
        B1490_RTS_OPTIONS
    ),
    verilog_pass!(
        "bsc.bugs/bluespec_inc/b1490::Bug1490MyBool.bsv",
        B1490_DIR,
        "Bug1490MyBool.bsv",
        None,
        B1490_RTS_OPTIONS
    ),
    verilog_pass!(
        "bsc.bugs/bluespec_inc/b1490::Bug1490MyUnion.bsv",
        B1490_DIR,
        "Bug1490MyUnion.bsv",
        None,
        B1490_RTS_OPTIONS
    ),
    verilog_pass!(
        "bsc.bugs/bluespec_inc/b1490::Bug1490MyEnum.bsv",
        B1490_DIR,
        "Bug1490MyEnum.bsv",
        None,
        B1490_RTS_OPTIONS
    ),
    verilog_pass!(
        "bsc.bugs/bluespec_inc/b1490::VsortOriginal.bsv",
        B1490_DIR,
        "VsortOriginal.bsv",
        None,
        B1490_RTS_OPTIONS
    ),
    verilog_pass!(
        "bsc.bugs/bluespec_inc/b1490::VsortWorkaround.bsv",
        B1490_DIR,
        "VsortWorkaround.bsv",
        None,
        B1490_RTS_OPTIONS
    ),
    // testsuite/bsc.names/portRenaming/conflicts/readyResult/readyResult.exp
    frontend_error!(
        "bsc.names/portRenaming/conflicts/readyResult::Test01.bsv::call-1",
        READY_RESULT_DIR,
        "Test01.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/readyResult::Test01.bsv::call-2",
        READY_RESULT_DIR,
        "Test01.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
    frontend_error!(
        "bsc.names/portRenaming/conflicts/readyResult::Test03.bsv",
        READY_RESULT_DIR,
        "Test03.bsv",
        "G0055",
        1,
        VERILOG_OPTION
    ),
];
