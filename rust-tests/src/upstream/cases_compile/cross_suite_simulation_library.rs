//! Origins:
//! - `testsuite/bsc.mcd/SyncReset/SyncReset.exp`
//! - `testsuite/bsc.lib/BuildVector/BuildVector.exp`

use super::CompileCase;

pub(super) const RST_TEST_E1: CompileCase = compile_verilog_fail_error_case!(
    "bsc.mcd/SyncReset::RstTest_E1.bsv::verilog",
    "testsuite/bsc.mcd/SyncReset",
    "RstTest_E1.bsv",
    "G0042"
);

pub(super) const TEST_BUILD_VECTOR_FAIL: CompileCase = compile_fail_golden_case!(
    "bsc.lib/BuildVector::TestBuildVectorFail.bsv",
    "testsuite/bsc.lib/BuildVector",
    "TestBuildVectorFail.bsv",
    "TestBuildVectorFail.bsv.bsc-out.expected"
);

pub(super) const CASES: &[CompileCase] = &[RST_TEST_E1, TEST_BUILD_VECTOR_FAIL];
