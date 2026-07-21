//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1037/b1037.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1045/b1045.exp`

use super::SimulationCase;

pub(super) const B1037_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bugs/bluespec_inc/b1037::Foo::bluesim",
    "testsuite/bsc.bugs/bluespec_inc/b1037",
    "Foo",
    "sysFoo.out.expected"
);

pub(super) const B1037_ICARUS: SimulationCase = icarus_case!(
    "bsc.bugs/bluespec_inc/b1037::Foo::icarus",
    "testsuite/bsc.bugs/bluespec_inc/b1037",
    "Foo",
    "sysFoo.out.expected"
);

pub(super) const B1045_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.bugs/bluespec_inc/b1045::Design::bluesim",
    "testsuite/bsc.bugs/bluespec_inc/b1045",
    "Design",
    "sysDesign.out.expected"
);

pub(super) const B1045_ICARUS: SimulationCase = icarus_case!(
    "bsc.bugs/bluespec_inc/b1045::Design::icarus",
    "testsuite/bsc.bugs/bluespec_inc/b1045",
    "Design",
    "sysDesign.out.expected"
);
