//! Origin: `testsuite/bsc.evaluator/dynamic/strings/dynamic_strings.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.evaluator/dynamic/strings";

macro_rules! string_cases {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        string_cases!(
            $bluesim,
            $icarus,
            $module,
            $crate::upstream::Requirement::VerilogEnabled
        );
    };
    ($bluesim:ident, $icarus:ident, $module:literal, $requirement:expr) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.evaluator/dynamic/strings::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.evaluator/dynamic/strings::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected"),
            &[],
            $requirement
        );
    };
}

string_cases!(MUX_BLUESIM, MUX_ICARUS, "StringMux");
string_cases!(CONCAT_BLUESIM, CONCAT_ICARUS, "StringConcat");
string_cases!(
    INTEGER_BLUESIM,
    INTEGER_ICARUS,
    "StringInteger",
    crate::upstream::Requirement::IcarusAtLeast(12)
);
string_cases!(
    INTEGER_WITH_NULL_BLUESIM,
    INTEGER_WITH_NULL_ICARUS,
    "StringIntegerWithNull",
    crate::upstream::Requirement::IcarusAtLeast(13)
);
string_cases!(EQ_BLUESIM, EQ_ICARUS, "StringEQ");
string_cases!(LT_BLUESIM, LT_ICARUS, "StringLT");
string_cases!(FORMAT_BLUESIM, FORMAT_ICARUS, "DynamicFormatString");

pub(super) const CASES: &[SimulationCase] = &[
    MUX_BLUESIM,
    MUX_ICARUS,
    CONCAT_BLUESIM,
    CONCAT_ICARUS,
    INTEGER_BLUESIM,
    INTEGER_ICARUS,
    INTEGER_WITH_NULL_BLUESIM,
    INTEGER_WITH_NULL_ICARUS,
    EQ_BLUESIM,
    EQ_ICARUS,
    LT_BLUESIM,
    LT_ICARUS,
    FORMAT_BLUESIM,
    FORMAT_ICARUS,
];
