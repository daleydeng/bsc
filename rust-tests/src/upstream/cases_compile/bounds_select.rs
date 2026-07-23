//! Origin: `testsuite/bsc.arrays/bounds/select/select.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.arrays/bounds/select";

macro_rules! out_of_bounds_case {
    ($constant:ident, $source:literal) => {
        pub(super) const $constant: CompileCase = compile_verilog_fail_error_case!(
            concat!("bsc.arrays/bounds/select::", $source),
            FIXTURE_DIR,
            $source,
            "S0015"
        );
    };
}

out_of_bounds_case!(ARRAY_OUT_OF_BOUNDS_1, "ArrayOutOfBounds1.bsv");
out_of_bounds_case!(ARRAY_OUT_OF_BOUNDS_2, "ArrayOutOfBounds2.bsv");
out_of_bounds_case!(LIST_OUT_OF_BOUNDS_1, "ListOutOfBounds1.bsv");
out_of_bounds_case!(LIST_OUT_OF_BOUNDS_2, "ListOutOfBounds2.bsv");
out_of_bounds_case!(VECTOR_OUT_OF_BOUNDS_1, "VectorOutOfBounds1.bsv");
out_of_bounds_case!(VECTOR_OUT_OF_BOUNDS_2, "VectorOutOfBounds2.bsv");
out_of_bounds_case!(LIST_N_OUT_OF_BOUNDS_1, "ListNOutOfBounds1.bsv");
out_of_bounds_case!(LIST_N_OUT_OF_BOUNDS_2, "ListNOutOfBounds2.bsv");
out_of_bounds_case!(BIT_OUT_OF_BOUNDS_1, "BitOutOfBounds1.bsv");
out_of_bounds_case!(BIT_OUT_OF_BOUNDS_2, "BitOutOfBounds2.bsv");

pub(super) const CASES: &[CompileCase] = &[
    ARRAY_OUT_OF_BOUNDS_1,
    ARRAY_OUT_OF_BOUNDS_2,
    LIST_OUT_OF_BOUNDS_1,
    LIST_OUT_OF_BOUNDS_2,
    VECTOR_OUT_OF_BOUNDS_1,
    VECTOR_OUT_OF_BOUNDS_2,
    LIST_N_OUT_OF_BOUNDS_1,
    LIST_N_OUT_OF_BOUNDS_2,
    BIT_OUT_OF_BOUNDS_1,
    BIT_OUT_OF_BOUNDS_2,
];
