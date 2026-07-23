//! Origin: `testsuite/bsc.typechecker/generics/generics.exp`.

use super::CompileCase;

pub(super) const GENERIC_NEGATIVE_TESTS: CompileCase = compile_fail_golden_case!(
    "bsc.typechecker/generics::GenericNegativeTests.bs",
    "testsuite/bsc.typechecker/generics",
    "GenericNegativeTests.bs",
    "GenericNegativeTests.bs.bsc-out.expected"
);

pub(super) const CASES: &[CompileCase] = &[GENERIC_NEGATIVE_TESTS];
