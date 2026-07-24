//! Origins:
//! - `testsuite/bsc.interra/MCD_library/BitSync/bitsync.exp`
//! - `testsuite/bsc.interra/MCD_library/BitSync1/bitsync1.exp`
//! - `testsuite/bsc.interra/MCD_library/PulseHandShakeSync/PulseHandShake.exp`
//! - `testsuite/bsc.interra/MCD_library/RegSync/SyncReg.exp`
//! - `testsuite/bsc.interra/MCD_library/SpecialSyncReg/SpecialSyncReg.exp`

use crate::upstream::{CompileCase, CompileExpectation, CompileMode, Requirement};

macro_rules! verilog_fail_case {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $module:literal) => {
        CompileCase {
            name: concat!($prefix, "::", $source, "::verilog"),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Fail,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog {
                module: Some($module),
            },
            requirement: Requirement::VerilogEnabled,
        }
    };
}

const BIT_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/BitSync";
const BIT_SYNC_1_DIR: &str = "testsuite/bsc.interra/MCD_library/BitSync1";
const PULSE_HANDSHAKE_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/PulseHandShakeSync";
const REG_SYNC_DIR: &str = "testsuite/bsc.interra/MCD_library/RegSync";
const SPECIAL_SYNC_REG_DIR: &str = "testsuite/bsc.interra/MCD_library/SpecialSyncReg";

pub(super) const CASES: &[CompileCase] = &[
    verilog_fail_case!(
        "bsc.interra/MCD_library/BitSync",
        BIT_SYNC_DIR,
        "Negative_testcase.bsv",
        "mkDesign"
    ),
    verilog_fail_case!(
        "bsc.interra/MCD_library/BitSync1",
        BIT_SYNC_1_DIR,
        "Negative_testcase.bsv",
        "mkDesign"
    ),
    verilog_fail_case!(
        "bsc.interra/MCD_library/PulseHandShakeSync",
        PULSE_HANDSHAKE_SYNC_DIR,
        "Negative_testcase.bsv",
        "mkDesign"
    ),
    verilog_fail_case!(
        "bsc.interra/MCD_library/RegSync",
        REG_SYNC_DIR,
        "Negative_testcase.bsv",
        "mkDesign"
    ),
    verilog_fail_case!(
        "bsc.interra/MCD_library/SpecialSyncReg",
        SPECIAL_SYNC_REG_DIR,
        "Negative.bsv",
        "mkDesign"
    ),
];
