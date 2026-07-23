//! Origin: `testsuite/bsc.scheduler/paths/paths.exp`.

use super::CompileCase;
use crate::upstream::{
    ArtifactAssertion, CompileExpectation, CompileMode, DiagnosticKind, Requirement, TextAssertion,
};

const FIXTURE_DIR: &str = "testsuite/bsc.scheduler/paths";

macro_rules! path_case {
    ($constant:ident, $source:literal, $expectation:expr, $assertions:expr) => {
        pub(super) const $constant: CompileCase = CompileCase {
            name: concat!("bsc.scheduler/paths::", $source),
            fixture_dir: FIXTURE_DIR,
            source: $source,
            fixtures: &[$source],
            assertions: $assertions,
            expectation: $expectation,
            golden: None,
            options: &[],
            nodeps: false,
            mode: CompileMode::Verilog { module: None },
            requirement: Requirement::VerilogEnabled,
        };
    };
}

macro_rules! pass {
    ($constant:ident, $source:literal) => {
        path_case!($constant, $source, CompileExpectation::Pass, &[]);
    };
    ($constant:ident, $source:literal, [$($assertion:expr),+ $(,)?]) => {
        path_case!(
            $constant,
            $source,
            CompileExpectation::Pass,
            &[$($assertion),+]
        );
    };
}

macro_rules! fail_g0032 {
    ($constant:ident, $source:literal) => {
        fail_g0032!($constant, $source, []);
    };
    ($constant:ident, $source:literal, [$($assertion:expr),* $(,)?]) => {
        path_case!(
            $constant,
            $source,
            CompileExpectation::FailWithDiagnostic {
                kind: DiagnosticKind::Error,
                tag: "G0032",
                count: 1,
            },
            &[$($assertion),*]
        );
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

pass!(
    METHOD_ENABLE_TO_WILL_FIRE,
    "MethodEnableToWillFire.bsv",
    [regex!(
        "mkMethodEnableToWillFire.v",
        r#"
// Combinational paths from inputs to outputs:
//   EN_inp -> result
"#
    )]
);

pass!(
    METHOD_ENABLE_TO_ARG_MUX,
    "MethodEnableToArgMux.bsv",
    [regex!(
        "sysMethodEnableToArgMux.v",
        r#"
// Combinational paths from inputs to outputs:
//   \(m_i\, EN_m\) -> m
"#
    )]
);

fail_g0032!(PORT_PATH, "PortPath.bsv");

pass!(
    SPLIT_OUTPUT_PATH,
    "SplitOutputPath.bs",
    [
        regex!(
            "mkSplitOutputPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_p1 -> get_p1
"#
        ),
        regex_does_not_match!("mkSplitOutputPath.v", r#"-> get_p2"#),
    ]
);

pass!(
    SPLIT_SUBMOD_PATH,
    "SplitSubmodPath.bs",
    [
        regex!(
            "mkSubmodPathComb.v",
            r#"
// Combinational paths from inputs to outputs:
//   top_1 -> top
"#
        ),
        regex!(
            "mkSubmodPathReg.v",
            r#"
// No combinational paths from inputs to outputs
"#
        ),
    ]
);

pass!(
    SPLIT_AV_PATH,
    "SplitAVPath.bs",
    [
        regex!(
            "mkSplitAVPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   upd_1_p1 -> upd_p1
"#
        ),
        regex_does_not_match!("mkSplitAVPath.v", r#"-> upd_p2"#),
    ]
);

fail_g0032!(SPLIT_OUTPUT_LOOP, "SplitOutputLoop.bs");

pass!(
    DEEP_LEAF_BASIC,
    "DeepLeafBasic.bs",
    [
        regex!(
            "mkDeepLeafBasic.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_x_a -> get_x_a
//   get_1_y_b -> get_y_b
"#
        ),
        regex_does_not_match!("mkDeepLeafBasic.v", r#"-> get_x_b"#),
        regex_does_not_match!("mkDeepLeafBasic.v", r#"-> get_y_a"#),
    ]
);

pass!(
    DEEP_LEAF_CROSS,
    "DeepLeafCross.bs",
    [
        regex!(
            "mkDeepLeafCross.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_x_a -> get_y_b
//   get_1_y_b -> get_x_a
"#
        ),
        regex_does_not_match!("mkDeepLeafCross.v", r#"get_1_x_a -> get_x_a"#),
        regex_does_not_match!("mkDeepLeafCross.v", r#"get_1_y_b -> get_y_b"#),
    ]
);

pass!(
    DEEP_LEAF_TWO_LEVELS,
    "DeepLeafTwoLevels.bs",
    [
        regex!(
            "mkDeepLeafTwoLevels.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_m_p_lo -> get_m_p_lo
//   get_1_n_q_hi -> get_n_q_hi
"#
        ),
        regex_does_not_match!("mkDeepLeafTwoLevels.v", r#"-> get_m_p_hi"#),
        regex_does_not_match!("mkDeepLeafTwoLevels.v", r#"get_1_m_q_lo ->"#),
    ]
);

pass!(
    DEEP_LEAF_CONST,
    "DeepLeafConst.bs",
    [
        regex!(
            "mkDeepLeafConst.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_x_a -> get_x_a
"#
        ),
        regex_does_not_match!("mkDeepLeafConst.v", r#"-> get_x_b"#),
        regex_does_not_match!("mkDeepLeafConst.v", r#"-> get_y_a"#),
        regex_does_not_match!("mkDeepLeafConst.v", r#"get_1_y_b ->"#),
    ]
);

fail_g0032!(DEEP_LEAF_LOOP, "DeepLeafLoop.bs");

pass!(
    IO_MATRIX_CROSS,
    "IOMatrixCross.bs",
    [
        regex!(
            "mkIOMatrixCross.v",
            r#"
//   get_1_p1 -> get_p2
//   get_1_p2 -> get_p1
"#
        ),
        regex_does_not_match!("mkIOMatrixCross.v", r#"get_1_p1 -> get_p1"#),
        regex_does_not_match!("mkIOMatrixCross.v", r#"get_1_p2 -> get_p2"#),
    ]
);

pass!(
    IO_MATRIX_COMBINE,
    "IOMatrixCombine.bs",
    [
        regex!("mkIOMatrixCombine.v", r#"\(get_1_p1, get_1_p2\) -> get_p1"#),
        regex_does_not_match!("mkIOMatrixCombine.v", r#"-> get_p2"#),
    ]
);

pass!(
    IO_MATRIX_FANOUT,
    "IOMatrixFanout.bs",
    [
        regex!(
            "mkIOMatrixFanout.v",
            r#"
//   get_1_p1 -> get_p1
//   get_1_p1 -> get_p2
"#
        ),
        regex_does_not_match!("mkIOMatrixFanout.v", r#"get_1_p2 ->"#),
    ]
);

pass!(
    IO_MATRIX_DEEP_CROSS,
    "IOMatrixDeepCross.bs",
    [
        regex!(
            "mkIOMatrixDeepCross.v",
            r#"
//   get_1_x_a -> get_x_b
//   get_1_x_b -> get_x_a
//   get_1_y -> get_y
"#
        ),
        regex_does_not_match!("mkIOMatrixDeepCross.v", r#"get_1_x_a -> get_x_a"#),
        regex_does_not_match!("mkIOMatrixDeepCross.v", r#"get_1_x_b -> get_x_b"#),
    ]
);

pass!(
    IO_MATRIX_DEEP_COMBINE,
    "IOMatrixDeepCombine.bs",
    [
        regex!(
            "mkIOMatrixDeepCombine.v",
            r#"\(get_1_x_a, get_1_y\) -> get_x_a"#
        ),
        regex!(
            "mkIOMatrixDeepCombine.v",
            r#"\(get_1_x_b, get_1_y\) -> get_x_b"#
        ),
        regex_does_not_match!("mkIOMatrixDeepCombine.v", r#"-> get_y"#),
    ]
);

pass!(
    AV_SPLIT_IN_PATH,
    "AVSplitInPath.bs",
    [
        regex!(
            "mkAVSplitInPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   upd_1_p1 -> upd_p2
//   upd_1_p2 -> upd_p1
"#
        ),
        regex_does_not_match!("mkAVSplitInPath.v", r#"upd_1_p1 -> upd_p1"#),
        regex_does_not_match!("mkAVSplitInPath.v", r#"upd_1_p2 -> upd_p2"#),
    ]
);

pass!(
    AV_SUBMOD_IN_PATH,
    "AVSubmodInPath.bs",
    [
        regex!(
            "mkAVSubmodInPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   upd_1_p1 -> upd_p1
"#
        ),
        regex_does_not_match!("mkAVSubmodInPath.v", r#"upd_1_p2 ->"#),
        regex_does_not_match!("mkAVSubmodInPath.v", r#"-> upd_p2"#),
    ]
);

pass!(
    AV_EN_ARG_MUX_PATH,
    "AVEnArgMuxPath.bs",
    [
        regex!(
            "mkAVEnArgMuxPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   upd_1_p1 -> upd_p1
"#
        ),
        regex_does_not_match!("mkAVEnArgMuxPath.v", r#"EN_upd ->"#),
        regex_does_not_match!("mkAVEnArgMuxPath.v", r#"-> upd_p2"#),
    ]
);

pass!(
    SUBMOD_INPUT_ARG_PATH,
    "SubmodInputArgPath.bs",
    [
        regex!(
            "mkSubmodInputArgPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   top_1_p1 -> top_p1
"#
        ),
        regex_does_not_match!("mkSubmodInputArgPath.v", r#"-> top_p2"#),
    ]
);

pass!(
    SUBMOD_DEEP_LEAF_PATH,
    "SubmodDeepLeafPath.bs",
    [
        regex!(
            "mkSubmodDeepLeafPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   top_1_x_a -> top_x_a
//   top_1_y -> top_y
"#
        ),
        regex_does_not_match!("mkSubmodDeepLeafPath.v", r#"-> top_x_b"#),
    ]
);

pass!(
    SUBMOD_SELECT_OUT_PATH,
    "SubmodSelectOutPath.bs",
    [regex!(
        "mkSubmodSelectOutPath.v",
        r#"
// Combinational paths from inputs to outputs:
//   \(top_1\, top_2\) -> top
"#
    )]
);

pass!(
    SUBMOD_CROSSBAR_PATH,
    "SubmodCrossbarPath.bs",
    [
        regex!(
            "mkSubmodCrossbarPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   top_1_q1 -> top_q2
//   top_1_q2 -> top_q1
"#
        ),
        regex_does_not_match!("mkSubmodCrossbarPath.v", r#"top_1_q1 -> top_q1"#),
    ]
);

pass!(
    LOOP_NO_FALSE_LOOP,
    "LoopNoFalseLoop.bs",
    [regex!(
        "mkLoopNoFalseLoop.v",
        r#"
// Combinational paths from inputs to outputs:
//   top_1 -> top
"#
    )]
);

fail_g0032!(LOOP_INPUT_ARG_LOOP, "LoopInputArgLoop.bs");
fail_g0032!(LOOP_AV_PORT_LOOP, "LoopAVPortLoop.bs");

pass!(
    LOOP_SWAP_NO_LOOP,
    "LoopSwapNoLoop.bs",
    [regex!(
        "mkLoopSwapNoLoop.v",
        r#"
// No combinational paths from inputs to outputs
"#
    )]
);

pass!(
    GRAN_SHALLOW_NESTED,
    "GranShallowNested.bs",
    [
        regex!(
            "mkGranShallowNested.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_x -> get_x
"#
        ),
        regex_does_not_match!("mkGranShallowNested.v", r#"get_x_a"#),
    ]
);

pass!(
    GRAN_DEEP_NESTED,
    "GranDeepNested.bs",
    [
        regex!(
            "mkGranDeepNested.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_x_a -> get_x_a
"#
        ),
        regex_does_not_match!("mkGranDeepNested.v", r#"-> get_x_b"#),
        regex_does_not_match!("mkGranDeepNested.v", r#"-> get_y"#),
    ]
);

pass!(
    GRAN_TWO_ARGS,
    "GranTwoArgs.bs",
    [
        regex!(
            "mkGranTwoArgs.v",
            r#"
// Combinational paths from inputs to outputs:
//   combine_lhs_p1 -> combine_p1
//   combine_rhs_p2 -> combine_p2
"#
        ),
        regex_does_not_match!("mkGranTwoArgs.v", r#"combine_lhs_p2 -> combine_p2"#),
        regex_does_not_match!("mkGranTwoArgs.v", r#"combine_rhs_p1 -> combine_p1"#),
    ]
);

pass!(
    GRAN_VEC,
    "GranVec.bs",
    [
        regex!(
            "mkGranVec.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_2 -> get_2
"#
        ),
        regex_does_not_match!("mkGranVec.v", r#"-> get_0"#),
    ]
);

pass!(
    GRAN_NO_SPLIT,
    "GranNoSplit.bs",
    [
        regex!(
            "mkGranNoSplit.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_x -> get_x
"#
        ),
        regex_does_not_match!("mkGranNoSplit.v", r#"get_x_a"#),
        regex_does_not_match!("mkGranNoSplit.v", r#"-> get_y"#),
    ]
);

pass!(
    EN_RDY_EN_TO_SPLIT_OUT,
    "EnRdyEnToSplitOut.bs",
    [
        regex!(
            "mkEnRdyEnToSplitOut.v",
            r#"
// Combinational paths from inputs to outputs:
//   EN_doit -> rd_p1
"#
        ),
        regex_does_not_match!("mkEnRdyEnToSplitOut.v", r#"-> rd_p2"#),
    ]
);

pass!(
    EN_RDY_IN_TO_RDY,
    "EnRdyInToRdy.bs",
    [
        regex!(
            "mkEnRdyInToRdy.v",
            r#"
// Combinational paths from inputs to outputs:
//   \(put_1_p1, EN_put\) -> RDY_gate
"#
        ),
        regex_does_not_match!("mkEnRdyInToRdy.v", r#"put_1_p2 ->"#),
        regex_does_not_match!("mkEnRdyInToRdy.v", r#"-> RDY_put"#),
    ]
);

pass!(
    EN_RDY_DEEP_EN_OUT,
    "EnRdyDeepEnOut.bs",
    [
        regex!(
            "mkEnRdyDeepEnOut.v",
            r#"
// Combinational paths from inputs to outputs:
//   EN_fire -> rd_x_a
"#
        ),
        regex_does_not_match!("mkEnRdyDeepEnOut.v", r#"-> rd_x_b"#),
        regex_does_not_match!("mkEnRdyDeepEnOut.v", r#"-> rd_y"#),
    ]
);

pass!(
    EN_RDY_AV_EN_RESULT,
    "EnRdyAVEnResult.bs",
    [
        regex!(
            "mkEnRdyAVEnResult.v",
            r#"
// Combinational paths from inputs to outputs:
//   upd_1_p1 -> upd_p1
//   upd_1_p2 -> upd_p2
//   EN_upd -> busy
"#
        ),
        regex_does_not_match!("mkEnRdyAVEnResult.v", r#"upd_1_p1 -> upd_p2"#),
        regex_does_not_match!("mkEnRdyAVEnResult.v", r#"upd_1_p2 -> upd_p1"#),
    ]
);

pass!(
    EN_RDY_TWO_EN_SPLIT,
    "EnRdyTwoEnSplit.bs",
    [
        regex!(
            "mkEnRdyTwoEnSplit.v",
            r#"
// Combinational paths from inputs to outputs:
//   EN_setA -> rd_p1
//   EN_setB -> rd_p2
"#
        ),
        regex_does_not_match!("mkEnRdyTwoEnSplit.v", r#"EN_setA -> rd_p2"#),
        regex_does_not_match!("mkEnRdyTwoEnSplit.v", r#"EN_setB -> rd_p1"#),
    ]
);

pass!(
    SPLIT_IN_PACKED_ONE_FIELD,
    "SplitInPackedOneField.bs",
    [
        regex!(
            "mkSplitInPackedOneField.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_p1 -> get
"#
        ),
        regex_does_not_match!("mkSplitInPackedOneField.v", r#"get_1_p2 -> get"#),
    ]
);

pass!(
    PACKED_IN_SPLIT_OUT,
    "PackedInSplitOut.bs",
    [
        regex!(
            "mkPackedInSplitOut.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1 -> get_p1
"#
        ),
        regex_does_not_match!("mkPackedInSplitOut.v", r#"get_1 -> get_p2"#),
    ]
);

pass!(
    SPLIT_IN_PACKED_COMBINE,
    "SplitInPackedCombine.bs",
    [
        regex!(
            "mkSplitInPackedCombine.v",
            r#"
// Combinational paths from inputs to outputs:
//   \(get_1_p1\, get_1_p2\) -> get
"#
        ),
        regex_does_not_match!("mkSplitInPackedCombine.v", r#"get_1_p1 -> get"#),
    ]
);

pass!(
    RWIRE_FANOUT,
    "RWireFanout.bs",
    [
        regex!(
            "mkRWireFanout.v",
            r#"// Combinational paths from inputs to outputs:
//   \(put_1_p1\, EN_put\) -> outA
//   \(put_1_p1\, EN_put\) -> outB"#
        ),
        regex_does_not_match!("mkRWireFanout.v", r#"put_1_p2 ->"#),
    ]
);

pass!(
    DEEP_WIRE_FANOUT,
    "DeepWireFanout.bs",
    [
        regex!(
            "mkDeepWireFanout.v",
            r#"// Combinational paths from inputs to outputs:
//   \(put_1_a_x\, EN_put\) -> outA
//   \(put_1_a_x\, EN_put\) -> outB"#
        ),
        regex_does_not_match!("mkDeepWireFanout.v", r#"put_1_a_y ->"#),
        regex_does_not_match!("mkDeepWireFanout.v", r#"put_1_b ->"#),
    ]
);

pass!(
    SIBLING_DISJOINT_FANOUT,
    "SiblingDisjointFanout.bs",
    [
        regex!(
            "mkSiblingDisjointFanout.v",
            r#"// Combinational paths from inputs to outputs:
//   \(put_1_p1\, EN_put\) -> outA
//   \(put_1_p1\, EN_put\) -> outB
//   \(put_1_p2\, EN_put\) -> outC"#
        ),
        regex_does_not_match!(
            "mkSiblingDisjointFanout.v",
            r#"put_1_p1\, EN_put\) -> outC"#
        ),
        regex_does_not_match!(
            "mkSiblingDisjointFanout.v",
            r#"put_1_p2\, EN_put\) -> outA"#
        ),
    ]
);

pass!(
    MAYBE_SHALLOW,
    "MaybeShallow.bs",
    [
        regex!(
            "mkMaybeShallow.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_mb -> get_flag
"#
        ),
        regex_does_not_match!("mkMaybeShallow.v", r#"-> get_mb"#),
        regex_does_not_match!("mkMaybeShallow.v", r#"get_1_flag ->"#),
    ]
);

pass!(
    MAYBE_DEEP,
    "MaybeDeep.bs",
    [
        regex!(
            "mkMaybeDeep.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_mb -> get_mb
"#
        ),
        regex_does_not_match!("mkMaybeDeep.v", r#"-> get_lo"#),
        regex_does_not_match!("mkMaybeDeep.v", r#"get_1_lo ->"#),
        regex_does_not_match!("mkMaybeDeep.v", r#"get_1_mb_"#),
    ]
);

pass!(
    UNION_PAYLOAD,
    "UnionPayload.bs",
    [
        regex!(
            "mkUnionPayload.v",
            r#"
// Combinational paths from inputs to outputs:
//   step_1_cmd -> step_echo
"#
        ),
        regex_does_not_match!("mkUnionPayload.v", r#"-> step_ack"#),
        regex_does_not_match!("mkUnionPayload.v", r#"step_1_sel ->"#),
    ]
);

pass!(
    SELECTOR_MUX_PATH,
    "SelectorMuxPath.bs",
    [
        regex!(
            "mkSelectorMuxPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   \(run_1_sel, run_1_a, run_1_b\) -> run
"#
        ),
        regex_does_not_match!("mkSelectorMuxPath.v", r#"run_1_dead ->"#),
    ]
);

pass!(
    SELECTOR_SPLIT_OUT_PATH,
    "SelectorSplitOutPath.bs",
    [
        regex!(
            "mkSelectorSplitOutPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   \(run_1_sel, run_1_a\) -> run_o1
//   \(run_1_sel, run_1_b\) -> run_o2
"#
        ),
        regex_does_not_match!("mkSelectorSplitOutPath.v", r#"run_1_b\) -> run_o1"#),
        regex_does_not_match!("mkSelectorSplitOutPath.v", r#"run_1_a\) -> run_o2"#),
        regex_does_not_match!("mkSelectorSplitOutPath.v", r#"run_1_dead.*->"#),
    ]
);

pass!(
    ARG_SELECTOR_PATH,
    "ArgSelectorPath.bs",
    [
        regex!(
            "mkArgSelectorPath.v",
            r#"
// Combinational paths from inputs to outputs:
//   \(run_1_sel, run_1_d, v\) -> run
"#
        ),
        regex_does_not_match!("mkArgSelectorPath.v", r#"run_1_dead ->"#),
    ]
);

pass!(
    VEC_STRUCT_DEEP_MULTI,
    "VecStructDeepMulti.bs",
    [
        regex!(
            "mkVecStructDeepMulti.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_0_p1 -> get_0_p1
//   get_1_1_p2 -> get_2_p2
"#
        ),
        regex_does_not_match!("mkVecStructDeepMulti.v", r#"get_1_0_p2 -> get_0_p1"#),
        regex_does_not_match!("mkVecStructDeepMulti.v", r#"get_1_2_p2 -> get_2_p2"#),
    ]
);

pass!(
    VEC_STRUCT_SHALLOW,
    "VecStructShallow.bs",
    [
        regex!(
            "mkVecStructShallow.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_0 -> get_0
//   get_1_1 -> get_2
"#
        ),
        regex_does_not_match!("mkVecStructShallow.v", r#"get_0_p1"#),
        regex_does_not_match!("mkVecStructShallow.v", r#"get_1_2 ->"#),
    ]
);

pass!(
    VEC_NESTED_DEEP,
    "VecNestedDeep.bs",
    [
        regex!(
            "mkVecNestedDeep.v",
            r#"
// Combinational paths from inputs to outputs:
//   get_1_0_c_a -> get_0_c_a
//   get_1_0_d -> get_1_d
"#
        ),
        regex_does_not_match!("mkVecNestedDeep.v", r#"get_1_1_d -> get_1_d"#),
        regex_does_not_match!("mkVecNestedDeep.v", r#"-> get_0_c_b"#),
    ]
);

fail_g0032!(
    LOOP_CONFINED_P1,
    "LoopConfinedP1.bs",
    [
        regex!(
            "LoopConfinedP1.bs.bsc-out",
            r#"Argument 1 port 1 of method `get'"#
        ),
        regex!(
            "LoopConfinedP1.bs.bsc-out",
            r#"Return value 1 of method `get'"#
        ),
        regex_does_not_match!(
            "LoopConfinedP1.bs.bsc-out",
            r#"Argument 1 port 2 of method `get'"#
        ),
        regex_does_not_match!(
            "LoopConfinedP1.bs.bsc-out",
            r#"Return value 2 of method `get'"#
        ),
    ]
);

fail_g0032!(
    LOOP_CONFINED_P2,
    "LoopConfinedP2.bs",
    [
        regex!(
            "LoopConfinedP2.bs.bsc-out",
            r#"Argument 1 port 2 of method `get'"#
        ),
        regex!(
            "LoopConfinedP2.bs.bsc-out",
            r#"Return value 2 of method `get'"#
        ),
        regex_does_not_match!(
            "LoopConfinedP2.bs.bsc-out",
            r#"Argument 1 port 1 of method `get'"#
        ),
        regex_does_not_match!(
            "LoopConfinedP2.bs.bsc-out",
            r#"Return value 1 of method `get'"#
        ),
    ]
);

fail_g0032!(
    LOOP_CONFINED_ARG2,
    "LoopConfinedArg2.bs",
    [
        regex!(
            "LoopConfinedArg2.bs.bsc-out",
            r#"Argument 2 port 1 of method `op'"#
        ),
        regex!(
            "LoopConfinedArg2.bs.bsc-out",
            r#"Return value 1 of method `op'"#
        ),
        regex_does_not_match!(
            "LoopConfinedArg2.bs.bsc-out",
            r#"Argument 1 port 1 of method `op'"#
        ),
        regex_does_not_match!(
            "LoopConfinedArg2.bs.bsc-out",
            r#"Argument 1 port 2 of method `op'"#
        ),
        regex_does_not_match!(
            "LoopConfinedArg2.bs.bsc-out",
            r#"Return value 2 of method `op'"#
        ),
    ]
);

pub(super) const CASES: &[CompileCase] = &[
    METHOD_ENABLE_TO_WILL_FIRE,
    METHOD_ENABLE_TO_ARG_MUX,
    PORT_PATH,
    SPLIT_OUTPUT_PATH,
    SPLIT_SUBMOD_PATH,
    SPLIT_AV_PATH,
    SPLIT_OUTPUT_LOOP,
    DEEP_LEAF_BASIC,
    DEEP_LEAF_CROSS,
    DEEP_LEAF_TWO_LEVELS,
    DEEP_LEAF_CONST,
    DEEP_LEAF_LOOP,
    IO_MATRIX_CROSS,
    IO_MATRIX_COMBINE,
    IO_MATRIX_FANOUT,
    IO_MATRIX_DEEP_CROSS,
    IO_MATRIX_DEEP_COMBINE,
    AV_SPLIT_IN_PATH,
    AV_SUBMOD_IN_PATH,
    AV_EN_ARG_MUX_PATH,
    SUBMOD_INPUT_ARG_PATH,
    SUBMOD_DEEP_LEAF_PATH,
    SUBMOD_SELECT_OUT_PATH,
    SUBMOD_CROSSBAR_PATH,
    LOOP_NO_FALSE_LOOP,
    LOOP_INPUT_ARG_LOOP,
    LOOP_AV_PORT_LOOP,
    LOOP_SWAP_NO_LOOP,
    GRAN_SHALLOW_NESTED,
    GRAN_DEEP_NESTED,
    GRAN_TWO_ARGS,
    GRAN_VEC,
    GRAN_NO_SPLIT,
    EN_RDY_EN_TO_SPLIT_OUT,
    EN_RDY_IN_TO_RDY,
    EN_RDY_DEEP_EN_OUT,
    EN_RDY_AV_EN_RESULT,
    EN_RDY_TWO_EN_SPLIT,
    SPLIT_IN_PACKED_ONE_FIELD,
    PACKED_IN_SPLIT_OUT,
    SPLIT_IN_PACKED_COMBINE,
    RWIRE_FANOUT,
    DEEP_WIRE_FANOUT,
    SIBLING_DISJOINT_FANOUT,
    MAYBE_SHALLOW,
    MAYBE_DEEP,
    UNION_PAYLOAD,
    SELECTOR_MUX_PATH,
    SELECTOR_SPLIT_OUT_PATH,
    ARG_SELECTOR_PATH,
    VEC_STRUCT_DEEP_MULTI,
    VEC_STRUCT_SHALLOW,
    VEC_NESTED_DEEP,
    LOOP_CONFINED_P1,
    LOOP_CONFINED_P2,
    LOOP_CONFINED_ARG2,
];
