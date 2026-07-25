//! Origins:
//! - `testsuite/bsc.bsv_examples/AmbaAdapters/amba_adapters.exp`
//! - `testsuite/bsc.bsv_examples/AmbaSynthesis/amba_syn.exp`
//! - `testsuite/bsc.bsv_examples/cache-controller/cache-controller.exp`
//! - `testsuite/bsc.bsv_examples/sudoku/sudoku.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationLinkInput, SimulationTimeouts, VcdContract,
};

macro_rules! backend_scenario {
    (
        $constant:ident,
        name: $name:literal,
        fixture_dir: $fixture_dir:expr,
        source: $source:literal,
        fixtures: $fixtures:expr,
        top: $top:literal,
        link_inputs: $link_inputs:expr,
        compile_options: $compile_options:expr,
        expected: $expected:literal,
        backend_name: $backend_name:literal,
        backend: $backend:ident,
        vcd: $vcd:expr,
        requirement: $requirement:ident
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: concat!($name, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: $compile_options,
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($name, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        };
    };
}

macro_rules! shared_scenario {
    (
        $constant:ident,
        name: $name:literal,
        fixture_dir: $fixture_dir:expr,
        source: $source:literal,
        fixtures: $fixtures:expr,
        top: $top:literal,
        link_inputs: $link_inputs:expr,
        compile_options: $compile_options:expr,
        expected: $expected:literal,
        bluesim_vcd: $bluesim_vcd:expr,
        icarus_vcd: $icarus_vcd:expr
    ) => {
        pub(super) const $constant: SimulationScenario = SimulationScenario {
            name: $name,
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: $fixtures,
            top: $top,
            link_inputs: $link_inputs,
            compile_options: $compile_options,
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($name, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: $bluesim_vcd,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($name, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: $icarus_vcd,
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        };
    };
}

const AMBA_ADAPTERS_DIR: &str = "testsuite/bsc.bsv_examples/AmbaAdapters";
const AMBA_ADAPTERS_DMA_SOURCES: &[&str] = &[
    "DMA_Envir.bsv",
    "Slaves.bsv",
    "AmbaAdapters.bsv",
    "Interfaces.bsv",
    "Buses.bsv",
    "Masters.bsv",
    "DMA.bsv",
];
const AMBA_ADAPTERS_DMA_LINK_INPUTS: &[SimulationLinkInput] = &[
    SimulationLinkInput::GeneratedModule("mkDMA"),
    SimulationLinkInput::GeneratedModule("mkSlaveRx"),
    SimulationLinkInput::GeneratedModule("mkSlaveTx"),
    SimulationLinkInput::GeneratedModule("defaultSlave"),
];
const AMBA_OPTIONS: &[&str] = &["-keep-fires", "-relax-method-earliness"];

backend_scenario!(
    AMBA_ADAPTERS_DMA_ICARUS,
    name: "bsc.bsv_examples/AmbaAdapters::DMA_Envir::sysDMA",
    fixture_dir: AMBA_ADAPTERS_DIR,
    source: "DMA_Envir.bsv",
    fixtures: &[
        AMBA_ADAPTERS_DMA_SOURCES[0],
        AMBA_ADAPTERS_DMA_SOURCES[1],
        AMBA_ADAPTERS_DMA_SOURCES[2],
        AMBA_ADAPTERS_DMA_SOURCES[3],
        AMBA_ADAPTERS_DMA_SOURCES[4],
        AMBA_ADAPTERS_DMA_SOURCES[5],
        AMBA_ADAPTERS_DMA_SOURCES[6],
        "sysDMA.v.out.expected",
    ],
    top: "sysDMA",
    link_inputs: AMBA_ADAPTERS_DMA_LINK_INPUTS,
    compile_options: AMBA_OPTIONS,
    expected: "sysDMA.v.out.expected",
    backend_name: "icarus",
    backend: Icarus,
    vcd: Some(VcdContract::parse()),
    requirement: VerilogEnabled
);
backend_scenario!(
    AMBA_ADAPTERS_DMA_BLUESIM,
    name: "bsc.bsv_examples/AmbaAdapters::DMA_Envir::sysDMA",
    fixture_dir: AMBA_ADAPTERS_DIR,
    source: "DMA_Envir.bsv",
    fixtures: &[
        AMBA_ADAPTERS_DMA_SOURCES[0],
        AMBA_ADAPTERS_DMA_SOURCES[1],
        AMBA_ADAPTERS_DMA_SOURCES[2],
        AMBA_ADAPTERS_DMA_SOURCES[3],
        AMBA_ADAPTERS_DMA_SOURCES[4],
        AMBA_ADAPTERS_DMA_SOURCES[5],
        AMBA_ADAPTERS_DMA_SOURCES[6],
        "sysDMA.out.expected",
    ],
    top: "sysDMA",
    link_inputs: AMBA_ADAPTERS_DMA_LINK_INPUTS,
    compile_options: AMBA_OPTIONS,
    expected: "sysDMA.out.expected",
    backend_name: "bluesim",
    backend: Bluesim,
    vcd: Some(VcdContract::output_matches_normal()),
    requirement: BluesimEnabled
);
shared_scenario!(
    AMBA_ADAPTERS_BASELINE,
    name: "bsc.bsv_examples/AmbaAdapters::TBbaseline",
    fixture_dir: AMBA_ADAPTERS_DIR,
    source: "TBbaseline.bsv",
    fixtures: &[
        "TBbaseline.bsv",
        "Interfaces.bsv",
        "Buses.bsv",
        "Slaves.bsv",
        "Masters.bsv",
        "AmbaAdapters.bsv",
        "sysM1_25.out.expected",
    ],
    top: "sysM1_25",
    link_inputs: &[],
    compile_options: &[],
    expected: "sysM1_25.out.expected",
    bluesim_vcd: Some(VcdContract::output_matches_normal()),
    icarus_vcd: Some(VcdContract::parse())
);

const AMBA_SYNTHESIS_DIR: &str = "testsuite/bsc.bsv_examples/AmbaSynthesis";
backend_scenario!(
    AMBA_SYNTHESIS_DMA_ICARUS,
    name: "bsc.bsv_examples/AmbaSynthesis::DMA_Envir::sysDMA",
    fixture_dir: AMBA_SYNTHESIS_DIR,
    source: "DMA_Envir.bsv",
    fixtures: &[
        "DMA_Envir.bsv",
        "Slaves.bsv",
        "AmbaSynthesis.bsv",
        "DMA.bsv",
        "sysDMA.out.expected",
    ],
    top: "sysDMA",
    link_inputs: &[
        SimulationLinkInput::GeneratedModule("mkDMA"),
        SimulationLinkInput::GeneratedModule("mkSink"),
        SimulationLinkInput::GeneratedModule("mkSource"),
        SimulationLinkInput::GeneratedModule("mkSlaveTx"),
        SimulationLinkInput::GeneratedModule("mkSlaveRx"),
        SimulationLinkInput::GeneratedModule("mkSRAM64k"),
        SimulationLinkInput::GeneratedModule("defaultSlave"),
    ],
    compile_options: AMBA_OPTIONS,
    expected: "sysDMA.out.expected",
    backend_name: "icarus",
    backend: Icarus,
    vcd: Some(VcdContract::parse()),
    requirement: VerilogEnabled
);

const CACHE_CONTROLLER_DIR: &str = "testsuite/bsc.bsv_examples/cache-controller";
const CACHE_CONTROLLER_IMPORTS: &[&str] = &[
    "Cache.bsv",
    "Cache_Controller.bsv",
    "External_Interfaces.bsv",
    "SRAM_Fake.bsv",
    "SRAM_Interfaces.bsv",
];
const CACHE_CONTROLLER_LINK_INPUTS: &[SimulationLinkInput] = &[
    SimulationLinkInput::GeneratedModule("cache"),
    SimulationLinkInput::GeneratedModule("cache_controller"),
];
const CACHE_CONTROLLER_OPTIONS: &[&str] = &[
    "-opt-bool",
    "-opt-bit-const",
    "-opt-undetermined-vals",
    "-opt-if-mux",
    "-opt-mux-const",
    "-opt-sched",
    "-opt-ATS",
    "-inline-rwire",
    "-let-gen",
];

shared_scenario!(
    CACHE_CONTROLLER_TESTBENCH,
    name: "bsc.bsv_examples/cache-controller::Testbench::testbench",
    fixture_dir: CACHE_CONTROLLER_DIR,
    source: "Testbench.bsv",
    fixtures: &[
        "Testbench.bsv",
        CACHE_CONTROLLER_IMPORTS[0],
        CACHE_CONTROLLER_IMPORTS[1],
        CACHE_CONTROLLER_IMPORTS[2],
        CACHE_CONTROLLER_IMPORTS[3],
        CACHE_CONTROLLER_IMPORTS[4],
        "testbench.out.expected",
    ],
    top: "testbench",
    link_inputs: CACHE_CONTROLLER_LINK_INPUTS,
    compile_options: CACHE_CONTROLLER_OPTIONS,
    expected: "testbench.out.expected",
    bluesim_vcd: Some(VcdContract::output_matches_normal()),
    icarus_vcd: Some(VcdContract::parse())
);
shared_scenario!(
    CACHE_CONTROLLER_RANDOM_TESTBENCH,
    name: "bsc.bsv_examples/cache-controller::RandomTestbench::random_testbench",
    fixture_dir: CACHE_CONTROLLER_DIR,
    source: "RandomTestbench.bsv",
    fixtures: &[
        "RandomTestbench.bsv",
        CACHE_CONTROLLER_IMPORTS[0],
        CACHE_CONTROLLER_IMPORTS[1],
        CACHE_CONTROLLER_IMPORTS[2],
        CACHE_CONTROLLER_IMPORTS[3],
        CACHE_CONTROLLER_IMPORTS[4],
        "random_testbench.out.expected",
    ],
    top: "random_testbench",
    link_inputs: CACHE_CONTROLLER_LINK_INPUTS,
    compile_options: CACHE_CONTROLLER_OPTIONS,
    expected: "random_testbench.out.expected",
    bluesim_vcd: Some(VcdContract::output_matches_normal()),
    icarus_vcd: Some(VcdContract::parse())
);

const SUDOKU_DIR: &str = "testsuite/bsc.bsv_examples/sudoku";
pub(super) const SUDOKU_GENERATE_TEST_3: SimulationScenario = SimulationScenario {
    name: "bsc.bsv_examples/sudoku::GenerateTest3",
    fixture_dir: SUDOKU_DIR,
    source: "GenerateTest3.bsv",
    fixtures: &[
        "GenerateTest3.bsv",
        "Generator.bsv",
        "SatMath.bsv",
        "Solver.bsv",
        "Sudoku.bsv",
        "Tactics.bsv",
        "TypeUtil.bsv",
        "mkGenerateTest3.out.expected",
    ],
    top: "mkGenerateTest3",
    link_inputs: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_HEAVY_TIMEOUT),
    resource: ResourceClass::Heavy,
    contracts: &[
        SimulationContract {
            name: "bsc.bsv_examples/sudoku::GenerateTest3::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "mkGenerateTest3.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: None,
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.bsv_examples/sudoku::GenerateTest3::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "mkGenerateTest3.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: None,
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    AMBA_ADAPTERS_DMA_ICARUS,
    AMBA_ADAPTERS_DMA_BLUESIM,
    AMBA_ADAPTERS_BASELINE,
    AMBA_SYNTHESIS_DMA_ICARUS,
    CACHE_CONTROLLER_TESTBENCH,
    CACHE_CONTROLLER_RANDOM_TESTBENCH,
    SUDOKU_GENERATE_TEST_3,
];
