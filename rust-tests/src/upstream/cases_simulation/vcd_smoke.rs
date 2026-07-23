//! Origins:
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev1/paclib_radix_rev1.exp`
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev2/paclib_radix_rev2.exp`
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev3/paclib_radix_rev3.exp`
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev4/paclib_radix_rev4.exp`
//! - `testsuite/bsc.if/split-execution/TurboFIFO/attribute/execute.exp`
//! - `testsuite/bsc.if/split-execution/TurboFIFO/original/execute.exp`

use super::SimulationCase;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, VcdExpectation,
};

macro_rules! radix_sort_case {
    ($constant:ident, $revision:literal, $fixture_dir:literal) => {
        pub(super) const $constant: SimulationCase = SimulationCase {
            name: concat!("bsc.lib/PAClib/RadixSort/", $revision, "::Tb::bluesim-vcd"),
            fixture_dir: $fixture_dir,
            source: "Tb.bsv",
            fixtures: &["Tb.bsv", "RadixSort.bsv", "Types.bsv", "sysTb.out.expected"],
            top: "sysTb",
            generated_modules: &[],
            expected: "sysTb.out.expected",
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: SimulationBackend::Bluesim,
            generation: GenerationStrategy::BackendSpecific,
            vcd: VcdExpectation::BluesimOutputMatchesNormal,
            requirement: Requirement::BluesimEnabled,
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
        };
    };
}

radix_sort_case!(
    RADIX_SORT_REV1,
    "rev1",
    "testsuite/bsc.lib/PAClib/RadixSort/rev1"
);
radix_sort_case!(
    RADIX_SORT_REV2,
    "rev2",
    "testsuite/bsc.lib/PAClib/RadixSort/rev2"
);
radix_sort_case!(
    RADIX_SORT_REV3,
    "rev3",
    "testsuite/bsc.lib/PAClib/RadixSort/rev3"
);
radix_sort_case!(
    RADIX_SORT_REV4,
    "rev4",
    "testsuite/bsc.lib/PAClib/RadixSort/rev4"
);

macro_rules! turbo_fifo_pair {
    ($bluesim:ident, $icarus:ident, $variant:literal, $fixture_dir:literal) => {
        pub(super) const $bluesim: SimulationCase = SimulationCase {
            name: concat!(
                "bsc.if/split-execution/TurboFIFO/",
                $variant,
                "::TurboFIFOTest::bluesim-vcd"
            ),
            fixture_dir: $fixture_dir,
            source: "TurboFIFOTest.bsv",
            fixtures: &[
                "TurboFIFOTest.bsv",
                "TurboFIFO.bsv",
                "sysTurboFIFOTest.out.expected",
            ],
            top: "sysTurboFIFOTest",
            generated_modules: &[],
            expected: "sysTurboFIFOTest.out.expected",
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: SimulationBackend::Bluesim,
            generation: GenerationStrategy::SharedElaboration,
            vcd: VcdExpectation::BluesimOutputMatchesNormal,
            requirement: Requirement::BluesimEnabled,
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
        };
        pub(super) const $icarus: SimulationCase = SimulationCase {
            name: concat!(
                "bsc.if/split-execution/TurboFIFO/",
                $variant,
                "::TurboFIFOTest::icarus-vcd"
            ),
            fixture_dir: $fixture_dir,
            source: "TurboFIFOTest.bsv",
            fixtures: &[
                "TurboFIFOTest.bsv",
                "TurboFIFO.bsv",
                "sysTurboFIFOTest.out.expected",
            ],
            top: "sysTurboFIFOTest",
            generated_modules: &[],
            expected: "sysTurboFIFOTest.out.expected",
            compile_options: &[],
            link_options: &[],
            simulation_options: &[],
            sort_output: false,
            backend: SimulationBackend::Icarus,
            generation: GenerationStrategy::SharedElaboration,
            vcd: VcdExpectation::IcarusSmoke,
            requirement: Requirement::VerilogEnabled,
            timeout: $crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
        };
    };
}

turbo_fifo_pair!(
    TURBO_FIFO_ATTRIBUTE_BLUESIM,
    TURBO_FIFO_ATTRIBUTE_ICARUS,
    "attribute",
    "testsuite/bsc.if/split-execution/TurboFIFO/attribute"
);
turbo_fifo_pair!(
    TURBO_FIFO_ORIGINAL_BLUESIM,
    TURBO_FIFO_ORIGINAL_ICARUS,
    "original",
    "testsuite/bsc.if/split-execution/TurboFIFO/original"
);

pub(super) const CASES: &[SimulationCase] = &[
    RADIX_SORT_REV1,
    RADIX_SORT_REV2,
    RADIX_SORT_REV3,
    RADIX_SORT_REV4,
    TURBO_FIFO_ATTRIBUTE_BLUESIM,
    TURBO_FIFO_ATTRIBUTE_ICARUS,
    TURBO_FIFO_ORIGINAL_BLUESIM,
    TURBO_FIFO_ORIGINAL_ICARUS,
];
