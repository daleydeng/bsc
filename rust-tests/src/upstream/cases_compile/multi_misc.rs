//! Origin: `testsuite/bsc.lib/Stmt/Server/Server.exp`.

use super::CompileCase;

pub(super) const SEQUENCE_BIND_FAIL: CompileCase = compile_fail_error_case!(
    "bsc.lib/Stmt/Server::SequenceBind_Fail.bsv",
    "testsuite/bsc.lib/Stmt/Server",
    "SequenceBind_Fail.bsv",
    "P0220"
);

pub(super) const SEQUENCE_UPDATE_BIND_FAIL: CompileCase = compile_fail_error_case!(
    "bsc.lib/Stmt/Server::SequenceUpdateBind_Fail.bsv",
    "testsuite/bsc.lib/Stmt/Server",
    "SequenceUpdateBind_Fail.bsv",
    "P0220"
);

pub(super) const CASES: &[CompileCase] = &[SEQUENCE_BIND_FAIL, SEQUENCE_UPDATE_BIND_FAIL];
