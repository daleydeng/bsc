//! Origin: `testsuite/bsc.syntax/bsv05/case/case.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.syntax/bsv05/case";

macro_rules! case_simulation_cases {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.syntax/bsv05/case::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.syntax/bsv05/case::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
    };
}

case_simulation_cases!(
    MATCHES_MIXED_LIT_BLUESIM,
    MATCHES_MIXED_LIT_ICARUS,
    "CaseMatches_MixedLit"
);
case_simulation_cases!(MIXED_HEX_BLUESIM, MIXED_HEX_ICARUS, "CaseMixedHex");
case_simulation_cases!(MIXED_OCT_BLUESIM, MIXED_OCT_ICARUS, "CaseMixedOct");
