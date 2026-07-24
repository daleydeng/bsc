//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1302/b1302.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1314/b1314.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1353/b1353.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b431/b431.exp`
//! - `testsuite/bsc.bsv_examples/Misc/example_misc.exp`
//! - `testsuite/bsc.bsv_examples/stepcounter/stepcounter.exp`
//! - `testsuite/bsc.evaluator/prims/isancestor/isancestor.exp`
//! - `testsuite/bsc.lib/ClientServer/ClientServer.exp`
//! - `testsuite/bsc.lib/fork/fork.exp`
//! - `testsuite/bsc.lib/list_ops/list_ops.exp`
//! - `testsuite/bsc.lib/RegA/rega.exp`
//! - `testsuite/bsc.lib/regtwo/regtwo.exp`
//! - `testsuite/bsc.lib/Reserved/Reserved.exp`
//! - `testsuite/bsc.lib/Stmt/Modules/Modules.exp`
//! - `testsuite/bsc.lib/Tieoff/Tieoff.exp`
//! - `testsuite/bsc.misc/crc/crc.exp`
//! - `testsuite/bsc.typechecker/reflect/reflect.exp`
//! - `testsuite/bsc.evaluator/prims/build_module/build_module.exp`
//! - `testsuite/bsc.lib/Complex/Complex.exp`
//! - `testsuite/bsc.bsv_examples/xbar/xbar.exp`

use super::SimulationScenario;
use crate::upstream::{
    ExpectedOutcome, GenerationStrategy, OutputNormalization, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, SimulationTimeouts, VcdContract,
};

macro_rules! shared_scenario_with_fixtures {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal, $fixtures:expr) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
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
                    name: concat!($prefix, "::", $module, "::icarus"),
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

macro_rules! shared_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        shared_scenario_with_fixtures!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            &[concat!($module, ".bsv"), $expected]
        )
    };
}

macro_rules! backend_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal, $backend_name:literal, $backend:ident, $vcd:expr, $requirement:ident) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::", $backend_name),
                assertions: &[],
                link_options: &[],
                simulation_options: &[],
                expectation: ExpectedOutcome::Pass { output: $expected },
                output: OutputNormalization::Preserve,
                backend: SimulationBackend::$backend,
                vcd: $vcd,
                requirement: Requirement::$requirement,
            }],
        }
    };
}

macro_rules! bluesim_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "bluesim",
            Bluesim,
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        )
    };
}

macro_rules! icarus_scenario {
    ($prefix:literal, $fixture_dir:expr, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "icarus",
            Icarus,
            Some(VcdContract::parse()),
            VerilogEnabled
        )
    };
}

const B1302_DIR: &str = "testsuite/bsc.bugs/bluespec_inc/b1302";
const B1302_FIXTURES: &[&str] = &[
    "RFile2.bsv",
    "EHR2.bsv",
    "EHR_new.bsv",
    "sysRFile2.out.expected",
];

const STEP_COUNTER_DIR: &str = "testsuite/bsc.bsv_examples/stepcounter";
const STEP_COUNTER_FIXTURES: &[&str] = &[
    "TestStepCounter.bsv",
    "StepCounter.bsv",
    "sysTestStepCounter.out.expected",
];

const XBAR_DIR: &str = "testsuite/bsc.bsv_examples/xbar";
const XBAR_FIXTURES: &[&str] = &["Tb.bsv", "XBar.bsv", "sysTb.out.expected"];

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    shared_scenario_with_fixtures!(
        "bsc.bugs/bluespec_inc/b1302",
        B1302_DIR,
        "RFile2",
        "sysRFile2.out.expected",
        B1302_FIXTURES
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1314",
        "testsuite/bsc.bugs/bluespec_inc/b1314",
        "Test",
        "sysTest.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1353",
        "testsuite/bsc.bugs/bluespec_inc/b1353",
        "Bug1353",
        "sysBug1353.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b431",
        "testsuite/bsc.bugs/bluespec_inc/b431",
        "Bug431",
        "sysBug431.out.expected"
    ),
    shared_scenario!(
        "bsc.bsv_examples/Misc",
        "testsuite/bsc.bsv_examples/Misc",
        "TestShifter64",
        "sysTestShifter64.out.expected"
    ),
    shared_scenario_with_fixtures!(
        "bsc.bsv_examples/stepcounter",
        STEP_COUNTER_DIR,
        "TestStepCounter",
        "sysTestStepCounter.out.expected",
        STEP_COUNTER_FIXTURES
    ),
    shared_scenario!(
        "bsc.evaluator/prims/isancestor",
        "testsuite/bsc.evaluator/prims/isancestor",
        "IsAncestor",
        "sysIsAncestor.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/ClientServer",
        "testsuite/bsc.lib/ClientServer",
        "TestToGPClientServer",
        "sysTestToGPClientServer.out.expected"
    ),
    icarus_scenario!(
        "bsc.lib/fork",
        "testsuite/bsc.lib/fork",
        "ForkTest",
        "sysForkTest.out.expected"
    ),
    bluesim_scenario!(
        "bsc.lib/list_ops",
        "testsuite/bsc.lib/list_ops",
        "SortGroupTest",
        "sysSortGroupTest.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/RegA",
        "testsuite/bsc.lib/RegA",
        "TestRegA",
        "sysTestRegA.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/regtwo",
        "testsuite/bsc.lib/regtwo",
        "RegTwoTest",
        "sysRegTwoTest.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/Reserved",
        "testsuite/bsc.lib/Reserved",
        "ReservedTest",
        "sysReservedTest.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/Stmt/Modules",
        "testsuite/bsc.lib/Stmt/Modules",
        "AlwaysFSM_OneAction",
        "sysAlwaysFSM_OneAction.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/Tieoff",
        "testsuite/bsc.lib/Tieoff",
        "TieOffTest",
        "sysTieOffTest.out.expected"
    ),
    shared_scenario!(
        "bsc.misc/crc",
        "testsuite/bsc.misc/crc",
        "CRCTest1",
        "sysCRCTest1.out.expected"
    ),
    shared_scenario!(
        "bsc.typechecker/reflect",
        "testsuite/bsc.typechecker/reflect",
        "TypeOf",
        "sysTypeOf.out.expected"
    ),
    shared_scenario!(
        "bsc.typechecker/reflect",
        "testsuite/bsc.typechecker/reflect",
        "TypeEQ",
        "sysTypeEQ.out.expected"
    ),
    shared_scenario!(
        "bsc.evaluator/prims/build_module",
        "testsuite/bsc.evaluator/prims/build_module",
        "RoseTest",
        "sysRoseTest.out.expected"
    ),
    shared_scenario!(
        "bsc.evaluator/prims/build_module",
        "testsuite/bsc.evaluator/prims/build_module",
        "FShowFIFO",
        "sysFShowFIFO.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/Complex",
        "testsuite/bsc.lib/Complex",
        "CmplxTest",
        "sysCmplxTest.out.expected"
    ),
    shared_scenario!(
        "bsc.lib/Complex",
        "testsuite/bsc.lib/Complex",
        "CmplxSatAdd",
        "sysCmplxSatAdd.out.expected"
    ),
    shared_scenario_with_fixtures!(
        "bsc.bsv_examples/xbar",
        XBAR_DIR,
        "Tb",
        "sysTb.out.expected",
        XBAR_FIXTURES
    ),
];
