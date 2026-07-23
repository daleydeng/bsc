//! Origin: `testsuite/bsc.doc/doc.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.doc";

macro_rules! documented_verilog_case {
    ($constant:ident, $source:literal, $actual:literal, $expected:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.doc::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, $expected],
            assertions: &[ArtifactAssertion::Matches {
                actual: $actual,
                expected: $expected,
                normalization: ArtifactNormalization::Verilog,
            }],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

documented_verilog_case!(
    USER_GUIDE_GCD,
    "UserGuide_GCD.bsv",
    "mkGCD.v",
    "UserGuide_mkGCD.v.expected"
);
documented_verilog_case!(
    USER_GUIDE_REG_INSTS,
    "UserGuide_RegInsts.bsv",
    "sysUserGuide_RegInsts.v",
    "sysUserGuide_RegInsts.v.expected"
);

pub(super) const CASES: &[CompileCase] = &[USER_GUIDE_GCD, USER_GUIDE_REG_INSTS];
