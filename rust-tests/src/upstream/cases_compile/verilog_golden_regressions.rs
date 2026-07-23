//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1354/b1354.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1540/b1540.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b293/b293.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b302/b302.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b569/b569.exp`
//! - `testsuite/bsc.names/portRenaming/vectorTests/vectorTests.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, Requirement,
};

pub(super) const VECTOR_TEST_01: CompileCase = CompileCase {
    name: "bsc.names/portRenaming/vectorTests::Test01.bsv",
    fixture_dir: "testsuite/bsc.names/portRenaming/vectorTests",
    source: "Test01.bsv",
    fixtures: &["Test01.bsv", "mkTest01.v.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkTest01.v",
        expected: "mkTest01.v.expected",
        normalization: ArtifactNormalization::Verilog,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const VECTOR_TEST_02: CompileCase = CompileCase {
    name: "bsc.names/portRenaming/vectorTests::Test02.bsv",
    fixture_dir: "testsuite/bsc.names/portRenaming/vectorTests",
    source: "Test02.bsv",
    fixtures: &["Test02.bsv", "mkTest02.v.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkTest02.v",
        expected: "mkTest02.v.expected",
        normalization: ArtifactNormalization::Verilog,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B1354: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1354::Test.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1354",
    source: "Test.bsv",
    fixtures: &["Test.bsv", "mkMulti.v.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkMulti.v",
        expected: "mkMulti.v.expected",
        normalization: ArtifactNormalization::Verilog,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B1540: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b1540::foo.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b1540",
    source: "foo.bsv",
    fixtures: &["foo.bsv", "mkFOO.v.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkFOO.v",
        expected: "mkFOO.v.expected",
        normalization: ArtifactNormalization::Verilog,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B293: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b293::Design1.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b293",
    source: "Design1.bsv",
    fixtures: &["Design1.bsv", "mkDesign1.v.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkDesign1.v",
        expected: "mkDesign1.v.expected",
        normalization: ArtifactNormalization::Verilog,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B302: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b302::Design.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b302",
    source: "Design.bsv",
    fixtures: &["Design.bsv", "mkDesign.v.expected"],
    assertions: &[ArtifactAssertion::Matches {
        actual: "mkDesign.v",
        expected: "mkDesign.v.expected",
        normalization: ArtifactNormalization::Verilog,
    }],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: None },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const B569: CompileCase = CompileCase {
    name: "bsc.bugs/bluespec_inc/b569::Test.bsv",
    fixture_dir: "testsuite/bsc.bugs/bluespec_inc/b569",
    source: "Test.bsv",
    fixtures: &[
        "Test.bsv",
        "ArithModules.bsv",
        "tb.v.expected",
        "mkAddSub.v.expected",
    ],
    assertions: &[
        ArtifactAssertion::Matches {
            actual: "tb.v",
            expected: "tb.v.expected",
            normalization: ArtifactNormalization::Verilog,
        },
        ArtifactAssertion::Matches {
            actual: "mkAddSub.v",
            expected: "mkAddSub.v.expected",
            normalization: ArtifactNormalization::Verilog,
        },
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    nodeps: false,
    mode: CompileMode::Verilog { module: Some("tb") },
    requirement: Requirement::VerilogEnabled,
};

pub(super) const CASES: &[CompileCase] = &[
    VECTOR_TEST_01,
    VECTOR_TEST_02,
    B1354,
    B1540,
    B293,
    B302,
    B569,
];
