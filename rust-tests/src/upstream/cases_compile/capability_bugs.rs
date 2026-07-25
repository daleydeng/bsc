//! Origins:
//! - `testsuite/bsc.bugs/bluespec_inc/b1018/b1018.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1066/b1066.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1249/b1249.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1390/b1390.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1402/b1402.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1619/b1619.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1621/b1621.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1690/b1690.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1720/b1720.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1753/b1753.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b1758/b1758.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b232/b232.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b312/b312.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b323/b323.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b359/b359.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b399/b399.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b484/b484.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b517/b517.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b518/b518.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b568/b568.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b628/b628.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b631/b631.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b690/b690.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b765/b765.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b864/b864.exp`
//! - `testsuite/bsc.bugs/bluespec_inc/b893/b893.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, ArtifactNormalization, CompileExpectation, CompileMode, DiagnosticKind,
    GoldenExpectation, Requirement, TextAssertion,
};

macro_rules! compile_case {
    (
        $constant:ident,
        $origin:literal,
        $source:literal,
        $fixtures:expr,
        $assertions:expr,
        $expectation:expr,
        $golden:expr,
        $options:expr,
        $mode:expr
    ) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.bugs/bluespec_inc/", $origin, "::", $source),
            fixture_dir: concat!("testsuite/bsc.bugs/bluespec_inc/", $origin),
            source: $source,
            fixtures: $fixtures,
            assertions: $assertions,
            expectation: $expectation,
            golden: $golden,
            options: $options,
            nodeps: false,
            mode: $mode,
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! line_count {
    ($path:literal, $text:literal, $count:expr) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::LineCount {
                text: $text,
                count: $count,
            },
        }
    };
}

macro_rules! contains {
    ($path:literal, $text:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Contains { text: $text },
        }
    };
}

macro_rules! does_not_contain {
    ($path:literal, $text:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::DoesNotContain { text: $text },
        }
    };
}

macro_rules! regex {
    ($path:literal, $pattern:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::Regex { pattern: $pattern },
        }
    };
}

macro_rules! regex_does_not_match {
    ($path:literal, $pattern:literal) => {
        ArtifactAssertion::Text {
            path: $path,
            assertion: TextAssertion::RegexDoesNotMatch { pattern: $pattern },
        }
    };
}

macro_rules! matches {
    ($actual:literal, $expected:literal, $normalization:ident) => {
        ArtifactAssertion::Matches {
            actual: $actual,
            expected: $expected,
            normalization: ArtifactNormalization::$normalization,
        }
    };
}

macro_rules! golden {
    ($expected:literal) => {
        Some(GoldenExpectation {
            expected: $expected,
        })
    };
}

compile_case!(
    B1018_CASE,
    "b1018",
    "Case.bsv",
    &["Case.bsv"],
    &[line_count!("mkCase.v", "abc$EN = 1'b1", 1)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkCase")
    }
);

compile_case!(
    B1066_TEST,
    "b1066",
    "Test.bsv",
    &["Test.bsv", "Sub1.bsv", "Sub2.bsv"],
    &[regex!("mkTest.v", r"input  \[7 : 0\] VAL;")],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1066_TEST_2,
    "b1066",
    "Test2.bsv",
    &["Test2.bsv", "Sub1.bsv", "Sub2.bsv"],
    &[],
    CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0107",
        count: 1,
    },
    None,
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B1249_BUG,
    "b1249",
    "Bug1249.bsv",
    &["Bug1249.bsv"],
    &[line_count!("sysBug.v", "m$CLK_GATE_OUT", 4)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B1390_TEST,
    "b1390",
    "Test.bsv",
    &[
        "Test.bsv",
        "Test.bsv.bsc-vcomp-out.expected",
        "mkTest.sched.expected",
        "mkTest.v.expected",
    ],
    &[
        matches!("mkTest.sched", "mkTest.sched.expected", GoldenOutput),
        matches!("mkTest.v", "mkTest.v.expected", Verilog),
    ],
    CompileExpectation::Pass,
    golden!("Test.bsv.bsc-vcomp-out.expected"),
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1390_TEST_2,
    "b1390",
    "Test2.bsv",
    &[
        "Test2.bsv",
        "Test2.bsv.bsc-vcomp-out.expected",
        "mkTest2.sched.expected",
        "mkTest2.v.expected",
    ],
    &[
        matches!("mkTest2.sched", "mkTest2.sched.expected", GoldenOutput),
        matches!("mkTest2.v", "mkTest2.v.expected", Verilog),
    ],
    CompileExpectation::Pass,
    golden!("Test2.bsv.bsc-vcomp-out.expected"),
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B1402_TEST,
    "b1402",
    "Test.bsv",
    &["Test.bsv"],
    &[
        does_not_contain!("sysTest.v", "%"),
        contains!("sysTest.v", "assign m$D_IN = { 4'd0, r[3:0] } ;"),
        contains!("sysTest.v", "assign r$D_IN = r >> 4 ;"),
    ],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B1619_A,
    "b1619",
    "Bug1619A.bsv",
    &["Bug1619A.bsv", "Bug1619A.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("Bug1619A.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);
compile_case!(
    B1619_B,
    "b1619",
    "Bug1619B.bsv",
    &["Bug1619B.bsv", "Bug1619B.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("Bug1619B.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);
compile_case!(
    B1619_C,
    "b1619",
    "Bug1619C.bsv",
    &["Bug1619C.bsv", "Bug1619C.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("Bug1619C.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);

compile_case!(
    B1621_BUG,
    "b1621",
    "Bug1621.bsv",
    &["Bug1621.bsv"],
    &[
        line_count!("module_summer.v", "assign summer = summer_a + 32'd86 ;", 1),
        line_count!(
            "module_summer2.v",
            "assign summer2 = summer2_a + 32'd86 ;",
            1
        ),
    ],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B1690_MUT_EX,
    "b1690",
    "MutEx.bsv",
    &["MutEx.bsv", "MutEx.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("MutEx.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);
compile_case!(
    B1690_MUT_EX_BIG,
    "b1690",
    "MutExBig.bsv",
    &["MutExBig.bsv", "MutExBig.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("MutExBig.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);

compile_case!(
    B1720_BUG,
    "b1720",
    "Bug1720-1.bsv",
    &["Bug1720-1.bsv", "Bug1720-1.bsv.bsc-vcomp-out.expected"],
    &[],
    CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "T0031",
        count: 2,
    },
    golden!("Bug1720-1.bsv.bsc-vcomp-out.expected"),
    &[],
    CompileMode::Verilog {
        module: Some("mkTb")
    }
);

compile_case!(
    B1753_BUG,
    "b1753",
    "Bug1753.bsv",
    &["Bug1753.bsv"],
    &[],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1753_SHADOW_IN_EXPR,
    "b1753",
    "ShadowInExpr.bsv",
    &["ShadowInExpr.bsv"],
    &[line_count!(
        "sysShadowInExpr.v",
        "assign lastdata$D_IN = { ending[5], ending[4:0] + 5'd1 } ;",
        1
    )],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1753_SHADOW_IN_RULE,
    "b1753",
    "ShadowInRule.bsv",
    &["ShadowInRule.bsv"],
    &[line_count!(
        "sysShadowInRule.v",
        "assign lastdata$D_IN = { 1'd1, ending[4:0] + 5'd1 } ;",
        1
    )],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1753_SHADOW_IN_METHOD,
    "b1753",
    "ShadowInMethod.bsv",
    &["ShadowInMethod.bsv"],
    &[line_count!(
        "sysShadowInMethod.v",
        "assign lastdata$D_IN = { 1'd1, ending[4:0] + 5'd1 } ;",
        1
    )],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1753_SHADOW_IN_PATTERN,
    "b1753",
    "ShadowInPattern.bsv",
    &["ShadowInPattern.bsv"],
    &[],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1753_METHOD_NAME_SHADOW,
    "b1753",
    "MethodNameShadow.bsv",
    &["MethodNameShadow.bsv"],
    &[],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B1758_ZERO_BIT_VALUE_METHOD,
    "b1758",
    "ZeroBitValueMethod.bsv",
    &["ZeroBitValueMethod.bsv"],
    &[
        regex_does_not_match!(
            "ZeroBitValueMethod.bsv.bsc-out",
            r"= \.ZeroBitValueMethod\.getVal g"
        ),
        regex!(
            "ZeroBitValueMethod.bsv.bsc-out",
            r#"==> Prelude\.\$display#0 "v = %b" 0"#
        ),
    ],
    CompileExpectation::Pass,
    None,
    &["-dexpanded", "-dATS"],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1758_ZERO_BIT_ACTION_VALUE_METHOD,
    "b1758",
    "ZeroBitActionValueMethod.bsv",
    &["ZeroBitActionValueMethod.bsv"],
    &[
        regex_does_not_match!(
            "ZeroBitActionValueMethod.bsv.bsc-out",
            r"= \.Prelude\.avValue_ ·0 \(\.ZeroBitActionValueMethod\.get g\)"
        ),
        regex!("ZeroBitActionValueMethod.bsv.bsc-out", r"g\.get; p\.put;"),
    ],
    CompileExpectation::Pass,
    None,
    &["-dexpanded", "-dATS"],
    CompileMode::Verilog { module: None }
);
compile_case!(
    B1758_ZERO_BIT_ACTION_VALUE_FOREIGN_WITH_ARGS,
    "b1758",
    "ZeroBitActionValueForeignWithArgs.bsv",
    &["ZeroBitActionValueForeignWithArgs.bsv"],
    &[regex!(
        "sysZeroBitActionValueForeignWithArgs.v",
        r"\$imported_my_time\(8'd0\)"
    )],
    CompileExpectation::Pass,
    None,
    &["-dexpanded", "-dATS"],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B232_DESIGN,
    "b232",
    "Design.bs",
    &["Design.bs"],
    &[line_count!("mkDesign.v", "nextState", 0)],
    CompileExpectation::Pass,
    None,
    &["-remove-unused-modules"],
    CompileMode::Verilog {
        module: Some("mkDesign")
    }
);

compile_case!(
    B312_BUG,
    "b312",
    "Bug312.bsv",
    &["Bug312.bsv"],
    &[line_count!("Bug312.bsv.bsc-out", "acheck", 0)],
    CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "G0053",
        count: 1,
    },
    None,
    &[],
    CompileMode::Verilog {
        module: Some("sysBug312")
    }
);

compile_case!(
    B323_TEST,
    "b323",
    "Test.bsv",
    &["Test.bsv", "Test.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("Test.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);

compile_case!(
    B359_MODULE_BIND,
    "b359",
    "ModuleBind.bs",
    &["ModuleBind.bs"],
    &[],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("sysFoo")
    }
);
compile_case!(
    B359_BUG,
    "b359",
    "Bug359.bsv",
    &["Bug359.bsv", "Bug359.bsv.bsc-vcomp-out.expected"],
    &[],
    CompileExpectation::Fail,
    golden!("Bug359.bsv.bsc-vcomp-out.expected"),
    &[],
    CompileMode::Verilog {
        module: Some("mkTestbench")
    }
);
compile_case!(
    B359_BUG_2,
    "b359",
    "Bug359_2.bs",
    &["Bug359_2.bs"],
    &[],
    CompileExpectation::Fail,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("sysBug359_2")
    }
);

compile_case!(
    B399_BUG,
    "b399",
    "Bug399.bsv",
    &["Bug399.bsv"],
    &[
        does_not_contain!("mkBug399.v", "input  CLK;"),
        does_not_contain!("mkBug399.v", "input  CLK_GATE;"),
        does_not_contain!("mkBug399.v", "input  RST_N;"),
    ],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B484_DESIGN,
    "b484",
    "Design.bsv",
    &["Design.bsv"],
    &[line_count!("mkDesign.v", "reg_a$EN = 1'd1", 1)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkDesign")
    }
);

compile_case!(
    B517_DESIGN,
    "b517",
    "Design.bsv",
    &["Design.bsv"],
    &[line_count!("mkDesign.v", "actualSpeed$EN = 1'b1", 1)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkDesign")
    }
);

compile_case!(
    B518_METHOD_URGENCY,
    "b518",
    "MethodUrg.bsv",
    &["MethodUrg.bsv", "MethodUrg.bsv.bsc-sched-out.expected"],
    &[],
    CompileExpectation::Pass,
    golden!("MethodUrg.bsv.bsc-sched-out.expected"),
    &[],
    CompileMode::VerilogSchedule { module: None }
);

compile_case!(
    B568_DESIGN,
    "b568",
    "Design.bsv",
    &["Design.bsv"],
    &[line_count!("mkDesign.v", "state$EN = 1'b1", 0)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkDesign")
    }
);
compile_case!(
    B568_DESIGN_DEFAULT,
    "b568",
    "Design_def.bsv",
    &["Design_def.bsv"],
    &[line_count!("mkDesign_def.v", "state$EN = 1'd1", 1)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkDesign_def")
    }
);
compile_case!(
    B568_DESIGN_FULL,
    "b568",
    "Design_full.bsv",
    &["Design_full.bsv"],
    &[line_count!("mkDesign_full.v", "state$EN = 1'b1", 1)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkDesign_full")
    }
);

compile_case!(
    B628_TEST,
    "b628",
    "Test628.bsv",
    &["Test628.bsv", "Test628.bsv.bsc-vcomp-out.expected"],
    &[],
    CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "S0015",
        count: 1,
    },
    golden!("Test628.bsv.bsc-vcomp-out.expected"),
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B631_SELECT,
    "b631",
    "Select.bsv",
    &["Select.bsv", "Select.bsv.bsc-vcomp-out.expected"],
    &[],
    CompileExpectation::FailWithDiagnostic {
        kind: DiagnosticKind::Error,
        tag: "S0015",
        count: 1,
    },
    golden!("Select.bsv.bsc-vcomp-out.expected"),
    &[],
    CompileMode::Verilog { module: None }
);

compile_case!(
    B690_ALWAYS_READY,
    "b690",
    "AlwaysReadyOnMethods.bsv",
    &["AlwaysReadyOnMethods.bsv"],
    &[
        does_not_contain!("mkARTest.v", "RDY_do_foo"),
        does_not_contain!("mkARTest.v", "RDY_the_reg__write"),
        contains!("mkARTest.v", "RDY_the_reg__read"),
        contains!("mkARTest.v", "EN_do_foo"),
        contains!("mkARTest.v", "EN_the_reg__write"),
    ],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkARTest")
    }
);
compile_case!(
    B690_ALWAYS_ENABLED,
    "b690",
    "AlwaysEnabledOnMethods.bsv",
    &["AlwaysEnabledOnMethods.bsv"],
    &[
        does_not_contain!("mkAEFoo.v", "EN_the_reg__write"),
        does_not_contain!("mkAEFoo.v", "RDY_the_reg__write"),
        contains!("mkAEFoo.v", "RDY_the_reg__read"),
        contains!("mkAEFoo.v", "RDY_do_foo"),
        contains!("mkAEFoo.v", "EN_do_foo"),
        does_not_contain!("mkAETest.v", "EN_the_reg__write"),
        does_not_contain!("mkAETest.v", "RDY_the_reg__write"),
        contains!("mkAETest.v", "RDY_the_reg__read"),
        contains!("mkAETest.v", "RDY_do_foo"),
        contains!("mkAETest.v", "EN_do_foo"),
    ],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkAETest")
    }
);
compile_case!(
    B690_DIFFERENT_PRAGMAS,
    "b690",
    "DifferentPragmasSameInterface.bsv",
    &["DifferentPragmasSameInterface.bsv"],
    &[
        does_not_contain!("mkDiffAE.v", "EN_the_reg__write"),
        does_not_contain!("mkDiffAE.v", "RDY_the_reg__write"),
        contains!("mkDiffAE.v", "RDY_the_reg__read"),
        contains!("mkDiffAE.v", "RDY_do_foo"),
        contains!("mkDiffAE.v", "EN_do_foo"),
        does_not_contain!("mkDiffAR.v", "RDY_do_foo"),
        does_not_contain!("mkDiffAR.v", "RDY_the_reg__write"),
        contains!("mkDiffAR.v", "RDY_the_reg__read"),
        contains!("mkDiffAR.v", "EN_do_foo"),
        contains!("mkDiffAR.v", "EN_the_reg__write"),
    ],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkDiffTest")
    }
);

compile_case!(
    B765_FOUR,
    "b765",
    "Four.bsv",
    &["Four.bsv"],
    &[line_count!("mkFour.v", "state$EN = 1'b1", 1)],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkFour")
    }
);

compile_case!(
    B864_D,
    "b864",
    "D.bsv",
    &["D.bsv"],
    &[regex!("mkD.v", r"display.*4'd1")],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("mkD")
    }
);
compile_case!(
    B864_DEFS,
    "b864",
    "OptAcrossRWireInDefs.bsv",
    &["OptAcrossRWireInDefs.bsv"],
    &[contains!("sysOptAcrossRWireInDefs.v", "x$D_IN = x")],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("sysOptAcrossRWireInDefs")
    }
);
compile_case!(
    B864_FOREIGN,
    "b864",
    "OptAcrossRWireInForeign.bsv",
    &["OptAcrossRWireInForeign.bsv"],
    &[contains!("sysOptAcrossRWireInForeign.v", "display(x)")],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("sysOptAcrossRWireInForeign")
    }
);
compile_case!(
    B864_INSTANCE,
    "b864",
    "OptAcrossRWireInInst.bsv",
    &["OptAcrossRWireInInst.bsv"],
    &[contains!("sysOptAcrossRWireInInst.v", "foo(.x(x)")],
    CompileExpectation::Pass,
    None,
    &[],
    CompileMode::Verilog {
        module: Some("sysOptAcrossRWireInInst")
    }
);

compile_case!(
    B893_BUG,
    "b893",
    "Bug893.bsv",
    &["Bug893.bsv"],
    &[line_count!(
        "sysBug893.v",
        "assign RDY_get = r ? f1$EMPTY_N : f2$EMPTY_N ;",
        1
    )],
    CompileExpectation::Pass,
    None,
    &["-aggressive-conditions"],
    CompileMode::Verilog { module: None }
);

pub(super) const CASES: &[CompileCase] = &[
    B1018_CASE,
    B1066_TEST,
    B1066_TEST_2,
    B1249_BUG,
    B1390_TEST,
    B1390_TEST_2,
    B1402_TEST,
    B1619_A,
    B1619_B,
    B1619_C,
    B1621_BUG,
    B1690_MUT_EX,
    B1690_MUT_EX_BIG,
    B1720_BUG,
    B1753_BUG,
    B1753_SHADOW_IN_EXPR,
    B1753_SHADOW_IN_RULE,
    B1753_SHADOW_IN_METHOD,
    B1753_SHADOW_IN_PATTERN,
    B1753_METHOD_NAME_SHADOW,
    B1758_ZERO_BIT_VALUE_METHOD,
    B1758_ZERO_BIT_ACTION_VALUE_METHOD,
    B1758_ZERO_BIT_ACTION_VALUE_FOREIGN_WITH_ARGS,
    B232_DESIGN,
    B312_BUG,
    B323_TEST,
    B359_MODULE_BIND,
    B359_BUG,
    B359_BUG_2,
    B399_BUG,
    B484_DESIGN,
    B517_DESIGN,
    B518_METHOD_URGENCY,
    B568_DESIGN,
    B568_DESIGN_DEFAULT,
    B568_DESIGN_FULL,
    B628_TEST,
    B631_SELECT,
    B690_ALWAYS_READY,
    B690_ALWAYS_ENABLED,
    B690_DIFFERENT_PRAGMAS,
    B765_FOUR,
    B864_D,
    B864_DEFS,
    B864_FOREIGN,
    B864_INSTANCE,
    B893_BUG,
];
