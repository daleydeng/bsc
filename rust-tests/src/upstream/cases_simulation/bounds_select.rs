//! Origin: `testsuite/bsc.arrays/bounds/select/select.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.arrays/bounds/select";

macro_rules! bounds_cases {
    ($bluesim:ident, $icarus:ident, $module:literal) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.arrays/bounds/select::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.arrays/bounds/select::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            concat!("sys", $module, ".out.expected")
        );
    };
}

bounds_cases!(ARRAY_1_BLUESIM, ARRAY_1_ICARUS, "ArrayInBounds1");
bounds_cases!(ARRAY_2_BLUESIM, ARRAY_2_ICARUS, "ArrayInBounds2");
bounds_cases!(LIST_1_BLUESIM, LIST_1_ICARUS, "ListInBounds1");
bounds_cases!(LIST_2_BLUESIM, LIST_2_ICARUS, "ListInBounds2");
bounds_cases!(VECTOR_1_BLUESIM, VECTOR_1_ICARUS, "VectorInBounds1");
bounds_cases!(VECTOR_2_BLUESIM, VECTOR_2_ICARUS, "VectorInBounds2");
bounds_cases!(LIST_N_1_BLUESIM, LIST_N_1_ICARUS, "ListNInBounds1");
bounds_cases!(LIST_N_2_BLUESIM, LIST_N_2_ICARUS, "ListNInBounds2");
bounds_cases!(BIT_1_BLUESIM, BIT_1_ICARUS, "BitInBounds1");
bounds_cases!(BIT_2_BLUESIM, BIT_2_ICARUS, "BitInBounds2");

pub(super) const CASES: &[SimulationCase] = &[
    ARRAY_1_BLUESIM,
    ARRAY_1_ICARUS,
    ARRAY_2_BLUESIM,
    ARRAY_2_ICARUS,
    LIST_1_BLUESIM,
    LIST_1_ICARUS,
    LIST_2_BLUESIM,
    LIST_2_ICARUS,
    VECTOR_1_BLUESIM,
    VECTOR_1_ICARUS,
    VECTOR_2_BLUESIM,
    VECTOR_2_ICARUS,
    LIST_N_1_BLUESIM,
    LIST_N_1_ICARUS,
    LIST_N_2_BLUESIM,
    LIST_N_2_ICARUS,
    BIT_1_BLUESIM,
    BIT_1_ICARUS,
    BIT_2_BLUESIM,
    BIT_2_ICARUS,
];
