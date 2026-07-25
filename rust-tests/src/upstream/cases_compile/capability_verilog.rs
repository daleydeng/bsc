//! Origins:
//! - `testsuite/bsc.verilog/splitports/splitports.exp`
//! - `testsuite/bsc.verilog/portprops/portprops.exp`
//! - `testsuite/bsc.verilog/undet/undet.exp`
//! - `testsuite/bsc.verilog/parameters/real/real_param.exp`
//! - `testsuite/bsc.verilog/positivereset/nameclash/nameclash.exp`
//! - `testsuite/bsc.verilog/inline/inline.exp`
//! - `testsuite/bsc.codegen/vector_modargs/vector_modargs.exp`
//! - `testsuite/bsc.names/portRenaming/alwaysEnabled/alwaysEnabled.exp`
//! - `testsuite/bsc.names/portRenaming/alwaysReady/alwaysReady.exp`
//! - `testsuite/bsc.names/portRenaming/bugs/bugs.exp`
//! - `testsuite/bsc.names/portRenaming/paths/portnames.exp`
//! - `testsuite/bsc.bsv_examples/configbus/configbus.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, GoldenExpectation,
    Requirement, TextAssertion,
};

macro_rules! text {
    ($path:expr, contains $value:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Contains { text: $value },
        }
    };
    ($path:expr, excludes $value:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::DoesNotContain { text: $value },
        }
    };
    ($path:expr, lines $value:expr, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $value,
                count: $count,
            },
        }
    };
    ($path:expr, regex $value:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Regex { pattern: $value },
        }
    };
    ($path:expr, regex_count $value:expr, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::RegexCount {
                pattern: $value,
                count: $count,
            },
        }
    };
    ($path:expr, diagnostic $tag:expr, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::DiagnosticCount {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
        }
    };
}

macro_rules! verilog_case {
    (
        $constant:ident,
        name: $name:expr,
        dir: $dir:expr,
        source: $source:expr,
        fixtures: $fixtures:expr,
        assertions: $assertions:expr,
        expectation: $expectation:expr,
        golden: $golden:expr,
        options: $options:expr,
        module: $module:expr $(,)?
    ) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: $name,
            fixture_dir: $dir,
            source: $source,
            fixtures: $fixtures,
            assertions: $assertions,
            expectation: $expectation,
            golden: $golden,
            options: $options,
            nodeps: false,
            mode: CompileMode::Verilog { module: $module },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! pass {
    ($constant:ident, $origin:literal, $dir:literal, $source:literal) => {
        verilog_case!(
            $constant,
            name: concat!($origin, "::", $source),
            dir: $dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            module: None,
        );
    };
}

macro_rules! fail {
    ($constant:ident, $origin:literal, $dir:literal, $source:literal) => {
        verilog_case!(
            $constant,
            name: concat!($origin, "::", $source),
            dir: $dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Fail,
            golden: None,
            options: &[],
            module: None,
        );
    };
}

macro_rules! fail_error {
    ($constant:ident, $origin:literal, $dir:literal, $source:literal, $tag:literal) => {
        fail_error!($constant, $origin, $dir, $source, $tag, 1);
    };
    ($constant:ident, $origin:literal, $dir:literal, $source:literal, $tag:literal, $count:expr) => {
        verilog_case!(
            $constant,
            name: concat!($origin, "::", $source),
            dir: $dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: $count,
            },
            golden: None,
            options: &[],
            module: None,
        );
    };
}

const SPLIT_DIR: &str = "testsuite/bsc.verilog/splitports";

macro_rules! split_error_golden {
    ($constant:ident, $source:literal, $tag:literal) => {
        verilog_case!(
            $constant,
            name: concat!("bsc.verilog/splitports::", $source),
            dir: SPLIT_DIR,
            source: $source,
            fixtures: &[$source, concat!($source, ".bsc-vcomp-out.expected")],
            assertions: &[],
            expectation: CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: $tag,
                count: 1,
            },
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-vcomp-out.expected"),
            }),
            options: &[],
            module: None,
        );
    };
}

split_error_golden!(SPLIT_TOO_MANY_ARG_NAMES, "TooManyArgNames.bs", "S0015");
split_error_golden!(SPLIT_PORT_NAME_CONFLICT, "PortNameConflict.bs", "G0055");
split_error_golden!(
    SPLIT_ARG_NAMES_PORT_NAME_CONFLICT,
    "ArgNamesPragma_PortNameConflict.bs",
    "G0055"
);
split_error_golden!(
    SPLIT_BAD_INSTANCE_PORT_NAME_CONFLICT,
    "BadSplitInst_PortNameConflict.bs",
    "G0055"
);
split_error_golden!(
    SPLIT_BAD_INSTANCE_TOO_MANY_PORT_NAMES,
    "BadSplitInst_TooManyPortNames.bs",
    "S0015"
);

pass!(
    SPLIT_IF_TUPLE,
    "bsc.verilog/splitports",
    "testsuite/bsc.verilog/splitports",
    "SplitIfTuple.bsv"
);
verilog_case!(
    SPLIT_IF_TUPLE_NO_LIFT,
    name: "bsc.verilog/splitports::SplitIfTuple.bsv::no-lift",
    dir: SPLIT_DIR,
    source: "SplitIfTuple.bsv",
    fixtures: &["SplitIfTuple.bsv"],
    assertions: &[],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-no-lift"],
    module: None,
);
pass!(
    SPLIT_CROSS_RULE,
    "bsc.verilog/splitports",
    "testsuite/bsc.verilog/splitports",
    "CrossRule.bsv"
);

const PORTPROPS_DIR: &str = "testsuite/bsc.verilog/portprops";

macro_rules! portprops_golden {
    ($constant:ident, $source:literal) => {
        verilog_case!(
            $constant,
            name: concat!("bsc.verilog/portprops::", $source),
            dir: PORTPROPS_DIR,
            source: $source,
            fixtures: &[$source, concat!($source, ".bsc-vcomp-out.expected")],
            assertions: &[],
            expectation: CompileExpectation::Pass,
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-vcomp-out.expected"),
            }),
            options: &["-dIOproperties"],
            module: None,
        );
    };
}

portprops_golden!(PORTPROPS_IN_HIGH, "InHigh.bsv");
portprops_golden!(PORTPROPS_INPUT_UNUSED, "InputArg_Unused.bsv");
portprops_golden!(PORTPROPS_INPUT_ONE_REG, "InputArg_OneReg.bsv");
portprops_golden!(PORTPROPS_INPUT_TWO_REG, "InputArg_TwoReg.bsv");
portprops_golden!(
    PORTPROPS_INPUT_ONE_REG_ONE_LOGIC_REG,
    "InputArg_OneRegOneLogicReg.bsv"
);
portprops_golden!(
    PORTPROPS_INPUT_ONE_REG_ONE_UNUSED,
    "InputArg_OneRegOneUnused.bsv"
);
portprops_golden!(PORTPROPS_INPUT_CONCAT_REG, "InputArg_ConcatReg.bsv");
portprops_golden!(
    PORTPROPS_INPUT_EXTRACT_REG_UNUSED,
    "InputArg_ExtractRegAndUnused.bsv"
);
portprops_golden!(PORTPROPS_METHOD_ARG_ONE_REG, "MethodArg_OneReg.bsv");
portprops_golden!(
    PORTPROPS_INPUT_GATE_METHOD_READY,
    "InputGate_OnlyInMethodReady.bsv"
);
portprops_golden!(PORTPROPS_OUTPUT_CLOCK_RESET, "OutputClockAndReset.bsv");
portprops_golden!(PORTPROPS_METHOD_VALUE_CONST, "MethodValue_Const.bsv");
portprops_golden!(PORTPROPS_METHOD_VALUE_ONE_REG, "MethodValue_OneReg.bsv");
portprops_golden!(PORTPROPS_METHOD_VALUE_LOGIC, "MethodValue_Logic.bsv");
portprops_golden!(
    PORTPROPS_METHOD_VALUE_EXTRACT_REG,
    "MethodValue_ExtractReg.bsv"
);
portprops_golden!(
    PORTPROPS_METHOD_VALUE_CONCAT_TWO_REG,
    "MethodValue_ConcatTwoReg.bsv"
);
portprops_golden!(
    PORTPROPS_METHOD_VALUE_CONCAT_REG_LOGIC,
    "MethodValue_ConcatRegAndLogic.bsv"
);
portprops_golden!(
    PORTPROPS_METHOD_VALUE_CONCAT_REG_CONST,
    "MethodValue_ConcatRegAndConst.bsv"
);
portprops_golden!(PORTPROPS_INOUT_ARG_TO_IFC, "InoutProps_ArgToIfc.bsv");
portprops_golden!(PORTPROPS_INOUT_BVI_ARG, "InoutProps_BVIArg.bsv");
portprops_golden!(PORTPROPS_INOUT_BVI_IFC, "InoutProps_BVIIfc.bsv");
portprops_golden!(PORTPROPS_INOUT_UNUSED_ARG, "InoutProps_UnusedArg.bsv");
fail_error!(
    PORTPROPS_INOUT_UNUSED_IFC,
    "bsc.verilog/portprops",
    "testsuite/bsc.verilog/portprops",
    "InoutProps_UnusedIfc.bsv",
    "G0049"
);
portprops_golden!(
    PORTPROPS_INOUT_UNUSED_ARG_BVI,
    "InoutProps_UnusedArgBVI.bsv"
);
verilog_case!(
    PORTPROPS_INCORRECT_PORT_MAPPING,
    name: "bsc.verilog/portprops::IncorrectPortMapping.bsv",
    dir: PORTPROPS_DIR,
    source: "IncorrectPortMapping.bsv",
    fixtures: &["IncorrectPortMapping.bsv"],
    assertions: &[
        text!("mkExample.v", regex r"\.A_2\(someModule\$A_2\)"),
        text!("mkExample.v", regex r"\.B_1\(someModule\$B_1\)"),
        text!("mkExample.v", regex r"\.B_3\(someModule\$B_3\)"),
        ArtifactAssertion::Text {
            path: "mkExample.v",
            assertion: TextAssertion::RegexDoesNotMatch {
                pattern: r"\.A_2\(someModule\$A_3\)",
            },
        },
        ArtifactAssertion::Text {
            path: "mkExample.v",
            assertion: TextAssertion::RegexDoesNotMatch {
                pattern: r"\.B_1\(someModule\$A_2\)",
            },
        },
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);

const UNDET_DIR: &str = "testsuite/bsc.verilog/undet";
verilog_case!(
    UNDET_MAYBE_MUX,
    name: "bsc.verilog/undet::MaybeMux.bsv",
    dir: UNDET_DIR,
    source: "MaybeMux.bsv",
    fixtures: &["MaybeMux.bsv"],
    assertions: &[text!(
        "sysMaybeMux.v",
        regex r"assign data\$D_IN = b \? data : incoming ;"
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    UNDET_IF,
    name: "bsc.verilog/undet::UndetIf.bsv",
    dir: UNDET_DIR,
    source: "UndetIf.bsv",
    fixtures: &["UndetIf.bsv"],
    assertions: &[text!(
        "sysUndetIf.v",
        regex r"assign d\$D_IN = p2 \? d : i ;"
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    UNDET_CONSTANT_TREE,
    name: "bsc.verilog/undet::UndetComp.bs",
    dir: UNDET_DIR,
    source: "UndetComp.bs",
    fixtures: &["UndetComp.bs"],
    assertions: &[
        text!("sysUndetComp.v", excludes "13'd"),
        text!("sysUndetComp.v", excludes "!="),
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);

fail_error!(
    REAL_PARAMETER_KEYWORD_REQUIRED,
    "bsc.verilog/parameters/real",
    "testsuite/bsc.verilog/parameters/real",
    "RealParamErr1.bsv",
    "G0120"
);
fail!(
    REAL_BVI_PORT_REJECTED,
    "bsc.verilog/parameters/real",
    "testsuite/bsc.verilog/parameters/real",
    "RealParamErr2.bsv"
);

const RESET_CLASH_DIR: &str = "testsuite/bsc.verilog/positivereset/nameclash";
verilog_case!(
    RESET_PREFIX_INVALID,
    name: "bsc.verilog/positivereset/nameclash::Clash1.bsv",
    dir: RESET_CLASH_DIR,
    source: "Clash1.bsv",
    fixtures: &["Clash1.bsv"],
    assertions: &[text!("Clash1.bsv.bsc-out", diagnostic "P0185", 1)],
    expectation: CompileExpectation::Fail,
    golden: None,
    options: &["-reset-prefix", "1foo"],
    module: None,
);
verilog_case!(
    RESET_PREFIX_CLASH,
    name: "bsc.verilog/positivereset/nameclash::Clash2.bsv",
    dir: RESET_CLASH_DIR,
    source: "Clash2.bsv",
    fixtures: &["Clash2.bsv"],
    assertions: &[text!("Clash2.bsv.bsc-out", diagnostic "G0107", 1)],
    expectation: CompileExpectation::Fail,
    golden: None,
    options: &["-reset-prefix", "foo"],
    module: None,
);

const INLINE_DIR: &str = "testsuite/bsc.verilog/inline";
verilog_case!(
    INLINE_RWIRE_ONE_USE,
    name: "bsc.verilog/inline::RWireOneUse.bsv",
    dir: INLINE_DIR,
    source: "RWireOneUse.bsv",
    fixtures: &["RWireOneUse.bsv"],
    assertions: &[
        text!("sysRWireOneUse.v", lines "assign rw$wget = ", 1),
        text!("sysRWireOneUse.v", lines "assign rw$whas = ", 1),
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &["-keep-inlined-boundaries"],
    module: None,
);
verilog_case!(
    INLINE_NO_RESET,
    name: "bsc.verilog/inline::NoReset.bsv",
    dir: INLINE_DIR,
    source: "NoReset.bsv",
    fixtures: &["NoReset.bsv"],
    assertions: &[
        text!("mkTestNoReset.v", lines "1'd1", 0),
        text!("mkTestSubTop.v", lines "1'd1", 0),
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    INLINE_PROBE,
    name: "bsc.verilog/inline::ProbeTest.bsv",
    dir: INLINE_DIR,
    source: "ProbeTest.bsv",
    fixtures: &["ProbeTest.bsv"],
    assertions: &[
        text!("sysProbeTest.v", lines "x$PROBE", 4),
        text!("sysProbeTest.v", lines "assign x$PROBE = ", 1),
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);

const VECTOR_DIR: &str = "testsuite/bsc.codegen/vector_modargs";
fail_error!(
    VECTOR_WRONG_CLOCK,
    "bsc.codegen/vector_modargs",
    "testsuite/bsc.codegen/vector_modargs",
    "VecClockResetToRegIfc_WrongClock.bsv",
    "G0007",
    4
);
verilog_case!(
    VECTOR_CLOCK,
    name: "bsc.codegen/vector_modargs::VecClock.bsv",
    dir: VECTOR_DIR,
    source: "VecClock.bsv",
    fixtures: &["VecClock.bsv"],
    assertions: &[
        text!(
            "sysVecClock.v",
            lines "assign CLK_clks_out_0_clk_out = CLK_clks_in_0 ;",
            1
        ),
        text!(
            "sysVecClock.v",
            lines "assign CLK_clks_out_1_clk_out = CLK_clks_in_1 ;",
            1
        ),
        text!(
            "sysVecClock.v",
            lines "assign CLK_clks_out_2_clk_out = CLK_clks_in_2 ;",
            1
        ),
        text!(
            "sysVecClock.v",
            lines "assign CLK_clks_out_3_clk_out = CLK_clks_in_3 ;",
            1
        ),
    ],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
macro_rules! vector_fail_with_module {
    ($constant:ident, $source:literal, $module:literal) => {
        verilog_case!(
            $constant,
            name: concat!("bsc.codegen/vector_modargs::", $source),
            dir: VECTOR_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: &[],
            expectation: CompileExpectation::Fail,
            golden: None,
            options: &[],
            module: Some($module),
        );
    };
}
vector_fail_with_module!(VECTOR_NAME_COLLISION, "NameCollision.bsv", "P0183");
vector_fail_with_module!(
    VECTOR_NAME_COLLISION_RENAME,
    "NameCollision_Rename.bsv",
    "P0183"
);
vector_fail_with_module!(VECTOR_INVALID_PORT_NAME, "InvalidPortName.bsv", "P0185");
verilog_case!(
    VECTOR_PARAMETER,
    name: "bsc.codegen/vector_modargs::VecParam.bsv",
    dir: VECTOR_DIR,
    source: "VecParam.bsv",
    fixtures: &["VecParam.bsv"],
    assertions: &[text!(
        "sysVecParam.v",
        regex_count r"parameter \[0 : 0\] bs_\d =",
        4
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    VECTOR_RENAME_PORT,
    name: "bsc.codegen/vector_modargs::RenamePort.bsv",
    dir: VECTOR_DIR,
    source: "RenamePort.bsv",
    fixtures: &["RenamePort.bsv"],
    assertions: &[text!(
        "sysRenamePort.v",
        lines "module sysRenamePort(B_0,",
        1
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    VECTOR_RENAME_RESET,
    name: "bsc.codegen/vector_modargs::RenameReset.bsv",
    dir: VECTOR_DIR,
    source: "RenameReset.bsv",
    fixtures: &["RenameReset.bsv"],
    assertions: &[text!(
        "sysRenameReset.v",
        lines "module sysRenameReset(R_0,",
        1
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
fail_error!(
    VECTOR_RENAME_RESET_REJECTED,
    "bsc.codegen/vector_modargs",
    "testsuite/bsc.codegen/vector_modargs",
    "RenameResetFail.bsv",
    "P0181"
);
verilog_case!(
    VECTOR_RENAME_CLOCK,
    name: "bsc.codegen/vector_modargs::RenameClock.bsv",
    dir: VECTOR_DIR,
    source: "RenameClock.bsv",
    fixtures: &["RenameClock.bsv"],
    assertions: &[text!(
        "sysRenameClock.v",
        regex r"module sysRenameClock\(O_0,(\s*)G_0,(\s*)O_1,(\s*)G_1,"
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    VECTOR_GATE_INPUT_CLOCKS,
    name: "bsc.codegen/vector_modargs::GateInputClocks_VecClock.bsv",
    dir: VECTOR_DIR,
    source: "GateInputClocks_VecClock.bsv",
    fixtures: &["GateInputClocks_VecClock.bsv"],
    assertions: &[text!(
        "sysGateInputClocks_VecClock.v",
        regex r"module sysGateInputClocks_VecClock\(CLK_clks_0,(\s*)CLK_GATE_clks_0,(\s*)CLK_clks_1,(\s*)CLK_GATE_clks_1,"
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
verilog_case!(
    VECTOR_GATE_ALL_CLOCKS,
    name: "bsc.codegen/vector_modargs::GateAllClocks_VecClock.bsv",
    dir: VECTOR_DIR,
    source: "GateAllClocks_VecClock.bsv",
    fixtures: &["GateAllClocks_VecClock.bsv"],
    assertions: &[text!(
        "sysGateAllClocks_VecClock.v",
        regex r"module sysGateAllClocks_VecClock\(CLK_clks_0,(\s*)CLK_GATE_clks_0,(\s*)CLK_clks_1,(\s*)CLK_GATE_clks_1,"
    )],
    expectation: CompileExpectation::Pass,
    golden: None,
    options: &[],
    module: None,
);
fail_error!(
    VECTOR_CLOCKED_BY_CLOCK_REJECTED,
    "bsc.codegen/vector_modargs",
    "testsuite/bsc.codegen/vector_modargs",
    "ClockedByClock.bsv",
    "P0181"
);
vector_fail_with_module!(
    VECTOR_CLOCKED_BY_VECTOR_CLOCK_REJECTED,
    "ClockedByPort_VecClock.bsv",
    "P0196"
);
vector_fail_with_module!(
    VECTOR_CLOCKED_BY_VECTOR_RESET_REJECTED,
    "ClockedByPort_VecReset.bsv",
    "P0198"
);
pass!(
    VECTOR_SIZE_ZERO,
    "bsc.codegen/vector_modargs",
    "testsuite/bsc.codegen/vector_modargs",
    "SizeZero.bsv"
);

macro_rules! name_pass {
    ($constant:ident, $origin:literal, $dir:literal, $source:literal, $fixtures:expr, $assertions:expr) => {
        verilog_case!(
            $constant,
            name: concat!($origin, "::", $source),
            dir: $dir,
            source: $source,
            fixtures: $fixtures,
            assertions: $assertions,
            expectation: CompileExpectation::Pass,
            golden: None,
            options: &[],
            module: None,
        );
    };
}

name_pass!(
    ALWAYS_ENABLED_TEST_01,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test01.bsv",
    &["Test01.bsv"],
    &[
        text!("mkDesign_01.v", excludes "RDY_start"),
        text!("mkDesign_01.v", excludes "RDY_check"),
        text!("mkDesign_01.v", excludes "RDY_result"),
        text!("mkDesign_01.v", excludes "EN_start"),
        text!("mkDesign_01.v", excludes "EN_check"),
        text!("mkDesign_01.v", excludes "EN_result"),
    ]
);
name_pass!(
    ALWAYS_ENABLED_TEST_02,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test02.bsv",
    &["Test02.bsv"],
    &[
        text!("mkDesign_02.v", excludes "RDY_start"),
        text!("mkDesign_02.v", excludes "RDY_check"),
        text!("mkDesign_02.v", excludes "RDY_result"),
        text!("mkDesign_02.v", excludes "EN_start"),
        text!("mkDesign_02.v", excludes "EN_check"),
        text!("mkDesign_02.v", excludes "EN_result"),
    ]
);
name_pass!(
    ALWAYS_ENABLED_TEST_03,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test03.bsv",
    &["Test03.bsv"],
    &[
        text!("mkDesign_03.v", excludes "RDY_check"),
        text!("mkDesign_03.v", excludes "RDY_result"),
        text!("mkDesign_03.v", excludes "EN_check"),
        text!("mkDesign_03.v", excludes "EN_result"),
    ]
);
name_pass!(
    ALWAYS_ENABLED_TEST_07,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test07.bsv",
    &["Test07.bsv"],
    &[
        text!("mkDesign_07.v", excludes "RDY_subIFC_0_start"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_0_check"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_0_result"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_1_start"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_1_check"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_1_result"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_2_start"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_2_check"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_2_result"),
        text!("mkDesign_07.v", excludes "EN_subIFC_0_start"),
        text!("mkDesign_07.v", excludes "EN_subIFC_0_check"),
        text!("mkDesign_07.v", excludes "EN_subIFC_0_result"),
        text!("mkDesign_07.v", excludes "EN_subIFC_1_start"),
        text!("mkDesign_07.v", excludes "EN_subIFC_1_check"),
        text!("mkDesign_07.v", excludes "EN_subIFC_1_result"),
        text!("mkDesign_07.v", excludes "EN_subIFC_2_start"),
        text!("mkDesign_07.v", excludes "EN_subIFC_2_check"),
        text!("mkDesign_07.v", excludes "EN_subIFC_2_result"),
    ]
);
fail_error!(
    ALWAYS_ENABLED_TEST_08,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test08.bsv",
    "G0006",
    3
);
pass!(
    ALWAYS_ENABLED_IFC_1,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "IFC1.bsv"
);
name_pass!(
    ALWAYS_ENABLED_TEST_04,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test04.bsv",
    &["Test04.bsv", "IFC1.bsv"],
    &[
        text!("mkDesign_04.v", excludes "RDY_start"),
        text!("mkDesign_04.v", excludes "RDY_check"),
        text!("mkDesign_04.v", excludes "RDY_result"),
        text!("mkDesign_04.v", excludes "EN_start"),
        text!("mkDesign_04.v", excludes "EN_check"),
        text!("mkDesign_04.v", excludes "EN_result"),
    ]
);
pass!(
    ALWAYS_ENABLED_IFC_2,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "IFC2.bsv"
);
name_pass!(
    ALWAYS_ENABLED_TEST_05,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test05.bsv",
    &["Test05.bsv", "IFC2.bsv"],
    &[
        text!("mkDesign_05.v", excludes "RDY_check"),
        text!("mkDesign_05.v", excludes "RDY_result"),
        text!("mkDesign_05.v", excludes "EN_check"),
        text!("mkDesign_05.v", excludes "EN_result"),
    ]
);
pass!(
    ALWAYS_ENABLED_S1,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "S1.bsv"
);
name_pass!(
    ALWAYS_ENABLED_TEST_06,
    "bsc.names/portRenaming/alwaysEnabled",
    "testsuite/bsc.names/portRenaming/alwaysEnabled",
    "Test06.bsv",
    &["Test06.bsv", "S1.bsv"],
    &[
        text!("mkDesign_06.v", excludes "RDY_check"),
        text!("mkDesign_06.v", excludes "RDY_result"),
        text!("mkDesign_06.v", excludes "EN_check"),
        text!("mkDesign_06.v", excludes "EN_result"),
    ]
);

name_pass!(
    ALWAYS_READY_TEST_01,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test01.bsv",
    &["Test01.bsv"],
    &[
        text!("mkDesign_01.v", excludes "RDY_start"),
        text!("mkDesign_01.v", excludes "RDY_check"),
        text!("mkDesign_01.v", excludes "RDY_result"),
    ]
);
name_pass!(
    ALWAYS_READY_TEST_02,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test02.bsv",
    &["Test02.bsv"],
    &[
        text!("mkDesign_02.v", excludes "RDY_start"),
        text!("mkDesign_02.v", excludes "RDY_check"),
        text!("mkDesign_02.v", excludes "RDY_result"),
    ]
);
name_pass!(
    ALWAYS_READY_TEST_03,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test03.bsv",
    &["Test03.bsv"],
    &[
        text!("mkDesign_03.v", excludes "RDY_check"),
        text!("mkDesign_03.v", excludes "RDY_result"),
    ]
);
name_pass!(
    ALWAYS_READY_TEST_07,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test07.bsv",
    &["Test07.bsv"],
    &[
        text!("mkDesign_07.v", excludes "RDY_subIFC_0_start"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_0_check"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_0_result"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_1_start"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_1_check"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_1_result"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_2_start"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_2_check"),
        text!("mkDesign_07.v", excludes "RDY_subIFC_2_result"),
    ]
);
fail_error!(
    ALWAYS_READY_TEST_08,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test08.bsv",
    "G0006",
    3
);
pass!(
    ALWAYS_READY_IFC_1,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "IFC1.bsv"
);
name_pass!(
    ALWAYS_READY_TEST_04,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test04.bsv",
    &["Test04.bsv", "IFC1.bsv"],
    &[
        text!("mkDesign_04.v", excludes "RDY_start"),
        text!("mkDesign_04.v", excludes "RDY_check"),
        text!("mkDesign_04.v", excludes "RDY_result"),
    ]
);
pass!(
    ALWAYS_READY_IFC_2,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "IFC2.bsv"
);
name_pass!(
    ALWAYS_READY_TEST_05,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test05.bsv",
    &["Test05.bsv", "IFC2.bsv"],
    &[
        text!("mkDesign_05.v", excludes "RDY_check"),
        text!("mkDesign_05.v", excludes "RDY_result"),
    ]
);
pass!(
    ALWAYS_READY_S1,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "S1.bsv"
);
name_pass!(
    ALWAYS_READY_TEST_06,
    "bsc.names/portRenaming/alwaysReady",
    "testsuite/bsc.names/portRenaming/alwaysReady",
    "Test06.bsv",
    &["Test06.bsv", "S1.bsv"],
    &[
        text!("mkDesign_06.v", excludes "RDY_check"),
        text!("mkDesign_06.v", excludes "RDY_result"),
    ]
);

name_pass!(
    PORT_RENAMING_QUALIFIED_LOOKUP,
    "bsc.names/portRenaming/bugs",
    "testsuite/bsc.names/portRenaming/bugs",
    "IfcPragmaQualLookup.bsv",
    &["IfcPragmaQualLookup.bsv", "IfcPragmaQualLookup_Sub.bsv"],
    &[text!(
        "sysIfcPragmaQualLookup.v",
        regex r"module sysIfcPragmaQualLookup\(CLK,
			      RST_N,

			      EN_newname,
			      RDY_newname,

			      EN_m2,
			      RDY_m2\);"
    )]
);
name_pass!(
    PORT_RENAMING_PATHS,
    "bsc.names/portRenaming/paths",
    "testsuite/bsc.names/portRenaming/paths",
    "PathTest.bsv",
    &["PathTest.bsv"],
    &[
        text!(
                                    "mkPathTestSub.v",
                                    regex r"
// Combinational paths from inputs to outputs:
//   \(put_x, EN_put\) -> req_info
//   EN_put -> req
"
                                ),
        text!(
                                    "mkPathTestTop.v",
                                    regex r"
// Combinational paths from inputs to outputs:
//   \(put_x, EN_put\) -> req_info
//   EN_put -> req
"
                                ),
    ]
);
name_pass!(
    CONFIG_BUS,
    "bsc.bsv_examples/configbus",
    "testsuite/bsc.bsv_examples/configbus",
    "CBusExample1.bsv",
    &["CBusExample1.bsv", "CBus.bsv"],
    &[
        text!("mkCounterSynth.v", lines "unnamed", 0),
        text!("mkCBusExample.v", lines "unnamed", 0),
    ]
);

pub(super) const CASES: &[CompileCase] = &[
    SPLIT_TOO_MANY_ARG_NAMES,
    SPLIT_PORT_NAME_CONFLICT,
    SPLIT_ARG_NAMES_PORT_NAME_CONFLICT,
    SPLIT_BAD_INSTANCE_PORT_NAME_CONFLICT,
    SPLIT_BAD_INSTANCE_TOO_MANY_PORT_NAMES,
    SPLIT_IF_TUPLE,
    SPLIT_IF_TUPLE_NO_LIFT,
    SPLIT_CROSS_RULE,
    PORTPROPS_IN_HIGH,
    PORTPROPS_INPUT_UNUSED,
    PORTPROPS_INPUT_ONE_REG,
    PORTPROPS_INPUT_TWO_REG,
    PORTPROPS_INPUT_ONE_REG_ONE_LOGIC_REG,
    PORTPROPS_INPUT_ONE_REG_ONE_UNUSED,
    PORTPROPS_INPUT_CONCAT_REG,
    PORTPROPS_INPUT_EXTRACT_REG_UNUSED,
    PORTPROPS_METHOD_ARG_ONE_REG,
    PORTPROPS_INPUT_GATE_METHOD_READY,
    PORTPROPS_OUTPUT_CLOCK_RESET,
    PORTPROPS_METHOD_VALUE_CONST,
    PORTPROPS_METHOD_VALUE_ONE_REG,
    PORTPROPS_METHOD_VALUE_LOGIC,
    PORTPROPS_METHOD_VALUE_EXTRACT_REG,
    PORTPROPS_METHOD_VALUE_CONCAT_TWO_REG,
    PORTPROPS_METHOD_VALUE_CONCAT_REG_LOGIC,
    PORTPROPS_METHOD_VALUE_CONCAT_REG_CONST,
    PORTPROPS_INOUT_ARG_TO_IFC,
    PORTPROPS_INOUT_BVI_ARG,
    PORTPROPS_INOUT_BVI_IFC,
    PORTPROPS_INOUT_UNUSED_ARG,
    PORTPROPS_INOUT_UNUSED_IFC,
    PORTPROPS_INOUT_UNUSED_ARG_BVI,
    PORTPROPS_INCORRECT_PORT_MAPPING,
    UNDET_MAYBE_MUX,
    UNDET_IF,
    UNDET_CONSTANT_TREE,
    REAL_PARAMETER_KEYWORD_REQUIRED,
    REAL_BVI_PORT_REJECTED,
    RESET_PREFIX_INVALID,
    RESET_PREFIX_CLASH,
    INLINE_RWIRE_ONE_USE,
    INLINE_NO_RESET,
    INLINE_PROBE,
    VECTOR_WRONG_CLOCK,
    VECTOR_CLOCK,
    VECTOR_NAME_COLLISION,
    VECTOR_NAME_COLLISION_RENAME,
    VECTOR_INVALID_PORT_NAME,
    VECTOR_PARAMETER,
    VECTOR_RENAME_PORT,
    VECTOR_RENAME_RESET,
    VECTOR_RENAME_RESET_REJECTED,
    VECTOR_RENAME_CLOCK,
    VECTOR_GATE_INPUT_CLOCKS,
    VECTOR_GATE_ALL_CLOCKS,
    VECTOR_CLOCKED_BY_CLOCK_REJECTED,
    VECTOR_CLOCKED_BY_VECTOR_CLOCK_REJECTED,
    VECTOR_CLOCKED_BY_VECTOR_RESET_REJECTED,
    VECTOR_SIZE_ZERO,
    ALWAYS_ENABLED_TEST_01,
    ALWAYS_ENABLED_TEST_02,
    ALWAYS_ENABLED_TEST_03,
    ALWAYS_ENABLED_TEST_07,
    ALWAYS_ENABLED_TEST_08,
    ALWAYS_ENABLED_IFC_1,
    ALWAYS_ENABLED_TEST_04,
    ALWAYS_ENABLED_IFC_2,
    ALWAYS_ENABLED_TEST_05,
    ALWAYS_ENABLED_S1,
    ALWAYS_ENABLED_TEST_06,
    ALWAYS_READY_TEST_01,
    ALWAYS_READY_TEST_02,
    ALWAYS_READY_TEST_03,
    ALWAYS_READY_TEST_07,
    ALWAYS_READY_TEST_08,
    ALWAYS_READY_IFC_1,
    ALWAYS_READY_TEST_04,
    ALWAYS_READY_IFC_2,
    ALWAYS_READY_TEST_05,
    ALWAYS_READY_S1,
    ALWAYS_READY_TEST_06,
    PORT_RENAMING_QUALIFIED_LOOKUP,
    PORT_RENAMING_PATHS,
    CONFIG_BUS,
];
