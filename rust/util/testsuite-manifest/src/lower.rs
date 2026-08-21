use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Tree};

use crate::model::{
    ArtifactTransferAction, ArtifactTransferOperation, AssertionContract, BasicOptionsContract,
    BluetclInvocation, BluetclRunAction, BluetclSyntax, Bsc2BsvAction, BscOptionsOverlay,
    BscParsePrettyAction, CObjectBuildAction, Capability, ComparisonContract, CompileContract,
    CompileObjectAction, Contract, CreateDirectoryAction, DelayAction, DumpIntermediateAction,
    EnsureDirectoryAbsentAction, EraseArtifactAction, ExternalContractKind, ExternalSetContract,
    GenerationStrategy, GoldenMacroValue, Guard, IntermediateDumpView, LinkErrorDiagnostic,
    LinkObjectsAction, LinkVerilogAction, MakeTestDataAction, NoSourceCompileContract, OvlContract,
    RemoveUserReadAction, RenderGoldenAction, RenderGoldenContract, RenderM4CurdirAction,
    RewriteDarwinCppIncludePathAction, RunBluesimAction, RunSystemcAction, RunVerilogAction,
    ScriptManifest, ShowRulesAction, SimulationBackend, SimulationContract, SourceSpan,
    SystemcBuildAction, SystemcLinkAction, TextNormalizeAction, TouchArtifactAction,
    TouchCreateArtifactAction, UnsupportedConstruct, UnsupportedReason, VerilogFilterAction,
    WorkflowAction,
};
use crate::parse_static_tcl_list;
use bsc_test_plan::{
    BluetclInstalledScript, BluetclMakedependCommand, BluetclPackage, ExpectedExit,
    IcarusSimulatorSelector, OperationExpectation, TextNormalization, VerilogFilterProfile,
};

const SCHEDULER_SAT_ORIGIN: &str = "testsuite/bsc.scheduler/sat/sat.exp";
const SCHEDULER_SAT_SHA256: &str =
    "b70bc87c015d741f370717c67dfff2d2cb7aeaf2456303f5ab6b3f7ab85c81c7";
const DIVMOD_ORIGIN: &str = "testsuite/bsc.misc/divmod/divmod.exp";
const DIVMOD_SHA256: &str = "3511f8aa3c5105554d6b215809e7817666330ba8dbd48174aa522a01a51d0405";
const CPP_ORIGIN: &str = "testsuite/bsc.driver/cpp/cpp.exp";
const CPP_SHA256: &str = "ff0764bdcf5d57315d61c7ed6f1669bd620f4c45f4e38f7b9ca51db413dea3ae";
const EXPAND_PORTS_ORIGIN: &str = "testsuite/bsc.bluetcl/packages/expandPorts/expandPorts.exp";
const EXPAND_PORTS_SHA256: &str =
    "25b0d00fc55a1b50540ebe05fa3124645009781e52b49c20641406d635c32e0c";
const MAKEDEPEND_ORIGIN: &str = "testsuite/bsc.bluetcl/packages/makedepend/makedepend.exp";
const VERILOG_E_ORIGIN: &str = "testsuite/bsc.options/verilog-e/verilog-e.exp";
const VERILOG_E_SHA256: &str = "e4dcf5c7a9a138fe1e46787f5bbd76ead21c5e1e2ea8059bbe942d78c7118aff";
const VERILOG_FILTER_ORIGIN: &str = "testsuite/bsc.verilog/filter/filter.exp";
const VERILOG_FILTER_SHA256: &str =
    "dab5a0a4a05da9f969e530881ee5b08e2cca0e8900bff469d44b55293e492dd0";
const TASKS_ORIGIN: &str = "testsuite/bsc.verilog/tasks/tasks.exp";
const TASKS_SHA256: &str = "7c05c40220b9810bac26648914f07beff281540c51877840cdf6bf94a39b0dd2";
const SHOWRULES_ORIGIN: &str = "testsuite/bsc.showrules/showrules.exp";
const SHOWRULES_SHA256: &str = "286d9f4f1b8a82fdba3af0ebf8e3ca12e3dd3e230b1bacb3d159be9bdd4b40e4";
const OPTIONS_ORIGIN: &str = "testsuite/bsc.options/options.exp";
const OPTIONS_SHA256: &str = "636b8c7a49224cf3737a679dd3f5b04989fb63f868528a9e41e77fe50e7aebcd";

/// Recognize only the closed Tcl scope used by positive-reset tests:
/// save inherited BSC_OPTIONS, append a static suffix, then restore that value.
/// This is deliberately not a general Tcl environment evaluator.
fn recognize_bsc_options_overlay(source: &[u8]) -> Option<RecognizedBscOptionsOverlay> {
    let text = std::str::from_utf8(source).ok()?;
    let pattern = Regex::new(
        r#"(?m)^(?<save>set\s+(?<saved>[A-Za-z_][A-Za-z0-9_]*)\s+\$::env\(BSC_OPTIONS\)\s*\r?)$\n^(?<assign>set\s+::env\(BSC_OPTIONS\)\s+\"\$::env\(BSC_OPTIONS\)(?<append>[^\"]+)\"\s*\r?)$"#,
    )
    .expect("valid closed BSC_OPTIONS overlay regex");
    let start = pattern.captures(text)?;
    let saved = start.name("saved")?.as_str();
    let append = start.name("append")?.as_str().trim();
    if append.is_empty() {
        return None;
    }
    let restore = Regex::new(&format!(
        r#"(?m)^set\s+::env\(BSC_OPTIONS\)\s+\${}\s*\r?$"#,
        regex::escape(saved)
    ))
    .expect("valid closed BSC_OPTIONS restore regex");
    let start_match = start.get(0)?;
    let end = restore.find_at(text, start_match.end())?;
    let span = |offset: usize| SourceSpan {
        start_byte: offset,
        end_byte: offset,
        start_line: text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1,
        start_column: 1,
        end_line: text[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1,
        end_column: 1,
    };
    Some(RecognizedBscOptionsOverlay {
        model: BscOptionsOverlay {
            append: append.to_owned(),
            start: span(start_match.start()),
            end: span(end.end()),
        },
        assignment_spans: vec![
            (start.name("save")?.start(), start.name("save")?.end()),
            (start.name("assign")?.start(), start.name("assign")?.end()),
            (end.start(), end.end()),
        ],
    })
}

pub(crate) fn lower_script<'a>(origin: String, source: &'a [u8], tree: &'a Tree) -> ScriptManifest {
    let source_sha256 = format!("{:x}", Sha256::digest(source));
    let mut lowerer = Lowerer {
        origin: origin.clone(),
        source,
        constants: workspace_root_constants(source),
        working_directory: None,
        saved_working_directories: BTreeMap::new(),
        declared_lists: BTreeMap::new(),
        procedures: BTreeMap::new(),
        call_stack: Vec::new(),
        invocation_stack: Vec::new(),
        guard: Guard::Always,
        contracts: Vec::new(),
        comparisons: Vec::new(),
        workflow_events: Vec::new(),
        filtered_times_sources: BTreeSet::new(),
        split_if_canonical_sources: BTreeMap::new(),
        make_test_data_actions: Vec::new(),
        bsc_options_overlay: recognize_bsc_options_overlay(source),
        unsupported: Vec::new(),
        ovl_bootstrap_here: false,
        ovl_common_loaded: false,
    };
    if origin == VERILOG_E_ORIGIN && source_sha256 == VERILOG_E_SHA256 {
        lowerer.lower_pinned_verilog_e_script();
    } else if origin == VERILOG_FILTER_ORIGIN && source_sha256 == VERILOG_FILTER_SHA256 {
        lowerer.lower_pinned_verilog_filter_script();
    } else if origin == EXPAND_PORTS_ORIGIN && source_sha256 == EXPAND_PORTS_SHA256 {
        lowerer.lower_pinned_expand_ports_script();
    } else if let Some(contract) = closed_binary_ghcrts_contract(&origin, source) {
        lowerer.contracts.push(Contract::Compile(contract));
    } else {
        lowerer.lower_script_node(tree.root_node());
    }

    if origin == SCHEDULER_SAT_ORIGIN
        && source_sha256 == SCHEDULER_SAT_SHA256
        && lowerer.contracts.is_empty()
        && lowerer
            .workflow_events
            .iter()
            .all(|event| matches!(event, crate::workflow::WorkflowEvent::Boundary))
        && lowerer.comparisons.is_empty()
        && lowerer.unsupported.len() == 2
        && lowerer
            .unsupported
            .iter()
            .all(|unsupported| unsupported.command.as_deref() == Some("foreach"))
    {
        let sources = lowerer
            .declared_lists
            .get("sources")
            .map(|(values, span)| (values, span))
            .or_else(|| {
                lowerer
                    .constants
                    .get("sources")
                    .and_then(|value| match value {
                        StaticValue::List { values, span } => Some((values, span)),
                        StaticValue::Scalar(_) => None,
                    })
            });
        if let Some((values, span)) = sources {
            lowerer
                .contracts
                .push(Contract::ExternalSet(ExternalSetContract {
                    external_kind: ExternalContractKind::SchedulerSat,
                    cases: values.clone(),
                    guard: Guard::Capability {
                        capability: Capability::Verilog,
                    },
                    span: *span,
                    expansion: Vec::new(),
                }));
            lowerer.unsupported.clear();
        }
    }
    let (bluesim_sequences, workflow_actions, assertions) =
        crate::workflow::compose_bluesim_sequences(lowerer.workflow_events);
    let (bluesim_workflows, workflow_actions) =
        crate::workflow::compose_bluesim_workflows(workflow_actions);
    let (systemc_workflows, workflow_actions) =
        crate::workflow::compose_systemc_workflows(workflow_actions);
    ScriptManifest {
        origin,
        source_sha256,
        contracts: lowerer.contracts,
        assertions,
        comparisons: lowerer.comparisons,
        bluesim_sequences,
        bluesim_workflows,
        systemc_workflows,
        workflow_actions,
        make_test_data_actions: lowerer.make_test_data_actions,
        bsc_options_overlays: lowerer
            .bsc_options_overlay
            .as_ref()
            .map(|overlay| vec![overlay.model.clone()])
            .unwrap_or_default(),
        unsupported: lowerer.unsupported,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StaticValue {
    Scalar(String),
    List {
        values: Vec<String>,
        span: SourceSpan,
    },
}

impl StaticValue {
    fn as_string(&self) -> String {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::List { values, .. } => values.join(" "),
        }
    }
}

#[derive(Debug, Clone)]
struct RecognizedBscOptionsOverlay {
    model: BscOptionsOverlay,
    assignment_spans: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
struct Procedure<'a> {
    parameters: Vec<ProcedureParameter>,
    body: Node<'a>,
}

#[derive(Clone, Debug)]
struct ProcedureParameter {
    name: String,
    default: Option<String>,
}

struct Lowerer<'a> {
    origin: String,
    source: &'a [u8],
    constants: BTreeMap<String, StaticValue>,
    working_directory: Option<String>,
    saved_working_directories: BTreeMap<String, Option<String>>,
    declared_lists: BTreeMap<String, (Vec<String>, SourceSpan)>,
    procedures: BTreeMap<String, Procedure<'a>>,
    call_stack: Vec<String>,
    invocation_stack: Vec<SourceSpan>,
    guard: Guard,
    contracts: Vec<Contract>,
    comparisons: Vec<ComparisonContract>,
    workflow_events: Vec<crate::workflow::WorkflowEvent>,
    filtered_times_sources: BTreeSet<String>,
    split_if_canonical_sources: BTreeMap<String, String>,
    make_test_data_actions: Vec<MakeTestDataAction>,
    bsc_options_overlay: Option<RecognizedBscOptionsOverlay>,
    unsupported: Vec<UnsupportedConstruct>,
    ovl_bootstrap_here: bool,
    ovl_common_loaded: bool,
}

impl<'a> Lowerer<'a> {
    fn lower_pinned_verilog_e_script(&mut self) {
        let span = |line| source_line_range_span(self.source, line, line);
        self.contracts.push(Contract::Compile(CompileContract {
            source: "Hello.bsv".to_owned(),
            working_directory: None,
            helper: "compile_verilog_pass".to_owned(),
            arguments: vec!["Hello.bsv".to_owned(), "sysHello".to_owned()],
            guard: Guard::Always,
            span: span(7),
            expansion: Vec::new(),
        }));

        let link = |line, expected_exit, simulator, options: &[&str]| {
            WorkflowAction::LinkVerilog(LinkVerilogAction {
                objects: "sysHello.v".to_owned(),
                top: "sysHello".to_owned(),
                options: options.join(" "),
                no_main: false,
                expected_exit,
                simulator,
                expectation: OperationExpectation::Required,
                guard: Guard::Always,
                span: span(line),
                expansion: Vec::new(),
            })
        };
        let run = |line| {
            WorkflowAction::RunVerilog(RunVerilogAction {
                executable: "sysHello".to_owned(),
                options: String::new(),
                stdout: "sysHello.out".to_owned(),
                expected_exits: Vec::new(),
                vcd: false,
                guard: Guard::Always,
                span: span(line),
                expansion: Vec::new(),
            })
        };
        let transfer = |line, destination: &str| {
            WorkflowAction::TransferArtifact(ArtifactTransferAction {
                operation: ArtifactTransferOperation::Move,
                source: "sysHello.bsc-vcomp-out".to_owned(),
                destination: destination.to_owned(),
                guard: Guard::Always,
                span: span(line),
                expansion: Vec::new(),
            })
        };
        let render = |line, template: &str, output: &str| {
            WorkflowAction::RenderGolden(RenderGoldenAction {
                template: template.to_owned(),
                output: output.to_owned(),
                macro_value: GoldenMacroValue::BluespecDir,
                guard: Guard::Always,
                span: span(line),
                expansion: Vec::new(),
            })
        };

        self.workflow_events.extend([
            crate::workflow::WorkflowEvent::Action(link(
                10,
                ExpectedExit::Success,
                IcarusSimulatorSelector::Default,
                &[],
            )),
            crate::workflow::WorkflowEvent::Action(run(11)),
        ]);
        self.comparisons.push(ComparisonContract {
            helper: "compare_file".to_owned(),
            arguments: vec![
                "sysHello.out".to_owned(),
                "sysHello.out.expected".to_owned(),
            ],
            guard: Guard::Always,
            span: span(12),
            expansion: Vec::new(),
        });
        self.workflow_events.extend([
            crate::workflow::WorkflowEvent::Action(link(
                15,
                ExpectedExit::Success,
                IcarusSimulatorSelector::BluespecDirInstalledBuilder,
                &[],
            )),
            crate::workflow::WorkflowEvent::Action(run(16)),
        ]);
        self.comparisons.push(ComparisonContract {
            helper: "compare_file".to_owned(),
            arguments: vec![
                "sysHello.out".to_owned(),
                "sysHello.out.expected".to_owned(),
            ],
            guard: Guard::Always,
            span: span(17),
            expansion: Vec::new(),
        });
        self.workflow_events.extend([
            crate::workflow::WorkflowEvent::Action(link(
                20,
                ExpectedExit::Success,
                IcarusSimulatorSelector::PosixEchoProbe,
                &[],
            )),
            crate::workflow::WorkflowEvent::Action(transfer(21, "sysHello.sim-echo.bsc-vcomp-out")),
            crate::workflow::WorkflowEvent::Action(render(
                22,
                "bsc-sim-echo.expected",
                "bsc-sim-echo.expected.post-m4",
            )),
        ]);
        self.comparisons.push(ComparisonContract {
            helper: "compare_file".to_owned(),
            arguments: vec![
                "sysHello.sim-echo.bsc-vcomp-out".to_owned(),
                "bsc-sim-echo.expected.post-m4".to_owned(),
            ],
            guard: Guard::Always,
            span: span(23),
            expansion: Vec::new(),
        });
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Action(link(
                26,
                ExpectedExit::Failure,
                IcarusSimulatorSelector::LiteralBogus,
                &[],
            )));
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Assertion(
                AssertionContract {
                    helper: "find_n_error".to_owned(),
                    arguments: vec![
                        "sysHello.bsc-vcomp-out".to_owned(),
                        "S0035".to_owned(),
                        "1".to_owned(),
                    ],
                    guard: Guard::Always,
                    span: span(27),
                    expansion: Vec::new(),
                },
            ));
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Action(link(
                30,
                ExpectedExit::Failure,
                IcarusSimulatorSelector::BluespecDirBogus,
                &[],
            )));
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Assertion(
                AssertionContract {
                    helper: "find_n_error".to_owned(),
                    arguments: vec![
                        "sysHello.bsc-vcomp-out".to_owned(),
                        "S0035".to_owned(),
                        "1".to_owned(),
                    ],
                    guard: Guard::Always,
                    span: span(31),
                    expansion: Vec::new(),
                },
            ));
        self.workflow_events.extend([
            crate::workflow::WorkflowEvent::Action(link(
                34,
                ExpectedExit::Success,
                IcarusSimulatorSelector::PosixEchoProbe,
                &["-D", "foo", "-D", "bar=128"],
            )),
            crate::workflow::WorkflowEvent::Action(transfer(35, "sysHello.D-test.bsc-vcomp-out")),
            crate::workflow::WorkflowEvent::Action(render(
                36,
                "bsc-D-test.expected",
                "bsc-D-test.expected.post-m4",
            )),
        ]);
        self.comparisons.push(ComparisonContract {
            helper: "compare_file".to_owned(),
            arguments: vec![
                "sysHello.D-test.bsc-vcomp-out".to_owned(),
                "bsc-D-test.expected.post-m4".to_owned(),
            ],
            guard: Guard::Always,
            span: span(37),
            expansion: Vec::new(),
        });
    }

    fn lower_pinned_verilog_filter_script(&mut self) {
        let span = |line| source_line_range_span(self.source, line, line);
        let episodes: &[(
            usize,
            Option<usize>,
            &[VerilogFilterProfile],
            ExpectedExit,
            Option<(usize, &str)>,
        )] = &[
            (
                18,
                None,
                &[VerilogFilterProfile::RenameFire],
                ExpectedExit::Success,
                Some((19, "mkRenameTest.v.renamed.expected")),
            ),
            (
                24,
                Some(23),
                &[
                    VerilogFilterProfile::RenameFire,
                    VerilogFilterProfile::RenameFire,
                ],
                ExpectedExit::Success,
                Some((25, "mkRenameTest.v.renamed.expected")),
            ),
            (
                29,
                Some(28),
                &[
                    VerilogFilterProfile::RenameFire,
                    VerilogFilterProfile::ClockToClock,
                ],
                ExpectedExit::Success,
                Some((30, "mkRenameTest.v.renamed2.expected")),
            ),
            (
                37,
                Some(36),
                &[
                    VerilogFilterProfile::RenameFire,
                    VerilogFilterProfile::WfToWF,
                ],
                ExpectedExit::Success,
                Some((38, "mkRenameTest.v.renamed3.expected")),
            ),
            (
                42,
                Some(41),
                &[
                    VerilogFilterProfile::RenameFire,
                    VerilogFilterProfile::MissingSed,
                ],
                ExpectedExit::Failure,
                None,
            ),
        ];
        for (compile_line, erase_line, profiles, expected_exit, comparison) in episodes {
            if let Some(line) = erase_line {
                self.workflow_events
                    .push(crate::workflow::WorkflowEvent::Action(
                        WorkflowAction::EraseArtifact(EraseArtifactAction {
                            path: "RenameTest.bo".to_owned(),
                            guard: Guard::Always,
                            span: span(*line),
                            expansion: Vec::new(),
                        }),
                    ));
            }
            self.contracts.push(Contract::Compile(CompileContract {
                source: "RenameTest.bsv".to_owned(),
                working_directory: None,
                helper: "compile_verilog_pass".to_owned(),
                arguments: vec![
                    "RenameTest.bsv".to_owned(),
                    String::new(),
                    "-keep-fires".to_owned(),
                ],
                guard: Guard::Always,
                span: span(*compile_line),
                expansion: Vec::new(),
            }));
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::VerilogFilter(VerilogFilterAction {
                        path: "mkRenameTest.v".to_owned(),
                        profiles: profiles.to_vec(),
                        expected_exit: *expected_exit,
                        guard: Guard::Always,
                        span: span(*compile_line),
                        expansion: Vec::new(),
                    }),
                ));
            if let Some((line, expected)) = comparison {
                self.comparisons.push(ComparisonContract {
                    helper: "compare_verilog".to_owned(),
                    arguments: vec!["mkRenameTest.v".to_owned(), (*expected).to_owned()],
                    guard: Guard::Always,
                    span: span(*line),
                    expansion: Vec::new(),
                });
            }
        }
    }

    fn lower_script_node(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "comment" => {}
                "command" => self.lower_command(child),
                "set" => self.lower_set(child),
                "if" => self.lower_if(child),
                "global" => self.lower_global(child),
                "procedure" => self.lower_procedure(child),
                "foreach" => self.lower_foreach(child),
                kind if is_builtin_control_or_state(kind) => self.push_unsupported(
                    child,
                    Some(kind),
                    UnsupportedReason::UnsupportedControlFlow,
                ),
                "ERROR" => self.push_unsupported(child, None, UnsupportedReason::UnsupportedSyntax),
                _ => self.lower_script_node(child),
            }
        }
    }

    fn lower_foreach(&mut self, node: Node<'a>) {
        const PACLIB_ORIGIN: &str = "testsuite/bsc.lib/PAClib/unit_tests/unit_test.exp";
        const PACLIB_CASES: [&str; 12] = [
            "ForFold_1",
            "ForFold_2",
            "fork_join",
            "ForLoop",
            "IfThenElse",
            "Map",
            "Map_with_funnel_indexed",
            "Reorder",
            "SynchPipe",
            "WhileFold_1",
            "WhileFold_2",
            "WhileLoop",
        ];
        let normalized = self
            .text(node)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let cases = self
            .constants
            .get("packages")
            .and_then(|value| match value {
                StaticValue::List { values, .. } => Some(values.clone()),
                StaticValue::Scalar(_) => None,
            });
        if self.origin == PACLIB_ORIGIN
            && normalized
                == "foreach pack $packages { test_c_veri_bsv_modules_options $pack {} {-aggressive-conditions} }"
            && cases.as_deref()
                == Some(
                    PACLIB_CASES
                        .iter()
                        .map(|case| (*case).to_owned())
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
        {
            let span = self.span(node);
            let expansion = self.invocation_stack.clone();
            for package in cases.expect("audited PAClib cases are present") {
                let arguments = vec![
                    package.clone(),
                    String::new(),
                    "-aggressive-conditions".to_owned(),
                ];
                let simulation = simulation_shape("test_c_veri_bsv_modules_options", &arguments)
                    .expect("audited PAClib helper shape");
                for backend in simulation.backends.iter().copied() {
                    self.contracts
                        .push(Contract::Simulation(SimulationContract {
                            source: simulation.source.clone(),
                            helper: "test_c_veri_bsv_modules_options".to_owned(),
                            arguments: arguments.clone(),
                            backend,
                            generation: simulation.generation_for(backend),
                            guard: self.guard.clone(),
                            span,
                            expansion: expansion.clone(),
                        }));
                }
            }
            return;
        }
        self.push_unsupported(
            node,
            Some("foreach"),
            UnsupportedReason::UnsupportedControlFlow,
        );
    }

    fn lower_pinned_expand_ports_script(&mut self) {
        const CASES: [&str; 13] = [
            "Test1", "Test10", "Test1a", "Test1b", "Test2", "Test3", "Test4", "Test5", "Test6",
            "Test7", "Test7a", "Test7b", "Test12",
        ];
        let guard = Guard::Capability {
            capability: Capability::BluetclPackage(BluetclPackage::ExpandPorts),
        };
        let foreach_span = source_line_range_span(self.source, 13, 43);
        let compile_span = source_line_range_span(self.source, 14, 14);
        let run_without_rename_span = source_line_range_span(self.source, 35, 38);
        let run_with_rename_span = source_line_range_span(self.source, 29, 32);
        let wrapper_compare_span = source_line_range_span(self.source, 41, 41);
        let include_compare_span = source_line_range_span(self.source, 42, 42);

        for package in CASES {
            let source = format!("{package}.bsv");
            self.contracts.push(Contract::Compile(CompileContract {
                source,
                working_directory: None,
                helper: "bsc_compile".to_owned(),
                arguments: vec![package.to_owned() + ".bsv", "-verilog -elab".to_owned()],
                guard: guard.clone(),
                span: compile_span,
                expansion: vec![foreach_span],
            }));

            let module = format!("mk{package}");
            let wrapper = format!("{module}.wrapper.got.v");
            let include = format!("{module}.includes.got.vh");
            let mut args = vec!["-quiet".to_owned()];
            let mut artifact_inputs = vec![
                format!("{package}.bo"),
                format!("{module}.ba"),
                format!("{module}.v"),
            ];
            let run_span = if package == "Test7b" {
                let rename = format!("{package}.rename.tcl");
                args.extend(["-rename".to_owned(), rename.clone()]);
                artifact_inputs.push(rename);
                run_with_rename_span
            } else {
                run_without_rename_span
            };
            args.extend([
                "-wrapper".to_owned(),
                wrapper.clone(),
                "-include".to_owned(),
                include.clone(),
                package.to_owned(),
                module.clone(),
                format!("{module}.v"),
            ]);
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::BluetclRun(BluetclRunAction {
                        invocation: BluetclInvocation::InstalledScript {
                            script: BluetclInstalledScript::ExpandPorts,
                            args,
                        },
                        working_directory: None,
                        artifact_inputs,
                        artifact_outputs: vec![wrapper.clone(), include.clone()],
                        expected_exit: ExpectedExit::Success,
                        stdout: format!("{package}.expandPorts.bluetcl-out"),
                        guard: guard.clone(),
                        span: run_span,
                        expansion: vec![foreach_span],
                    }),
                ));
            self.comparisons.extend([
                ComparisonContract {
                    helper: "compare_bluetcl".to_owned(),
                    arguments: vec![wrapper, format!("{package}.wrapper.exp.v")],
                    guard: guard.clone(),
                    span: wrapper_compare_span,
                    expansion: vec![foreach_span],
                },
                ComparisonContract {
                    helper: "compare_bluetcl".to_owned(),
                    arguments: vec![include, format!("{package}.includes.exp.vh")],
                    guard: guard.clone(),
                    span: include_compare_span,
                    expansion: vec![foreach_span],
                },
            ]);
        }
    }

    fn lower_procedure(&mut self, node: Node<'a>) {
        let Some(name_node) = node.child_by_field_name("name") else {
            self.push_unsupported(
                node,
                Some("proc"),
                UnsupportedReason::UnsupportedControlFlow,
            );
            return;
        };
        let Some(arguments) = node.child_by_field_name("arguments") else {
            self.push_unsupported(
                node,
                Some("proc"),
                UnsupportedReason::UnsupportedControlFlow,
            );
            return;
        };
        let Some(body) = node.child_by_field_name("body") else {
            self.push_unsupported(
                node,
                Some("proc"),
                UnsupportedReason::UnsupportedControlFlow,
            );
            return;
        };
        let name = self.text(name_node).trim().to_owned();
        let mut parameters = Vec::new();
        let mut cursor = arguments.walk();
        for argument in arguments.named_children(&mut cursor) {
            let Some(parameter) = argument
                .child_by_field_name("name")
                .or_else(|| argument.named_child(0))
            else {
                self.push_unsupported(
                    argument,
                    Some("proc"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                return;
            };
            if argument.named_child_count() > 2 {
                self.push_unsupported(
                    argument,
                    Some("proc"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                return;
            }
            let default = if argument.named_child_count() == 2 {
                match self.static_word(argument.named_child(1).expect("audited default node")) {
                    Some(value) => Some(value),
                    None => {
                        self.push_unsupported(
                            argument,
                            Some("proc"),
                            UnsupportedReason::UnsupportedControlFlow,
                        );
                        return;
                    }
                }
            } else if parameters
                .iter()
                .any(|parameter: &ProcedureParameter| parameter.default.is_some())
            {
                self.push_unsupported(
                    argument,
                    Some("proc"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                return;
            } else {
                None
            };
            let parameter = self.text(parameter).trim().to_owned();
            if !is_static_variable_name(&parameter) {
                self.push_unsupported(
                    argument,
                    Some("proc"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                return;
            }
            parameters.push(ProcedureParameter {
                name: parameter,
                default,
            });
        }
        self.procedures.insert(name, Procedure { parameters, body });
    }

    fn has_closed_parse_pretty_helpers(&self, extension: &str) -> bool {
        if !matches!(extension, "bs" | "bsv") {
            return false;
        }
        let body = |name: &str| {
            let procedure = self.procedures.get(name)?;
            let text = self.text(procedure.body).trim();
            let text = text
                .strip_prefix('{')
                .and_then(|text| text.strip_suffix('}'))
                .unwrap_or(text);
            Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
        };
        let main = body("bsc_compile_prettyprint_parse");
        let pass = body("compile_ppp_pass");
        let bug = body("compile_ppp_pass_bug");
        let expected_main = format!(
            "set outfile \"${{source}}-pretty-out.{extension}\" if [bsc_compile $source \"$options -dparsed=$outfile\"] then {{ strip_dump_wrapper $outfile return [bsc_compile $outfile $options] }} else {{ return 0 }}"
        );
        main.as_deref() == Some(expected_main.as_str())
            && pass.as_deref()
                == Some(
                    "incr_stat \"compile_ppp_pass\" if [bsc_compile_prettyprint_parse $source $options] { pass \"`$source' compiles, pretty-prints, and compiles again\" } else { fail \"`$source' should compile, pretty-print, and compile again\" }",
                )
            && bug.as_deref()
                == Some(
                    "global target_triplet setup_xfail $target_triplet $bug compile_ppp_pass $source $options",
                )
    }

    fn lower_global(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let names = node
            .named_children(&mut cursor)
            .map(|child| self.text(child).trim())
            .collect::<Vec<_>>();
        if names.is_empty() || names.iter().any(|name| !is_static_variable_name(name)) {
            self.push_unsupported(node, Some("global"), UnsupportedReason::DynamicArguments);
        }
    }

    fn lower_set(&mut self, node: Node<'a>) {
        if self.is_pinned_options_source()
            && matches!(self.span(node).start_line, 200 | 308 | 310 | 313)
        {
            return;
        }
        if self.bsc_options_overlay.as_ref().is_some_and(|overlay| {
            overlay
                .assignment_spans
                .iter()
                .any(|&(start, end)| start <= node.start_byte() && node.end_byte() <= end)
        }) {
            return;
        }
        if is_ovl_bootstrap_assignment(self.text(node)) {
            self.ovl_bootstrap_here = true;
            return;
        }
        let Some(name_node) = node.named_child(0) else {
            self.push_unsupported(node, Some("set"), UnsupportedReason::DynamicAssignment);
            return;
        };
        let Some(value_node) = node.named_child(1) else {
            self.push_unsupported(node, Some("set"), UnsupportedReason::DynamicAssignment);
            return;
        };
        let name = self.text(name_node).trim().to_owned();
        if !is_static_variable_name(&name) {
            self.push_unsupported(node, Some("set"), UnsupportedReason::DynamicAssignment);
            return;
        }

        if self.lower_audited_working_directory_assignment(node, &name) {
            return;
        }
        if is_workspace_root_assignment(self.text(node), &name) {
            self.constants
                .insert(name, StaticValue::Scalar("HERE".to_owned()));
            return;
        }
        if is_workspace_root_filter_assignment(self.text(node), &self.constants) {
            self.constants
                .insert(name, StaticValue::Scalar("s+HERE+HERE+g".to_owned()));
            return;
        }
        if let Some(values) = self.static_list(value_node) {
            let span = self.span(node);
            self.declared_lists
                .entry(name.clone())
                .or_insert_with(|| (values.clone(), span));
            self.constants
                .insert(name, StaticValue::List { values, span });
            return;
        }
        let Some(value) = self.static_word(value_node) else {
            self.constants.remove(&name);
            self.push_unsupported(node, Some("set"), UnsupportedReason::DynamicAssignment);
            return;
        };
        self.constants.insert(name, StaticValue::Scalar(value));
    }

    fn lower_audited_working_directory_assignment(&mut self, node: Node<'a>, name: &str) -> bool {
        let normalized = self
            .text(node)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if self.origin == "testsuite/bsc.preprocessor/include/include.exp"
            && name == "curdir"
            && normalized == "set curdir [file join [absolute $srcdir] $subdir]"
        {
            self.constants.insert(
                "curdir".to_owned(),
                StaticValue::Scalar("WORKDIR".to_owned()),
            );
            return true;
        }
        if !matches!(
            self.origin.as_str(),
            "testsuite/bsc.driver/depend/depend.exp" | "testsuite/bsc.driver/imports/imports.exp"
        ) {
            return false;
        }
        if name == "prev_subdir" && normalized == "set prev_subdir $subdir" {
            self.saved_working_directories
                .insert(name.to_owned(), self.working_directory.clone());
            return true;
        }
        if name == "subdir"
            && normalized.starts_with("set subdir [file join $subdir ")
            && normalized.ends_with(']')
        {
            let argument = normalized
                .trim_start_matches("set subdir [file join $subdir ")
                .trim_end_matches(']')
                .trim_matches(['"', '{', '}']);
            let directory = argument
                .strip_prefix('$')
                .and_then(|variable| self.constants.get(variable))
                .map(StaticValue::as_string)
                .unwrap_or_else(|| argument.to_owned());
            if directory.is_empty()
                || !directory
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
            {
                return false;
            }
            self.working_directory = Some(match self.working_directory.as_deref() {
                Some(current) => format!("{current}/{directory}"),
                None => directory,
            });
            return true;
        }
        if name == "subdir" && normalized == "set subdir $prev_subdir" {
            let Some(saved) = self.saved_working_directories.remove("prev_subdir") else {
                return false;
            };
            self.working_directory = saved;
            return true;
        }
        false
    }

    fn workspace_path(&self, path: &str) -> String {
        self.working_directory.as_ref().map_or_else(
            || path.to_owned(),
            |directory| format!("{directory}/{path}"),
        )
    }

    fn lower_if(&mut self, node: Node<'a>) {
        if self.lower_pinned_showrules_branch(node)
            || self.lower_pinned_cpp_darwin_branch(node)
            || self.lower_pinned_divmod_architecture_exit(node)
        {
            return;
        }
        let Some(condition) = node.child_by_field_name("condition") else {
            self.push_unsupported(node, Some("if"), UnsupportedReason::UnsupportedControlFlow);
            return;
        };
        let Some(consequence) = node.child_by_field_name("consequence") else {
            self.push_unsupported(node, Some("if"), UnsupportedReason::UnsupportedControlFlow);
            return;
        };

        let condition_text = self.text(condition).to_owned();
        if let Some(condition_value) = self.static_boolean_condition(&condition_text) {
            if condition_value {
                self.lower_script_node(consequence);
            } else if let Some(alternative) = node.child_by_field_name("alternative") {
                if alternative.kind() == "else" {
                    if let Some(body) = alternative.child_by_field_name("consequence") {
                        self.lower_script_node(body);
                    }
                } else {
                    self.push_unsupported(
                        alternative,
                        Some(alternative.kind()),
                        UnsupportedReason::UnsupportedControlFlow,
                    );
                }
            }
            return;
        }

        let condition_guard = match capability_condition(self.text(condition)) {
            Some(guard) => guard,
            None => {
                self.push_unsupported(
                    condition,
                    Some("if"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                Guard::UnsupportedExpression {
                    source: self.text(condition).trim().to_owned(),
                    span: self.span(condition),
                }
            }
        };
        let previous_guard = self.guard.clone();
        let previous_constants = self.constants.clone();
        self.lower_guarded_body(
            consequence,
            combine_guards(previous_guard.clone(), condition_guard.clone()),
            &previous_constants,
        );

        if let Some(alternative) = node.child_by_field_name("alternative") {
            if alternative.kind() != "else" {
                self.push_unsupported(
                    alternative,
                    Some(alternative.kind()),
                    UnsupportedReason::UnsupportedControlFlow,
                );
            } else if let Some(body) = alternative.child_by_field_name("consequence") {
                self.lower_guarded_body(
                    body,
                    combine_guards(previous_guard.clone(), negate_guard(condition_guard)),
                    &previous_constants,
                );
            } else {
                self.push_unsupported(
                    alternative,
                    Some("else"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
            }
        }

        self.guard = previous_guard;
        self.constants = previous_constants;
    }

    fn lower_pinned_showrules_branch(&mut self, node: Node<'a>) -> bool {
        let span = self.span(node);
        if !self.is_pinned_showrules_source()
            || span.start_line != 4
            || span.end_line != 267
            || self
                .text(node.child_by_field_name("condition").unwrap_or(node))
                .trim()
                != r#"{ ! [file exists "$showrules"] }"#
        {
            return false;
        }
        let Some(alternative) = node.child_by_field_name("alternative") else {
            return false;
        };
        let Some(body) = (alternative.kind() == "else")
            .then(|| alternative.child_by_field_name("consequence"))
            .flatten()
        else {
            return false;
        };
        let previous_guard = self.guard.clone();
        let previous_constants = self.constants.clone();
        self.lower_guarded_body(
            body,
            combine_guards(
                previous_guard.clone(),
                Guard::Capability {
                    capability: Capability::ShowRules,
                },
            ),
            &previous_constants,
        );
        self.guard = previous_guard;
        self.constants = previous_constants;
        true
    }

    fn is_pinned_showrules_source(&self) -> bool {
        self.origin == SHOWRULES_ORIGIN
            && format!("{:x}", Sha256::digest(self.source)) == SHOWRULES_SHA256
    }

    fn lower_pinned_cpp_darwin_branch(&mut self, node: Node<'a>) -> bool {
        let span = self.span(node);
        if self.origin != CPP_ORIGIN
            || format!("{:x}", Sha256::digest(self.source)) != CPP_SHA256
            || span.start_line != 14
            || span.end_line != 20
            || node.child_by_field_name("alternative").is_some()
        {
            return false;
        }
        let Some(condition) = node.child_by_field_name("condition") else {
            return false;
        };
        let condition = self
            .text(condition)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !matches!(
            condition.as_str(),
            r#"[which_os] == "Darwin""# | r#"{ [which_os] == "Darwin" }"#
        ) {
            return false;
        }
        let Some(consequence) = node.child_by_field_name("consequence") else {
            return false;
        };
        let previous_guard = self.guard.clone();
        let previous_constants = self.constants.clone();
        self.lower_guarded_body(
            consequence,
            combine_guards(
                previous_guard.clone(),
                Guard::Capability {
                    capability: Capability::Darwin,
                },
            ),
            &previous_constants,
        );
        self.guard = previous_guard;
        self.constants = previous_constants;
        true
    }

    fn lower_pinned_divmod_architecture_exit(&mut self, node: Node<'a>) -> bool {
        let span = self.span(node);
        if self.origin != DIVMOD_ORIGIN
            || format!("{:x}", Sha256::digest(self.source)) != DIVMOD_SHA256
            || span.start_line != 15
            || span.end_line != 20
        {
            return false;
        }
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Action(
                WorkflowAction::RunBluesim(RunBluesimAction {
                    executable: "sysDivideByZero".to_owned(),
                    options: String::new(),
                    stdout: "sysDivideByZero.out".to_owned(),
                    expected_exits: vec![8, 136],
                    aarch64_expected_exits: Some(vec![0]),
                    windows_expected_exits: Some(vec![127]),
                    guard: self.guard.clone(),
                    span,
                    expansion: self.invocation_stack.clone(),
                }),
            ));
        true
    }

    fn pinned_windows_simulation_exits(&self, executable: &str) -> Option<Vec<i32>> {
        (self.origin == DIVMOD_ORIGIN
            && format!("{:x}", Sha256::digest(self.source)) == DIVMOD_SHA256
            && executable == "sysDivideByZeroWide")
            .then(|| vec![3])
    }

    fn static_boolean_condition(&mut self, condition: &str) -> Option<bool> {
        static_literal_boolean_condition(condition)
            .or_else(|| static_string_compare_empty_condition(condition, &self.constants))
            .or_else(|| self.pinned_iverilog_condition(condition))
    }

    fn pinned_iverilog_condition(&mut self, condition: &str) -> Option<bool> {
        let compiler = self.constants.get("verilog_compiler")?.as_string();
        let version = self.constants.get("verilog_compiler_version")?.as_string();
        let normalized = condition
            .trim()
            .strip_prefix('{')
            .and_then(|value| value.strip_suffix('}'))
            .unwrap_or(condition)
            .split_whitespace()
            .collect::<String>();
        let regexp_prefix =
            r#"$verilog_compiler=="iverilog"&&[regexp{^\d+\.\d+}$verilog_compiler_versionmajmin]"#;
        if let Some(suffix) = normalized.strip_prefix(regexp_prefix) {
            let major_minor = pinned_iverilog_major_minor(&version)?;
            self.constants.insert(
                "majmin".to_owned(),
                StaticValue::Scalar(major_minor.to_owned()),
            );
            if compiler != "iverilog" {
                return Some(false);
            }
            return match suffix {
                "" => Some(true),
                suffix if suffix.starts_with("&&$majmin<") => suffix
                    .strip_prefix("&&$majmin<")?
                    .parse::<u32>()
                    .ok()
                    .map(|threshold| pinned_iverilog_major(&major_minor) < threshold),
                suffix if suffix.starts_with("&&$majmin==\"") && suffix.ends_with('"') => Some(
                    major_minor
                        == suffix
                            .strip_prefix("&&$majmin==\"")
                            .and_then(|value| value.strip_suffix('"'))
                            .expect("audited quoted majmin equality"),
                ),
                _ => None,
            };
        }
        if let Some(major_minor) = self.constants.get("majmin").map(StaticValue::as_string) {
            if let Some(threshold) = normalized
                .strip_prefix("$majmin<")
                .and_then(|value| value.parse::<u32>().ok())
            {
                return Some(pinned_iverilog_major(&major_minor) < threshold);
            }
        }
        match normalized.as_str() {
            "$verilog_compiler==\"iverilog\"&&($verilog_compiler_version==\"10.1\"||$verilog_compiler_version==\"10.2\")" => {
                Some(compiler == "iverilog" && (version == "10.1" || version == "10.2"))
            }
            _ => None,
        }
    }

    fn lower_guarded_body(
        &mut self,
        body: Node<'a>,
        guard: Guard,
        constants: &BTreeMap<String, StaticValue>,
    ) {
        self.guard = guard;
        self.constants.clone_from(constants);
        self.lower_script_node(body);
    }

    fn lower_pinned_tasks_text_transform(&mut self, node: Node<'a>, name: &str) -> bool {
        if self.origin != TASKS_ORIGIN
            || format!("{:x}", Sha256::digest(self.source)) != TASKS_SHA256
        {
            return false;
        }
        let text = self.text(node).trim();
        let (source, destination, transform) = match (name, text) {
            (
                "sort",
                "sort sysModuleDisplay.v.out sysModuleDisplay.sorted.v.out {-k 1,1n -k 2}",
            ) => (
                "sysModuleDisplay.v.out",
                "sysModuleDisplay.sorted.v.out",
                TextNormalization::SortNumericField1ThenField2,
            ),
            (
                "awk",
                r#"awk sysModuleDisplay.sorted.v.out sysModuleDisplay.trimmed.v.out {{{ gsub("main\\.", "", $0); for (i=2; i<=NF; i=i+1) printf "%s ", $i; print ""; }}}"#,
            ) => (
                "sysModuleDisplay.sorted.v.out",
                "sysModuleDisplay.trimmed.v.out",
                TextNormalization::VerilogTaskProjection,
            ),
            (
                "sort",
                "sort sysModuleDisplay.c.out sysModuleDisplay.sorted.c.out {-k 1,1n -k 2}",
            ) => (
                "sysModuleDisplay.c.out",
                "sysModuleDisplay.sorted.c.out",
                TextNormalization::SortNumericField1ThenField2,
            ),
            (
                "awk",
                r#"awk sysModuleDisplay.sorted.c.out sysModuleDisplay.trimmed.c.out {{{ for (i=2; i<=NF; i=i+1) printf "%s ", $i; print ""; }}}"#,
            ) => (
                "sysModuleDisplay.sorted.c.out",
                "sysModuleDisplay.trimmed.c.out",
                TextNormalization::BluesimTaskProjection,
            ),
            _ => return false,
        };
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Action(
                WorkflowAction::TextNormalize(TextNormalizeAction {
                    source: source.to_owned(),
                    destination: destination.to_owned(),
                    transform,
                    guard: self.guard.clone(),
                    span: self.span(node),
                    expansion: self.invocation_stack.clone(),
                }),
            ));
        true
    }

    fn lower_command(&mut self, node: Node<'a>) {
        let Some(name_node) = node.child_by_field_name("name") else {
            self.push_unsupported(node, None, UnsupportedReason::UnsupportedSyntax);
            return;
        };
        // Tcl word semantics reduce backslash-escaped characters to their
        // literal values, so upstream scripts may spell helper names like
        // `find\_regexp`. The parser splits such a name into adjacent
        // fragments inside the command node; rebuild it by concatenating
        // every fragment before the argument list and stripping the escapes.
        let mut raw_name = self.text(name_node).to_owned();
        let arguments_node = node.child_by_field_name("arguments");
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if Some(child.id()) == arguments_node.map(|arguments| arguments.id()) {
                break;
            }
            if child.id() != name_node.id() && child.is_named() {
                raw_name.push_str(self.text(child));
            }
        }
        let name = raw_name.trim().replace('\\', "").trim().to_owned();
        if name == "set" && is_workspace_root_assignment(self.text(node), "here") {
            return;
        }
        if self.is_pinned_options_source()
            && name == "set"
            && matches!(self.span(node).start_line, 200 | 308 | 310 | 313)
        {
            return;
        }
        if self.is_pinned_options_source()
            && name == "compile_pass"
            && self.span(node).start_line == 201
        {
            self.workflow_boundary();
            self.contracts.push(Contract::Compile(CompileContract {
                source: "IncludeTest.bsv".to_owned(),
                working_directory: None,
                helper: "compile_pass".to_owned(),
                arguments: vec!["IncludeTest.bsv".to_owned()],
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            }));
            return;
        }
        if name == "m4_process" {
            self.lower_m4_process(node);
            return;
        }

        if name == "source" && self.ovl_bootstrap_here && is_ovl_common_source(self.text(node)) {
            self.ovl_common_loaded = true;
            return;
        }
        let arguments = match node.child_by_field_name("arguments") {
            Some(arguments) => match self.static_arguments(arguments) {
                Some(arguments) => arguments,
                None => {
                    self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                    return;
                }
            },
            None => Vec::new(),
        };

        if self.is_pinned_showrules_source()
            && name == "bsc_initialize"
            && arguments.is_empty()
            && self.span(node).start_line == 2
        {
            return;
        }

        if self.lower_pinned_tasks_text_transform(node, &name) {
            return;
        }

        if name == "perl" {
            if let [program, input, output] = arguments.as_slice() {
                if program == "canonicalize.pl"
                    && input.ends_with(".splitIf.dump")
                    && output.ends_with(".canon.dump")
                    && input.strip_suffix(".splitIf.dump") == output.strip_suffix(".canon.dump")
                {
                    self.split_if_canonical_sources
                        .insert(output.clone(), input.clone());
                    return;
                }
            }
        }

        if name == "awk"
            && self.origin == "testsuite/bsc.lib/fifo/fifo.exp"
            && format!("{:x}", Sha256::digest(self.source))
                == "d320ed864c1d492057083fd34ecf592b922c80d2da10469ceca53e9cb5bb9016"
            && matches!(
                arguments.as_slice(),
                [template, output, _recipe]
                    if template == "sysFIFOErrors.out.expected"
                        && output == "sysFIFOErrors.c.out.expected"
            )
        {
            self.workflow_boundary();
            self.contracts
                .push(Contract::RenderGolden(RenderGoldenContract {
                    template: arguments[0].clone(),
                    output: arguments[1].clone(),
                    macro_value: GoldenMacroValue::FifoWarningLocations,
                    guard: self.guard.clone(),
                    span: self.span(node),
                    expansion: self.invocation_stack.clone(),
                }));
            return;
        }

        if name == "make_pass" && arguments == ["-f convert.mk", "convert.o"] {
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::BuildCObject(CObjectBuildAction {
                        source: "convert.c".to_owned(),
                        makefile: "convert.mk".to_owned(),
                        output: "convert.o".to_owned(),
                        guard: self.guard.clone(),
                        span: self.span(node),
                        expansion: self.invocation_stack.clone(),
                    }),
                ));
            return;
        }

        if name == "make_pass" && arguments == ["test_data", "-f Makefile.data"] {
            self.make_test_data_actions.push(MakeTestDataAction {
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            });
            return;
        }

        if matches!(name.as_str(), "compile_ppp_pass" | "compile_ppp_pass_bug")
            && arguments.first().is_some_and(|source| {
                source
                    .rsplit_once('.')
                    .is_some_and(|(_, extension)| self.has_closed_parse_pretty_helpers(extension))
            })
        {
            let (source, bug, options) = match (name.as_str(), arguments.as_slice()) {
                ("compile_ppp_pass", [source]) => (source, None, ""),
                ("compile_ppp_pass", [source, options]) => (source, None, options.as_str()),
                ("compile_ppp_pass_bug", [source]) => (source, Some(""), ""),
                ("compile_ppp_pass_bug", [source, bug]) => (source, Some(bug.as_str()), ""),
                ("compile_ppp_pass_bug", [source, bug, options]) => {
                    (source, Some(bug.as_str()), options.as_str())
                }
                _ => {
                    self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                    return;
                }
            };
            let extension = if source.ends_with(".bsv") {
                "bsv"
            } else if source.ends_with(".bs") {
                "bs"
            } else {
                self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                return;
            };
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::BscParsePretty(BscParsePrettyAction {
                        source: source.clone(),
                        options: options.to_owned(),
                        pretty_output: format!("{source}-pretty-out.{extension}"),
                        expectation: bug.map_or(OperationExpectation::Required, |bug| {
                            unannotated_known_failure_expectation(bug)
                        }),
                        guard: self.guard.clone(),
                        span: self.span(node),
                        expansion: self.invocation_stack.clone(),
                    }),
                ));
            return;
        }
        if let Some(procedure) = self.procedures.get(&name).cloned() {
            self.lower_procedure_call(node, &name, arguments, procedure);
            return;
        }
        if name == "global" && arguments.iter().all(|name| is_static_variable_name(name)) {
            return;
        }

        if self.is_pinned_options_source() && name == "erase_many" {
            let [pattern] = arguments.as_slice() else {
                self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                return;
            };
            let paths: &[&str] = match pattern.as_str() {
                "vpi_*.{c,h,o}" => &[
                    "vpi_wrapper_my_time.c",
                    "vpi_wrapper_my_time.h",
                    "vpi_wrapper_my_time.o",
                    "vpi_startup_array.c",
                    "vpi_startup_array.h",
                    "vpi_startup_array.o",
                ],
                "vfiles/vpi_*.o vpi_*.{c,h,o}" => &[
                    "vfiles/vpi_wrapper_my_time.o",
                    "vfiles/vpi_startup_array.o",
                    "vpi_wrapper_my_time.c",
                    "vpi_wrapper_my_time.h",
                    "vpi_wrapper_my_time.o",
                    "vpi_startup_array.c",
                    "vpi_startup_array.h",
                    "vpi_startup_array.o",
                ],
                _ => {
                    self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                    return;
                }
            };
            let span = self.span(node);
            let expansion = self.invocation_stack.clone();
            self.workflow_events.extend(paths.iter().map(|path| {
                crate::workflow::WorkflowEvent::Action(WorkflowAction::EraseArtifact(
                    EraseArtifactAction {
                        path: (*path).to_owned(),
                        guard: self.guard.clone(),
                        span,
                        expansion: expansion.clone(),
                    },
                ))
            }));
            return;
        }

        if name == "run_bsc2bsv" {
            let [source] = arguments.as_slice() else {
                self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                return;
            };
            if !source.ends_with(".bs") {
                self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                return;
            }
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::Bsc2Bsv(Bsc2BsvAction {
                        source: source.clone(),
                        stdout: format!("{source}.bsc2bsv-out"),
                        guard: combine_guards(
                            self.guard.clone(),
                            Guard::Capability {
                                capability: Capability::InternalChecks,
                            },
                        ),
                        span: self.span(node),
                        expansion: self.invocation_stack.clone(),
                    }),
                ));
            return;
        }
        if self.lower_bluetcl_helper(node, &name, &arguments) {
            return;
        }
        if self.lower_workflow_action(node, &name, &arguments) {
            return;
        }
        if name == "test_ovl" {
            let [top, library] = arguments.as_slice() else {
                self.push_unsupported(node, Some("test_ovl"), UnsupportedReason::DynamicArguments);
                return;
            };
            if !self.ovl_common_loaded || !is_safe_ovl_segment(top) || !is_safe_ovl_library(library)
            {
                self.push_unsupported(node, Some("test_ovl"), UnsupportedReason::DynamicArguments);
                return;
            }
            self.workflow_boundary();
            self.contracts.push(Contract::Ovl(OvlContract {
                case_dir: top.clone(),
                top: top.clone(),
                library: library.clone(),
                guard: Guard::Capability {
                    capability: Capability::Verilog,
                },
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            }));
            return;
        }
        if name == "test_basic_options" {
            self.workflow_boundary();
            let [options, output, expected] = arguments.as_slice() else {
                self.push_unsupported(
                    node,
                    Some("test_basic_options"),
                    UnsupportedReason::DynamicArguments,
                );
                return;
            };
            if output.is_empty() || expected.is_empty() {
                self.push_unsupported(
                    node,
                    Some("test_basic_options"),
                    UnsupportedReason::DynamicArguments,
                );
                return;
            }
            self.contracts
                .push(Contract::BasicOptions(BasicOptionsContract {
                    options: options.clone(),
                    output: output.clone(),
                    expected: expected.clone(),
                    guard: self.guard.clone(),
                    span: self.span(node),
                    expansion: self.invocation_stack.clone(),
                }));
            return;
        }
        if name == "compile_no_source_fail_error" {
            self.workflow_boundary();
            let (test_name, options, diagnostic, count) = match arguments.as_slice() {
                [test_name, options, diagnostic] => {
                    (test_name, options, diagnostic, "1".to_owned())
                }
                [test_name, options, diagnostic, count] if count.parse::<u64>().is_ok() => {
                    (test_name, options, diagnostic, count.clone())
                }
                _ => {
                    self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                    return;
                }
            };
            self.contracts
                .push(Contract::NoSourceCompile(NoSourceCompileContract {
                    name: test_name.clone(),
                    options: options.clone(),
                    diagnostic: diagnostic.clone(),
                    count,
                    guard: self.guard.clone(),
                    span: self.span(node),
                    expansion: self.invocation_stack.clone(),
                }));
            return;
        }
        if is_compile_helper(&name) {
            self.workflow_boundary();
            let Some(source) = arguments.first().filter(|source| !source.is_empty()) else {
                self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                return;
            };
            self.contracts.push(Contract::Compile(CompileContract {
                source: source.clone(),
                working_directory: self.working_directory.clone(),
                helper: name,
                arguments,
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            }));
            return;
        }
        if let Some(simulation) = simulation_shape(&name, &arguments) {
            self.workflow_boundary();
            let span = self.span(node);
            for backend in simulation.backends.iter().copied() {
                self.contracts
                    .push(Contract::Simulation(SimulationContract {
                        source: simulation.source.clone(),
                        helper: name.clone(),
                        arguments: arguments.clone(),
                        backend,
                        generation: simulation.generation_for(backend),
                        guard: self.guard.clone(),
                        span,
                        expansion: self.invocation_stack.clone(),
                    }));
            }
            return;
        }
        if name == "files_exist" {
            let [paths] = arguments.as_slice() else {
                self.push_unsupported(
                    node,
                    Some("files_exist"),
                    UnsupportedReason::DynamicArguments,
                );
                return;
            };
            let Ok(paths) = parse_static_tcl_list(paths) else {
                self.push_unsupported(
                    node,
                    Some("files_exist"),
                    UnsupportedReason::DynamicArguments,
                );
                return;
            };
            if paths.is_empty() {
                self.push_unsupported(
                    node,
                    Some("files_exist"),
                    UnsupportedReason::DynamicArguments,
                );
                return;
            }
            let span = self.span(node);
            let expansion = self.invocation_stack.clone();
            self.workflow_events.extend(paths.into_iter().map(|path| {
                crate::workflow::WorkflowEvent::Assertion(AssertionContract {
                    helper: "files_exist".to_owned(),
                    arguments: vec![path],
                    guard: self.guard.clone(),
                    span,
                    expansion: expansion.clone(),
                })
            }));
            return;
        }
        if is_assertion_helper(&name) {
            let guard = if matches!(name.as_str(), "vcdcheck_pass" | "vcdcheck_fail") {
                combine_guards(
                    self.guard.clone(),
                    Guard::Capability {
                        capability: Capability::InternalChecks,
                    },
                )
            } else {
                self.guard.clone()
            };
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Assertion(
                    AssertionContract {
                        helper: name,
                        arguments,
                        guard,
                        span: self.span(node),
                        expansion: self.invocation_stack.clone(),
                    },
                ));
            return;
        }
        if name == "check_verilog_output" {
            self.workflow_boundary();
            let (helper, arguments) = match arguments.as_slice() {
                [output, expected, bug] if bug.is_empty() => (
                    "compare_file".to_owned(),
                    vec![output.clone(), expected.clone()],
                ),
                [output, expected, bug] if bug.parse::<u64>().is_ok() => (
                    "compare_file_bug".to_owned(),
                    vec![output.clone(), expected.clone(), bug.clone()],
                ),
                _ => {
                    self.push_unsupported(
                        node,
                        Some("check_verilog_output"),
                        UnsupportedReason::DynamicArguments,
                    );
                    return;
                }
            };
            self.comparisons.push(ComparisonContract {
                helper,
                arguments,
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            });
            return;
        }
        if matches!(
            name.as_str(),
            "compare_file"
                | "compare_file_bug"
                | "compare_file_list"
                | "compare_file_filtered"
                | "compare_file_filter_ids"
                | "compare_file_filter_prelude"
                | "compare_verilog"
                | "compare_verilog_bug"
        ) {
            self.workflow_boundary();
            let (name, arguments) = match (name.as_str(), arguments.as_slice()) {
                // A prior `sed X X.filtered {} {s/\[.*\]/\[TIME\]/g}` derives
                // the compared artifact by filtering, so compare the original
                // output under the closed bracketed-times normalization.
                ("compare_file", [actual])
                    if self.split_if_canonical_sources.contains_key(actual) =>
                {
                    (
                        "compare_file_split_if_rules".to_owned(),
                        vec![
                            self.split_if_canonical_sources
                                .get(actual)
                                .expect("matched split-if canonical output")
                                .clone(),
                            format!("{actual}.expected"),
                        ],
                    )
                }
                ("compare_file", [actual, expected])
                    if actual.ends_with(".filtered")
                        && self
                            .filtered_times_sources
                            .contains(actual.strip_suffix(".filtered").unwrap_or_default()) =>
                {
                    (
                        "compare_file_filtered_times".to_owned(),
                        vec![
                            actual
                                .strip_suffix(".filtered")
                                .unwrap_or_default()
                                .to_owned(),
                            expected.clone(),
                        ],
                    )
                }
                ("compare_file_bug", [actual, bug]) if is_numeric_bug_id(bug) => {
                    (name, vec![actual.clone(), String::new(), bug.clone()])
                }
                _ => (name, arguments),
            };
            self.comparisons.push(ComparisonContract {
                helper: name,
                arguments,
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            });
            return;
        }
        if self.origin == MAKEDEPEND_ORIGIN
            && name == "bluetcl_compare"
            && arguments == ["minusO.depend-out"]
        {
            self.comparisons.push(ComparisonContract {
                helper: "compare_bluetcl".to_owned(),
                arguments: vec!["minusO.depend-out".to_owned()],
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            });
            return;
        }
        if name == "sed" {
            if let [input, output, bre_options, ere_options] = arguments.as_slice() {
                if self.is_pinned_options_source()
                    && self.span(node).start_line == 362
                    && input == "sysGCD.bsc-vcomp-out"
                    && output == "sysGCD.bsc-vcomp-out.filtered"
                    && bre_options.starts_with("-e /WARNING:\\ IVerilog/d -e /not\\ guaranteed/d")
                    && ere_options.is_empty()
                {
                    self.workflow_events
                        .push(crate::workflow::WorkflowEvent::Action(
                            WorkflowAction::TextNormalize(TextNormalizeAction {
                                source: input.clone(),
                                destination: output.clone(),
                                transform: TextNormalization::IverilogQuietOutput,
                                guard: self.guard.clone(),
                                span: self.span(node),
                                expansion: self.invocation_stack.clone(),
                            }),
                        ));
                    return;
                }
                if self.is_pinned_options_source()
                    && input == "sysGCD.bsc-ccomp-out"
                    && output == "sysGCD.bsc-ccomp-out.parallel.filtered"
                    && bre_options
                        == "-e /make.*:\\ Entering\\ directory/d -e /make.*:\\ Leaving\\ directory/d"
                    && ere_options.is_empty()
                {
                    self.workflow_events
                        .push(crate::workflow::WorkflowEvent::Action(
                            WorkflowAction::TextNormalize(TextNormalizeAction {
                                source: input.clone(),
                                destination: output.clone(),
                                transform: TextNormalization::MakeDirectoryMessages,
                                guard: self.guard.clone(),
                                span: self.span(node),
                                expansion: self.invocation_stack.clone(),
                            }),
                        ));
                    return;
                }
                let (empty, replacement) = (bre_options, ere_options);
                if empty.is_empty()
                    && output == &format!("{input}.filtered")
                    && replacement == r"s/\\\[.*\\\]/\\\[TIME\\\]/"
                {
                    self.filtered_times_sources.insert(input.clone());
                    return;
                }
                if self.origin == CPP_ORIGIN
                    && format!("{:x}", Sha256::digest(self.source)) == CPP_SHA256
                    && input == "Cpreprocess_line.bsv.bsc-out"
                    && output == "Cpreprocess_line.bsv.bsc-out.filtered"
                    && empty.is_empty()
                    && replacement == r#"-e {s/".*\/more.bsv"/"more.bsv"/g}"#
                    && self.guard
                        == (Guard::Capability {
                            capability: Capability::Darwin,
                        })
                {
                    self.workflow_events
                        .push(crate::workflow::WorkflowEvent::Action(
                            WorkflowAction::RewriteDarwinCppIncludePath(
                                RewriteDarwinCppIncludePathAction {
                                    source: input.clone(),
                                    destination: output.clone(),
                                    guard: self.guard.clone(),
                                    span: self.span(node),
                                    expansion: self.invocation_stack.clone(),
                                },
                            ),
                        ));
                    return;
                }
            }
        }
        self.push_unsupported(node, Some(&name), UnsupportedReason::UnsupportedCommand);
    }

    fn lower_m4_process(&mut self, node: Node<'a>) {
        let Some(arguments) = node.child_by_field_name("arguments") else {
            self.push_unsupported(
                node,
                Some("m4_process"),
                UnsupportedReason::DynamicArguments,
            );
            return;
        };
        let Ok(mut arguments) = parse_static_tcl_list(self.text(arguments)) else {
            self.push_unsupported(
                node,
                Some("m4_process"),
                UnsupportedReason::DynamicArguments,
            );
            return;
        };
        for argument in &mut arguments {
            for (name, value) in &self.constants {
                let value = value.as_string();
                *argument = argument.replace(&format!("${{{name}}}"), &value);
                *argument = argument.replace(&format!("${name}"), &value);
            }
        }
        let [definition, template, output] = arguments.as_slice() else {
            self.push_unsupported(
                node,
                Some("m4_process"),
                UnsupportedReason::DynamicArguments,
            );
            return;
        };
        if self.is_pinned_options_source() {
            let transform = match (definition.as_str(), template.as_str(), output.as_str()) {
                (
                    "-DIfNested=SplitIfNested",
                    "IfNested.bs.expandif.atsexpand",
                    "SplitIfNested.bs.expandif.atsexpand.expected",
                ) => Some(TextNormalization::IfNestedToSplitIfNested),
                (
                    "-DIfNested=NoSplitIfNested",
                    "IfNested.bs.noexpandif.atsexpand",
                    "NoSplitIfNested.bs.noexpandif.atsexpand.expected",
                ) => Some(TextNormalization::IfNestedToNoSplitIfNested),
                _ => None,
            };
            if let Some(transform) = transform {
                self.workflow_boundary();
                self.workflow_events
                    .push(crate::workflow::WorkflowEvent::Action(
                        WorkflowAction::TextNormalize(TextNormalizeAction {
                            source: template.clone(),
                            destination: output.clone(),
                            transform,
                            guard: self.guard.clone(),
                            span: self.span(node),
                            expansion: self.invocation_stack.clone(),
                        }),
                    ));
                self.workflow_boundary();
                return;
            }
        }
        let macro_value = match definition.as_str() {
            "-DBLUESPECDIR=$bsdir" | "-DBLUESPECDIR=[get_default_bsdir]" => {
                GoldenMacroValue::BluespecDir
            }
            "-DHERE=$here" | "-DCURDIR=WORKDIR" => GoldenMacroValue::WorkDir,
            "-DHERE=HERE" if self.is_pinned_options_source() => GoldenMacroValue::WorkDir,
            _ => {
                self.push_unsupported(
                    node,
                    Some("m4_process"),
                    UnsupportedReason::DynamicArguments,
                );
                return;
            }
        };
        if template.is_empty() || output.is_empty() {
            self.push_unsupported(
                node,
                Some("m4_process"),
                UnsupportedReason::DynamicArguments,
            );
            return;
        }
        self.workflow_boundary();
        if macro_value == GoldenMacroValue::WorkDir
            && self.origin == "testsuite/bsc.preprocessor/include/include.exp"
            && definition == "-DCURDIR=WORKDIR"
            && matches!(
                (template.as_str(), output.as_str()),
                ("IncludeAbsolute.bsv.pre-m4", "IncludeAbsolute.bsv")
                    | (
                        "IncludeAbsolute.bsv.bsc-vcomp-out.expected.pre-m4",
                        "IncludeAbsolute.bsv.bsc-vcomp-out.expected"
                    )
            )
        {
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::RenderM4Curdir(RenderM4CurdirAction {
                        template: template.clone(),
                        output: output.clone(),
                        guard: self.guard.clone(),
                        span: self.span(node),
                        expansion: self.invocation_stack.clone(),
                    }),
                ));
        } else {
            self.contracts
                .push(Contract::RenderGolden(RenderGoldenContract {
                    template: template.clone(),
                    output: output.clone(),
                    macro_value,
                    guard: self.guard.clone(),
                    span: self.span(node),
                    expansion: self.invocation_stack.clone(),
                }));
        }
        self.workflow_boundary();
    }

    fn lower_bluetcl_helper(&mut self, node: Node<'a>, name: &str, arguments: &[String]) -> bool {
        if self.origin == MAKEDEPEND_ORIGIN
            && matches!(
                name,
                "bluetcl_exec_compare_pass" | "bluetcl_exec_compare_fail"
            )
        {
            let [command_line, output_name] = arguments else {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            };
            let Some(contract) = closed_makedepend_invocation(command_line, output_name) else {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            };
            let expected_exit = if name == "bluetcl_exec_compare_pass" {
                ExpectedExit::Success
            } else {
                ExpectedExit::Failure
            };
            if expected_exit != contract.expected_exit {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            }
            let span = self.span(node);
            let stdout = contract.working_directory.map_or_else(
                || format!("{output_name}.bluetcl-out"),
                |directory| format!("{directory}/{output_name}.bluetcl-out"),
            );
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::BluetclRun(BluetclRunAction {
                        invocation: BluetclInvocation::Makedepend {
                            command: contract.command,
                            args: contract.args,
                        },
                        working_directory: contract.working_directory.map(str::to_owned),
                        artifact_inputs: contract.artifact_inputs,
                        artifact_outputs: contract.artifact_outputs,
                        expected_exit,
                        stdout: stdout.clone(),
                        guard: self.guard.clone(),
                        span,
                        expansion: self.invocation_stack.clone(),
                    }),
                ));
            self.comparisons.push(ComparisonContract {
                helper: "compare_bluetcl".to_owned(),
                arguments: vec![stdout, format!("{output_name}.bluetcl-out.expected")],
                guard: self.guard.clone(),
                span,
                expansion: self.invocation_stack.clone(),
            });
            return true;
        }

        if name == "bluetcl_run_compare_pass" {
            let Some((script, optional)) = arguments.split_first() else {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            };
            let Some(contract) = closed_bluetcl_script_contract(&self.origin, script, optional)
            else {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            };

            let span = self.span(node);
            let expansion = self.invocation_stack.clone();
            for (syntax, stdout) in [
                (BluetclSyntax::Bsv, format!("{script}.bluetcl-out")),
                (BluetclSyntax::Bh, format!("{script}.bluetcl-bh-out")),
            ] {
                self.workflow_events
                    .push(crate::workflow::WorkflowEvent::Action(
                        WorkflowAction::BluetclRun(BluetclRunAction {
                            invocation: BluetclInvocation::Script {
                                script: script.clone(),
                                args: Vec::new(),
                                syntax,
                            },
                            working_directory: None,
                            artifact_inputs: contract
                                .artifact_inputs
                                .iter()
                                .map(|path| (*path).to_owned())
                                .collect(),
                            artifact_outputs: contract
                                .artifact_outputs
                                .iter()
                                .map(|path| (*path).to_owned())
                                .collect(),
                            expected_exit: ExpectedExit::Success,
                            stdout: stdout.clone(),
                            guard: self.guard.clone(),
                            span,
                            expansion: expansion.clone(),
                        }),
                    ));
                self.comparisons.push(ComparisonContract {
                    helper: contract.comparison_helper.to_owned(),
                    arguments: vec![stdout],
                    guard: self.guard.clone(),
                    span,
                    expansion: expansion.clone(),
                });
            }
            return true;
        }

        if matches!(
            name,
            "bluetcl_exec_compare_pass" | "bluetcl_exec_compare_fail"
        ) {
            let [command_line, output_name, optional @ ..] = arguments else {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            };
            let command = parse_static_tcl_list(command_line).ok();
            let Some([flag, script, module]) = command.as_deref() else {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            };
            if flag != "-exec"
                || script != "dump_poss.tcl"
                || optional.len() > 3
                || optional.iter().any(|argument| !argument.is_empty())
                || output_name != module
                || !is_closed_bluetcl_module(module)
            {
                self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedCommand);
                return true;
            }

            let stdout = format!("{output_name}.bluetcl-out");
            let span = self.span(node);
            let expansion = self.invocation_stack.clone();
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Action(
                    WorkflowAction::BluetclRun(BluetclRunAction {
                        invocation: BluetclInvocation::Exec {
                            script: script.clone(),
                            args: vec![module.clone()],
                        },
                        working_directory: None,
                        artifact_inputs: vec![format!("{module}.ba")],
                        artifact_outputs: Vec::new(),
                        expected_exit: if name == "bluetcl_exec_compare_pass" {
                            ExpectedExit::Success
                        } else {
                            ExpectedExit::Failure
                        },
                        stdout: stdout.clone(),
                        guard: self.guard.clone(),
                        span,
                        expansion: expansion.clone(),
                    }),
                ));
            self.comparisons.push(ComparisonContract {
                helper: "compare_bluetcl".to_owned(),
                arguments: vec![stdout],
                guard: self.guard.clone(),
                span,
                expansion,
            });
            return true;
        }

        false
    }

    fn lower_workflow_action(&mut self, node: Node<'a>, name: &str, arguments: &[String]) -> bool {
        if matches!(name, "mkdir" | "nukedir" | "touch" | "chmod") {
            self.workflow_boundary();
        }
        let guard = self.guard.clone();
        let span = self.span(node);
        let expansion = self.invocation_stack.clone();
        let action = match (name, arguments) {
            ("after", [milliseconds]) => {
                let Ok(milliseconds) = milliseconds.parse::<u64>() else {
                    return false;
                };
                if !(1..=10_000).contains(&milliseconds) {
                    return false;
                }
                WorkflowAction::Delay(DelayAction {
                    milliseconds,
                    guard,
                    span,
                    expansion,
                })
            }
            ("exec", [command, seconds]) if command == "sleep" => {
                let Ok(seconds) = seconds.parse::<u64>() else {
                    return false;
                };
                let Some(milliseconds) = seconds.checked_mul(1_000) else {
                    return false;
                };
                if !(1..=10_000).contains(&milliseconds) {
                    return false;
                }
                WorkflowAction::Delay(DelayAction {
                    milliseconds,
                    guard,
                    span,
                    expansion,
                })
            }
            ("dumpbi", [input]) if is_intermediate_object_path(input) => {
                WorkflowAction::DumpIntermediate(DumpIntermediateAction {
                    input: input.clone(),
                    output: format!("{input}.dumpbi-out"),
                    view: IntermediateDumpView::Bi,
                    guard,
                    span,
                    expansion,
                })
            }
            ("dumpbo", [input]) if is_intermediate_object_path(input) => {
                WorkflowAction::DumpIntermediate(DumpIntermediateAction {
                    input: input.clone(),
                    output: format!("{input}.dumpbo-out"),
                    view: IntermediateDumpView::Bo,
                    guard,
                    span,
                    expansion,
                })
            }
            ("compile_object_pass" | "bsc_compile_to_object", [source]) => {
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: source.clone(),
                    module: None,
                    options: String::new(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("compile_object_pass" | "bsc_compile_to_object", [source, module]) => {
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: source.clone(),
                    module: (!module.is_empty()).then(|| module.clone()),
                    options: String::new(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("compile_object_pass" | "bsc_compile_to_object", [source, module, options]) => {
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: source.clone(),
                    module: (!module.is_empty()).then(|| module.clone()),
                    options: options.clone(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass" | "bsc_link_objects", [objects, top]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Success,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass" | "bsc_link_objects", [objects, top, options]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    expected_exit: ExpectedExit::Success,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_fail", [objects, top]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Failure,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_fail", [objects, top, options]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    expected_exit: ExpectedExit::Failure,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass_bug", [objects, top]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Success,
                    expectation: unannotated_known_failure_expectation(""),
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass_bug", [objects, top, bug]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Success,
                    expectation: known_bug_expectation(bug),
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass_bug", [objects, top, bug, options]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    expected_exit: ExpectedExit::Success,
                    expectation: known_bug_expectation(bug),
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_fail_error", [objects, top, code]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Failure,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: Some(LinkErrorDiagnostic {
                        code: code.clone(),
                        count: "1".to_owned(),
                    }),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_fail_error", [objects, top, code, count]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Failure,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: Some(LinkErrorDiagnostic {
                        code: code.clone(),
                        count: count.clone(),
                    }),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_fail_error", [objects, top, code, count, options]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    expected_exit: ExpectedExit::Failure,
                    expectation: OperationExpectation::Required,
                    error_diagnostic: Some(LinkErrorDiagnostic {
                        code: code.clone(),
                        count: count.clone(),
                    }),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_fail", [objects, top]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    no_main: false,
                    expected_exit: ExpectedExit::Failure,
                    simulator: IcarusSimulatorSelector::Default,
                    expectation: OperationExpectation::Required,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_fail", [objects, top, options])
                if !options.split_whitespace().any(|option| option == "-vsim") =>
            {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    no_main: false,
                    expected_exit: ExpectedExit::Failure,
                    simulator: IcarusSimulatorSelector::Default,
                    expectation: OperationExpectation::Required,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_pass", [objects, top]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    no_main: false,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: OperationExpectation::Required,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_pass", [objects, top, options]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    no_main: false,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: OperationExpectation::Required,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_pass_bug", [objects, top]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    no_main: false,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: unannotated_known_failure_expectation(""),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_pass_bug", [objects, top, bug]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    no_main: false,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: unannotated_known_failure_expectation(bug),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_pass_bug", [objects, top, bug, options]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    no_main: false,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: unannotated_known_failure_expectation(bug),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_no_main_pass", [objects, top]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    no_main: true,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: OperationExpectation::Required,
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_verilog_no_main_pass", [objects, top, options]) => {
                WorkflowAction::LinkVerilog(LinkVerilogAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    no_main: true,
                    expected_exit: ExpectedExit::Success,
                    simulator: bsc_test_plan::IcarusSimulatorSelector::Default,
                    expectation: OperationExpectation::Required,
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_output", [executable]) => WorkflowAction::RunBluesim(RunBluesimAction {
                executable: executable.clone(),
                options: String::new(),
                stdout: format!("{executable}.out"),
                expected_exits: Vec::new(),
                aarch64_expected_exits: None,
                windows_expected_exits: None,
                guard,
                span,
                expansion,
            }),
            ("sim_output", [executable, options]) => WorkflowAction::RunBluesim(RunBluesimAction {
                executable: executable.clone(),
                options: options.clone(),
                stdout: format!("{executable}.out"),
                expected_exits: Vec::new(),
                aarch64_expected_exits: None,
                windows_expected_exits: None,
                guard,
                span,
                expansion,
            }),
            ("sim_output_status", [executable, status]) => {
                let Some(expected_exits) = static_exit_statuses(status) else {
                    return false;
                };
                WorkflowAction::RunBluesim(RunBluesimAction {
                    executable: executable.clone(),
                    options: String::new(),
                    stdout: format!("{executable}.out"),
                    expected_exits,
                    aarch64_expected_exits: None,
                    windows_expected_exits: self.pinned_windows_simulation_exits(executable),
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_final_state", [executable, cycles]) if cycles.parse::<u64>().is_ok() => {
                WorkflowAction::RunBluesim(RunBluesimAction {
                    executable: executable.clone(),
                    options: format!("-m {cycles} -s"),
                    stdout: format!("{executable}.final-state"),
                    expected_exits: Vec::new(),
                    aarch64_expected_exits: None,
                    windows_expected_exits: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_output_status", [executable, status, options]) => {
                let Some(expected_exits) = static_exit_statuses(status) else {
                    return false;
                };
                WorkflowAction::RunBluesim(RunBluesimAction {
                    executable: executable.clone(),
                    options: options.clone(),
                    stdout: format!("{executable}.out"),
                    expected_exits,
                    aarch64_expected_exits: None,
                    windows_expected_exits: self.pinned_windows_simulation_exits(executable),
                    guard,
                    span,
                    expansion,
                })
            }
            ("create_systemc_objects_pass", [objects, top]) => {
                WorkflowAction::LinkSystemc(SystemcLinkAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Success,
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("create_systemc_objects_pass", [objects, top, options]) => {
                WorkflowAction::LinkSystemc(SystemcLinkAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    expected_exit: ExpectedExit::Success,
                    error_diagnostic: None,
                    guard,
                    span,
                    expansion,
                })
            }
            ("create_systemc_objects_fail_error", [objects, top, code]) => {
                WorkflowAction::LinkSystemc(SystemcLinkAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Failure,
                    error_diagnostic: Some(LinkErrorDiagnostic {
                        code: code.clone(),
                        count: "1".to_owned(),
                    }),
                    guard,
                    span,
                    expansion,
                })
            }
            ("create_systemc_objects_fail_error", [objects, top, code, count]) => {
                WorkflowAction::LinkSystemc(SystemcLinkAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    expected_exit: ExpectedExit::Failure,
                    error_diagnostic: Some(LinkErrorDiagnostic {
                        code: code.clone(),
                        count: count.clone(),
                    }),
                    guard,
                    span,
                    expansion,
                })
            }
            ("create_systemc_objects_fail_error", [objects, top, code, count, options]) => {
                WorkflowAction::LinkSystemc(SystemcLinkAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    expected_exit: ExpectedExit::Failure,
                    error_diagnostic: Some(LinkErrorDiagnostic {
                        code: code.clone(),
                        count: count.clone(),
                    }),
                    guard,
                    span,
                    expansion,
                })
            }
            ("build_systemc_executable_pass", [executable, sources, top_modules]) => {
                WorkflowAction::BuildSystemc(SystemcBuildAction {
                    executable: executable.clone(),
                    sources: sources.clone(),
                    top_modules: top_modules.clone(),
                    other_modules: String::new(),
                    options: String::new(),
                    guard,
                    span,
                    expansion,
                })
            }
            (
                "build_systemc_executable_pass",
                [executable, sources, top_modules, other_modules],
            ) => WorkflowAction::BuildSystemc(SystemcBuildAction {
                executable: executable.clone(),
                sources: sources.clone(),
                top_modules: top_modules.clone(),
                other_modules: other_modules.clone(),
                options: String::new(),
                guard,
                span,
                expansion,
            }),
            (
                "build_systemc_executable_pass",
                [executable, sources, top_modules, other_modules, options],
            ) => WorkflowAction::BuildSystemc(SystemcBuildAction {
                executable: executable.clone(),
                sources: sources.clone(),
                top_modules: top_modules.clone(),
                other_modules: other_modules.clone(),
                options: options.clone(),
                guard,
                span,
                expansion,
            }),
            ("run_systemc_executable", [executable]) => {
                WorkflowAction::RunSystemc(RunSystemcAction {
                    executable: executable.clone(),
                    options: String::new(),
                    expected: format!("{executable}.out.expected"),
                    sort_output: false,
                    guard,
                    span,
                    expansion,
                })
            }
            ("run_systemc_executable", [executable, options]) => {
                WorkflowAction::RunSystemc(RunSystemcAction {
                    executable: executable.clone(),
                    options: options.clone(),
                    expected: format!("{executable}.out.expected"),
                    sort_output: false,
                    guard,
                    span,
                    expansion,
                })
            }
            ("run_systemc_executable", [executable, options, expected]) => {
                WorkflowAction::RunSystemc(RunSystemcAction {
                    executable: executable.clone(),
                    options: options.clone(),
                    expected: if expected.is_empty() {
                        format!("{executable}.out.expected")
                    } else {
                        expected.clone()
                    },
                    sort_output: false,
                    guard,
                    span,
                    expansion,
                })
            }
            ("run_systemc_executable", [executable, options, expected, sort_args])
                if sort_args == "-n" =>
            {
                WorkflowAction::RunSystemc(RunSystemcAction {
                    executable: executable.clone(),
                    options: options.clone(),
                    expected: if expected.is_empty() {
                        format!("{executable}.out.expected")
                    } else {
                        expected.clone()
                    },
                    sort_output: true,
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_verilog", [executable]) | ("sim_verilog_vcd", [executable]) => {
                WorkflowAction::RunVerilog(RunVerilogAction {
                    executable: executable.clone(),
                    options: String::new(),
                    stdout: format!("{executable}.out"),
                    expected_exits: Vec::new(),
                    vcd: name == "sim_verilog_vcd",
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_verilog", [executable, options]) | ("sim_verilog_vcd", [executable, options]) => {
                WorkflowAction::RunVerilog(RunVerilogAction {
                    executable: executable.clone(),
                    options: options.clone(),
                    stdout: format!("{executable}.out"),
                    expected_exits: Vec::new(),
                    vcd: name == "sim_verilog_vcd",
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_verilog_status", [executable, status]) => {
                let Some(expected_exits) = static_exit_statuses(status) else {
                    return false;
                };
                WorkflowAction::RunVerilog(RunVerilogAction {
                    executable: executable.clone(),
                    options: String::new(),
                    stdout: format!("{executable}.out"),
                    expected_exits,
                    vcd: false,
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_verilog_status", [executable, status, options]) => {
                let Some(expected_exits) = static_exit_statuses(status) else {
                    return false;
                };
                WorkflowAction::RunVerilog(RunVerilogAction {
                    executable: executable.clone(),
                    options: options.clone(),
                    stdout: format!("{executable}.out"),
                    expected_exits,
                    vcd: false,
                    guard,
                    span,
                    expansion,
                })
            }
            ("showrules", [top, input, output])
                if self.is_pinned_showrules_source()
                    && guard_contains_capability(&guard, Capability::ShowRules) =>
            {
                WorkflowAction::ShowRules(ShowRulesAction {
                    top: top.clone(),
                    input: self.workspace_path(input),
                    output: self.workspace_path(output),
                    stdout: self.workspace_path(&format!("{input}.showrules-out")),
                    guard,
                    span,
                    expansion,
                })
            }
            ("copy", [source, destination]) | ("move", [source, destination]) => {
                let operation = if name == "copy" {
                    ArtifactTransferOperation::Copy
                } else {
                    ArtifactTransferOperation::Move
                };
                WorkflowAction::TransferArtifact(ArtifactTransferAction {
                    operation,
                    source: self.workspace_path(source),
                    destination: self.workspace_path(destination),
                    guard,
                    span,
                    expansion,
                })
            }
            ("erase", [path]) => WorkflowAction::EraseArtifact(EraseArtifactAction {
                path: self.workspace_path(path),
                guard,
                span,
                expansion,
            }),
            ("nukedir", [path]) => {
                WorkflowAction::EnsureDirectoryAbsent(EnsureDirectoryAbsentAction {
                    path: self.workspace_path(path),
                    guard,
                    span,
                    expansion,
                })
            }
            ("mkdir", [path]) => WorkflowAction::CreateDirectory(CreateDirectoryAction {
                path: self.workspace_path(path),
                guard,
                span,
                expansion,
            }),
            ("touch", [path]) => {
                let path = self.workspace_path(path);
                if matches!(
                    self.origin.as_str(),
                    "testsuite/bsc.driver/depend/depend.exp"
                        | "testsuite/bsc.driver/imports/imports.exp"
                        | "testsuite/bsc.preprocessor/include/include.exp"
                        | OPTIONS_ORIGIN
                ) {
                    WorkflowAction::TouchCreateArtifact(TouchCreateArtifactAction {
                        path,
                        delay_milliseconds: 1_000,
                        guard,
                        span,
                        expansion,
                    })
                } else {
                    WorkflowAction::TouchArtifact(TouchArtifactAction {
                        path,
                        guard,
                        span,
                        expansion,
                    })
                }
            }
            ("chmod", [mode, path]) if mode == "u-r" => {
                WorkflowAction::RemoveUserRead(RemoveUserReadAction {
                    path: self.workspace_path(path),
                    guard,
                    span,
                    expansion,
                })
            }
            _ => return false,
        };
        self.workflow_events
            .push(crate::workflow::WorkflowEvent::Action(action));
        if matches!(name, "mkdir" | "nukedir" | "touch" | "chmod") {
            self.workflow_boundary();
        }
        true
    }

    fn is_pinned_options_source(&self) -> bool {
        self.origin == OPTIONS_ORIGIN
            && format!("{:x}", Sha256::digest(self.source)) == OPTIONS_SHA256
    }

    fn lower_procedure_call(
        &mut self,
        node: Node<'a>,
        name: &str,
        arguments: Vec<String>,
        procedure: Procedure<'a>,
    ) {
        let required = procedure
            .parameters
            .iter()
            .take_while(|parameter| parameter.default.is_none())
            .count();
        if arguments.len() < required
            || arguments.len() > procedure.parameters.len()
            || self.call_stack.iter().any(|item| item == name)
        {
            self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedControlFlow);
            return;
        }

        let previous_constants = self.constants.clone();
        let mut provided = arguments.into_iter();
        for parameter in procedure.parameters.iter() {
            let value = match provided.next() {
                Some(value) => value,
                None => match parameter.default.as_deref() {
                    Some(value) => value.to_owned(),
                    None => {
                        self.push_unsupported(
                            node,
                            Some(name),
                            UnsupportedReason::UnsupportedControlFlow,
                        );
                        self.constants = previous_constants;
                        return;
                    }
                },
            };
            self.constants
                .insert(parameter.name.clone(), StaticValue::Scalar(value));
        }
        self.call_stack.push(name.to_owned());
        self.invocation_stack.push(self.span(node));
        self.lower_script_node(procedure.body);
        self.invocation_stack.pop();
        self.call_stack.pop();
        self.constants = previous_constants;
    }

    fn static_arguments(&self, arguments: Node<'a>) -> Option<Vec<String>> {
        let mut values = Vec::<String>::new();
        let mut previous_end = None;
        let mut cursor = arguments.walk();
        for fragment in arguments.named_children(&mut cursor) {
            let value = self.static_word(fragment)?;
            let starts_new_word = previous_end.is_none_or(|end| {
                self.source[end..fragment.start_byte()]
                    .iter()
                    .any(u8::is_ascii_whitespace)
            });
            if starts_new_word {
                values.push(value);
            } else {
                values.last_mut()?.push_str(&value);
            }
            previous_end = Some(fragment.end_byte());
        }
        Some(values)
    }

    fn static_list(&self, node: Node<'a>) -> Option<Vec<String>> {
        if node.kind() != "command_substitution" {
            return None;
        }
        let command = node.named_child(0)?;
        let name = command.child_by_field_name("name")?;
        if self.text(name).trim() != "list" {
            return None;
        }
        match command.child_by_field_name("arguments") {
            Some(arguments) => self.static_arguments(arguments),
            None => Some(Vec::new()),
        }
    }

    fn static_word(&self, node: Node<'a>) -> Option<String> {
        let raw = self.text(node);
        match node.kind() {
            "simple_word" | "id" => Some(raw.to_owned()),
            "braced_word" | "braced_word_simple" => {
                Some(strip_delimiters(raw, '{', '}').to_owned())
            }
            "quoted_word" => self.static_composite_word(node, true),
            "variable_substitution" => {
                let variable = raw
                    .strip_prefix("${")
                    .and_then(|value| value.strip_suffix('}'))
                    .or_else(|| raw.strip_prefix('$'))?;
                Some(self.constants.get(variable)?.as_string())
            }
            "command_substitution" => self.static_command_substitution(node),
            "escaped_character" => raw.strip_prefix('\\').map(str::to_owned),
            _ if node.named_child_count() != 0 => self.static_composite_word(node, false),
            _ => (!contains_dynamic_syntax(raw))
                .then(|| raw.trim_matches(['"', '{', '}', '[', ']']).to_owned()),
        }
    }

    fn static_command_substitution(&self, node: Node<'a>) -> Option<String> {
        let command = node.named_child(0)?;
        let name = command.child_by_field_name("name")?;
        let name = self.text(name).trim();
        let arguments = match command.child_by_field_name("arguments") {
            Some(arguments) => self.static_arguments(arguments)?,
            None => Vec::new(),
        };
        match (name, arguments.as_slice()) {
            ("list", _) => Some(arguments.join(" ")),
            ("make_bsc_output_name", [source]) => Some(format!("{source}.bsc-out")),
            ("make_bsc_ccomp_output_name", [source]) => Some(format!("{source}.bsc-ccomp-out")),
            ("make_bsc_vcomp_output_name", [source]) => Some(format!("{source}.bsc-vcomp-out")),
            ("make_bsc_sched_output_name", [source]) => Some(format!("{source}.bsc-sched-out")),
            ("make_dumpbi_output_name", [input]) if is_intermediate_object_path(input) => {
                Some(format!("{input}.dumpbi-out"))
            }
            ("make_dumpbo_output_name", [input]) if is_intermediate_object_path(input) => {
                Some(format!("{input}.dumpbo-out"))
            }
            _ => None,
        }
    }

    fn static_composite_word(&self, node: Node<'a>, quoted: bool) -> Option<String> {
        let mut start = node.start_byte();
        let mut end = node.end_byte();
        if quoted {
            start += 1;
            end = end.checked_sub(1)?;
        }

        let mut value = String::new();
        let mut offset = start;
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.start_byte() < start || child.end_byte() > end {
                continue;
            }
            let prefix = std::str::from_utf8(&self.source[offset..child.start_byte()]).ok()?;
            if child.kind() == "variable_substitution" && prefix.ends_with('\\') {
                value.push_str(prefix.strip_suffix('\\').expect("checked suffix"));
                value.push_str(self.text(child));
            } else {
                value.push_str(prefix);
                value.push_str(&self.static_word(child)?);
            }
            offset = child.end_byte();
        }
        value.push_str(std::str::from_utf8(&self.source[offset..end]).ok()?);
        Some(value)
    }

    fn workflow_boundary(&mut self) {
        if !matches!(
            self.workflow_events.last(),
            Some(crate::workflow::WorkflowEvent::Boundary)
        ) {
            self.workflow_events
                .push(crate::workflow::WorkflowEvent::Boundary);
        }
    }

    fn push_unsupported(
        &mut self,
        node: Node<'a>,
        command: Option<&str>,
        reason: UnsupportedReason,
    ) {
        self.workflow_boundary();
        self.unsupported.push(UnsupportedConstruct {
            command: command.map(str::to_owned),
            reason,
            span: self.span(node),
            expansion: self.invocation_stack.clone(),
        });
    }

    fn text(&self, node: Node<'a>) -> &str {
        std::str::from_utf8(&self.source[node.byte_range()]).unwrap_or_default()
    }

    fn span(&self, node: Node<'a>) -> SourceSpan {
        let (start_line, start_column) = source_position(self.source, node.start_byte());
        let (end_line, end_column) = source_position(self.source, node.end_byte());
        SourceSpan {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}

fn source_line_range_span(source: &[u8], start_line: usize, end_line: usize) -> SourceSpan {
    let mut line_starts = vec![0usize];
    line_starts.extend(
        source
            .iter()
            .enumerate()
            .filter_map(|(index, byte)| (*byte == b'\n').then_some(index + 1)),
    );
    let start_byte = line_starts[start_line - 1];
    let end_byte = line_starts
        .get(end_line)
        .copied()
        .unwrap_or(source.len())
        .saturating_sub(1)
        .max(start_byte);
    let (end_line, end_column) = source_position(source, end_byte);
    SourceSpan {
        start_byte,
        end_byte,
        start_line,
        start_column: 1,
        end_line,
        end_column,
    }
}

struct SimulationShape {
    source: String,
    backends: Vec<SimulationBackend>,
    separate_generation: bool,
}

impl SimulationShape {
    fn generation_for(&self, backend: SimulationBackend) -> GenerationStrategy {
        if self.backends.len() == 2 && !self.separate_generation {
            GenerationStrategy::Shared
        } else {
            match backend {
                SimulationBackend::Bluesim => GenerationStrategy::Bluesim,
                SimulationBackend::Icarus => GenerationStrategy::Icarus,
            }
        }
    }
}

fn simulation_shape(name: &str, arguments: &[String]) -> Option<SimulationShape> {
    let module = arguments.first()?.trim();
    if module.is_empty() {
        return None;
    }
    if name == "test_c_veri_worker" {
        if !(9..=11).contains(&arguments.len()) {
            return None;
        }
        let extension = arguments.get(3)?.trim();
        if extension.is_empty() {
            return None;
        }
        let enabled = |index: usize| {
            arguments
                .get(index)?
                .parse::<i64>()
                .ok()
                .map(|value| value != 0)
        };
        let mut backends = Vec::new();
        if enabled(4)? {
            backends.push(SimulationBackend::Bluesim);
        }
        if enabled(5)? {
            backends.push(SimulationBackend::Icarus);
        }
        if backends.is_empty() {
            return None;
        }
        return Some(SimulationShape {
            source: format!("{module}.{extension}"),
            backends,
            separate_generation: false,
        });
    }
    let extension = if matches!(
        name,
        "test_c_veri"
            | "test_c_veri_bs_modules"
            | "test_c_veri_bs_modules_options"
            | "test_c_only"
            | "test_c_only_bs_modules_options"
            | "test_veri_only"
    ) {
        "bs"
    } else {
        "bsv"
    };
    let source = format!("{module}.{extension}");
    let separate_generation = name.contains("separately")
        || name == "test_c_only"
        || name.starts_with("test_c_only_")
        || name == "test_veri_only"
        || name.starts_with("test_veri_only_");
    let backends = if name == "test_c_only" || name.starts_with("test_c_only_") {
        vec![SimulationBackend::Bluesim]
    } else if name == "test_veri_only" || name.starts_with("test_veri_only_") {
        vec![SimulationBackend::Icarus]
    } else if matches!(
        name,
        "test_c_veri_bsv_multi_options" | "test_c_veri_bsv_multi_options_separately"
    ) {
        let enabled = |index: usize| match arguments.get(index).map(String::as_str) {
            None | Some("") | Some("1") => Some(true),
            Some("0") => Some(false),
            Some(_) => None,
        };
        let bluesim = enabled(7)?;
        let icarus = enabled(8)?;
        let mut backends = Vec::new();
        if bluesim {
            backends.push(SimulationBackend::Bluesim);
        }
        if icarus {
            backends.push(SimulationBackend::Icarus);
        }
        if backends.is_empty() {
            return None;
        }
        backends
    } else if is_simulation_helper(name) {
        vec![SimulationBackend::Bluesim, SimulationBackend::Icarus]
    } else {
        return None;
    };
    Some(SimulationShape {
        source,
        backends,
        separate_generation,
    })
}

fn static_string_compare_empty_condition(
    condition: &str,
    constants: &BTreeMap<String, StaticValue>,
) -> Option<bool> {
    let normalized = condition
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .unwrap_or(condition)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let variable = normalized
        .strip_prefix("[string compare $")?
        .strip_suffix(" \"\"] == 0")?;
    if !is_static_variable_name(variable) {
        return None;
    }
    constants
        .get(variable)
        .map(StaticValue::as_string)
        .map(|value| value.is_empty())
}

fn static_literal_boolean_condition(condition: &str) -> Option<bool> {
    let normalized = condition
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .unwrap_or(condition)
        .trim()
        .trim_matches('"')
        .trim();
    if is_statically_true_probe(normalized) {
        return Some(true);
    }
    match normalized.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        value => value.parse::<i64>().ok().map(|value| value != 0),
    }
}

fn pinned_iverilog_major_minor(version: &str) -> Option<String> {
    let mut segments = version.split('.');
    let major = segments.next()?.parse::<u32>().ok()?;
    let minor = segments.next()?.parse::<u32>().ok()?;
    Some(format!("{major}.{minor}"))
}

fn pinned_iverilog_major(major_minor: &str) -> u32 {
    major_minor
        .split_once('.')
        .and_then(|(major, _)| major.parse::<u32>().ok())
        .expect("pinned Icarus major/minor is numeric")
}

fn configured_iverilog_major_minor() -> Option<String> {
    include_str!("../../../../pixi.toml")
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("iverilog = \"")?.strip_suffix("\""))
        .and_then(pinned_iverilog_major_minor)
}

struct ClosedMakedependInvocation {
    command: BluetclMakedependCommand,
    args: Vec<String>,
    working_directory: Option<&'static str>,
    artifact_inputs: Vec<String>,
    artifact_outputs: Vec<String>,
    expected_exit: ExpectedExit,
}

fn closed_makedepend_invocation(
    command_line: &str,
    output_name: &str,
) -> Option<ClosedMakedependInvocation> {
    const INPUTS: [&str; 7] = [
        "Dep1.bsv",
        "Foo.bsv",
        "IncDep1.bsv",
        "IncDep2.bsv",
        "Test.bsv",
        "include1.inc",
        "subinclude.inc",
    ];
    let words = parse_static_tcl_list(command_line).ok()?;
    let [flag, executable, args @ ..] = words.as_slice() else {
        return None;
    };
    if flag != "-exec" {
        return None;
    }
    let command = match executable.as_str() {
        "makedepend" => BluetclMakedependCommand::Makedepend,
        "makedepend.tcl" => BluetclMakedependCommand::MakedependTcl,
        _ => return None,
    };
    let expected = match output_name {
        "usage1" if words == ["-exec", "makedepend"] => ExpectedExit::Failure,
        "usage2" if words == ["-exec", "makedepend.tcl"] => ExpectedExit::Failure,
        "nofile" if words == ["-exec", "makedepend", "-v"] => ExpectedExit::Failure,
        "badflag" if words == ["-exec", "makedepend", "-xxx", "Dep1.bsv"] => ExpectedExit::Failure,
        "error1" if words == ["-exec", "makedepend", "-D", "SYNTAXERROR", "Dep1.bsv"] => {
            ExpectedExit::Failure
        }
        "error2" if words == ["-exec", "makedepend", "-D", "CIRCERROR", "Dep1.bsv"] => {
            ExpectedExit::Failure
        }
        "test1" if words == ["-exec", "makedepend", "-no-show-timestamps", "Dep1.bsv"] => {
            ExpectedExit::Success
        }
        "patterns" if words == ["-exec", "makedepend", "-no-show-timestamps", "*.bsv"] => {
            ExpectedExit::Success
        }
        "defines"
            if words
                == [
                    "-exec",
                    "makedepend",
                    "-no-show-timestamps",
                    "-D",
                    "INC1",
                    "Dep1.bsv",
                ] =>
        {
            ExpectedExit::Success
        }
        "bdir"
            if words
                == [
                    "-exec",
                    "makedepend",
                    "-no-show-timestamps",
                    "-bdir",
                    "objs",
                    "-D",
                    "INC1",
                    "Dep1.bsv",
                ] =>
        {
            ExpectedExit::Success
        }
        "updir"
            if words
                == [
                    "-exec",
                    "makedepend",
                    "-no-show-timestamps",
                    "-bdir",
                    "objs",
                    "-p",
                    "../makedepend/:%/Libraries",
                    "-D",
                    "INC1",
                    "Dep1.bsv",
                ] =>
        {
            ExpectedExit::Success
        }
        "minus_o"
            if words
                == [
                    "-exec",
                    "makedepend",
                    "-no-show-timestamps",
                    "-o",
                    "minusO.depend-out",
                    "Dep1.bsv",
                ] =>
        {
            ExpectedExit::Success
        }
        _ => return None,
    };
    let working_directory = (output_name == "updir").then_some("makedepend");
    let needs_inputs = !matches!(output_name, "usage1" | "usage2" | "nofile");
    let artifact_inputs = needs_inputs
        .then(|| {
            INPUTS
                .iter()
                .map(|path| {
                    working_directory.map_or_else(
                        || (*path).to_owned(),
                        |directory| format!("{directory}/{path}"),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let artifact_outputs = (output_name == "minus_o")
        .then(|| vec!["minusO.depend-out".to_owned()])
        .unwrap_or_default();
    Some(ClosedMakedependInvocation {
        command,
        args: args.to_vec(),
        working_directory,
        artifact_inputs,
        artifact_outputs,
        expected_exit: expected,
    })
}

struct ClosedBluetclScriptContract {
    artifact_inputs: &'static [&'static str],
    artifact_outputs: &'static [&'static str],
    comparison_helper: &'static str,
}

fn closed_bluetcl_script_contract(
    origin: &str,
    script: &str,
    optional: &[String],
) -> Option<ClosedBluetclScriptContract> {
    if optional.len() > 4 || !is_closed_bluetcl_script(script) {
        return None;
    }
    let argument = |index: usize| optional.get(index).map_or("", |value| value.trim());
    if !argument(0).is_empty() || !argument(1).is_empty() {
        return None;
    }
    let filter = (argument(2), argument(3));
    let comparison_helper = match filter {
        ("", "") => "compare_bluetcl",
        ("-e /position.*%/s/\\[0-9\\]/N/g", "") => "compare_bluetcl_position_digits",
        ("", "-e s/CReg\\[0-9\\]\\+/CRegNNNN/g") => "compare_bluetcl_creg_positions",
        ("-e s/Libraries.*Library/IGNORED/", "") => "compare_bluetcl_libraries",
        ("-e s/Prelude.*Library/IGNORED/", "") => "compare_bluetcl_prelude_library",
        _ => return None,
    };

    let artifact_inputs: &'static [&'static str] = match (origin, script) {
        ("testsuite/bsc.bluetcl/commands/commands.exp", "help.tcl")
        | ("testsuite/bsc.bluetcl/commands/commands.exp", "syntax.tcl")
        | ("testsuite/bsc.bluetcl/commands/commands.exp", "syntax_errs.tcl") => &[],
        ("testsuite/bsc.bluetcl/commands/commands.exp", "bpackage.tcl")
        | ("testsuite/bsc.bluetcl/commands/commands.exp", "browsepackage.tcl") => &["Test.bo"],
        ("testsuite/bsc.bluetcl/commands/commands.exp", "type.tcl")
        | ("testsuite/bsc.bluetcl/commands/commands.exp", "type_bh.tcl") => {
            &["pprint.tcl", "Test.bo", "TaggedUnionPoly.bo"]
        }
        ("testsuite/bsc.bluetcl/commands/commands.exp", "module.tcl")
        | ("testsuite/bsc.bluetcl/commands/commands.exp", "submodule.tcl")
        | ("testsuite/bsc.bluetcl/commands/commands.exp", "browseinst.tcl") => &["mkT.ba"],
        ("testsuite/bsc.bluetcl/commands/commands.exp", "schedule.tcl") => &["mkS.ba", "mkM.ba"],
        ("testsuite/bsc.bluetcl/commands/commands.exp", "schedule_err.tcl") => {
            &["mkTestSchedErr.ba"]
        }
        ("testsuite/bsc.bluetcl/commands/commands.exp", "browsemodule.tcl") => {
            &["mkT.ba", "mkM.ba"]
        }
        ("testsuite/bsc.bluetcl/commands/commands.exp", "depend.tcl") => &[
            "Test.bsv",
            "Dep1.bsv",
            "IncDep1.bsv",
            "IncDep2.bsv",
            "subdir/Foo.bs",
        ],
        ("testsuite/bsc.bluetcl/commands/commands.exp", "browseinst2.tcl") => &["mkTest.ba"],
        ("testsuite/bsc.bluetcl/hierarchy/hierarchy.exp", "Design.tcl") => &["mkDesign.ba"],
        ("testsuite/bsc.bluetcl/hierarchy/hierarchy.exp", "Example.tcl") => &["mkExample.ba"],
        ("testsuite/bsc.bluetcl/targeted/port_types/port_types.exp", "inhigh.tcl") => {
            &["sysInhighEnable.ba"]
        }
        ("testsuite/bsc.bluetcl/targeted/port_types/port_types.exp", "zero_size.tcl") => {
            &["sysZeroSize.ba"]
        }
        ("testsuite/bsc.bluetcl/targeted/port_types/port_types.exp", "prims.tcl") => {
            &["sysPrims.ba"]
        }
        ("testsuite/bsc.bluetcl/targeted/port_types/port_types.exp", "split_port_types.tcl") => {
            &["mkSplitPortTypes.ba"]
        }
        ("testsuite/bsc.bluetcl/targeted/type/type.exp", "polyfield.tcl") => &["PolyField.bo"],
        (_, "utils_test.tcl") if origin.ends_with("/packages/utils/utils.exp") => &[],
        _ if filter == ("", "") => &[],
        _ => return None,
    };

    let artifact_outputs = match (origin, script) {
        ("testsuite/bsc.bluetcl/packages/InstSynth/InstSynth.exp", "instsynth.tcl") => {
            &["FIFO.include.bsv", "FIFOLevel.include.bsv"][..]
        }
        _ => &[],
    };

    Some(ClosedBluetclScriptContract {
        artifact_inputs,
        artifact_outputs,
        comparison_helper,
    })
}

fn is_closed_bluetcl_script(path: &str) -> bool {
    path.ends_with(".tcl")
        && path.len() > ".tcl".len()
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn is_closed_bluetcl_module(module: &str) -> bool {
    !module.is_empty()
        && module
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_intermediate_object_path(path: &str) -> bool {
    !path.is_empty()
        && path.ends_with(".bo")
        && !path.contains(['\\', '/', '*', '?', '[', ']', '$'])
}

/// Parses the deliberately closed capability expression subset used by upstream tests.
///
/// This is not a Tcl expression evaluator: it accepts only known capability atoms,
/// redundant parentheses, and conjunction. Runtime probes, negation, disjunction,
/// comparisons outside the enumerated atoms, and command substitution all remain
/// unsupported unless they are the exact `do_internal_checks` capability atom.
fn capability_condition(condition: &str) -> Option<Guard> {
    parse_capability_conjunction(strip_capability_wrappers(condition.trim())?)
}

fn parse_capability_conjunction(expression: &str) -> Option<Guard> {
    let expression = strip_balanced_parentheses(expression.trim())?;
    let terms = split_top_level_conjunction(expression)?;
    if terms.len() > 1 {
        let non_probe = terms
            .iter()
            .filter(|term| !is_statically_true_probe(term))
            .copied()
            .collect::<Vec<_>>();
        if non_probe.is_empty() {
            return None;
        }
        let guards = non_probe
            .iter()
            .map(|term| parse_capability_conjunction(term))
            .collect::<Option<Vec<_>>>()?;
        if guards.len() == 1 {
            return guards.into_iter().next();
        }
        return Some(Guard::All { guards });
    }
    if is_statically_true_probe(expression) {
        return None;
    }
    let capability = match normalize_capability_atom(expression).as_str() {
        "$ctest" | "$ctest == 1" | "$ctest != 0" => Capability::Bluesim,
        "$vtest" | "$vtest == 1" | "$vtest != 0" => Capability::Verilog,
        "$systemctest == 1" => Capability::SystemC,
        "[do_internal_checks]" => Capability::InternalChecks,
        "[bluetcl_package_available InstSynth] == 1" => {
            Capability::BluetclPackage(BluetclPackage::InstSynth)
        }
        _ => return None,
    };
    Some(Guard::Capability { capability })
}

/// Recognizes toolchain probes that are statically true under the pinned
/// runner environment. `isPositiveReset` greps `BSC_OPTIONS` for
/// `BSV_POSITIVE_RESET`; the runner leaves `BSC_OPTIONS` empty unless a
/// scenario explicitly appends options, and no audited plan appends the
/// positive-reset define, so the probe answers 0.
fn is_statically_true_probe(term: &str) -> bool {
    matches!(
        normalize_capability_atom(term).as_str(),
        "[isPositiveReset] == 0"
    )
}

fn strip_capability_wrappers(expression: &str) -> Option<&str> {
    let expression = expression.trim();
    if let Some(inner) = expression.strip_prefix('{') {
        return inner.strip_suffix('}').map(str::trim);
    }
    (!expression.ends_with('}')).then_some(expression)
}

fn strip_balanced_parentheses(mut expression: &str) -> Option<&str> {
    loop {
        expression = expression.trim();
        if !(expression.starts_with('(') && expression.ends_with(')')) {
            return Some(expression);
        }
        let mut depth = 0usize;
        let mut encloses_all = true;
        for (index, character) in expression.char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 && index + character.len_utf8() != expression.len() {
                        encloses_all = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 || !encloses_all {
            return Some(expression);
        }
        expression = &expression[1..expression.len() - 1];
    }
}

fn split_top_level_conjunction(expression: &str) -> Option<Vec<&str>> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut terms = Vec::new();
    let bytes = expression.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.checked_sub(1)?,
            b'&' if depth == 0 && bytes.get(index + 1) == Some(&b'&') => {
                let term = expression[start..index].trim();
                if term.is_empty() {
                    return None;
                }
                terms.push(term);
                index += 1;
                start = index + 1;
            }
            b'&' | b'|' if depth == 0 => return None,
            b'!' if depth == 0 && bytes.get(index + 1) != Some(&b'=') => return None,
            _ => {}
        }
        index += 1;
    }
    if depth != 0 {
        return None;
    }
    let term = expression[start..].trim();
    if term.is_empty() {
        return None;
    }
    terms.push(term);
    Some(terms)
}

fn normalize_capability_atom(expression: &str) -> String {
    expression.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn known_bug_expectation(annotation: &str) -> OperationExpectation {
    let annotation = annotation.trim();
    if annotation.is_empty() {
        OperationExpectation::Required
    } else {
        OperationExpectation::Xfail {
            reason: format!("upstream bug {annotation}"),
        }
    }
}

fn unannotated_known_failure_expectation(annotation: &str) -> OperationExpectation {
    let annotation = annotation.trim();
    OperationExpectation::Xfail {
        reason: if annotation.is_empty() {
            "upstream unannotated known failure".to_owned()
        } else {
            format!("upstream bug {annotation}")
        },
    }
}

fn negate_guard(guard: Guard) -> Guard {
    match guard {
        Guard::Not { guard } => *guard,
        guard => Guard::Not {
            guard: Box::new(guard),
        },
    }
}

fn guard_contains_capability(guard: &Guard, expected: Capability) -> bool {
    match guard {
        Guard::Capability { capability } => *capability == expected,
        Guard::All { guards } => guards
            .iter()
            .any(|guard| guard_contains_capability(guard, expected)),
        Guard::Always | Guard::Not { .. } | Guard::UnsupportedExpression { .. } => false,
    }
}

fn combine_guards(outer: Guard, inner: Guard) -> Guard {
    match (outer, inner) {
        (Guard::Always, guard) | (guard, Guard::Always) => guard,
        (left, right) if left == right => left,
        (Guard::All { mut guards }, Guard::All { guards: inner }) => {
            guards.extend(inner);
            Guard::All { guards }
        }
        (Guard::All { mut guards }, guard) => {
            guards.push(guard);
            Guard::All { guards }
        }
        (guard, Guard::All { mut guards }) => {
            guards.insert(0, guard);
            Guard::All { guards }
        }
        (left, right) => Guard::All {
            guards: vec![left, right],
        },
    }
}

fn static_exit_statuses(value: &str) -> Option<Vec<i32>> {
    let statuses = crate::parse_static_tcl_list(value).ok()?;
    if statuses.is_empty() {
        return None;
    }
    let mut codes = Vec::new();
    for status in statuses {
        if status == "SIGFPE" {
            codes.extend([8, 136]);
        } else {
            codes.push(status.parse::<i32>().ok()?);
        }
    }
    codes.sort_unstable();
    codes.dedup();
    Some(codes)
}

fn is_compile_helper(name: &str) -> bool {
    matches!(
        name,
        "compile_pass"
            | "bsc_compile"
            | "bsc_compile_verilog"
            | "compile_backend_pass"
            | "compile_pass_bug"
            | "compile_pass_bug_error"
            | "compile_pass_warning"
            | "compile_pass_warning_bug"
            | "compile_pass_no_warning"
            | "compile_fail"
            | "compile_fail_bug"
            | "compile_fail_error"
            | "compile_fail_error_bug"
            | "compile_fail_error_warnings"
            | "compile_object_pass_bug"
            | "compile_object_pass_warning"
            | "compile_object_fail"
            | "compile_object_fail_error"
            | "compile_verilog_pass"
            | "compile_verilog_pass_bug"
            | "compile_synthesize_verilog_pass_bug"
            | "compile_verilog_pass_bug_error"
            | "compile_verilog_pass_no_warning"
            | "compile_verilog_pass_no_warning_bug"
            | "compile_verilog_fail"
            | "compile_verilog_fail_bug"
            | "compile_verilog_fail_error"
            | "compile_verilog_fail_error_bug"
            | "compile_verilog_fail_no_internal_error"
            | "compile_verilog_pass_warning"
            | "compile_verilog_pass_warning_bug"
            | "compile_verilog_schedule_pass"
            | "compile_verilog_schedule_pass_bug"
            | "compile_verilog_schedule_fail"
            | "compile_verilog_schedule_fail_bug"
    )
}

fn is_simulation_helper(name: &str) -> bool {
    matches!(
        name,
        "test_c_only"
            | "test_veri_only"
            | "test_c_veri"
            | "test_c_veri_bs_modules"
            | "test_c_veri_bs_modules_options"
            | "test_c_veri_bsv"
            | "test_c_veri_bsv_modules"
            | "test_c_veri_bsv_modules_options"
            | "test_c_veri_bsv_separately"
            | "test_c_veri_bsv_modules_options_separately"
            | "test_c_veri_bsv_multi"
            | "test_c_veri_bsv_multi_options"
            | "test_c_veri_bsv_multi_options_separately"
            | "test_c_only_bs_modules_options"
            | "test_c_only_bsv"
            | "test_c_only_bsv_modules"
            | "test_c_only_bsv_modules_options"
            | "test_c_only_bsv_multi"
            | "test_c_only_bsv_multi_options"
            | "test_veri_only_bsv"
            | "test_veri_only_bsv_modules"
            | "test_veri_only_bsv_modules_options"
            | "test_veri_only_bsv_multi"
            | "test_veri_only_bsv_multi_options"
            | "test_c_veri_worker"
    )
}

fn closed_binary_ghcrts_contract(origin: &str, source: &[u8]) -> Option<CompileContract> {
    if origin != "testsuite/bsc.binary/binary.exp" {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?;
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized
        != "if [info exists env(GHCRTS)] { set ghcrts_save $env(GHCRTS) } set env(GHCRTS) \"-M1.2G\" compile_verilog_pass ManyMeths.bsv if [info exists ghcrts_save] { set env(GHCRTS) $ghcrts_save } else { unset env(GHCRTS) }"
    {
        return None;
    }
    let command = "compile_verilog_pass ManyMeths.bsv";
    let start_byte = text.find(command)?;
    let end_byte = start_byte + command.len();
    let (start_line, start_column) = source_position(source, start_byte);
    let (end_line, end_column) = source_position(source, end_byte);
    Some(CompileContract {
        source: "ManyMeths.bsv".to_owned(),
        working_directory: None,
        helper: "compile_verilog_pass_ghcrts_m1_2g".to_owned(),
        arguments: vec!["ManyMeths.bsv".to_owned()],
        guard: Guard::Always,
        span: SourceSpan {
            start_byte,
            end_byte,
            start_line,
            start_column,
            end_line,
            end_column,
        },
        expansion: Vec::new(),
    })
}

fn is_assertion_helper(name: &str) -> bool {
    matches!(
        name,
        "find_n_strings"
            | "find_n_error"
            | "find_n_warning"
            | "no_warnings"
            | "string_occurs"
            | "string_does_not_occur"
            | "find_n_strings_bug"
            | "find_regexp"
            | "find_regexp_bug"
            | "find_regexp_fail"
            | "find_regexp_fail_bug"
            | "find_n_regexp"
            | "find_n_emsg"
            | "vcdcheck_pass"
            | "vcdcheck_fail"
    )
}

fn is_builtin_control_or_state(kind: &str) -> bool {
    matches!(
        kind,
        "procedure" | "while" | "try" | "catch" | "expr_cmd" | "global" | "namespace" | "regexp"
    )
}

fn is_static_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn workspace_root_constants(source: &[u8]) -> BTreeMap<String, StaticValue> {
    let text = std::str::from_utf8(source).unwrap_or_default();
    let mut constants = BTreeMap::new();
    if is_workspace_root_assignment(text, "here") {
        constants.insert("here".to_owned(), StaticValue::Scalar("HERE".to_owned()));
    }
    if let Some(version) = configured_iverilog_major_minor() {
        constants.insert(
            "verilog_compiler".to_owned(),
            StaticValue::Scalar("iverilog".to_owned()),
        );
        constants.insert(
            "verilog_compiler_version".to_owned(),
            StaticValue::Scalar(version),
        );
    }
    constants
}

fn is_workspace_root_assignment(command: &str, name: &str) -> bool {
    name == "here"
        && command.contains("set here")
        && command.contains("file join")
        && command.contains("absolute $srcdir")
        && command.contains("$subdir")
}

fn is_workspace_root_filter_assignment(
    command: &str,
    constants: &BTreeMap<String, StaticValue>,
) -> bool {
    matches!(constants.get("here"), Some(StaticValue::Scalar(value)) if value == "HERE")
        && command.contains("s+$here+HERE+g")
}

fn contains_dynamic_syntax(value: &str) -> bool {
    value.contains('$') || value.contains('[') || value.contains(']')
}

fn is_numeric_bug_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok()
}

fn strip_delimiters(value: &str, start: char, end: char) -> &str {
    value
        .strip_prefix(start)
        .and_then(|value| value.strip_suffix(end))
        .unwrap_or(value)
}

fn is_ovl_bootstrap_assignment(command: &str) -> bool {
    command
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .eq(b"sethere[filejoin[absolute$srcdir]$subdir]".iter().copied())
}

fn is_ovl_common_source(command: &str) -> bool {
    command.replace('\\', "/").contains("/../common.tcl")
}

fn is_safe_ovl_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_safe_ovl_library(value: &str) -> bool {
    value.strip_suffix(".vlib").is_some_and(is_safe_ovl_segment)
}

fn source_position(source: &[u8], offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len() + 1, |newline| prefix.len() - newline);
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TclParser;
    use std::path::Path;

    fn lower(source: &str) -> ScriptManifest {
        lower_at("testsuite/sample.exp", source)
    }

    fn lower_at(origin: &str, source: &str) -> ScriptManifest {
        let mut parser = TclParser::new().expect("load Tcl grammar");
        let (tree, _) = parser
            .parse_contract(source.as_bytes(), Path::new(origin))
            .expect("parse Tcl fixture");
        lower_script(origin.to_owned(), source.as_bytes(), &tree)
    }

    fn workspace_source(origin: &str) -> String {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|candidate| candidate.join("testsuite").is_dir())
            .expect("workspace root containing testsuite");
        std::fs::read_to_string(root.join(origin)).expect("read audited upstream script")
    }

    #[test]
    fn lowers_the_hash_pinned_verilog_e_batch_to_finite_simulator_selectors() {
        let source = workspace_source(VERILOG_E_ORIGIN);
        let manifest = lower_at(VERILOG_E_ORIGIN, &source);
        assert!(manifest.unsupported.is_empty());
        let links = manifest
            .workflow_actions
            .iter()
            .filter_map(|action| match action {
                WorkflowAction::LinkVerilog(action) => Some(action),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 6);
        assert_eq!(
            links.iter().map(|link| link.simulator).collect::<Vec<_>>(),
            [
                IcarusSimulatorSelector::Default,
                IcarusSimulatorSelector::BluespecDirInstalledBuilder,
                IcarusSimulatorSelector::PosixEchoProbe,
                IcarusSimulatorSelector::LiteralBogus,
                IcarusSimulatorSelector::BluespecDirBogus,
                IcarusSimulatorSelector::PosixEchoProbe,
            ]
        );
        assert_eq!(links[2].expected_exit, ExpectedExit::Success);
        assert_eq!(links[3].expected_exit, ExpectedExit::Failure);
        assert_eq!(links[4].expected_exit, ExpectedExit::Failure);
        assert_eq!(links[5].options, "-D foo -D bar=128");
        assert!(manifest.workflow_actions.iter().any(|action| matches!(
            action,
            WorkflowAction::RenderGolden(RenderGoldenAction {
                template,
                output,
                macro_value: GoldenMacroValue::BluespecDir,
                ..
            }) if template == "bsc-sim-echo.expected"
                && output == "bsc-sim-echo.expected.post-m4"
        )));

        let changed = lower_at(VERILOG_E_ORIGIN, &(source + "\n"));
        assert!(!changed.unsupported.is_empty());
        assert!(!changed.workflow_actions.iter().any(|action| matches!(
            action,
            WorkflowAction::LinkVerilog(LinkVerilogAction {
                simulator: IcarusSimulatorSelector::BluespecDirInstalledBuilder
                    | IcarusSimulatorSelector::PosixEchoProbe
                    | IcarusSimulatorSelector::LiteralBogus
                    | IcarusSimulatorSelector::BluespecDirBogus,
                ..
            })
        )));
    }

    #[test]
    fn lowers_the_hash_pinned_filter_batch_without_compiler_filter_argv() {
        let source = workspace_source(VERILOG_FILTER_ORIGIN);
        let manifest = lower_at(VERILOG_FILTER_ORIGIN, &source);
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts.len(), 5);
        for contract in &manifest.contracts {
            let Contract::Compile(contract) = contract else {
                panic!("filter episode must compile Verilog")
            };
            assert_eq!(contract.arguments, ["RenameTest.bsv", "", "-keep-fires"]);
            assert!(!contract
                .arguments
                .iter()
                .any(|argument| argument.contains("-verilog-filter")));
        }
        let filters = manifest
            .workflow_actions
            .iter()
            .filter_map(|action| match action {
                WorkflowAction::VerilogFilter(action) => Some(action),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(filters.len(), 5);
        assert_eq!(filters[0].profiles, [VerilogFilterProfile::RenameFire]);
        assert_eq!(
            filters[1].profiles,
            [
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::RenameFire
            ]
        );
        assert_eq!(
            filters[2].profiles,
            [
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::ClockToClock
            ]
        );
        assert_eq!(
            filters[3].profiles,
            [
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::WfToWF
            ]
        );
        assert_eq!(
            filters[4].profiles,
            [
                VerilogFilterProfile::RenameFire,
                VerilogFilterProfile::MissingSed
            ]
        );
        assert_eq!(filters[4].expected_exit, ExpectedExit::Failure);

        let changed = lower_at(VERILOG_FILTER_ORIGIN, &(source + "\n"));
        assert!(!changed.unsupported.is_empty());
        assert!(!changed
            .workflow_actions
            .iter()
            .any(|action| matches!(action, WorkflowAction::VerilogFilter(_))));
    }

    #[test]
    fn lowers_the_hash_pinned_task_transforms_and_rejects_recipe_near_matches() {
        let source = workspace_source(TASKS_ORIGIN);
        let manifest = lower_at(TASKS_ORIGIN, &source);
        let transforms = manifest
            .workflow_actions
            .iter()
            .filter_map(|action| match action {
                WorkflowAction::TextNormalize(action) => Some((
                    action.source.as_str(),
                    action.destination.as_str(),
                    action.transform,
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            transforms,
            [
                (
                    "sysModuleDisplay.v.out",
                    "sysModuleDisplay.sorted.v.out",
                    TextNormalization::SortNumericField1ThenField2,
                ),
                (
                    "sysModuleDisplay.sorted.v.out",
                    "sysModuleDisplay.trimmed.v.out",
                    TextNormalization::VerilogTaskProjection,
                ),
                (
                    "sysModuleDisplay.c.out",
                    "sysModuleDisplay.sorted.c.out",
                    TextNormalization::SortNumericField1ThenField2,
                ),
                (
                    "sysModuleDisplay.sorted.c.out",
                    "sysModuleDisplay.trimmed.c.out",
                    TextNormalization::BluesimTaskProjection,
                ),
            ]
        );

        let changed = source.replacen("-k 1,1n -k 2", "-k 1,1n -k 2,2", 1);
        let rejected = lower_at(TASKS_ORIGIN, &changed);
        assert!(!rejected.unsupported.is_empty());
        assert!(!rejected
            .workflow_actions
            .iter()
            .any(|action| matches!(action, WorkflowAction::TextNormalize(_))));
    }

    #[test]
    fn lowers_only_audited_static_workspace_directories() {
        let manifest = lower_at(
            "testsuite/bsc.driver/imports/imports.exp",
            "mkdir libdir\nset prev_subdir $subdir\nset subdir [file join $subdir libdir]\ncompile_pass Demo.bsv\nerase Demo.bsv\nset subdir $prev_subdir\ncompile_pass Top.bsv\n",
        );
        assert!(manifest.unsupported.is_empty());
        let contracts = manifest
            .contracts
            .iter()
            .filter_map(|contract| match contract {
                Contract::Compile(contract) => Some(contract),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(contracts[0].working_directory.as_deref(), Some("libdir"));
        assert_eq!(contracts[1].working_directory, None);
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [
                WorkflowAction::CreateDirectory(CreateDirectoryAction { path, .. }),
                WorkflowAction::EraseArtifact(EraseArtifactAction { path: erased, .. })
            ] if path == "libdir" && erased == "libdir/Demo.bsv"
        ));

        let rejected = lower(
            "set prev_subdir $subdir\nset subdir [file join $subdir libdir]\ncompile_pass Demo.bsv\nset subdir $prev_subdir\n",
        );
        assert!(!rejected.unsupported.is_empty());
        let Contract::Compile(contract) = &rejected.contracts[0] else {
            panic!("expected compile contract")
        };
        assert_eq!(contract.working_directory, None);
    }

    #[test]
    fn lowers_target_touch_chmod_and_curdir_rendering_to_closed_actions() {
        let depend = lower_at(
            "testsuite/bsc.driver/depend/depend.exp",
            "touch Created.bsv\nchmod {u-r} lib/Created.bo\n",
        );
        assert!(depend.unsupported.is_empty());
        assert!(matches!(
            depend.workflow_actions.as_slice(),
            [
                WorkflowAction::TouchCreateArtifact(TouchCreateArtifactAction {
                    path,
                    delay_milliseconds: 1000,
                    ..
                }),
                WorkflowAction::RemoveUserRead(RemoveUserReadAction { path: unreadable, .. })
            ] if path == "Created.bsv" && unreadable == "lib/Created.bo"
        ));
        assert!(matches!(
            lower("touch Existing.bsv\n").workflow_actions.as_slice(),
            [WorkflowAction::TouchArtifact(TouchArtifactAction { path, .. })]
                if path == "Existing.bsv"
        ));
        assert!(!lower_at(
            "testsuite/bsc.driver/depend/depend.exp",
            "chmod {go-r} Created.bo\n",
        )
        .unsupported
        .is_empty());

        let include = lower_at(
            "testsuite/bsc.preprocessor/include/include.exp",
            "set curdir [file join [absolute $srcdir] $subdir]\nm4_process \"-DCURDIR=$curdir\" IncludeAbsolute.bsv.pre-m4 IncludeAbsolute.bsv\n",
        );
        assert!(include.unsupported.is_empty());
        assert!(matches!(
            include.workflow_actions.as_slice(),
            [WorkflowAction::RenderM4Curdir(RenderM4CurdirAction {
                template,
                output,
                ..
            })] if template == "IncludeAbsolute.bsv.pre-m4" && output == "IncludeAbsolute.bsv"
        ));
        assert!(!lower_at(
            "testsuite/bsc.preprocessor/include/include.exp",
            "m4_process -DOTHER=value arbitrary.m4 arbitrary.out\n",
        )
        .unsupported
        .is_empty());
    }

    #[test]
    fn lowers_closed_bluetcl_helpers_without_open_command_shapes() {
        let standalone = lower("bluetcl_run_compare_pass utils_test.tcl\n");
        assert!(standalone.unsupported.is_empty());
        assert!(matches!(
            standalone.workflow_actions.as_slice(),
            [
                WorkflowAction::BluetclRun(BluetclRunAction {
                    invocation: BluetclInvocation::Script {
                        script,
                        args,
                        syntax: BluetclSyntax::Bsv,
                    },
                    artifact_inputs,
                    expected_exit: ExpectedExit::Success,
                    stdout,
                    ..
                }),
                WorkflowAction::BluetclRun(BluetclRunAction {
                    invocation: BluetclInvocation::Script {
                        syntax: BluetclSyntax::Bh,
                        ..
                    },
                    ..
                })
            ] if script == "utils_test.tcl"
                && args.is_empty()
                && artifact_inputs.is_empty()
                && stdout == "utils_test.tcl.bluetcl-out"
        ));
        assert_eq!(
            standalone
                .comparisons
                .iter()
                .map(|comparison| comparison.helper.as_str())
                .collect::<Vec<_>>(),
            ["compare_bluetcl", "compare_bluetcl"]
        );

        let expanded = lower(
            "proc check_positions { modname } {\n\
                 set cmd \"-exec dump_poss.tcl $modname\"\n\
                 set outfile $modname\n\
                 bluetcl_exec_compare_pass $cmd $outfile\n\
             }\n\
             check_positions sysMethodConds_RegWrite\n",
        );
        assert!(expanded.unsupported.is_empty());
        assert!(matches!(
            expanded.workflow_actions.as_slice(),
            [WorkflowAction::BluetclRun(BluetclRunAction {
                invocation: BluetclInvocation::Exec { script, args },
                artifact_inputs,
                expected_exit: ExpectedExit::Success,
                stdout,
                expansion,
                ..
            })] if script == "dump_poss.tcl"
                && args == &["sysMethodConds_RegWrite"]
                && artifact_inputs == &["sysMethodConds_RegWrite.ba"]
                && stdout == "sysMethodConds_RegWrite.bluetcl-out"
                && !expansion.is_empty()
        ));
    }

    #[test]
    fn lowers_only_the_audited_bsc_compile_bluetcl_filter_and_artifact_shapes() {
        let manifest = lower_at(
            "testsuite/bsc.bluetcl/commands/commands.exp",
            "set sedPosition {-e /position.*%/s/\\[0-9\\]/N/g}\n\
             set sedCRegPosition { -e s/CReg\\[0-9\\]\\+/CRegNNNN/g }\n\
             bsc_compile Test.bsv {-verilog -elab}\n\
             bsc_compile TaggedUnionPoly.bsv\n\
             bluetcl_run_compare_pass bpackage.tcl {} {} {} $sedCRegPosition\n\
             bluetcl_run_compare_pass type.tcl {} {} $sedPosition\n",
        );
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [
                Contract::Compile(CompileContract {
                    helper: first_helper,
                    arguments: first_arguments,
                    ..
                }),
                Contract::Compile(CompileContract {
                    helper: second_helper,
                    arguments: second_arguments,
                    ..
                })
            ] if first_helper == "bsc_compile"
                && first_arguments == &["Test.bsv", "-verilog -elab"]
                && second_helper == "bsc_compile"
                && second_arguments == &["TaggedUnionPoly.bsv"]
        ));
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [
                WorkflowAction::BluetclRun(BluetclRunAction { artifact_inputs: first, .. }),
                WorkflowAction::BluetclRun(BluetclRunAction { artifact_inputs: second, .. }),
                WorkflowAction::BluetclRun(BluetclRunAction { artifact_inputs: third, .. }),
                WorkflowAction::BluetclRun(BluetclRunAction { artifact_inputs: fourth, .. })
            ] if first == &["Test.bo"]
                && second == &["Test.bo"]
                && third == &["pprint.tcl", "Test.bo", "TaggedUnionPoly.bo"]
                && fourth == &["pprint.tcl", "Test.bo", "TaggedUnionPoly.bo"]
        ));
        assert_eq!(
            manifest
                .comparisons
                .iter()
                .map(|comparison| comparison.helper.as_str())
                .collect::<Vec<_>>(),
            [
                "compare_bluetcl_creg_positions",
                "compare_bluetcl_creg_positions",
                "compare_bluetcl_position_digits",
                "compare_bluetcl_position_digits",
            ]
        );
    }

    #[test]
    fn lowers_only_the_exact_instsynth_package_guard() {
        let expected = Guard::Capability {
            capability: Capability::BluetclPackage(BluetclPackage::InstSynth),
        };
        assert_eq!(
            capability_condition("{ [bluetcl_package_available InstSynth] == 1 }"),
            Some(expected.clone())
        );

        let accepted = lower_at(
            "testsuite/bsc.bluetcl/packages/InstSynth/InstSynth.exp",
            "if { [bluetcl_package_available InstSynth] == 1 } {\n    bsc_compile Inst_auto.bsv {-verilog -keep-fires}\n}\n",
        );
        assert!(accepted.unsupported.is_empty());
        assert_eq!(accepted.contracts[0].guard(), &expected);

        for condition in [
            "[bluetcl_package_available InstSynth] == true",
            "[bluetcl_package_available InstSynth] != 0",
            "[bluetcl_package_available $package] == 1",
            "[bluetcl_package_available ExpandPorts] == 1",
            "[bluetcl_package_available Unknown] == 1",
        ] {
            assert_eq!(capability_condition(condition), None, "{condition}");
            let manifest = lower_at(
                "testsuite/bsc.bluetcl/packages/InstSynth/InstSynth.exp",
                &format!("if {{ {condition} }} {{\n    bsc_compile Inst_auto.bsv {{-verilog -keep-fires}}\n}}\n"),
            );
            assert!(!manifest.unsupported.is_empty(), "{condition}");
        }
    }

    #[test]
    fn lowers_only_exact_makedepend_helpers_and_exit_contracts() {
        let accepted = lower_at(
            MAKEDEPEND_ORIGIN,
            "bluetcl_exec_compare_fail {-exec makedepend} usage1\n\
             bluetcl_exec_compare_pass {-exec makedepend -no-show-timestamps Dep1.bsv} test1\n\
             bluetcl_exec_compare_pass {-exec makedepend -no-show-timestamps -o minusO.depend-out Dep1.bsv} minus_o\n\
             bluetcl_compare minusO.depend-out\n",
        );
        assert!(accepted.unsupported.is_empty());
        assert_eq!(accepted.workflow_actions.len(), 3);
        assert_eq!(accepted.comparisons.len(), 4);
        assert!(matches!(
            accepted.workflow_actions.as_slice(),
            [
                WorkflowAction::BluetclRun(BluetclRunAction {
                    invocation: BluetclInvocation::Makedepend {
                        command: BluetclMakedependCommand::Makedepend,
                        args: first_args,
                    },
                    expected_exit: ExpectedExit::Failure,
                    stdout: first_stdout,
                    ..
                }),
                WorkflowAction::BluetclRun(BluetclRunAction {
                    invocation: BluetclInvocation::Makedepend { args: second_args, .. },
                    expected_exit: ExpectedExit::Success,
                    stdout: second_stdout,
                    ..
                }),
                WorkflowAction::BluetclRun(BluetclRunAction {
                    invocation: BluetclInvocation::Makedepend { args: third_args, .. },
                    artifact_outputs,
                    expected_exit: ExpectedExit::Success,
                    stdout: third_stdout,
                    ..
                }),
            ] if first_args.is_empty()
                && first_stdout == "usage1.bluetcl-out"
                && second_args == &["-no-show-timestamps", "Dep1.bsv"]
                && second_stdout == "test1.bluetcl-out"
                && third_args == &["-no-show-timestamps", "-o", "minusO.depend-out", "Dep1.bsv"]
                && artifact_outputs == &["minusO.depend-out"]
                && third_stdout == "minus_o.bluetcl-out"
        ));
        assert_eq!(accepted.comparisons[3].arguments, ["minusO.depend-out"]);

        for source in [
            "bluetcl_exec_compare_fail {-exec makedepend} usage1 extra\n",
            "bluetcl_exec_compare_pass {-exec makedepend} usage1\n",
            "bluetcl_exec_compare_pass {-exec makedepend -no-show-timestamps Dep2.bsv} test1\n",
            "bluetcl_exec_compare_pass {-exec arbitrary Dep1.bsv} test1\n",
            "bluetcl_compare minusO.depend-out extra\n",
        ] {
            let rejected = lower_at(MAKEDEPEND_ORIGIN, source);
            assert!(rejected.workflow_actions.is_empty(), "{source}");
            assert!(rejected.comparisons.is_empty(), "{source}");
            assert!(!rejected.unsupported.is_empty(), "{source}");
        }
    }

    #[test]
    fn expands_only_the_pinned_finite_expand_ports_loop() {
        const SOURCE: &str =
            include_str!("../../../../testsuite/bsc.bluetcl/packages/expandPorts/expandPorts.exp");
        let accepted = lower_at(EXPAND_PORTS_ORIGIN, SOURCE);
        assert!(accepted.unsupported.is_empty());
        assert_eq!(accepted.contracts.len(), 13);
        assert_eq!(accepted.workflow_actions.len(), 13);
        assert_eq!(accepted.comparisons.len(), 26);
        assert!(accepted.workflow_actions.iter().all(|action| matches!(
            action,
            WorkflowAction::BluetclRun(BluetclRunAction {
                invocation: BluetclInvocation::InstalledScript {
                    script: BluetclInstalledScript::ExpandPorts,
                    ..
                },
                guard: Guard::Capability {
                    capability: Capability::BluetclPackage(BluetclPackage::ExpandPorts),
                },
                ..
            })
        )));

        let mutations = [
            SOURCE.replace("Test12.bsv]", "Test12.bsv Test13.bsv]"),
            SOURCE.replace(
                "bsc_compile $bsv {-verilog -elab}",
                "bsc_compile $bsv {-verilog -elab} extra",
            ),
            SOURCE.replace(
                "bluetcl_compare $wrapper $wrapperExp",
                "bluetcl_compare $wrapper $wrapperExp extra",
            ),
            SOURCE.replace("if [file exists $renameFile] {", "if {1} {"),
        ];
        for mutated in mutations {
            let rejected = lower_at(EXPAND_PORTS_ORIGIN, &mutated);
            assert!(!rejected.unsupported.is_empty());
            assert!(!rejected.workflow_actions.iter().any(|action| matches!(
                action,
                WorkflowAction::BluetclRun(BluetclRunAction {
                    invocation: BluetclInvocation::InstalledScript { .. },
                    ..
                })
            )));
        }
    }

    #[test]
    fn rejects_bluetcl_filters_probes_and_nonfixture_exec_targets() {
        for source in [
            "bluetcl_run_compare_pass utils_test.tcl {} {} { -e s/x/y/}\n",
            "bluetcl_package_available InstSynth\n",
            "bluetcl_exec_compare_pass {-exec makedepend -p ../makedepend/:%/Libraries Dep1.bsv} updir\n",
        ] {
            let manifest = lower(source);
            assert!(manifest.workflow_actions.is_empty(), "{source}");
            assert!(!manifest.unsupported.is_empty(), "{source}");
        }
    }

    #[test]
    fn lowers_only_the_audited_binary_ghcrts_scope() {
        let source = r#"
if [info exists env(GHCRTS)] {
  set ghcrts_save $env(GHCRTS)
}
set env(GHCRTS) "-M1.2G"
compile_verilog_pass ManyMeths.bsv
if [info exists ghcrts_save] {
  set env(GHCRTS) $ghcrts_save
} else {
  unset env(GHCRTS)
}
"#;
        let manifest = lower_at("testsuite/bsc.binary/binary.exp", source);
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [Contract::Compile(CompileContract { source, helper, arguments, .. })]
                if source == "ManyMeths.bsv"
                    && helper == "compile_verilog_pass_ghcrts_m1_2g"
                    && arguments == &["ManyMeths.bsv"]
        ));

        let changed = source.replace("-M1.2G", "-M2G");
        let rejected = lower_at("testsuite/bsc.binary/binary.exp", &changed);
        assert!(rejected.contracts.iter().any(|contract| matches!(
            contract,
            Contract::Compile(CompileContract { helper, .. }) if helper == "compile_verilog_pass"
        )));
        assert!(!rejected.unsupported.is_empty());
    }

    #[test]
    fn expands_only_the_audited_paclib_static_case_set() {
        let source = r#"
set packages [list ForFold_1 ForFold_2 fork_join ForLoop IfThenElse Map Map_with_funnel_indexed Reorder SynchPipe WhileFold_1 WhileFold_2 WhileLoop]
foreach pack $packages {
    test_c_veri_bsv_modules_options $pack {} {-aggressive-conditions}
}
"#;
        let manifest = lower_at("testsuite/bsc.lib/PAClib/unit_tests/unit_test.exp", source);
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts.len(), 24);
        for (index, package) in [
            "ForFold_1",
            "ForFold_2",
            "fork_join",
            "ForLoop",
            "IfThenElse",
            "Map",
            "Map_with_funnel_indexed",
            "Reorder",
            "SynchPipe",
            "WhileFold_1",
            "WhileFold_2",
            "WhileLoop",
        ]
        .iter()
        .enumerate()
        {
            let pair = &manifest.contracts[index * 2..index * 2 + 2];
            assert!(pair.iter().all(|contract| matches!(
                contract,
                Contract::Simulation(SimulationContract {
                    source,
                    helper,
                    arguments,
                    generation: GenerationStrategy::Shared,
                    ..
                }) if source == &format!("{package}.bsv")
                    && helper == "test_c_veri_bsv_modules_options"
                    && arguments == &[
                        (*package).to_owned(),
                        String::new(),
                        "-aggressive-conditions".to_owned(),
                    ]
            )));
        }

        for changed in [
            source.replace("WhileLoop", "Other"),
            source.replace("{-aggressive-conditions}", "{-keep-fires}"),
        ] {
            let rejected = lower_at(
                "testsuite/bsc.lib/PAClib/unit_tests/unit_test.exp",
                &changed,
            );
            assert!(rejected.contracts.is_empty());
            assert!(!rejected.unsupported.is_empty());
        }
    }

    #[test]
    fn lowers_only_the_closed_fifo_warning_golden_derivation() {
        let source = include_str!("../../../../testsuite/bsc.lib/fifo/fifo.exp");
        let manifest = lower_at("testsuite/bsc.lib/fifo/fifo.exp", source);

        assert!(
            manifest.unsupported.is_empty(),
            "unexpected lowering result: {manifest:#?}"
        );
        assert!(manifest.contracts.iter().any(|contract| matches!(
            contract,
            Contract::RenderGolden(render)
                if render.template == "sysFIFOErrors.out.expected"
                    && render.output == "sysFIFOErrors.c.out.expected"
                    && render.macro_value == GoldenMacroValue::FifoWarningLocations
        )));

        let wrong_origin = lower_at("testsuite/sample.exp", source);
        assert!(wrong_origin
            .unsupported
            .iter()
            .any(|unsupported| unsupported.command.as_deref() == Some("awk")));
    }

    #[test]
    fn lowers_only_the_fixed_convert_object_make_shape() {
        let manifest = lower("make_pass {-f convert.mk} convert.o\n");

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [WorkflowAction::BuildCObject(CObjectBuildAction {
                source,
                makefile,
                output,
                guard: Guard::Always,
                ..
            })] if source == "convert.c"
                && makefile == "convert.mk"
                && output == "convert.o"
        ));

        for source in [
            "make_pass {-f other.mk} convert.o\n",
            "make_pass {-f convert.mk} other.o\n",
            "make_pass {-f convert.mk} convert.o extra\n",
        ] {
            let unsupported = lower(source);
            assert!(unsupported.workflow_actions.is_empty());
            assert!(unsupported
                .unsupported
                .iter()
                .any(|item| item.command.as_deref() == Some("make_pass")));
        }
    }

    #[test]
    fn lowers_only_the_fixed_upstream_make_test_data_helper() {
        let manifest = lower("make_pass test_data {-f Makefile.data}\n");

        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.make_test_data_actions.len(), 1);
        assert!(matches!(
            manifest.make_test_data_actions.as_slice(),
            [MakeTestDataAction {
                guard: Guard::Always,
                ..
            }]
        ));

        let unsupported = lower("make_pass other {-f Makefile.data}\n");
        assert!(unsupported.make_test_data_actions.is_empty());
        assert!(unsupported
            .unsupported
            .iter()
            .any(|unsupported| unsupported.command.as_deref() == Some("make_pass")));
    }

    #[test]
    fn recognizes_a_closed_bsc_options_append_scope_without_tcl_evaluation() {
        let manifest = lower(
            "set oldenv $::env(BSC_OPTIONS)\nset ::env(BSC_OPTIONS) \"$::env(BSC_OPTIONS) -reset-prefix RESET_P -D BSV_POSITIVE_RESET\"\ncompile_verilog_pass Demo.bsv\nset ::env(BSC_OPTIONS) $oldenv\n",
        );

        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.bsc_options_overlays.len(), 1);
        assert_eq!(
            manifest.bsc_options_overlays[0].append,
            "-reset-prefix RESET_P -D BSV_POSITIVE_RESET"
        );
    }

    #[test]
    fn folds_only_the_pinned_iverilog_profile_conditions() {
        let manifest = lower(
            "set pre12 {}\nset old_bug {}\nif { $verilog_compiler == \"iverilog\" && [regexp {^\\d+\\.\\d+} $verilog_compiler_version majmin] } {\n  if { $majmin < 12 } { set pre12 $verilog_compiler }\n}\nif { $verilog_compiler == \"iverilog\" && [regexp {^\\d+\\.\\d+} $verilog_compiler_version majmin] && $majmin < 10 } {\n  set old_bug $verilog_compiler\n}\ntest_c_veri_bsv Demo \"\" \"\" $pre12\ntest_c_veri_bsv Old \"\" \"\" $old_bug\n",
        );

        assert!(
            manifest.unsupported.is_empty(),
            "unsupported: {:?}",
            manifest.unsupported
        );
        assert!(manifest.contracts.iter().any(|contract| matches!(
            contract,
            Contract::Simulation(contract)
                if contract.source == "Demo.bsv"
                    && contract.arguments.last() == Some(&"iverilog".to_owned())
        )));
        assert!(manifest.contracts.iter().any(|contract| matches!(
            contract,
            Contract::Simulation(contract)
                if contract.source == "Old.bsv"
                    && contract.arguments.last() == Some(&String::new())
        )));
    }

    #[test]
    fn lowers_only_closed_bsc2bsv_invocations_with_an_internal_checks_guard() {
        let manifest = lower("run_bsc2bsv Bug611.bs\n");
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [WorkflowAction::Bsc2Bsv(Bsc2BsvAction {
                source,
                stdout,
                guard: Guard::Capability {
                    capability: Capability::InternalChecks,
                },
                ..
            })] if source == "Bug611.bs" && stdout == "Bug611.bs.bsc2bsv-out"
        ));

        for source in [
            "run_bsc2bsv\n",
            "run_bsc2bsv Demo.bsv\n",
            "run_bsc2bsv One.bs Two.bs\n",
        ] {
            let rejected = lower(source);
            assert!(rejected.workflow_actions.is_empty(), "{source}");
            assert!(rejected
                .unsupported
                .iter()
                .any(|unsupported| unsupported.command.as_deref() == Some("run_bsc2bsv")));
        }
    }

    #[test]
    fn lowers_only_bounded_static_delay_forms() {
        let manifest = lower("after 1500\nexec sleep 2\n");
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [
                WorkflowAction::Delay(DelayAction {
                    milliseconds: 1500,
                    ..
                }),
                WorkflowAction::Delay(DelayAction {
                    milliseconds: 2000,
                    ..
                })
            ]
        ));

        for source in [
            "after 0\n",
            "after 10001\n",
            "after $duration\n",
            "exec sleep 0\n",
            "exec sleep 11\n",
            "exec sleep 2 extra\n",
            "exec echo 2\n",
        ] {
            let rejected = lower(source);
            assert!(rejected.workflow_actions.is_empty(), "{source}");
            assert!(!rejected.unsupported.is_empty(), "{source}");
        }
    }

    #[test]
    fn lowers_only_calls_backed_by_the_closed_parse_pretty_helpers() {
        let helpers = r#"
proc bsc_compile_prettyprint_parse { source { options "" } } {
    set outfile "${source}-pretty-out.bs"
    if [bsc_compile $source "$options -dparsed=$outfile"] then {
        strip_dump_wrapper $outfile
        return [bsc_compile $outfile $options]
    } else {
        return 0
    }
}
proc compile_ppp_pass { source {options ""} } {
    incr_stat "compile_ppp_pass"
    if [bsc_compile_prettyprint_parse $source $options] {
        pass "`$source' compiles, pretty-prints, and compiles again"
    } else {
        fail "`$source' should compile, pretty-print, and compile again"
    }
}
proc compile_ppp_pass_bug { source {bug ""} {options ""}} {
    global target_triplet
    setup_xfail $target_triplet $bug
    compile_ppp_pass $source $options
}
"#;
        let manifest = lower(&format!(
            "{helpers}\ncompile_ppp_pass Demo.bs {{-p +:lib}}\ncompile_ppp_pass_bug Bug.bs github#7\n"
        ));
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [
                WorkflowAction::BscParsePretty(BscParsePrettyAction {
                    source,
                    options,
                    pretty_output,
                    expectation: OperationExpectation::Required,
                    ..
                }),
                WorkflowAction::BscParsePretty(BscParsePrettyAction {
                    source: bug_source,
                    expectation: OperationExpectation::Xfail { reason },
                    ..
                })
            ] if source == "Demo.bs"
                && options == "-p +:lib"
                && pretty_output == "Demo.bs-pretty-out.bs"
                && bug_source == "Bug.bs"
                && reason == "upstream bug github#7"
        ));

        let wrong_extension = lower(&format!("{helpers}\ncompile_ppp_pass Demo.bsv\n"));
        assert!(!wrong_extension
            .workflow_actions
            .iter()
            .any(|action| matches!(action, WorkflowAction::BscParsePretty(_))));
        assert!(!wrong_extension.unsupported.is_empty());

        let redefined = helpers.replace("return 0", "note changed\n        return 0");
        let redefined = lower(&format!("{redefined}\ncompile_ppp_pass Demo.bs\n"));
        assert!(!redefined
            .workflow_actions
            .iter()
            .any(|action| matches!(action, WorkflowAction::BscParsePretty(_))));
        assert!(!redefined.unsupported.is_empty());
    }

    #[test]
    fn lowers_vcdcheck_helpers_with_an_internal_checks_guard() {
        let manifest = lower(
            "vcdcheck_pass dump.vcd {-c \"main.top.signal exists\" -c \"main.top.signal toggles\"}\nvcdcheck_fail bad.vcd {-c \"main.top.missing exists\"}\n",
        );

        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.assertions.len(), 2);
        assert!(manifest.assertions.iter().all(|assertion| matches!(
            assertion.guard,
            Guard::Capability {
                capability: Capability::InternalChecks
            }
        )));
        assert_eq!(manifest.assertions[0].helper, "vcdcheck_pass");
        assert_eq!(manifest.assertions[1].helper, "vcdcheck_fail");
    }

    #[test]
    fn does_not_treat_an_unclosed_bsc_options_assignment_as_an_overlay() {
        let manifest = lower(
            "set oldenv $::env(BSC_OPTIONS)\nset ::env(BSC_OPTIONS) \"$::env(BSC_OPTIONS) -reset-prefix RESET_P\"\ncompile_verilog_pass Demo.bsv\n",
        );

        assert!(manifest.bsc_options_overlays.is_empty());
        assert!(manifest
            .unsupported
            .iter()
            .any(|unsupported| unsupported.command.as_deref() == Some("set")));
    }

    #[test]
    fn lowers_the_closed_ovl_bootstrap_and_helper_shape() {
        let manifest = lower(
            "set here [file join [absolute $srcdir] $subdir]\nsource $here/../common.tcl\ntest_ovl assertAlways1 assert_always.vlib",
        );

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [Contract::Ovl(OvlContract { case_dir, top, library, guard: Guard::Capability { capability: Capability::Verilog }, .. })]
                if case_dir == "assertAlways1"
                    && top == "assertAlways1"
                    && library == "assert_always.vlib"
        ));
    }

    #[test]
    fn rejects_ovl_without_the_closed_bootstrap() {
        let manifest = lower("test_ovl assertAlways1 assert_always.vlib");

        assert!(manifest.contracts.is_empty());
        assert!(manifest
            .unsupported
            .iter()
            .any(|unsupported| { unsupported.command.as_deref() == Some("test_ovl") }));
    }

    #[test]
    fn lowers_verified_ovl_bootstrap_to_a_typed_contract() {
        let manifest = lower_at(
            "testsuite/bsc.interra/OVL/assertAlways1/assertAlways1.exp",
            "set here [file join [absolute $srcdir] $subdir]\nsource $here/../common.tcl\ntest_ovl assertAlways1 assert_always.vlib\n",
        );

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [Contract::Ovl(OvlContract { case_dir, top, library, guard: Guard::Capability { capability: Capability::Verilog }, .. })]
                if case_dir == "assertAlways1" && top == "assertAlways1" && library == "assert_always.vlib"
        ));
    }

    #[test]
    fn rejects_ovl_without_the_verified_bootstrap() {
        let manifest = lower("test_ovl assertAlways1 assert_always.vlib");

        assert!(manifest.contracts.is_empty());
        assert!(manifest
            .unsupported
            .iter()
            .any(|unsupported| { unsupported.command.as_deref() == Some("test_ovl") }));
    }

    #[test]
    fn lowers_verified_ovl_bootstrap_to_a_closed_typed_contract() {
        let manifest = lower(
            "set here [file join [absolute $srcdir] $subdir]\nsource $here/../common.tcl\ntest_ovl assertAlways1 assert_always.vlib",
        );

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [Contract::Ovl(OvlContract { case_dir, top, library, guard: Guard::Capability { capability: Capability::Verilog }, .. })]
                if case_dir == "assertAlways1" && top == "assertAlways1" && library == "assert_always.vlib"
        ));
    }

    #[test]
    fn rejects_ovl_without_the_verified_common_tcl_bootstrap() {
        let manifest = lower("test_ovl assertAlways1 assert_always.vlib");

        assert!(manifest.contracts.is_empty());
        assert!(manifest
            .unsupported
            .iter()
            .any(|unsupported| { unsupported.command.as_deref() == Some("test_ovl") }));
    }

    #[test]
    fn lowers_restricted_m4_golden_rendering() {
        let manifest =
            lower("m4_process \"-DBLUESPECDIR=$bsdir\" flags.expected flags.expected.rendered");

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [Contract::RenderGolden(RenderGoldenContract {
                template,
                output,
                macro_value: GoldenMacroValue::BluespecDir,
                ..
            })] if template == "flags.expected" && output == "flags.expected.rendered"
        ));
    }

    #[test]
    fn lowers_static_basic_options_to_a_typed_contract() {
        let manifest = lower("test_basic_options {-print-flags -sim} flags.out flags.expected");

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.contracts.as_slice(),
            [Contract::BasicOptions(BasicOptionsContract { options, output, expected, .. })]
                if options == "-print-flags -sim"
                    && output == "flags.out"
                    && expected == "flags.expected"
        ));
    }

    #[test]
    fn lowers_static_files_exist_list_to_file_assertions() {
        let manifest = lower("files_exist { work/one.log {work/two log} }");

        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.assertions.len(), 2);
        assert!(manifest
            .assertions
            .iter()
            .all(|assertion| assertion.helper == "files_exist"));
        assert_eq!(
            manifest
                .assertions
                .iter()
                .map(|assertion| assertion.arguments.as_slice())
                .collect::<Vec<_>>(),
            vec![&["work/one.log"][..], &["work/two log"][..]]
        );
    }

    #[test]
    fn folds_static_boolean_if_branches() {
        let manifest = lower(
            r#"
if {0} {
    compile_fail Dead.bsv
}
if {1} {
    compile_pass Live.bsv
} else {
    compile_fail DeadElse.bsv
}
if {false} {
    compile_fail AlsoDead.bsv
} else {
    compile_pass Fallback.bsv
}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts.len(), 2);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Live.bsv"
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Compile(contract) if contract.source == "Fallback.bsv"
        ));
    }

    #[test]
    fn lowers_compile_simulation_and_capability_contracts() {
        let manifest = lower(
            r#"
set source Demo.bsv
compile_pass $source
if {$vtest == 1} {
    test_veri_only_bsv VerilogOnly expected
}
test_c_veri_bsv Both
test_c_veri_worker Worker mkWorker {Helper} bsv 1 1 worker.expected {} {}
test_c_veri_worker BluesimOnly mkBluesimOnly {} bs -2 0 bluesim.expected {} {} 0 0
test_c_veri_worker IcarusOnly mkIcarusOnly {} bsv 0 3 icarus.expected {} {}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts.len(), 8);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if contract.source == "Demo.bsv"
                    && contract.arguments == ["Demo.bsv"]
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Simulation(contract)
                if contract.backend == SimulationBackend::Icarus
                    && matches!(
                        contract.guard,
                        Guard::Capability {
                            capability: Capability::Verilog,
                        }
                    )
        ));
        assert!(matches!(
            (&manifest.contracts[4], &manifest.contracts[5]),
            (Contract::Simulation(bluesim), Contract::Simulation(icarus))
                if bluesim.helper == "test_c_veri_worker"
                    && bluesim.source == "Worker.bsv"
                    && bluesim.backend == SimulationBackend::Bluesim
                    && bluesim.generation == GenerationStrategy::Shared
                    && icarus.source == "Worker.bsv"
                    && icarus.backend == SimulationBackend::Icarus
                    && icarus.generation == GenerationStrategy::Shared
        ));
        assert!(matches!(
            &manifest.contracts[6],
            Contract::Simulation(contract)
                if contract.source == "BluesimOnly.bs"
                    && contract.backend == SimulationBackend::Bluesim
                    && contract.generation == GenerationStrategy::Bluesim
        ));
        assert!(matches!(
            &manifest.contracts[7],
            Contract::Simulation(contract)
                if contract.source == "IcarusOnly.bsv"
                    && contract.backend == SimulationBackend::Icarus
                    && contract.generation == GenerationStrategy::Icarus
        ));
    }

    #[test]
    fn lowers_classic_single_backend_simulation_helpers() {
        let manifest = lower(
            r#"
test_c_only ClassicC custom.expected
 test_veri_only ClassicV {} 138
test_c_only_bs_modules_options ClassicModules {Helper.ba} {-keep-fires}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts.len(), 3);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Simulation(contract)
                if contract.source == "ClassicC.bs"
                    && contract.helper == "test_c_only"
                    && contract.backend == SimulationBackend::Bluesim
                    && contract.generation == GenerationStrategy::Bluesim
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Simulation(contract)
                if contract.source == "ClassicV.bs"
                    && contract.helper == "test_veri_only"
                    && contract.backend == SimulationBackend::Icarus
                    && contract.generation == GenerationStrategy::Icarus
        ));
        assert!(matches!(
            &manifest.contracts[2],
            Contract::Simulation(contract)
                if contract.source == "ClassicModules.bs"
                    && contract.helper == "test_c_only_bs_modules_options"
                    && contract.backend == SimulationBackend::Bluesim
        ));
    }

    #[test]
    fn preserves_all_static_compile_helper_arguments() {
        let manifest = lower(
            r#"
compile_pass Demo.bsv {-p +:lib} 1
compile_verilog_fail_error Bad.bsv G0001 2 mkBad {-aggressive-conditions}
compile_pass_bug Frontend.bsv B123 {-show-range-conflict} 1
compile_verilog_schedule_fail_bug Verilog.bsv mkVerilog B456 {-keep-fires}
compile_pass_bug_error Error.bsv P0017 B789 2 {-continue-after-errors}
compile_verilog_pass_no_warning_bug Warning.bsv G0010 B101 1 mkWarning {-keep-fires}
compile_verilog_fail_no_internal_error Internal.bsv
compile_backend_pass Backend.bsv {-show-range-conflict} 1
compile_fail_error_bug FailError.bsv P0017 B202 3 {-continue-after-errors}
compile_verilog_fail_error_bug VerilogError.bsv G0028 B303 4 mkVerilogError {-keep-fires}
compile_object_pass_bug Object.bsv mkObject B404 {-p +:lib}
compile_object_pass_warning ObjectWarning.bsv G0023 2 mkObjectWarning {-keep-fires}
compile_fail_error_warnings ErrorWarnings.bsv T0066 1 {{P0102 2} P0103} {-continue-after-errors}
bsc_compile_verilog Worker.bsv mkWorker {-dATSexpand=%m.atsexpand -KILLATSexpand}
"#,
        );
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if contract.arguments == ["Demo.bsv", "-p +:lib", "1"]
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Compile(contract)
                if contract.arguments
                    == ["Bad.bsv", "G0001", "2", "mkBad", "-aggressive-conditions"]
        ));
        assert!(matches!(
            &manifest.contracts[2],
            Contract::Compile(contract)
                if contract.helper == "compile_pass_bug"
                    && contract.arguments
                        == ["Frontend.bsv", "B123", "-show-range-conflict", "1"]
        ));
        assert!(matches!(
            &manifest.contracts[3],
            Contract::Compile(contract)
                if contract.helper == "compile_verilog_schedule_fail_bug"
                    && contract.arguments
                        == ["Verilog.bsv", "mkVerilog", "B456", "-keep-fires"]
        ));
        assert!(matches!(
            &manifest.contracts[4],
            Contract::Compile(contract)
                if contract.helper == "compile_pass_bug_error"
                    && contract.arguments
                        == ["Error.bsv", "P0017", "B789", "2", "-continue-after-errors"]
        ));
        assert!(matches!(
            &manifest.contracts[5],
            Contract::Compile(contract)
                if contract.helper == "compile_verilog_pass_no_warning_bug"
                    && contract.arguments
                        == ["Warning.bsv", "G0010", "B101", "1", "mkWarning", "-keep-fires"]
        ));
        assert!(matches!(
            &manifest.contracts[6],
            Contract::Compile(contract)
                if contract.helper == "compile_verilog_fail_no_internal_error"
                    && contract.arguments == ["Internal.bsv"]
        ));
        assert!(matches!(
            &manifest.contracts[7],
            Contract::Compile(contract)
                if contract.helper == "compile_backend_pass"
                    && contract.arguments == ["Backend.bsv", "-show-range-conflict", "1"]
        ));
        assert!(matches!(
            &manifest.contracts[8],
            Contract::Compile(contract)
                if contract.helper == "compile_fail_error_bug"
                    && contract.arguments
                        == ["FailError.bsv", "P0017", "B202", "3", "-continue-after-errors"]
        ));
        assert!(matches!(
            &manifest.contracts[9],
            Contract::Compile(contract)
                if contract.helper == "compile_verilog_fail_error_bug"
                    && contract.arguments
                        == ["VerilogError.bsv", "G0028", "B303", "4", "mkVerilogError", "-keep-fires"]
        ));
        assert!(matches!(
            &manifest.contracts[10],
            Contract::Compile(contract)
                if contract.helper == "compile_object_pass_bug"
                    && contract.arguments == ["Object.bsv", "mkObject", "B404", "-p +:lib"]
        ));
        assert!(matches!(
            &manifest.contracts[11],
            Contract::Compile(contract)
                if contract.helper == "compile_object_pass_warning"
                    && contract.arguments
                        == ["ObjectWarning.bsv", "G0023", "2", "mkObjectWarning", "-keep-fires"]
        ));
        assert!(matches!(
            &manifest.contracts[12],
            Contract::Compile(contract)
                if contract.helper == "compile_fail_error_warnings"
                    && contract.arguments
                        == [
                            "ErrorWarnings.bsv",
                            "T0066",
                            "1",
                            "{P0102 2} P0103",
                            "-continue-after-errors"
                        ]
        ));
        assert!(matches!(
            &manifest.contracts[13],
            Contract::Compile(contract)
                if contract.helper == "bsc_compile_verilog"
                    && contract.arguments
                        == ["Worker.bsv", "mkWorker", "-dATSexpand=%m.atsexpand -KILLATSexpand"]
        ));
    }

    #[test]
    fn rejects_dynamic_compile_bug_annotations() {
        let manifest = lower(
            r#"
set bug [current_bug]
compile_pass_bug Demo.bsv $bug
compile_verilog_pass_warning_bug Warning.bsv G0010 $bug
compile_fail_error_bug Error.bsv P0017 $bug
compile_verilog_fail_error_bug VerilogError.bsv G0028 $bug
compile_object_pass_bug Object.bsv mkObject $bug
"#,
        );
        assert!(manifest.contracts.is_empty());
        assert_eq!(manifest.unsupported.len(), 6);
        assert_eq!(
            manifest.unsupported[0].reason,
            UnsupportedReason::DynamicAssignment
        );
        assert_eq!(
            manifest.unsupported[1].reason,
            UnsupportedReason::DynamicArguments
        );
        assert_eq!(
            manifest.unsupported[1].command.as_deref(),
            Some("compile_pass_bug")
        );
        assert_eq!(
            manifest.unsupported[2].reason,
            UnsupportedReason::DynamicArguments
        );
        assert_eq!(
            manifest.unsupported[2].command.as_deref(),
            Some("compile_verilog_pass_warning_bug")
        );
        assert_eq!(
            manifest.unsupported[3].reason,
            UnsupportedReason::DynamicArguments
        );
        assert_eq!(
            manifest.unsupported[3].command.as_deref(),
            Some("compile_fail_error_bug")
        );
        assert_eq!(
            manifest.unsupported[4].reason,
            UnsupportedReason::DynamicArguments
        );
        assert_eq!(
            manifest.unsupported[4].command.as_deref(),
            Some("compile_verilog_fail_error_bug")
        );
        assert_eq!(
            manifest.unsupported[5].reason,
            UnsupportedReason::DynamicArguments
        );
        assert_eq!(
            manifest.unsupported[5].command.as_deref(),
            Some("compile_object_pass_bug")
        );
    }

    #[test]
    fn lowers_static_bug_assertions_without_executing_tcl() {
        let manifest = lower(
            r#"
find_n_strings_bug Demo.out warning 2 123
find_regexp_bug Demo.out {warning.*detail} 456
find_regexp_fail_bug Demo.out {Internal.*Error}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.assertions.len(), 3);
        assert!(matches!(
            &manifest.assertions[0],
            AssertionContract { helper, arguments, .. }
                if helper == "find_n_strings_bug"
                    && arguments.as_slice() == ["Demo.out", "warning", "2", "123"]
        ));
        assert!(matches!(
            &manifest.assertions[1],
            AssertionContract { helper, arguments, .. }
                if helper == "find_regexp_bug"
                    && arguments.as_slice() == ["Demo.out", "warning.*detail", "456"]
        ));
        assert!(matches!(
            &manifest.assertions[2],
            AssertionContract { helper, arguments, .. }
                if helper == "find_regexp_fail_bug"
                    && arguments.as_slice() == ["Demo.out", "Internal.*Error"]
        ));
    }

    #[test]
    fn lowers_the_closed_workspace_root_filter_profile() {
        let manifest = lower(
            "set here [file join [absolute $srcdir] $subdir]\nset bre_options \"s+$here+HERE+g\"\ncompile_pass Demo.bsv {-dvpp=Demo.out}\ncompare_file_filtered Demo.out {} $bre_options\n",
        );

        assert!(
            manifest.unsupported.is_empty(),
            "{:?}",
            manifest.unsupported
        );
        assert!(matches!(
            manifest.comparisons.as_slice(),
            [ComparisonContract { helper, arguments, .. }]
                if helper == "compare_file_filtered"
                    && arguments.as_slice() == ["Demo.out", "", "s+HERE+HERE+g"]
        ));
    }

    #[test]
    fn canonicalizes_only_the_numeric_two_argument_compare_file_bug_overload() {
        let manifest = lower(
            "compare_file_bug Implicit.out 770\ncompare_file_bug Expected.out Expected.golden\ncompare_file_bug Explicit.out Explicit.golden 771\ncompare_file_bug StringExpected.out Rob_Brown\n",
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.comparisons.len(), 4);
        assert_eq!(
            manifest.comparisons[0].arguments,
            ["Implicit.out", "", "770"]
        );
        assert_eq!(
            manifest.comparisons[1].arguments,
            ["Expected.out", "Expected.golden"]
        );
        assert_eq!(
            manifest.comparisons[2].arguments,
            ["Explicit.out", "Explicit.golden", "771"]
        );
        assert_eq!(
            manifest.comparisons[3].arguments,
            ["StringExpected.out", "Rob_Brown"]
        );
    }

    #[test]
    fn lowers_static_golden_bug_comparisons() {
        let manifest = lower(
            r#"
compare_file_bug Demo.out Demo.expected 123
compare_file_bug Default.out {} 456
compare_file_filter_prelude Prelude.out Prelude.expected
compare_file_list Any.out {Any.out.0.expected Any.out.1.expected}
compare_file_filtered Filtered.out {} {/Bluespec\ Compiler.*/d}
compare_verilog_bug Demo.v 123 Demo.expected
compare_verilog_bug Default.v {}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.comparisons.len(), 7);
        assert!(matches!(
            &manifest.comparisons[0],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_file_bug"
                    && arguments.as_slice() == ["Demo.out", "Demo.expected", "123"]
        ));
        assert!(matches!(
            &manifest.comparisons[1],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_file_bug"
                    && arguments.as_slice() == ["Default.out", "", "456"]
        ));
        assert!(matches!(
            &manifest.comparisons[2],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_file_filter_prelude"
                    && arguments.as_slice() == ["Prelude.out", "Prelude.expected"]
        ));
        assert!(matches!(
            &manifest.comparisons[3],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_file_list"
                    && arguments.as_slice() == ["Any.out", "Any.out.0.expected Any.out.1.expected"]
        ));
        assert!(matches!(
            &manifest.comparisons[4],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_file_filtered"
                    && arguments.as_slice() == ["Filtered.out", "", "/Bluespec\\ Compiler.*/d"]
        ));
        assert!(matches!(
            &manifest.comparisons[5],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_verilog_bug"
                    && arguments.as_slice() == ["Demo.v", "123", "Demo.expected"]
        ));
        assert!(matches!(
            &manifest.comparisons[6],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_verilog_bug"
                    && arguments.as_slice() == ["Default.v", ""]
        ));
    }

    #[test]
    fn treats_static_global_declarations_as_procedure_scope_metadata() {
        let manifest = lower(
            r#"
proc compile_for_verilog {source} {
    global vtest srcdir
    if {$vtest == 1} {
        compile_verilog_pass $source
    }
}
compile_for_verilog Demo.bsv
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts.len(), 1);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if contract.source == "Demo.bsv"
                    && matches!(
                        contract.guard,
                        Guard::Capability {
                            capability: Capability::Verilog,
                        }
                    )
        ));
    }

    #[test]
    fn records_dynamic_control_flow_instead_of_executing_it() {
        let manifest = lower("foreach item $items { compile_pass $item }\n");
        assert_eq!(manifest.contracts.len(), 0);
        assert_eq!(manifest.unsupported.len(), 1);
        assert_eq!(
            manifest.unsupported[0].reason,
            UnsupportedReason::UnsupportedControlFlow
        );
    }

    #[test]
    fn maps_the_audited_elab_progress_sed_to_a_normalized_comparison() {
        let manifest = lower(
            r#"
proc test_elab { fname { modname "" } { options "" } } {
    compile_verilog_pass $fname $modname "-show-elab-progress $options"
    set outfile [make_bsc_vcomp_output_name $fname]
    sed $outfile $outfile.filtered {} {s/\\\[.*\\\]/\\\[TIME\\\]/}
    compare_file $outfile.filtered $outfile.expected
}
copy Test1.bsv Test1-hide.bsv
test_elab Test1-hide.bsv sysTest1
erase Test1-hide.bsv
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Test1-hide.bsv"
        ));
        assert!(matches!(
            &manifest.comparisons[0],
            ComparisonContract { helper, arguments, .. }
                if helper == "compare_file_filtered_times"
                    && arguments == &[
                        "Test1-hide.bsv.bsc-vcomp-out".to_owned(),
                        "Test1-hide.bsv.bsc-vcomp-out.expected".to_owned(),
                    ]
        ));

        let rejected = lower(
            "sed Demo.out Demo.filtered {} {s/x/y/g}\ncompare_file Demo.filtered Demo.expected\n",
        );
        assert!(!rejected.unsupported.is_empty());
        assert_eq!(rejected.comparisons[0].helper, "compare_file");
    }

    #[test]
    fn drops_the_statically_true_positive_reset_probe_from_guards() {
        let manifest = lower(
            "if { $vtest == 1 && [isPositiveReset] == 0 } {\ncompile_verilog_pass Demo.bsv\n}\n",
        );
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if contract.guard
                    == Guard::Capability {
                        capability: Capability::Verilog,
                    }
        ));

        let standalone = lower("if { [isPositiveReset] == 0 } {\ncompile_pass Demo.bsv\n}\n");
        assert!(matches!(
            &standalone.contracts[0],
            Contract::Compile(contract) if contract.guard == Guard::Always
        ));

        let inverted = lower(
            "if { $vtest == 1 && [isPositiveReset] == 1 } {\ncompile_verilog_pass Demo.bsv\n}\n",
        );
        assert!(matches!(
            &inverted.contracts[0],
            Contract::Compile(contract)
                if matches!(contract.guard, Guard::UnsupportedExpression { .. })
        ));
    }

    #[test]
    fn normalizes_backslash_escaped_command_names() {
        let manifest = lower("compile_pass Demo.bsv\nfind\\_regexp Demo.bsv {Internal.*Error}\n");
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Demo.bsv"
        ));
        assert!(matches!(
            &manifest.assertions[0],
            AssertionContract { helper, arguments, .. }
                if helper == "find_regexp" && arguments == &["Demo.bsv", "Internal.*Error"]
        ));
    }

    #[test]
    fn expands_procedure_calls_with_static_default_arguments() {
        let manifest = lower(
            r#"
proc check_lex_pos { srcname linepos colpos {errtag T0020} {count 1}} {
    set str "Error: \"$srcname\", line $linepos, column $colpos: ($errtag)"
    find_n_strings [make_bsc_output_name $srcname] $str $count
}
check_lex_pos LexPos_NumLit.bs 4 12
check_lex_pos LexPos_Task.bs 7 14
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(manifest.contracts.as_slice(), []));
        assert!(manifest
            .assertions
            .iter()
            .any(|assertion| assertion.helper == "find_n_strings"
                && assertion.arguments
                    == [
                        "LexPos_NumLit.bs.bsc-out".to_owned(),
                        "Error: \"LexPos_NumLit.bs\", line 4, column 12: (T0020)".to_owned(),
                        "1".to_owned(),
                    ]));
        assert!(manifest
            .assertions
            .iter()
            .any(|assertion| assertion.helper == "find_n_strings"
                && assertion.arguments[0] == "LexPos_Task.bs.bsc-out"));

        let too_few = lower(
            r#"
proc check { srcname {errtag T0020} } {
    find_n_strings [make_bsc_output_name $srcname] $errtag 1
}
check
"#,
        );
        assert!(!too_few.unsupported.is_empty());
    }

    #[test]
    fn folds_static_string_compare_empty_conditions_in_procedures() {
        let manifest = lower(
            r#"
proc compare_default {actual {expected ""}} {
    if {[string compare $expected ""] == 0} {
        set expected "$actual.expected"
    }
    compare_file $actual $expected
}
compare_default Default.out
compare_default Explicit.out Explicit.expected
"#,
        );

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.comparisons.as_slice(),
            [default, explicit]
                if default.arguments == ["Default.out", "Default.out.expected"]
                    && explicit.arguments == ["Explicit.out", "Explicit.expected"]
        ));
    }

    #[test]
    fn expands_non_recursive_static_procedure_calls() {
        let manifest = lower(
            r#"
proc compile_one {name} {
    compile_pass "${name}.bsv"
}
compile_one First
compile_one Second
"#,
        );
        assert_eq!(manifest.contracts.len(), 2);
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if contract.source == "First.bsv" && contract.expansion.len() == 1
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Compile(contract)
                if contract.source == "Second.bsv" && contract.expansion.len() == 1
        ));
    }

    #[test]
    fn concatenates_adjacent_unquoted_word_fragments() {
        let manifest = lower(
            r#"
proc compile_one {name} {
    compile_verilog_pass ${name}.bs
    compare_verilog mk${name}Reg.v
}
compile_one Orig
"#,
        );
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Orig.bs"
        ));
        assert_eq!(manifest.comparisons[0].arguments, ["mkOrigReg.v"]);
    }

    #[test]
    fn lowers_link_objects_fail_error_with_its_diagnostic_contract() {
        let manifest = lower(
            "compile_object_pass Design.bsv mkDesign\nlink_objects_fail_error mkDesign mkDesign G0099 2 -remove-dollar\n",
        );
        assert!(matches!(
            &manifest.bluesim_workflows[0].link,
            action
                if action.expected_exit == ExpectedExit::Failure
                    && action.options == "-remove-dollar"
                    && action.error_diagnostic
                        == Some(LinkErrorDiagnostic {
                            code: "G0099".to_owned(),
                            count: "2".to_owned(),
                        })
        ));
        assert!(manifest.unsupported.is_empty());
    }

    #[test]
    fn lowers_link_objects_pass_bug_with_its_optional_xfail_annotation() {
        let manifest = lower(
            "compile_object_pass Design.bsv mkDesign\nlink_objects_pass_bug mkDesign mkDesign B1731 -keep-fires\n",
        );
        assert!(manifest.contracts.is_empty());
        assert!(manifest.workflow_actions.is_empty());
        assert!(matches!(
            &manifest.bluesim_workflows[0].link,
            action
                if action.expected_exit == ExpectedExit::Success
                    && action.options == "-keep-fires"
                    && action.expectation
                        == OperationExpectation::Xfail {
                            reason: "upstream bug B1731".to_owned()
                        }
        ));

        let manifest = lower("link_objects_pass_bug mkDesign mkDesign\n");
        assert!(matches!(
            &manifest.workflow_actions[0],
            WorkflowAction::LinkObjects(action)
                if action.expectation
                    == OperationExpectation::Xfail {
                        reason: "upstream unannotated known failure".to_owned()
                    }
        ));

        let manifest = lower("link_objects_pass_bug mkDesign mkDesign \"\"\n");
        assert!(matches!(
            &manifest.workflow_actions[0],
            WorkflowAction::LinkObjects(action)
                if action.expectation == OperationExpectation::Required
        ));
    }

    #[test]
    fn lowers_link_objects_fail_with_an_expected_failure() {
        let manifest = lower("compile_object_pass Design.bsv mkDesign\nlink_objects_fail mkDesign mkDesign -remove-dollar\n");
        assert!(manifest.contracts.is_empty());
        assert!(manifest.workflow_actions.is_empty());
        assert!(matches!(
            &manifest.bluesim_workflows[0].link,
            action
                if action.objects == "mkDesign"
                    && action.top == "mkDesign"
                    && action.options == "-remove-dollar"
                    && action.expected_exit == ExpectedExit::Failure
        ));
        assert!(manifest.unsupported.is_empty());
    }

    #[test]
    fn lowers_static_bluesim_workflow_actions_without_executing_tcl() {
        let manifest = lower(
            r#"
if {$ctest == 1} {
    compile_object_pass Design.bsv mkDesign {-keep-fires}
    link_objects_pass {mkDesign helper.c} mkDesign {-v}
    sim_output mkDesign {-c {sim run; puts done}}
    copy mkDesign.out saved.out
    move dump.vcd saved.vcd
}
"#,
        );
        assert_eq!(manifest.bluesim_workflows.len(), 1);
        assert_eq!(manifest.workflow_actions.len(), 1);
        assert!(manifest.unsupported.is_empty());
        let workflow = &manifest.bluesim_workflows[0];
        assert_eq!(workflow.top, "mkDesign");
        assert!(matches!(
            &workflow.generations[0],
            action if action.source == "Design.bsv"
                && action.module.as_deref() == Some("mkDesign")
                && action.options == "-keep-fires"
                && matches!(
                    action.guard,
                    Guard::Capability {
                        capability: Capability::Bluesim,
                    }
                )
        ));
        assert!(
            workflow.link.objects == "mkDesign helper.c"
                && workflow.link.top == "mkDesign"
                && workflow.link.options == "-v"
        );
        assert_eq!(workflow.runs.len(), 1);
        assert!(
            workflow.runs[0].action.executable == "mkDesign"
                && workflow.runs[0].action.options == "-c {sim run; puts done}"
                && workflow.runs[0].action.stdout == "mkDesign.out"
        );
        assert!(matches!(
            &workflow.runs[0].transfers[0],
            action if action.operation == ArtifactTransferOperation::Copy
                && action.source == "mkDesign.out"
                && action.destination == "saved.out"
        ));
        assert!(matches!(
            &manifest.workflow_actions[0],
            WorkflowAction::TransferArtifact(action)
                if action.operation == ArtifactTransferOperation::Move
                    && action.source == "dump.vcd"
                    && action.destination == "saved.vcd"
        ));
    }

    #[test]
    fn lowers_static_verilog_workflow_actions_without_executing_tcl() {
        let manifest = lower(
            r#"
compile_verilog_pass Demo.bsv mkDemo
link_verilog_pass {} mkDemo {-L lib}
link_verilog_no_main_pass {Tb.v mkDemo.v} Tb {-ignored}
sim_verilog mkDemo {+arg}
sim_verilog_vcd mkDemo
sim_verilog_status mkDemo {3} {+status}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.workflow_actions.len(), 5);
        assert!(matches!(
            &manifest.workflow_actions[0],
            WorkflowAction::LinkVerilog(action)
                if action.objects.is_empty()
                    && action.top == "mkDemo"
                    && action.options == "-L lib"
        ));
        assert!(matches!(
            &manifest.workflow_actions[1],
            WorkflowAction::LinkVerilog(action)
                if action.objects == "Tb.v mkDemo.v"
                    && action.top == "Tb"
                    && action.options == "-ignored"
                    && action.no_main
        ));
        assert!(matches!(
            &manifest.workflow_actions[2],
            WorkflowAction::RunVerilog(action)
                if action.executable == "mkDemo"
                    && action.options == "+arg"
                    && action.stdout == "mkDemo.out"
                    && action.expected_exits.is_empty()
                    && !action.vcd
        ));
        assert!(matches!(
            &manifest.workflow_actions[3],
            WorkflowAction::RunVerilog(action)
                if action.executable == "mkDemo"
                    && action.options.is_empty()
                    && action.expected_exits.is_empty()
                    && action.vcd
        ));
        assert!(matches!(
            &manifest.workflow_actions[4],
            WorkflowAction::RunVerilog(action)
                if action.executable == "mkDemo"
                    && action.options == "+status"
                    && action.expected_exits == [3]
                    && !action.vcd
        ));
        assert!(manifest.workflow_actions.windows(2).all(|actions| {
            actions[0].helper_name() != actions[1].helper_name()
                || actions[0].guard() == actions[1].guard()
        }));
    }

    #[test]
    fn lowers_only_exact_static_verilog_link_failure_forms() {
        let manifest = lower(
            "link_verilog_fail sysOne.v sysOne\nlink_verilog_fail {sysTwo.v helper.v} sysTwo {-L lib}\nlink_verilog_fail sysBad.v sysBad {-vsim /tmp/tool}\n",
        );
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [WorkflowAction::LinkVerilog(first), WorkflowAction::LinkVerilog(second)]
                if first.objects == "sysOne.v"
                    && first.top == "sysOne"
                    && first.options.is_empty()
                    && first.expected_exit == ExpectedExit::Failure
                    && first.simulator == IcarusSimulatorSelector::Default
                    && second.objects == "sysTwo.v helper.v"
                    && second.top == "sysTwo"
                    && second.options == "-L lib"
                    && second.expected_exit == ExpectedExit::Failure
                    && second.simulator == IcarusSimulatorSelector::Default
        ));
        assert!(!manifest.unsupported.is_empty());
    }

    #[test]
    fn lowers_verilog_link_known_failures_with_and_without_bug_annotations() {
        let manifest = lower(
            "link_verilog_pass_bug sysOne.v sysOne\nlink_verilog_pass_bug sysTwo.v sysTwo 123 {-L lib}\n",
        );

        assert!(manifest.unsupported.is_empty());
        assert_eq!(
            manifest.workflow_actions[0].helper_name(),
            "link_verilog_pass_bug"
        );
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [WorkflowAction::LinkVerilog(unannotated), WorkflowAction::LinkVerilog(annotated)]
                if unannotated.options.is_empty()
                    && matches!(
                        &unannotated.expectation,
                        OperationExpectation::Xfail { reason }
                            if reason == "upstream unannotated known failure"
                    )
                    && annotated.options == "-L lib"
                    && matches!(
                        &annotated.expectation,
                        OperationExpectation::Xfail { reason } if reason == "upstream bug 123"
                    )
        ));
    }

    #[test]
    fn rejects_malformed_and_dynamic_verilog_workflow_arguments() {
        let manifest = lower(
            r#"
sim_verilog_status mkDemo {0 3}
sim_verilog_status mkBad {not-a-status}
set dynamic [current_options]
link_verilog_pass $dynamic mkDemo
sim_verilog mkDemo $dynamic
"#,
        );
        assert!(matches!(
            &manifest.workflow_actions[0],
            WorkflowAction::RunVerilog(action) if action.expected_exits == [0, 3]
        ));
        assert_eq!(manifest.unsupported.len(), 4);
        assert!(manifest
            .unsupported
            .iter()
            .any(|unsupported| { unsupported.command.as_deref() == Some("sim_verilog_status") }));
        assert!(manifest.unsupported.iter().any(|unsupported| {
            unsupported.command.as_deref() == Some("link_verilog_pass")
                && unsupported.reason == UnsupportedReason::DynamicArguments
        }));
        assert!(manifest.unsupported.iter().any(|unsupported| {
            unsupported.command.as_deref() == Some("sim_verilog")
                && unsupported.reason == UnsupportedReason::DynamicArguments
        }));
    }

    #[test]
    fn lowers_static_touch_as_an_ordered_workspace_action() {
        let manifest = lower("touch Source.bsv\ncompile_pass Source.bsv\n");

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [WorkflowAction::TouchArtifact(action)] if action.path == "Source.bsv"
        ));
    }

    #[test]
    fn lowers_static_nukedir_as_a_guarded_directory_action() {
        let manifest = lower(
            "if {$ctest != 0} {\n  nukedir bd\n  mkdir bd\n  compile_object_pass Top.bsv\n}\n",
        );

        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            manifest.workflow_actions.as_slice(),
            [
                WorkflowAction::EnsureDirectoryAbsent(remove),
                WorkflowAction::CreateDirectory(create),
                WorkflowAction::CompileObject(_),
            ] if remove.path == "bd"
                && create.path == "bd"
                && remove.guard == Guard::Capability { capability: Capability::Bluesim }
                && create.guard == remove.guard
                && remove.span.start_line == 2
        ));
    }

    #[test]
    fn preserves_static_simulation_arguments_and_directory_actions() {
        let manifest = lower(
            r#"
set simdir {dir:with,many;spec#ial=char%acters}
mkdir $simdir
test_c_only_bsv_multi_options TbGCD mkTbGCD {} \
    "-simdir \"$simdir\"" {} {} \
    "-simdir \"$simdir\" -v -parallel-sim-link 2"
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            &manifest.workflow_actions[0],
            WorkflowAction::CreateDirectory(action)
                if action.path == "dir:with,many;spec#ial=char%acters"
        ));
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Simulation(contract)
                if contract.arguments == [
                    "TbGCD",
                    "mkTbGCD",
                    "",
                    "-simdir \"dir:with,many;spec#ial=char%acters\"",
                    "",
                    "",
                    "-simdir \"dir:with,many;spec#ial=char%acters\" -v -parallel-sim-link 2",
                ]
        ));
    }

    #[test]
    fn evaluates_allowlisted_output_name_helpers() {
        let manifest = lower(
            "compare_file [make_bsc_output_name Failed.bsv]\n\
             compare_file [make_bsc_ccomp_output_name Failed.bsv]\n\
             compare_file [make_bsc_vcomp_output_name Failed.bsv]\n\
             compare_file_filter_ids [make_bsc_sched_output_name AVArgUse_C.bsv]\n",
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.comparisons.len(), 4);
        assert_eq!(manifest.comparisons[0].arguments[0], "Failed.bsv.bsc-out");
        assert_eq!(
            manifest.comparisons[1].arguments[0],
            "Failed.bsv.bsc-ccomp-out"
        );
        assert_eq!(
            manifest.comparisons[2].arguments[0],
            "Failed.bsv.bsc-vcomp-out"
        );
        assert_eq!(
            manifest.comparisons[3].arguments[0],
            "AVArgUse_C.bsv.bsc-sched-out"
        );
    }

    #[test]
    fn lowers_generated_id_comparisons_without_interpreting_custom_filters() {
        let manifest =
            lower("compare_file_filter_ids generated.out expected.out {} {-e s/foo/bar/g}\n");
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.comparisons.len(), 1);
        assert_eq!(manifest.comparisons[0].helper, "compare_file_filter_ids");
        assert_eq!(
            manifest.comparisons[0].arguments,
            ["generated.out", "expected.out", "", "-e s/foo/bar/g"]
        );
    }

    #[test]
    fn treats_escaped_dollars_in_quotes_as_literals() {
        let manifest = lower(r#"find_n_strings mkCase.v "abc\$EN = 1'b1" 1"#);
        assert_eq!(manifest.assertions.len(), 1);
        assert_eq!(manifest.assertions[0].arguments[1], "abc$EN = 1'b1");
    }

    #[test]
    fn lowers_error_and_warning_count_helpers_as_typed_assertions() {
        let manifest = lower(
            "find_n_error compile.out G0024 2\nfind_n_warning compile.out S0015 3\nno_warnings clean.out\n",
        );
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.assertions.len(), 3);
        assert_eq!(manifest.assertions[0].helper, "find_n_error");
        assert_eq!(
            manifest.assertions[0].arguments,
            ["compile.out", "G0024", "2"]
        );
        assert_eq!(manifest.assertions[1].helper, "find_n_warning");
        assert_eq!(
            manifest.assertions[1].arguments,
            ["compile.out", "S0015", "3"]
        );
        assert_eq!(manifest.assertions[2].helper, "no_warnings");
        assert_eq!(manifest.assertions[2].arguments, ["clean.out"]);
    }

    #[test]
    fn resolves_static_list_and_string_concatenation() {
        let manifest = lower(
            r#"
set modules {One.ba Two.ba}
set suffix Demo
compile_pass "${suffix}.bsv"
test_c_veri_bsv_multi_options Battery mkBattery "$modules common.c" {} {} {} {} 1 1
compile_verilog_pass Filter.bsv {} [list -verilog-filter ./filter]
"#,
        );
        assert_eq!(manifest.contracts.len(), 4);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Demo.bsv"
        ));
    }

    #[test]
    fn lowers_scheduler_source_list_as_an_external_contract_set() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|candidate| candidate.join("testsuite").is_dir())
            .expect("workspace root containing testsuite");
        let source = std::fs::read_to_string(root.join(SCHEDULER_SAT_ORIGIN)).unwrap();
        let manifest = lower_at(SCHEDULER_SAT_ORIGIN, &source);
        assert!(
            !manifest.contracts.is_empty(),
            "scheduler SAT manifest was not recognized: {manifest:#?}"
        );
        assert!(matches!(
            &manifest.contracts[0],
            Contract::ExternalSet(contract)
                if contract.external_kind == ExternalContractKind::SchedulerSat
                    && contract.cases.len() == 24
                    && contract.cases.first().map(String::as_str) == Some("BoolTest")
                    && contract.cases.last().map(String::as_str) == Some("SplitTupleMethodTest")
                    && matches!(
                        contract.guard,
                        Guard::Capability {
                            capability: Capability::Verilog,
                        }
                    )
        ));
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts[0].effective_count(), 24);

        let serialized = serde_json::to_value(&manifest.contracts[0]).unwrap();
        assert_eq!(serialized["kind"], "external_set");
        assert_eq!(serialized["external_kind"], "scheduler_sat");

        let changed = lower_at(SCHEDULER_SAT_ORIGIN, &(source + "\n"));
        assert!(changed.contracts.is_empty());
        assert_eq!(changed.unsupported.len(), 2);
    }

    #[test]
    fn lowers_closed_capability_conjunctions_and_systemc_guards() {
        let manifest = lower(
            r#"
if { ($ctest == 1) && ($systemctest == 1) } {
    compile_pass SystemC.bsv
}
if { ($vtest != 0) && [do_internal_checks] } {
    compile_verilog_pass Checked.bsv
}
"#,
        );
        assert!(manifest.unsupported.is_empty());
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if matches!(
                    &contract.guard,
                    Guard::All { guards }
                        if guards == &[
                            Guard::Capability { capability: Capability::Bluesim },
                            Guard::Capability { capability: Capability::SystemC },
                        ]
                )
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Compile(contract)
                if matches!(
                    &contract.guard,
                    Guard::All { guards }
                        if guards == &[
                            Guard::Capability { capability: Capability::Verilog },
                            Guard::Capability { capability: Capability::InternalChecks },
                        ]
                )
        ));
    }

    #[test]
    fn rejects_dynamic_or_disjunctive_capability_expressions() {
        for condition in [
            "$ctest || $vtest",
            "!$vtest",
            "$systemctest == 0",
            "[file exists Source.bsv]",
            "$verilog_compiler == iverilog",
        ] {
            assert!(capability_condition(condition).is_none(), "{condition}");
        }
    }

    #[test]
    fn lowers_both_sides_of_a_static_if_else_with_complementary_guards() {
        let manifest = lower(
            r#"
if {$vtest == 1} {
    compile_verilog_pass Verilog.bsv
} else {
    compile_pass Frontend.bsv
}
"#,
        );
        assert_eq!(manifest.contracts.len(), 2);
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Compile(contract)
                if matches!(
                    &contract.guard,
                    Guard::Not { guard }
                        if matches!(
                            guard.as_ref(),
                            Guard::Capability {
                                capability: Capability::Verilog,
                            }
                        )
                )
        ));
    }

    #[test]
    fn guarded_assignments_do_not_leak_outside_the_branch() {
        let manifest = lower(
            r#"
set source Outside.bsv
if {$vtest == 1} {
    set source Inside.bsv
    compile_pass $source
}
compile_pass $source
"#,
        );
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Inside.bsv"
        ));
        assert!(matches!(
            &manifest.contracts[1],
            Contract::Compile(contract) if contract.source == "Outside.bsv"
        ));
    }

    #[test]
    fn retains_contracts_behind_an_unresolved_if_guard() {
        let manifest = lower(
            r#"
if {[info exists env(ENABLE_TEST)]} {
    compile_pass Guarded.bsv
}
"#,
        );
        assert_eq!(manifest.contracts.len(), 1);
        assert_eq!(manifest.unsupported.len(), 1);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract)
                if matches!(
                    &contract.guard,
                    Guard::UnsupportedExpression { source, .. }
                        if source == "{[info exists env(ENABLE_TEST)]}"
                )
        ));
    }

    #[test]
    fn lowers_only_the_pinned_showrules_branch_and_exact_three_argument_calls() {
        let source = workspace_source(SHOWRULES_ORIGIN);
        let manifest = lower_at(SHOWRULES_ORIGIN, &source);
        assert!(
            manifest.unsupported.is_empty(),
            "{:?}",
            manifest.unsupported
        );
        let actions = manifest
            .workflow_actions
            .iter()
            .filter_map(|action| match action {
                WorkflowAction::ShowRules(action) => Some(action),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(actions.len(), 12);
        assert!(actions.iter().all(|action| {
            guard_contains_capability(&action.guard, Capability::ShowRules)
                && action.input.ends_with(".vcd")
                && action.output.ends_with("_rules.vcd")
                && action.stdout == format!("{}.showrules-out", action.input)
        }));
        assert!(!actions.iter().any(|action| action.top == "mkMCDTest"));

        for changed in [
            source.replace(
                r#"{ ! [file exists "$showrules"] }"#,
                r#"{ ! [file isfile "$showrules"] }"#,
            ),
            source.replacen(
                "showrules mkTbGCD gcd_bsim.vcd gcd_bsim_rules.vcd",
                "showrules mkTbGCD gcd_bsim.vcd gcd_bsim_rules.vcd -verbose",
                1,
            ),
        ] {
            let changed = lower_at(SHOWRULES_ORIGIN, &changed);
            assert!(!changed.unsupported.is_empty());
            assert!(!changed
                .workflow_actions
                .iter()
                .any(|action| matches!(action, WorkflowAction::ShowRules(_))));
        }
    }

    #[test]
    fn simplifies_closed_double_negation() {
        let guard = Guard::Capability {
            capability: Capability::ShowRules,
        };
        assert_eq!(negate_guard(negate_guard(guard.clone())), guard);
    }
}
