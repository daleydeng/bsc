//! Origin: `testsuite/bsc.lib/CompletionBuffer/CompletionBuffer.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.lib/CompletionBuffer";
const EXPECTED: &str = "sysTestCompletionBuffer.out.expected";

pub(super) const TEST_COMPLETION_BUFFER: SimulationScenario = SimulationScenario {
    name: "bsc.lib/CompletionBuffer::TestCompletionBuffer",
    fixture_dir: FIXTURE_DIR,
    source: "TestCompletionBuffer.bsv",
    fixtures: &["TestCompletionBuffer.bsv", EXPECTED],
    top: "sysTestCompletionBuffer",
    generated_modules: &[],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.lib/CompletionBuffer::TestCompletionBuffer::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass { output: EXPECTED },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.lib/CompletionBuffer::TestCompletionBuffer::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass { output: EXPECTED },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[TEST_COMPLETION_BUFFER];
