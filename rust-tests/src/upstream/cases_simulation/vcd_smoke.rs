//! Origins:
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev1/paclib_radix_rev1.exp`
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev2/paclib_radix_rev2.exp`
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev3/paclib_radix_rev3.exp`
//! - `testsuite/bsc.lib/PAClib/RadixSort/rev4/paclib_radix_rev4.exp`
//! - `testsuite/bsc.if/split-execution/TurboFIFO/attribute/execute.exp`
//! - `testsuite/bsc.if/split-execution/TurboFIFO/original/execute.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

macro_rules! radix_sort_scenario {
    ($constant:ident, $revision:literal, $fixture_dir:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.lib/PAClib/RadixSort/",
                $revision,
                "::Tb::bluesim-generation"
            ),
            fixture_dir: $fixture_dir,
            source: "Tb.bsv",
            fixtures: &["Tb.bsv", "RadixSort.bsv", "Types.bsv", "sysTb.out.expected"],
            top: "sysTb",
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::Bluesim),
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.lib/PAClib/RadixSort/", $revision, "::Tb::bluesim"),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: "sysTb.out.expected" },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::Bluesim,
                vcd: Some(VcdContract::output_matches_normal()),
                requirement: Requirement::BluesimEnabled,
            }],
        };
    };
}

radix_sort_scenario!(
    RADIX_SORT_REV1,
    "rev1",
    "testsuite/bsc.lib/PAClib/RadixSort/rev1"
);
radix_sort_scenario!(
    RADIX_SORT_REV2,
    "rev2",
    "testsuite/bsc.lib/PAClib/RadixSort/rev2"
);
radix_sort_scenario!(
    RADIX_SORT_REV3,
    "rev3",
    "testsuite/bsc.lib/PAClib/RadixSort/rev3"
);
radix_sort_scenario!(
    RADIX_SORT_REV4,
    "rev4",
    "testsuite/bsc.lib/PAClib/RadixSort/rev4"
);

macro_rules! turbo_fifo_scenario {
    ($constant:ident, $variant:literal, $fixture_dir:literal) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!(
                "bsc.if/split-execution/TurboFIFO/",
                $variant,
                "::TurboFIFOTest"
            ),
            fixture_dir: $fixture_dir,
            source: "TurboFIFOTest.bsv",
            fixtures: &[
                "TurboFIFOTest.bsv",
                "TurboFIFO.bsv",
                "sysTurboFIFOTest.out.expected",
            ],
            top: "sysTurboFIFOTest",
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform($crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!(
                        "bsc.if/split-execution/TurboFIFO/",
                        $variant,
                        "::TurboFIFOTest::bluesim"
                    ),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: "sysTurboFIFOTest.out.expected" },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!(
                        "bsc.if/split-execution/TurboFIFO/",
                        $variant,
                        "::TurboFIFOTest::icarus"
                    ),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: "sysTurboFIFOTest.out.expected" },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

turbo_fifo_scenario!(
    TURBO_FIFO_ATTRIBUTE,
    "attribute",
    "testsuite/bsc.if/split-execution/TurboFIFO/attribute"
);
turbo_fifo_scenario!(
    TURBO_FIFO_ORIGINAL,
    "original",
    "testsuite/bsc.if/split-execution/TurboFIFO/original"
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    RADIX_SORT_REV1,
    RADIX_SORT_REV2,
    RADIX_SORT_REV3,
    RADIX_SORT_REV4,
    TURBO_FIFO_ATTRIBUTE,
    TURBO_FIFO_ORIGINAL,
];
