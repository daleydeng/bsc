//! Origin: `testsuite/bsc.mcd/Gearbox/Gearbox.exp`.

use super::SimulationCase;

const FIXTURE_DIR: &str = "testsuite/bsc.mcd/Gearbox";

pub(super) const FULL_SPEED_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.mcd/Gearbox::GearboxFullSpeedTest::bluesim",
    FIXTURE_DIR,
    "GearboxFullSpeedTest",
    "sysGearboxFullSpeedTest.c.out.expected"
);
pub(super) const FULL_SPEED_ICARUS: SimulationCase = icarus_case!(
    "bsc.mcd/Gearbox::GearboxFullSpeedTest::icarus",
    FIXTURE_DIR,
    "GearboxFullSpeedTest",
    "sysGearboxFullSpeedTest.v.out.expected"
);
pub(super) const BUBBLE_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.mcd/Gearbox::GearboxBubbleTest::bluesim",
    FIXTURE_DIR,
    "GearboxBubbleTest",
    "sysGearboxBubbleTest.c.out.expected"
);
pub(super) const BUBBLE_ICARUS: SimulationCase = icarus_case!(
    "bsc.mcd/Gearbox::GearboxBubbleTest::icarus",
    FIXTURE_DIR,
    "GearboxBubbleTest",
    "sysGearboxBubbleTest.v.out.expected"
);
pub(super) const ONE_TO_ONE_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.mcd/Gearbox::Gearbox1to1Test::bluesim",
    FIXTURE_DIR,
    "Gearbox1to1Test",
    "sysGearbox1to1Test.c.out.expected"
);
pub(super) const ONE_TO_ONE_ICARUS: SimulationCase = icarus_case!(
    "bsc.mcd/Gearbox::Gearbox1to1Test::icarus",
    FIXTURE_DIR,
    "Gearbox1to1Test",
    "sysGearbox1to1Test.v.out.expected"
);
pub(super) const SAME_CLOCK_BLUESIM: SimulationCase = bluesim_case!(
    "bsc.mcd/Gearbox::GearboxSameClockTest::bluesim",
    FIXTURE_DIR,
    "GearboxSameClockTest",
    "sysGearboxSameClockTest.c.out.expected"
);
pub(super) const SAME_CLOCK_ICARUS: SimulationCase = icarus_case!(
    "bsc.mcd/Gearbox::GearboxSameClockTest::icarus",
    FIXTURE_DIR,
    "GearboxSameClockTest",
    "sysGearboxSameClockTest.v.out.expected"
);

pub(super) const CASES: &[SimulationCase] = &[
    FULL_SPEED_BLUESIM,
    FULL_SPEED_ICARUS,
    BUBBLE_BLUESIM,
    BUBBLE_ICARUS,
    ONE_TO_ONE_BLUESIM,
    ONE_TO_ONE_ICARUS,
    SAME_CLOCK_BLUESIM,
    SAME_CLOCK_ICARUS,
];
