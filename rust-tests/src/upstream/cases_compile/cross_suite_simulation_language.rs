//! Origins:
//! - `testsuite/bsc.verilog/schedule/schedule.exp`
//! - `testsuite/bsc.typechecker/display/display.exp`
//! - `testsuite/bsc.misc/bitextract/bitextract.exp`
//! - `testsuite/bsc.misc/format/format.exp`

use crate::upstream::{
    CompileCase, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation, Requirement,
};

macro_rules! compile_case {
    ($name:expr, $fixture_dir:expr, $source:literal, $fixtures:expr, $expectation:expr, $golden:expr, $options:expr, $mode:expr, $requirement:expr) => {
        CompileCase {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            assertions: &[],
            expectation: $expectation,
            golden: $golden,
            options: $options,
            nodeps: false,
            mode: $mode,
            requirement: $requirement,
        }
    };
}

macro_rules! frontend_pass {
    ($prefix:literal, $fixture_dir:expr, $source:literal) => {
        frontend_pass!($prefix, $fixture_dir, $source, &[])
    };
    ($prefix:literal, $fixture_dir:expr, $source:literal, $options:expr) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::Pass,
            None,
            $options,
            CompileMode::Frontend,
            Requirement::Always
        )
    };
}

macro_rules! frontend_error {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            None,
            &[],
            CompileMode::Frontend,
            Requirement::Always
        )
    };
}

macro_rules! frontend_fail_golden {
    ($prefix:literal, $fixture_dir:expr, $source:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            CompileExpectation::Fail,
            Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[],
            CompileMode::Frontend,
            Requirement::Always
        )
    };
}

macro_rules! frontend_error_golden {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source, concat!($source, ".bsc-out.expected")],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            Some(GoldenExpectation {
                expected: concat!($source, ".bsc-out.expected"),
            }),
            &[],
            CompileMode::Frontend,
            Requirement::Always
        )
    };
}

macro_rules! verilog_pass {
    ($prefix:literal, $fixture_dir:expr, $source:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::Pass,
            None,
            &[],
            CompileMode::Verilog { module: None },
            Requirement::VerilogEnabled
        )
    };
}

macro_rules! verilog_error {
    ($prefix:literal, $fixture_dir:expr, $source:literal, $tag:literal) => {
        compile_case!(
            concat!($prefix, "::", $source),
            $fixture_dir,
            $source,
            &[$source],
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            None,
            &[],
            CompileMode::Verilog { module: None },
            Requirement::VerilogEnabled
        )
    };
}

const SCHEDULE_DIR: &str = "testsuite/bsc.verilog/schedule";
const DISPLAY_DIR: &str = "testsuite/bsc.typechecker/display";
const BITEXTRACT_DIR: &str = "testsuite/bsc.misc/bitextract";
const FORMAT_DIR: &str = "testsuite/bsc.misc/format";

pub(super) const CASES: &[CompileCase] = &[
    // testsuite/bsc.verilog/schedule/schedule.exp
    verilog_error!(
        "bsc.verilog/schedule",
        SCHEDULE_DIR,
        "MethodNeverEnabled.bsv",
        "G0066"
    ),
    // testsuite/bsc.typechecker/display/display.exp
    frontend_pass!("bsc.typechecker/display", DISPLAY_DIR, "BasicDisplay.bs"),
    frontend_pass!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "DisplayTypeCheck.bs",
        &["-let-gen"]
    ),
    frontend_pass!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "NonConflictingBitTypes.bs",
        &["-let-gen"]
    ),
    frontend_error!(
        "bsc.typechecker/display",
        DISPLAY_DIR,
        "ConflictingBitTypes.bs",
        "T0033"
    ),
    frontend_fail_golden!("bsc.typechecker/display", DISPLAY_DIR, "NotDisplayable.bs"),
    frontend_pass!("bsc.typechecker/display", DISPLAY_DIR, "ListDisplay.bs"),
    frontend_fail_golden!("bsc.typechecker/display", DISPLAY_DIR, "NotListDisplay.bs"),
    frontend_pass!("bsc.typechecker/display", DISPLAY_DIR, "DisplayCurry.bs"),
    // testsuite/bsc.misc/bitextract/bitextract.exp
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitExtractOutOfRangeHigh.bsv",
        "S0015"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitExtractOutOfRangeLow.bsv",
        "T0130"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitExtractOutOfRangeBoth.bsv",
        "T0130"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitExtractNegative.bsv",
        "T0129"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitExtractZero.bsv",
        "T0129"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitUpdateOutOfRangeHigh.bsv",
        "S0015"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitUpdateOutOfRangeLow.bsv",
        "T0130"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitUpdateOutOfRangeBoth.bsv",
        "T0130"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitUpdateNegative.bsv",
        "T0129"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitUpdateZero.bsv",
        "T0129"
    ),
    verilog_error!(
        "bsc.misc/bitextract",
        BITEXTRACT_DIR,
        "BitArrayUpdateOutOfRangeHigh.bsv",
        "S0015"
    ),
    // testsuite/bsc.misc/format/format.exp
    frontend_error_golden!("bsc.misc/format", FORMAT_DIR, "ActionValue.bsv", "T0031"),
    verilog_pass!("bsc.misc/format", FORMAT_DIR, "DontCareFmt.bsv"),
    verilog_pass!("bsc.misc/format", FORMAT_DIR, "EmptyFormat.bsv"),
];
