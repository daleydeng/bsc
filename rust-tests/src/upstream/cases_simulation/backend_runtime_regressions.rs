//! Origins:
//! - `testsuite/bsc.scheduler/rulesort/rulesort.exp`
//! - `testsuite/bsc.verilog/dollar/renaming/rename.exp`
//! - `testsuite/bsc.verilog/dollar/renaming2/rename.exp`
//! - `testsuite/bsc.verilog/noinline/divbug/noinline_divbug.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const RULE_SORT: SimulationScenario = SimulationScenario {
    name: "bsc.scheduler/rulesort::RuleSort",
    fixture_dir: "testsuite/bsc.scheduler/rulesort",
    source: "RuleSort.bs",
    fixtures: &["RuleSort.bs", "sysRuleSort.out.expected"],
    top: "sysRuleSort",
    generated_modules: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.scheduler/rulesort::RuleSort::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysRuleSort.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.scheduler/rulesort::RuleSort::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysRuleSort.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const NOINLINE_DIVIDE: SimulationScenario = SimulationScenario {
    name: "bsc.verilog/noinline/divbug::DivTest",
    fixture_dir: "testsuite/bsc.verilog/noinline/divbug",
    source: "DivTest.bsv",
    fixtures: &["DivTest.bsv", "Div.bsv", "sysDivTest.out.expected"],
    top: "sysDivTest",
    generated_modules: &["module_divide"],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.verilog/noinline/divbug::DivTest::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysDivTest.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.verilog/noinline/divbug::DivTest::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysDivTest.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const REMOVE_DOLLAR_RENAMING: SimulationScenario = SimulationScenario {
    name: "bsc.verilog/dollar/renaming::TbGCD::icarus-generation",
    fixture_dir: "testsuite/bsc.verilog/dollar/renaming",
    source: "TbGCD.bsv",
    fixtures: &["TbGCD.bsv", "GCD.bsv", "mkTbGCD.out.expected"],
    top: "sysTbGCD",
    generated_modules: &[],
    compile_options: &["-remove-dollar"],
    generation: GenerationStrategy::BackendSpecific(SimulationBackend::Icarus),
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[SimulationContract {
        name: "bsc.verilog/dollar/renaming::TbGCD::icarus",
        assertions: &[],
        link_options: &[],
        simulation_options: &[],
        expectation: ExpectedOutcome::Pass {
            output: "mkTbGCD.out.expected",
        },
        output: OutputNormalization::Preserve,
        backend: SimulationBackend::Icarus,
        vcd: Some(VcdContract::parse()),
        requirement: Requirement::VerilogEnabled,
    }],
};

pub(super) const REMOVE_DOLLAR_RENAMING_2: SimulationScenario = SimulationScenario {
    name: "bsc.verilog/dollar/renaming2::TbGCD::icarus-generation",
    fixture_dir: "testsuite/bsc.verilog/dollar/renaming2",
    source: "TbGCD.bsv",
    fixtures: &["TbGCD.bsv", "GCD.bsv", "mkTbGCD.out.expected"],
    top: "sysTbGCD",
    generated_modules: &[],
    compile_options: &["-remove-dollar"],
    generation: GenerationStrategy::BackendSpecific(SimulationBackend::Icarus),
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[SimulationContract {
        name: "bsc.verilog/dollar/renaming2::TbGCD::icarus",
        assertions: &[],
        link_options: &[],
        simulation_options: &[],
        expectation: ExpectedOutcome::Pass {
            output: "mkTbGCD.out.expected",
        },
        output: OutputNormalization::Preserve,
        backend: SimulationBackend::Icarus,
        vcd: Some(VcdContract::parse()),
        requirement: Requirement::VerilogEnabled,
    }],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    RULE_SORT,
    NOINLINE_DIVIDE,
    REMOVE_DOLLAR_RENAMING,
    REMOVE_DOLLAR_RENAMING_2,
];
