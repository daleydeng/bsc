//! Origins:
//! - `testsuite/bsc.evaluator/prims/when/when.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1424/b1424.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1658/b1658.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b540/b540.exp`
//! - `testsuite/bsc.bsv_examples/SHA1/SHA1.exp`
//! - `testsuite/bsc.bsv_examples/SHA256/SHA2.exp`
//! - `testsuite/bsc.bsv_examples/SHA512/SHA2.exp`

use super::SimulationScenario;
use crate::upstream::{
    GenerationStrategy, Requirement, ResourceClass, SimulationBackend, SimulationContract,
    VcdExpectation,
};

macro_rules! shared_scenario {
    ($prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::SharedElaboration,
            timeout: crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::bluesim"),
                    expected: $expected,
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Bluesim,
                    vcd: VcdExpectation::BluesimOutputMatchesNormal,
                    requirement: Requirement::BluesimEnabled,
                },
                SimulationContract {
                    name: concat!($prefix, "::", $module, "::icarus"),
                    expected: $expected,
                    link_options: &[],
                    simulation_options: &[],
                    sort_output: false,
                    backend: SimulationBackend::Icarus,
                    vcd: VcdExpectation::IcarusSmoke,
                    requirement: Requirement::VerilogEnabled,
                },
            ],
        }
    };
}

macro_rules! backend_scenario {
    ($prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal, $backend_name:literal, $backend:ident, $vcd:ident, $requirement:ident) => {
        SimulationScenario {
            name: concat!($prefix, "::", $module, "::", $backend_name, "-generation"),
            fixture_dir: $fixture_dir,
            source: concat!($module, ".bsv"),
            fixtures: &[concat!($module, ".bsv"), $expected],
            top: concat!("sys", $module),
            generated_modules: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeout: crate::BSC_TIMEOUT,
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!($prefix, "::", $module, "::", $backend_name),
                expected: $expected,
                link_options: &[],
                simulation_options: &[],
                sort_output: false,
                backend: SimulationBackend::$backend,
                vcd: VcdExpectation::$vcd,
                requirement: Requirement::$requirement,
            }],
        }
    };
}

macro_rules! bluesim_scenario {
    ($prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "bluesim",
            Bluesim,
            BluesimOutputMatchesNormal,
            BluesimEnabled
        )
    };
}

macro_rules! icarus_scenario {
    ($prefix:literal, $fixture_dir:literal, $module:literal, $expected:literal) => {
        backend_scenario!(
            $prefix,
            $fixture_dir,
            $module,
            $expected,
            "icarus",
            Icarus,
            IcarusSmoke,
            VerilogEnabled
        )
    };
}

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    shared_scenario!(
        "bsc.evaluator/prims/when",
        "testsuite/bsc.evaluator/prims/when",
        "When",
        "sysWhen.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "InstOrder1",
        "sysInstOrder1.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "InstOrder2",
        "sysInstOrder2.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "FunctionLocation1",
        "sysFunctionLocation1.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "FunctionLocation2",
        "sysFunctionLocation2.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "ForLoop1",
        "sysForLoop1.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "RuleOrder1",
        "sysRuleOrder1.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "RuleOrder2",
        "sysRuleOrder2.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "RuleOrder3",
        "sysRuleOrder3.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "RuleNameClash1",
        "sysRuleNameClash1.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "RuleNameClash2",
        "sysRuleNameClash2.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "MethodOrder1",
        "sysMethodOrder1.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "MethodOrder1",
        "sysMethodOrder1.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "MethodOrder2",
        "sysMethodOrder2.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "MethodOrder2",
        "sysMethodOrder2.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "MethodOrder3",
        "sysMethodOrder3.c.out.expected"
    ),
    icarus_scenario!(
        "bsc.bugs/bluespec_inc/b1424",
        "testsuite/bsc.bugs/bluespec_inc/b1424",
        "MethodOrder3",
        "sysMethodOrder3.v.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1658",
        "testsuite/bsc.bugs/bluespec_inc/b1658",
        "MethodArg_ActionValue",
        "bug1658.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1658",
        "testsuite/bsc.bugs/bluespec_inc/b1658",
        "MethodArg_Value",
        "bug1658.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1658",
        "testsuite/bsc.bugs/bluespec_inc/b1658",
        "ModulePort",
        "bug1658.out.expected"
    ),
    bluesim_scenario!(
        "bsc.bugs/bluespec_inc/b1658",
        "testsuite/bsc.bugs/bluespec_inc/b1658",
        "ModuleParam",
        "bug1658.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b540",
        "testsuite/bsc.bugs/bluespec_inc/b540",
        "Bug540_1",
        "sysBug540_1.out.expected"
    ),
    shared_scenario!(
        "bsc.bugs/bluespec_inc/b540",
        "testsuite/bsc.bugs/bluespec_inc/b540",
        "Bug540_2",
        "sysBug540_2.out.expected"
    ),
    shared_scenario!(
        "bsc.bsv_examples/SHA1",
        "testsuite/bsc.bsv_examples/SHA1",
        "KenSha1",
        "sysKenSha1.out.expected"
    ),
    shared_scenario!(
        "bsc.bsv_examples/SHA256",
        "testsuite/bsc.bsv_examples/SHA256",
        "KenSha2",
        "sysKenSha2.out.expected"
    ),
    shared_scenario!(
        "bsc.bsv_examples/SHA512",
        "testsuite/bsc.bsv_examples/SHA512",
        "KenSha2",
        "sysKenSha2.out.expected"
    ),
];
