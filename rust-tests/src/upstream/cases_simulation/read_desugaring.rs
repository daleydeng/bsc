//! Origin: `testsuite/bsc.typechecker/read_desugaring/read_desugaring.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/read_desugaring";

macro_rules! read_desugaring_cases {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.typechecker/read_desugaring::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.typechecker/read_desugaring::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
    };
}

read_desugaring_cases!(LIST_DESUGAR_BLUESIM, LIST_DESUGAR_ICARUS, "ListDesugar");
read_desugaring_cases!(STRUCT_REG_BLUESIM, STRUCT_REG_ICARUS, "StructReg");
read_desugaring_cases!(
    TWO_D_UPDATE_TEST_BLUESIM,
    TWO_D_UPDATE_TEST_ICARUS,
    "TwoDUpdateTest"
);
