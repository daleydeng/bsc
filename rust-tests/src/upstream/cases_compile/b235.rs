//! Origin: `testsuite/bsc.bugs/bluespec_inc/b235/b235.exp`.

use super::CompileCase;

const FIXTURE_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b235";

macro_rules! b235_pass {
    ($constant:ident, $source:literal) => {
        pub(super) const $constant: CompileCase = compile_pass_case!(
            concat!("bsc.bugs/bluespec_inc/b235::", $source),
            FIXTURE_DIR,
            $source
        );
    };
}

macro_rules! b235_fail_golden {
    ($constant:ident, $source:literal) => {
        pub(super) const $constant: CompileCase = compile_fail_golden_case!(
            concat!("bsc.bugs/bluespec_inc/b235::", $source),
            FIXTURE_DIR,
            $source,
            concat!($source, ".bsc-out.expected")
        );
    };
}

b235_pass!(BUG_235_1, "Bug235-1.bsv");
b235_fail_golden!(BUG_235_2, "Bug235-2.bsv");
b235_pass!(BUG_235_3, "Bug235-3.bsv");
b235_pass!(BUG_235_4, "Bug235-4.bsv");
b235_fail_golden!(BUG_235_5, "Bug235-5.bsv");
b235_fail_golden!(BUG_235_6, "Bug235-6.bsv");

pub(super) const CASES: &[CompileCase] = &[
    BUG_235_1,
    BUG_235_2,
    BUG_235_3,
    BUG_235_4,
    BUG_235_5,
    BUG_235_6,
];
