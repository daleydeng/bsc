//! Origin: `testsuite/bsc.scheduler/use_cond/use_cond.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.scheduler/use_cond";

macro_rules! use_cond_case {
    ($constant:ident, $source:literal, $module:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.scheduler/use_cond::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, concat!($module, ".v.expected")],
            assertions: &[ArtifactAssertion::Matches {
                actual: concat!($module, ".v"),
                expected: concat!($module, ".v.expected"),
                normalization: ArtifactNormalization::Verilog,
            }],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog {
                module: Some($module),
            },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

use_cond_case!(TRUE_TRUE, "UseCondTrueTrue.bsv", "sysUseCondTrueTrue");
use_cond_case!(TRUE_FALSE, "UseCondTrueFalse.bsv", "sysUseCondTrueFalse");
use_cond_case!(FALSE_FALSE, "UseCondFalseFalse.bsv", "sysUseCondFalseFalse");
use_cond_case!(EQ_VAR_1, "UseCondEqVar1.bsv", "sysUseCondEqVar1");
use_cond_case!(EQ_VARS_1, "UseCondEqVars1.bsv", "sysUseCondEqVars1");
use_cond_case!(EQ_VARS_2, "UseCondEqVars2.bsv", "sysUseCondEqVars2");
use_cond_case!(NEQ_VAR_1, "UseCondNEqVar1.bsv", "sysUseCondNEqVar1");
use_cond_case!(NEQ_VAR_1B, "UseCondNEqVar1b.bsv", "sysUseCondNEqVar1b");
use_cond_case!(NEQ_VAR_2, "UseCondNEqVar2.bsv", "sysUseCondNEqVar2");
use_cond_case!(NEQ_VAR_3, "UseCondNEqVar3.bsv", "sysUseCondNEqVar3");
use_cond_case!(NEQ_VAR_4, "UseCondNEqVar4.bsv", "sysUseCondNEqVar4");
use_cond_case!(NEQ_VARS_1, "UseCondNEqVars1.bsv", "sysUseCondNEqVars1");
use_cond_case!(NEQ_VARS_2, "UseCondNEqVars2.bsv", "sysUseCondNEqVars2");
use_cond_case!(NEQ_VARS_3, "UseCondNEqVars3.bsv", "sysUseCondNEqVars3");
use_cond_case!(NEQ_VARS_4, "UseCondNEqVars4.bsv", "sysUseCondNEqVars4");
use_cond_case!(
    TRUE_FALSE_CROSS_1,
    "UseCondTrueFalseCross1.bsv",
    "sysUseCondTrueFalseCross1"
);
use_cond_case!(
    TRUE_FALSE_CROSS_2,
    "UseCondTrueFalseCross2.bsv",
    "sysUseCondTrueFalseCross2"
);
use_cond_case!(EQ_CROSS, "UseCondEqCross.bsv", "sysUseCondEqCross");
use_cond_case!(
    EQ_NEQ_CROSS_1,
    "UseCondEqNEqCross1.bsv",
    "sysUseCondEqNEqCross1"
);
use_cond_case!(
    EQ_NEQ_CROSS_2,
    "UseCondEqNEqCross2.bsv",
    "sysUseCondEqNEqCross2"
);

pub(super) const CASES: &[CompileCase] = &[
    TRUE_TRUE,
    TRUE_FALSE,
    FALSE_FALSE,
    EQ_VAR_1,
    EQ_VARS_1,
    EQ_VARS_2,
    NEQ_VAR_1,
    NEQ_VAR_1B,
    NEQ_VAR_2,
    NEQ_VAR_3,
    NEQ_VAR_4,
    NEQ_VARS_1,
    NEQ_VARS_2,
    NEQ_VARS_3,
    NEQ_VARS_4,
    TRUE_FALSE_CROSS_1,
    TRUE_FALSE_CROSS_2,
    EQ_CROSS,
    EQ_NEQ_CROSS_1,
    EQ_NEQ_CROSS_2,
];
