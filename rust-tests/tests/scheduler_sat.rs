//! Origin: `testsuite/bsc.scheduler/sat/sat.exp`.

use bsc_rust_tests::run_scheduler_sat_case;

macro_rules! scheduler_sat_cases {
    ($($test_name:ident => $case:literal),+ $(,)?) => {
        $(
            #[test]
            fn $test_name() {
                if let Err(error) = run_scheduler_sat_case($case) {
                    panic!("{error}");
                }
            }
        )+
    };
}

scheduler_sat_cases! {
    scheduler_sat_bool_test => "BoolTest",
    scheduler_sat_add_test => "AddTest",
    scheduler_sat_mult_test => "MultTest",
    scheduler_sat_div_test => "DivTest",
    scheduler_sat_rem_test => "RemTest",
    scheduler_sat_shift_r_test => "ShiftRTest",
    scheduler_sat_shift_ra_test => "ShiftRATest",
    scheduler_sat_shift_l_test => "ShiftLTest",
    scheduler_sat_less_than_s_test => "LessThanSTest",
    scheduler_sat_less_than_test => "LessThanTest",
    scheduler_sat_zext_test => "ZextTest",
    scheduler_sat_sext_test => "SextTest",
    scheduler_sat_ite_test => "IteTest",
    scheduler_sat_truncate_test => "TruncateTest",
    scheduler_sat_shift_ra_test_2 => "ShiftRATest2",
    scheduler_sat_array_select_test => "ArraySelectTest",
    scheduler_sat_case_test => "CaseTest",
    scheduler_sat_array_select_short_index_test => "ArraySelectShortIndexTest",
    scheduler_sat_array_select_long_index_test => "ArraySelectLongIndexTest",
    scheduler_sat_array_select_impl_cond_test => "ArraySelectImplCondTest",
    scheduler_sat_param_bool_test => "ParamBoolTest",
    scheduler_sat_param_bits_test => "ParamBitsTest",
    scheduler_sat_word_64_test => "Word64Test",
    scheduler_sat_split_tuple_method_test => "SplitTupleMethodTest",
}
