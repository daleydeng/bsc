//! Origin: `testsuite/bsc.mcd/DisabledClocks/disabled_clocks.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, DiagnosticKind,
    Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.mcd/DisabledClocks";

macro_rules! verilog_pass_with_golden {
    ($constant:ident, $source:literal, $module:literal) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.mcd/DisabledClocks::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, "AlwaysWrite.bsv", concat!($module, ".v.expected")],
            assertions: &[ArtifactAssertion::Matches {
                actual: concat!($module, ".v"),
                expected: concat!($module, ".v.expected"),
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

verilog_pass_with_golden!(DEFAULT_VALUE_1, "DefaultValue1.bsv", "mkDefaultValue1");
verilog_pass_with_golden!(
    DEFAULT_VALUE_2_OK_1,
    "DefaultValue2OK1.bsv",
    "mkDefaultValue2OK1"
);
verilog_pass_with_golden!(
    DEFAULT_VALUE_2_OK_2,
    "DefaultValue2OK2.bsv",
    "mkDefaultValue2OK2"
);

pub(super) const DEFAULT_VALUE_2_BROKEN: CompileCase = CompileCase {
    name: "bsc.mcd/DisabledClocks::DefaultValue2Broken.bsv",
    fixture_dir: FIXTURE_DIR,
    source: "DefaultValue2Broken.bsv",
    fixtures: &["DefaultValue2Broken.bsv", "AlwaysWrite.bsv"],
    assertions: &[],
    expectation: CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0007",
        count: 1,
    },
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[
    DEFAULT_VALUE_1,
    DEFAULT_VALUE_2_OK_1,
    DEFAULT_VALUE_2_OK_2,
    DEFAULT_VALUE_2_BROKEN,
];
