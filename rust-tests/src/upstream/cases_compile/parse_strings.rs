//! Origin: `testsuite/bsc.syntax/bsv05/strings/parse_strings.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bsv05/strings";

macro_rules! parse_error {
    ($source:literal, $tag:literal) => {
        compile_fail_error_case!(
            concat!("bsc.syntax/bsv05/strings::", $source),
            FIXTURE_DIR,
            $source,
            $tag
        )
    };
}

pub(super) const CASES: &[CompileCase] = &[
    parse_error!("UnterminatedString.bsv", "P0092"),
    parse_error!("BadStringEscape.bsv", "P0091"),
    parse_error!("BadOctalEscape.bsv", "P0091"),
    CompileCase {
        name: "bsc.syntax/bsv05/strings::OctalChars.bsv::verilog",
        fixture_dir: FIXTURE_DIR,
        source: "OctalChars.bsv",
        fixtures: &["OctalChars.bsv", "sysOctalChars.v.expected"],
        assertions: &[ArtifactAssertion::Matches {
            actual: "sysOctalChars.v",
            expected: "sysOctalChars.v.expected",
            normalization: ArtifactNormalization::Verilog,
        }],
        expectation: CompileExpectation::Pass,
        golden: None,
        options: &[],
        nodeps: false,
        mode: CompileMode::Verilog { module: None },
        requirement: Requirement::VerilogEnabled,
    },
    compile_pass_case!(
        "bsc.syntax/bsv05/strings::StringLit.bsv",
        FIXTURE_DIR,
        "StringLit.bsv"
    ),
];
