//! Origin: `testsuite/bsc.bsv_examples/AmbaTransModel/amba_tmodel.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

pub(super) const TB_1_MASTER_2_SLAVES: SimulationScenario = SimulationScenario {
    name: "bsc.bsv_examples/AmbaTransModel::TB1m2s",
    fixture_dir: "testsuite/bsc.bsv_examples/AmbaTransModel",
    source: "TB1m2s.bsv",
    fixtures: &[
        "TB1m2s.bsv",
        "Interfaces.bsv",
        "Buses.bsv",
        "Masters.bsv",
        "Slaves.bsv",
        "sysTB1m2s.out.expected",
    ],
    top: "sysTB1m2s",
    generated_modules: &[
        "defaultSlave",
        "mkSlave1",
        "mkSlave2",
        "bus_1m_2s",
        "mkMaster",
    ],
    compile_options: &[],
    generation: GenerationStrategy::SharedElaboration,
    timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
    resource: ResourceClass::Normal,
    contracts: &[
        SimulationContract {
            name: "bsc.bsv_examples/AmbaTransModel::TB1m2s::bluesim",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTB1m2s.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Bluesim,
            vcd: Some(VcdContract::output_matches_normal()),
            requirement: Requirement::BluesimEnabled,
        },
        SimulationContract {
            name: "bsc.bsv_examples/AmbaTransModel::TB1m2s::icarus",
            assertions: &[],
            link_options: &[],
            simulation_options: &[],
            expectation: ExpectedOutcome::Pass {
                output: "sysTB1m2s.out.expected",
            },
            output: OutputNormalization::Preserve,
            backend: SimulationBackend::Icarus,
            vcd: Some(VcdContract::parse()),
            requirement: Requirement::VerilogEnabled,
        },
    ],
};

pub(super) const SCENARIOS: &[SimulationScenario] = &[TB_1_MASTER_2_SLAVES];
