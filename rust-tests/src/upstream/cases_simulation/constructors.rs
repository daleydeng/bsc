//! Origin: `testsuite/bsc.typechecker/constructors/constructors.exp`.

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.typechecker/constructors";

macro_rules! classic_shared_scenario {
    ($name:literal, $source:literal, $expected:literal) => {
        SimulationScenario {
            name: concat!("bsc.typechecker/constructors::", $name),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source, $expected],
            top: concat!("sys", $name),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!("bsc.typechecker/constructors::", $name, "::bluesim"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Bluesim,
                    vcd: Some(VcdContract::output_matches_normal()),
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!("bsc.typechecker/constructors::", $name, "::icarus"),
                    assertions: &[],
                    link_options: &[],
                    simulation_options: &[],
                    expectation: ExpectedOutcome::Pass { output: $expected },
                    output: OutputNormalization::Preserve,
                    backend: SimulationBackend::Icarus,
                    vcd: Some(VcdContract::parse()),
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        }
    };
}

pub(super) const STRUCT_UPDATE_QUALIFIED_FIELD: SimulationScenario = classic_shared_scenario!(
    "StructUpd_QualImp_QualField",
    "StructUpd_QualImp_QualField.bs",
    "sysStructUpd_QualImp_QualField.out.expected"
);

pub(super) const INTERFACE_UPDATE: SimulationScenario =
    classic_shared_scenario!("IfcUpd", "IfcUpd.bs", "sysIfcUpd.out.expected");

pub(super) const SCENARIOS: &[SimulationScenario] =
    &[STRUCT_UPDATE_QUALIFIED_FIELD, INTERFACE_UPDATE];
