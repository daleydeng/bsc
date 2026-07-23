//! Origin: `testsuite/bsc.bugs/bluespec_inc/b810/b810.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b810";

macro_rules! b810_cases {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.bugs/bluespec_inc/b810::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.bugs/bluespec_inc/b810::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
    };
}

b810_cases!(BUG_810_1_BLUESIM, BUG_810_1_ICARUS, "Bug810_1");
b810_cases!(BUG_810_3_BLUESIM, BUG_810_3_ICARUS, "Bug810_3");
b810_cases!(OPT_BUG_BLUESIM, OPT_BUG_ICARUS, "Opt_bug");

pub(super) const CASES: &[SimulationCase] = &[
    BUG_810_1_BLUESIM,
    BUG_810_1_ICARUS,
    BUG_810_3_BLUESIM,
    BUG_810_3_ICARUS,
    OPT_BUG_BLUESIM,
    OPT_BUG_ICARUS,
];
