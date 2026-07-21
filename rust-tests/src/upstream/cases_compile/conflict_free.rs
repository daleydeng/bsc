//! Origin: `testsuite/bsc.scheduler/conflict_free/conflict_free.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.scheduler/conflict_free";

pub(super) const NOT_RESOURCE: CompileCase = compile_verilog_fail_error_case!(
    "bsc.scheduler/conflict_free::ConflictFreeNotResource.bsv",
    FIXTURE_DIR,
    "ConflictFreeNotResource.bsv",
    "G0002"
);

pub(super) const SINGLETON_WARNING: CompileCase = compile_verilog_pass_warning_case!(
    "bsc.scheduler/conflict_free::CFSingleton.bsv",
    FIXTURE_DIR,
    "CFSingleton.bsv",
    "G0010"
);
