//! Origins:
//! - `testsuite/bsc.lib/PAClib/qsort/bsv/paclib_qsort.exp`
//! - `testsuite/bsc.lib/PrintType/PrintType.exp`
//! - `testsuite/bsc.lib/Traversable/Traversable.exp`
//! - `testsuite/bsc.lib/listn/liblistn.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const PRINT_TYPE: SimulationScenario = SimulationScenario {
    name: "bsc.lib/PrintType::TestPrintType",
    fixture_dir: "testsuite/bsc.lib/PrintType",
    source: "TestPrintType.bs",
    fixtures: &["TestPrintType.bs", "sysTestPrintType.out.expected"],
    top: "sysTestPrintType",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.lib/PrintType::TestPrintType::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTestPrintType.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.lib/PrintType::TestPrintType::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTestPrintType.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const TRAVERSABLE: SimulationScenario = SimulationScenario {
    name: "bsc.lib/Traversable::TraversableTest",
    fixture_dir: "testsuite/bsc.lib/Traversable",
    source: "TraversableTest.bs",
    fixtures: &["TraversableTest.bs", "sysTraversableTest.out.expected"],
    top: "sysTraversableTest",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.lib/Traversable::TraversableTest::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTraversableTest.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.lib/Traversable::TraversableTest::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTraversableTest.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const APPLICATIVE_LIST_N: SimulationScenario = SimulationScenario {
    name: "bsc.lib/listn::ApplicativeListN",
    fixture_dir: "testsuite/bsc.lib/listn",
    source: "ApplicativeListN.bs",
    fixtures: &["ApplicativeListN.bs", "sysApplicativeListN.out.expected"],
    top: "sysApplicativeListN",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.lib/listn::ApplicativeListN::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysApplicativeListN.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.lib/listn::ApplicativeListN::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysApplicativeListN.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const PACLIB_QUICK_SORT: SimulationScenario = SimulationScenario {
    name: "bsc.lib/PAClib/qsort/bsv::Tb::bluesim-generation",
    fixture_dir: "testsuite/bsc.lib/PAClib/qsort/bsv",
    source: "Tb.bsv",
    fixtures: &[
        "Tb.bsv",
        "QuickSort.bsv",
        "Types.bsv",
        "MyPAClib.bsv",
        "sysTb.out.expected",
    ],
    top: "sysTb",
    link_inputs: &[],
    compile_options: &["-aggressive-conditions"],
    generation: GenerationStrategy::BackendSpecific(SimulationBackend::Bluesim),
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[SimulationContract {
        name: "bsc.lib/PAClib/qsort/bsv::Tb::bluesim",
        assertions: &[],
        link_options: &[],
        simulation_options: &[],
        expectation: ExpectedOutcome::Pass {
            output: "sysTb.out.expected",
        },
        output: OutputNormalization::Preserve,
        backend: SimulationBackend::Bluesim,
        vcd: Some(VcdContract::output_matches_normal()),
        requirement: Requirement::BluesimEnabled,
    }],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    PRINT_TYPE,
    TRAVERSABLE,
    APPLICATIVE_LIST_N,
    PACLIB_QUICK_SORT,
];
