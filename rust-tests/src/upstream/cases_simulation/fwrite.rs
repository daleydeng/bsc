//! Origin: `testsuite/bsc.misc/fwrite/fwrite.exp`.

use super::SimulationScenario;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, GenerationStrategy, Requirement, ResourceClass,
    SimulationBackend, SimulationContract, ExpectedOutcome, OutputNormalization, SimulationTimeouts, VcdContract,
};

const FIXTURE_DIR: &str = "testsuite/bsc.misc/fwrite";

macro_rules! compare_file {
    ($path:literal) => {
        ArtifactAssertion::Matches {
            actual: $path,
            expected: concat!($path, ".expected"),
            normalization: ArtifactNormalization::GoldenOutput,
        }
    };
}

macro_rules! scenario {
    ($module:literal, $expected:literal, $fixtures:expr, $assertions:expr, $backend_name:literal, $backend:ident, $vcd:expr, $requirement:ident) => {
        SimulationScenario {
            name: concat!(
                "bsc.misc/fwrite::",
                $module,
                "::",
                $backend_name,
                "-generation"
            ),
            fixture_dir: FIXTURE_DIR,
            source: concat!($module, ".bsv"),
            fixtures: $fixtures,
            top: concat!("sys", $module),
            link_inputs: &[],
            compile_options: &[],
            generation: GenerationStrategy::BackendSpecific(SimulationBackend::$backend),
            timeouts: SimulationTimeouts::uniform(crate::BSC_TIMEOUT),
            resource: ResourceClass::Normal,
            contracts: &[SimulationContract {
                name: concat!("bsc.misc/fwrite::", $module, "::", $backend_name),
                assertions: $assertions,
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

macro_rules! dual_scenario {
    ($bluesim:ident, $icarus:ident, $module:literal, $expected:literal, $fixtures:expr, $assertions:expr) => {
        pub(super) const $bluesim: SimulationScenario = scenario!(
            $module,
            $expected,
            $fixtures,
            $assertions,
            "bluesim",
            Bluesim,
            Some(VcdContract::output_matches_normal()),
            BluesimEnabled
        );
        pub(super) const $icarus: SimulationScenario = scenario!(
            $module,
            $expected,
            $fixtures,
            $assertions,
            "icarus",
            Icarus,
            Some(VcdContract::parse()),
            VerilogEnabled
        );
    };
}

dual_scenario!(
    FOPEN_BLUESIM,
    FOPEN_ICARUS,
    "FOpen",
    "sysFOpen.out.expected",
    &[
        "FOpen.bsv",
        "sysFOpen.out.expected",
        "FOpen.dat.out.expected"
    ],
    &[compare_file!("FOpen.dat.out")]
);
dual_scenario!(
    FOPEN_2_BLUESIM,
    FOPEN_2_ICARUS,
    "FOpen2",
    "sysFOpen2.out.expected",
    &[
        "FOpen2.bsv",
        "sysFOpen2.out.expected",
        "FOpen2.dat.out.expected"
    ],
    &[compare_file!("FOpen2.dat.out")]
);
dual_scenario!(
    FOPEN_MCD_BLUESIM,
    FOPEN_MCD_ICARUS,
    "FOpen_MCD",
    "sysFOpen_MCD.out.expected",
    &[
        "FOpen_MCD.bsv",
        "sysFOpen_MCD.out.expected",
        "FOpen_MCD.dat.out.expected"
    ],
    &[compare_file!("FOpen_MCD.dat.out")]
);
dual_scenario!(
    FOPEN_MCD_2_BLUESIM,
    FOPEN_MCD_2_ICARUS,
    "FOpen_MCD2",
    "sysFOpen_MCD2.out.expected",
    &[
        "FOpen_MCD2.bsv",
        "sysFOpen_MCD2.out.expected",
        "FOpen_MCD2.dat.out.expected"
    ],
    &[compare_file!("FOpen_MCD2.dat.out")]
);
dual_scenario!(
    FCLOSE_TYPES_BLUESIM,
    FCLOSE_TYPES_ICARUS,
    "FCloseTypes",
    "sysFCloseTypes.out.expected",
    &[
        "FCloseTypes.bsv",
        "sysFCloseTypes.out.expected",
        "FCloseTypes1.dat.out.expected",
        "FCloseTypes2.dat.out.expected",
        "FCloseTypes3.dat.out.expected"
    ],
    &[
        compare_file!("FCloseTypes1.dat.out"),
        compare_file!("FCloseTypes2.dat.out"),
        compare_file!("FCloseTypes3.dat.out")
    ]
);
pub(super) const FCLOSE_TYPES_BAD_BLUESIM: SimulationScenario = scenario!(
    "FCloseTypesBad",
    "sysFCloseTypes.out.expected",
    &[
        "FCloseTypesBad.bsv",
        "sysFCloseTypes.out.expected",
        "FCloseTypes1.dat.out.expected",
        "FCloseTypes2.dat.out.expected",
        "FCloseTypes3.dat.out.expected"
    ],
    &[
        compare_file!("FCloseTypes1.dat.out"),
        compare_file!("FCloseTypes2.dat.out"),
        compare_file!("FCloseTypes3.dat.out")
    ],
    "bluesim",
    Bluesim,
    Some(VcdContract::output_matches_normal()),
    BluesimEnabled
);
dual_scenario!(
    MCD_OPS_BLUESIM,
    MCD_OPS_ICARUS,
    "MCD_ops",
    "sysMCD_ops.out.expected",
    &[
        "MCD_ops.bsv",
        "sysMCD_ops.out.expected",
        "MCD_ops1.dat.out.expected",
        "MCD_ops2.dat.out.expected",
        "MCD_ops3.dat.out.expected"
    ],
    &[
        compare_file!("MCD_ops1.dat.out"),
        compare_file!("MCD_ops2.dat.out"),
        compare_file!("MCD_ops3.dat.out")
    ]
);
dual_scenario!(
    GETC_1_BLUESIM,
    GETC_1_ICARUS,
    "GetC1",
    "sysGetC1.out.expected",
    &["GetC1.bsv", "sysGetC1.out.expected", "gettests.dat"],
    &[]
);
dual_scenario!(
    GETC_3_BLUESIM,
    GETC_3_ICARUS,
    "GetC3",
    "sysGetC3.out.expected",
    &["GetC3.bsv", "sysGetC3.out.expected", "gettests.dat"],
    &[]
);
dual_scenario!(
    FWRITES_BLUESIM,
    FWRITES_ICARUS,
    "FWrites",
    "sysFWrites.out.expected",
    &[
        "FWrites.bsv",
        "sysFWrites.out.expected",
        "FWrites.dat.out.expected"
    ],
    &[compare_file!("FWrites.dat.out")]
);
dual_scenario!(
    FWRITE_2_BLUESIM,
    FWRITE_2_ICARUS,
    "FWrite2",
    "sysFWrite2.out.expected",
    &[
        "FWrite2.bsv",
        "sysFWrite2.out.expected",
        "FWrite2.dat.out.expected"
    ],
    &[compare_file!("FWrite2.dat.out")]
);
dual_scenario!(
    FWRITE_3_BLUESIM,
    FWRITE_3_ICARUS,
    "FWrite3",
    "sysFWrite3.out.expected",
    &[
        "FWrite3.bsv",
        "sysFWrite3.out.expected",
        "FWrite3o.dat.out.expected",
        "FWrite3h.dat.out.expected",
        "FWrite3d.dat.out.expected",
        "FWrite3b.dat.out.expected"
    ],
    &[
        compare_file!("FWrite3o.dat.out"),
        compare_file!("FWrite3h.dat.out"),
        compare_file!("FWrite3d.dat.out"),
        compare_file!("FWrite3b.dat.out")
    ]
);

pub(super) const SCENARIOS: &[SimulationScenario] = &[
    FOPEN_BLUESIM,
    FOPEN_ICARUS,
    FOPEN_2_BLUESIM,
    FOPEN_2_ICARUS,
    FOPEN_MCD_BLUESIM,
    FOPEN_MCD_ICARUS,
    FOPEN_MCD_2_BLUESIM,
    FOPEN_MCD_2_ICARUS,
    FCLOSE_TYPES_BLUESIM,
    FCLOSE_TYPES_ICARUS,
    FCLOSE_TYPES_BAD_BLUESIM,
    MCD_OPS_BLUESIM,
    MCD_OPS_ICARUS,
    GETC_1_BLUESIM,
    GETC_1_ICARUS,
    GETC_3_BLUESIM,
    GETC_3_ICARUS,
    FWRITES_BLUESIM,
    FWRITES_ICARUS,
    FWRITE_2_BLUESIM,
    FWRITE_2_ICARUS,
    FWRITE_3_BLUESIM,
    FWRITE_3_ICARUS,
];
