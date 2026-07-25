//! Origins:
//! - `testsuite/bsc.interra/bugs/bugID142/bugID142.exp`
//! - `testsuite/bsc.interra/bugs/bugID156/bugID156.exp`
//! - `testsuite/bsc.interra/bugs/bugID231/bugID231.exp`
//! - `testsuite/bsc.interra/bugs/bugID265/bugID265.exp`
//! - `testsuite/bsc.interra/bugs/bugID336/bugID336.exp`
//! - `testsuite/bsc.interra/bugs/bugID363/bugID363.exp`
//! - `testsuite/bsc.interra/bugs/bugID364/bugID364.exp`
//! - `testsuite/bsc.interra/bugs/bugID383/bugID383.exp`
//! - `testsuite/bsc.interra/bugs/bugID413/bugID413.exp`
//! - `testsuite/bsc.interra/bugs/bugID415/bugID415.exp`
//! - `testsuite/bsc.interra/messages/EArbitrate/EArbitrate.exp`
//! - `testsuite/bsc.interra/messages/EBadVeriType/EBadVeriType.exp`
//! - `testsuite/bsc.interra/messages/EBigLiteral/EBigLiteral.exp`
//! - `testsuite/bsc.interra/messages/EBitSel/EBitSel.exp`
//! - `testsuite/bsc.interra/messages/EGeneric/EGeneric.exp`
//! - `testsuite/bsc.interra/messages/EHasImplicit/EHasImplicit.exp`
//! - `testsuite/bsc.interra/messages/ERTSHeapExhausted/ERTSHeapExhausted.exp`
//! - `testsuite/bsc.interra/messages/ERTSOutOfMemory/ERTSOutOfMemory.exp`
//! - `testsuite/bsc.interra/messages/ERTSStackOverflow/ERTSStackOverflow.exp`
//! - `testsuite/bsc.interra/messages/ERuleAssertion/ERuleAssertion.exp`
//! - `testsuite/bsc.interra/messages/WCycleDrop/WCycleDrop.exp`
//! - `testsuite/bsc.interra/messages/WMissingRule/WMissingRule.exp`
//! - `testsuite/bsc.interra/messages/WUrgencyChoice/WUrgencyChoice.exp`

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, GoldenExpectation, Requirement,
    TextAssertion,
};

macro_rules! verilog_golden_case {
    (
        $constant:ident,
        $prefix:literal,
        $fixture_dir:literal,
        $source:literal,
        $top:literal,
        $expectation:expr,
        $options:expr
        $(, $extra_fixture:literal)*
        $(,)?
    ) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!($prefix, "::", $source),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[
                $source,
                concat!($source, ".bsc-vcomp-out.expected"),
                $($extra_fixture),*
            ],
            assertions: &[],
            expectation: $expectation,
            golden: Some(GoldenExpectation {
                expected: concat!($source, ".bsc-vcomp-out.expected"),
            }),
            options: $options,
            nodeps: false,
            mode: CompileMode::Verilog {
                module: Some($top),
            },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! verilog_find_case {
    (
        $constant:ident,
        $prefix:literal,
        $fixture_dir:literal,
        $source:literal,
        $top:literal,
        $text:literal,
        $count:literal,
        $options:expr
        $(,)?
    ) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!($prefix, "::", $source),
            fixture_dir: $fixture_dir,
            source: $source,
            fixtures: &[$source],
            assertions: &[ArtifactAssertion::Text {
                path: concat!($source, ".bsc-out"),
                assertion: TextAssertion::LineCount {
                    text: $text,
                    count: $count,
                },
            }],
            expectation: CompileExpectation::Fail,
            golden: None,
            options: $options,
            nodeps: false,
            mode: CompileMode::Verilog { module: Some($top) },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

verilog_golden_case!(
    BUG_ID_142_GCD,
    "bsc.interra/bugs/bugID142",
    "testsuite/bsc.interra/bugs/bugID142",
    "GCD.bs",
    "mkGCD",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    BUG_ID_156_SIZEBUG,
    "bsc.interra/bugs/bugID156",
    "testsuite/bsc.interra/bugs/bugID156",
    "Sizebug.bs",
    "top",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    BUG_ID_231_TEST_SYN,
    "bsc.interra/bugs/bugID231",
    "testsuite/bsc.interra/bugs/bugID231",
    "TestSyn.bs",
    "mkStack",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    BUG_ID_265_DESIGN_0,
    "bsc.interra/bugs/bugID265",
    "testsuite/bsc.interra/bugs/bugID265",
    "Design_0.bsv",
    "mkDesign_in",
    CompileExpectation::Pass,
    &[]
);
// The upstream script compiles Design_0 first in the same directory. Rust compile cases are
// isolated, so stage the source dependency and let this case's deterministic `-u` rebuild it.
verilog_golden_case!(
    BUG_ID_265_DESIGN_1,
    "bsc.interra/bugs/bugID265",
    "testsuite/bsc.interra/bugs/bugID265",
    "Design_1.bsv",
    "mkDesign",
    CompileExpectation::Pass,
    &[],
    "Design_0.bsv"
);
verilog_golden_case!(
    BUG_ID_336_TEST,
    "bsc.interra/bugs/bugID336",
    "testsuite/bsc.interra/bugs/bugID336",
    "Test.bsv",
    "mkTestbench",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    BUG_ID_363_DESIGN,
    "bsc.interra/bugs/bugID363",
    "testsuite/bsc.interra/bugs/bugID363",
    "Design.bsv",
    "mkDesign",
    CompileExpectation::Pass,
    &["-aggressive-conditions"]
);
verilog_golden_case!(
    BUG_ID_364_TRY,
    "bsc.interra/bugs/bugID364",
    "testsuite/bsc.interra/bugs/bugID364",
    "Try.bsv",
    "mkTestbench",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    BUG_ID_383_COMPLETION_BUFFER,
    "bsc.interra/bugs/bugID383",
    "testsuite/bsc.interra/bugs/bugID383",
    "MkCompletionBuffer.bsv",
    "mkDesign",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    BUG_ID_413_DESIGN,
    "bsc.interra/bugs/bugID413",
    "testsuite/bsc.interra/bugs/bugID413",
    "Design.bsv",
    "mkDesign",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    BUG_ID_415_DESIGN,
    "bsc.interra/bugs/bugID415",
    "testsuite/bsc.interra/bugs/bugID415",
    "Design.bsv",
    "mkDesign",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    BUG_ID_415_ALTERNATE,
    "bsc.interra/bugs/bugID415",
    "testsuite/bsc.interra/bugs/bugID415",
    "Alternate.bsv",
    "mkAlternate",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    E_ARBITRATE,
    "bsc.interra/messages/EArbitrate",
    "testsuite/bsc.interra/messages/EArbitrate",
    "ResourceTwoRules.bs",
    "sysResourceTwoRules",
    CompileExpectation::Pass,
    &["-resource-simple"]
);
verilog_golden_case!(
    E_BAD_VERI_TYPE,
    "bsc.interra/messages/EBadVeriType",
    "testsuite/bsc.interra/messages/EBadVeriType",
    "Fib.bs",
    "mkFib8",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    E_BIG_LITERAL,
    "bsc.interra/messages/EBigLiteral",
    "testsuite/bsc.interra/messages/EBigLiteral",
    "EBigLiteral.bs",
    "mkGCD",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    E_BIT_SEL,
    "bsc.interra/messages/EBitSel",
    "testsuite/bsc.interra/messages/EBitSel",
    "EBitSel.bs",
    "mkShifter64",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    E_GENERIC,
    "bsc.interra/messages/EGeneric",
    "testsuite/bsc.interra/messages/EGeneric",
    "EGeneric.bs",
    "mkTest",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    E_HAS_IMPLICIT_2,
    "bsc.interra/messages/EHasImplicit",
    "testsuite/bsc.interra/messages/EHasImplicit",
    "EHasImplicit2.bs",
    "mkTop",
    CompileExpectation::Fail,
    &[]
);
// These scripts use `find_n_strings`, not their commented-out golden comparisons. The Rust
// runner captures each BSC invocation in `<source>.bsc-out`, which is the asserted artifact.
verilog_find_case!(
    E_RTS_HEAP_EXHAUSTED_1,
    "bsc.interra/messages/ERTSHeapExhausted",
    "testsuite/bsc.interra/messages/ERTSHeapExhausted",
    "ERTSHeapExhausted1.bs",
    "mkERTSHeapExhausted1",
    "Heap exhausted",
    1,
    &["+RTS", "-H1M", "-M1M", "-RTS"]
);
verilog_find_case!(
    E_RTS_OUT_OF_MEMORY_1,
    "bsc.interra/messages/ERTSOutOfMemory",
    "testsuite/bsc.interra/messages/ERTSOutOfMemory",
    "ERTSOutOfMemory1.bs",
    "mkERTSOutOfMemory1",
    "Heap exhausted",
    1,
    &["+RTS", "-A128M", "-M128M", "-H128M", "-RTS"]
);
verilog_find_case!(
    E_RTS_STACK_OVERFLOW_1,
    "bsc.interra/messages/ERTSStackOverflow",
    "testsuite/bsc.interra/messages/ERTSStackOverflow",
    "ERTSStackOverflow1.bs",
    "mkERTSStackOverflow1",
    "Stack space overflow",
    1,
    &["+RTS", "-K1M", "-RTS"]
);
verilog_golden_case!(
    E_RULE_ASSERTION,
    "bsc.interra/messages/ERuleAssertion",
    "testsuite/bsc.interra/messages/ERuleAssertion",
    "ERuleAssertion.bs",
    "mkGCD",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    E_RULE_ASSERTION_2,
    "bsc.interra/messages/ERuleAssertion",
    "testsuite/bsc.interra/messages/ERuleAssertion",
    "ERuleAssertion2.bs",
    "mkGCDTest",
    CompileExpectation::Fail,
    &[]
);
verilog_golden_case!(
    W_CYCLE_DROP_1,
    "bsc.interra/messages/WCycleDrop",
    "testsuite/bsc.interra/messages/WCycleDrop",
    "WCycleDrop1.bs",
    "mkWCycleDrop1",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    W_MISSING_RULE_1,
    "bsc.interra/messages/WMissingRule",
    "testsuite/bsc.interra/messages/WMissingRule",
    "WMissingRule1.bs",
    "mkWMissingRule1",
    CompileExpectation::Pass,
    &["-show-rule-rel", "Five", "Six"]
);
verilog_golden_case!(
    W_URGENCY_CHOICE_1,
    "bsc.interra/messages/WUrgencyChoice",
    "testsuite/bsc.interra/messages/WUrgencyChoice",
    "WUrgencyChoice1.bs",
    "mkWUrgencyChoice1",
    CompileExpectation::Pass,
    &[]
);
verilog_golden_case!(
    W_URGENCY_CHOICE_2,
    "bsc.interra/messages/WUrgencyChoice",
    "testsuite/bsc.interra/messages/WUrgencyChoice",
    "WUrgencyChoice2.bs",
    "mkWUrgencyChoice2",
    CompileExpectation::Pass,
    &[]
);

pub(super) const CASES: &[CompileCase] = &[
    BUG_ID_142_GCD,
    BUG_ID_156_SIZEBUG,
    BUG_ID_231_TEST_SYN,
    BUG_ID_265_DESIGN_0,
    BUG_ID_265_DESIGN_1,
    BUG_ID_336_TEST,
    BUG_ID_363_DESIGN,
    BUG_ID_364_TRY,
    BUG_ID_383_COMPLETION_BUFFER,
    BUG_ID_413_DESIGN,
    BUG_ID_415_DESIGN,
    BUG_ID_415_ALTERNATE,
    E_ARBITRATE,
    E_BAD_VERI_TYPE,
    E_BIG_LITERAL,
    E_BIT_SEL,
    E_GENERIC,
    E_HAS_IMPLICIT_2,
    E_RTS_HEAP_EXHAUSTED_1,
    E_RTS_OUT_OF_MEMORY_1,
    E_RTS_STACK_OVERFLOW_1,
    E_RULE_ASSERTION,
    E_RULE_ASSERTION_2,
    W_CYCLE_DROP_1,
    W_MISSING_RULE_1,
    W_URGENCY_CHOICE_1,
    W_URGENCY_CHOICE_2,
];
