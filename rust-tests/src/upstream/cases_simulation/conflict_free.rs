//! Origin: `testsuite/bsc.scheduler/conflict_free/conflict_free.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.scheduler/conflict_free";

macro_rules! conflict_free_cases {
    (
        $bluesim:ident,
        $icarus:ident,
        $module:literal,
        $bluesim_expected:literal,
        $icarus_expected:literal,
        $compile_options:expr
    ) => {
        pub(super) const $bluesim: SimulationCase = bluesim_case!(
            concat!("bsc.scheduler/conflict_free::", $module, "::bluesim"),
            FIXTURE_DIR,
            $module,
            $bluesim_expected,
            $compile_options
        );
        pub(super) const $icarus: SimulationCase = icarus_case!(
            concat!("bsc.scheduler/conflict_free::", $module, "::icarus"),
            FIXTURE_DIR,
            $module,
            $icarus_expected,
            $compile_options
        );
    };
}

conflict_free_cases!(
    OK_BLUESIM,
    OK_ICARUS,
    "ConflictFreeOK",
    "sysConflictFreeOK.out.expected",
    "sysConflictFreeOK.out.expected",
    &[]
);
conflict_free_cases!(
    OK_2_BLUESIM,
    OK_2_ICARUS,
    "ConflictFreeOK2",
    "sysConflictFreeOK2.out.expected",
    "sysConflictFreeOK2.out.expected",
    &[]
);
conflict_free_cases!(
    OK_3_BLUESIM,
    OK_3_ICARUS,
    "ConflictFreeOK3",
    "sysConflictFreeOK3.out.expected",
    "sysConflictFreeOK3.out.expected",
    &["-aggressive-conditions"]
);
conflict_free_cases!(
    NOT_OK_BLUESIM,
    NOT_OK_ICARUS,
    "ConflictFreeNotOK",
    "sysConflictFreeNotOK.c.out.expected",
    "sysConflictFreeNotOK.v.out.expected",
    &[]
);
conflict_free_cases!(
    RESOURCE_BLUESIM,
    RESOURCE_ICARUS,
    "ConflictFreeResource",
    "sysConflictFreeResource.out.expected",
    "sysConflictFreeResource.out.expected",
    &[]
);
conflict_free_cases!(
    EXEC_ORDER_1_BLUESIM,
    EXEC_ORDER_1_ICARUS,
    "CFExecOrder1",
    "sysCFExecOrder1.c.out.expected",
    "sysCFExecOrder1.v.out.expected",
    &[]
);
conflict_free_cases!(
    EXEC_ORDER_2_BLUESIM,
    EXEC_ORDER_2_ICARUS,
    "CFExecOrder2",
    "sysCFExecOrder2.out.expected",
    "sysCFExecOrder2.out.expected",
    &[]
);
conflict_free_cases!(
    EXEC_ORDER_3_BLUESIM,
    EXEC_ORDER_3_ICARUS,
    "CFExecOrder3",
    "sysCFExecOrder3.out.expected",
    "sysCFExecOrder3.out.expected",
    &[]
);
conflict_free_cases!(
    SWITCH_BLUESIM,
    SWITCH_ICARUS,
    "CFSwitch",
    "sysCFSwitch.c.out.expected",
    "sysCFSwitch.v.out.expected",
    &[]
);
