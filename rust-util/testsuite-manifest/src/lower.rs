use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use tree_sitter::{Node, Tree};

use crate::model::{
    ArtifactTransferAction, ArtifactTransferOperation, AssertionContract, Capability,
    ComparisonContract, CompileContract, CompileObjectAction, Contract, ExternalContractKind,
    ExternalSetContract, GenerationStrategy, Guard, LinkObjectsAction, RunBluesimAction,
    ScriptManifest, SimulationBackend, SimulationContract, SourceSpan, UnsupportedConstruct,
    UnsupportedReason, WorkflowAction,
};

const SCHEDULER_SAT_ORIGIN: &str = "testsuite/bsc.scheduler/sat/sat.exp";

pub(crate) fn lower_script<'a>(origin: String, source: &'a [u8], tree: &'a Tree) -> ScriptManifest {
    let mut lowerer = Lowerer {
        source,
        constants: BTreeMap::new(),
        procedures: BTreeMap::new(),
        call_stack: Vec::new(),
        invocation_stack: Vec::new(),
        guard: Guard::Always,
        contracts: Vec::new(),
        assertions: Vec::new(),
        comparisons: Vec::new(),
        workflow_actions: Vec::new(),
        unsupported: Vec::new(),
    };
    lowerer.lower_script_node(tree.root_node());
    if origin == SCHEDULER_SAT_ORIGIN {
        if let Some(StaticValue::List { values, span }) = lowerer.constants.get("sources") {
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
        }
    }
    let (bluesim_workflows, workflow_actions) =
        crate::workflow::compose_bluesim_workflows(lowerer.workflow_actions);
    ScriptManifest {
        origin,
        source_sha256: format!("{:x}", Sha256::digest(source)),
        contracts: lowerer.contracts,
        assertions: lowerer.assertions,
        comparisons: lowerer.comparisons,
        bluesim_workflows,
        workflow_actions,
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
struct Procedure<'a> {
    parameters: Vec<String>,
    body: Node<'a>,
}

struct Lowerer<'a> {
    source: &'a [u8],
    constants: BTreeMap<String, StaticValue>,
    procedures: BTreeMap<String, Procedure<'a>>,
    call_stack: Vec<String>,
    invocation_stack: Vec<SourceSpan>,
    guard: Guard,
    contracts: Vec<Contract>,
    assertions: Vec<AssertionContract>,
    comparisons: Vec<ComparisonContract>,
    workflow_actions: Vec<WorkflowAction>,
    unsupported: Vec<UnsupportedConstruct>,
}

impl<'a> Lowerer<'a> {
    fn lower_script_node(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "comment" => {}
                "command" => self.lower_command(child),
                "set" => self.lower_set(child),
                "if" => self.lower_if(child),
                "procedure" => self.lower_procedure(child),
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
            if argument.named_child_count() != 1 {
                self.push_unsupported(
                    argument,
                    Some("proc"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                return;
            }
            let parameter = self.text(parameter).trim().to_owned();
            if !is_static_variable_name(&parameter) {
                self.push_unsupported(
                    argument,
                    Some("proc"),
                    UnsupportedReason::UnsupportedControlFlow,
                );
                return;
            }
            parameters.push(parameter);
        }
        self.procedures.insert(name, Procedure { parameters, body });
    }

    fn lower_set(&mut self, node: Node<'a>) {
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
        if let Some(values) = self.static_list(value_node) {
            self.constants.insert(
                name,
                StaticValue::List {
                    values,
                    span: self.span(node),
                },
            );
            return;
        }
        let Some(value) = self.static_word(value_node) else {
            self.constants.remove(&name);
            self.push_unsupported(node, Some("set"), UnsupportedReason::DynamicAssignment);
            return;
        };
        self.constants.insert(name, StaticValue::Scalar(value));
    }

    fn lower_if(&mut self, node: Node<'a>) {
        let Some(condition) = node.child_by_field_name("condition") else {
            self.push_unsupported(node, Some("if"), UnsupportedReason::UnsupportedControlFlow);
            return;
        };
        let Some(consequence) = node.child_by_field_name("consequence") else {
            self.push_unsupported(node, Some("if"), UnsupportedReason::UnsupportedControlFlow);
            return;
        };

        let condition_guard = match capability_condition(self.text(condition)) {
            Some(capability) => Guard::Capability { capability },
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
                    combine_guards(
                        previous_guard.clone(),
                        Guard::Not {
                            guard: Box::new(condition_guard),
                        },
                    ),
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

    fn lower_command(&mut self, node: Node<'a>) {
        let Some(name_node) = node.child_by_field_name("name") else {
            self.push_unsupported(node, None, UnsupportedReason::UnsupportedSyntax);
            return;
        };
        let name = self.text(name_node).trim().to_owned();
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

        if let Some(procedure) = self.procedures.get(&name).cloned() {
            self.lower_procedure_call(node, &name, arguments, procedure);
            return;
        }

        if self.lower_workflow_action(node, &name, &arguments) {
            return;
        }
        if is_compile_helper(&name) {
            let Some(source) = arguments.first().filter(|source| !source.is_empty()) else {
                self.push_unsupported(node, Some(&name), UnsupportedReason::DynamicArguments);
                return;
            };
            self.contracts.push(Contract::Compile(CompileContract {
                source: source.clone(),
                helper: name,
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            }));
            return;
        }
        if let Some(simulation) = simulation_shape(&name, &arguments) {
            let span = self.span(node);
            for backend in simulation.backends.iter().copied() {
                self.contracts
                    .push(Contract::Simulation(SimulationContract {
                        source: simulation.source.clone(),
                        helper: name.clone(),
                        backend,
                        generation: simulation.generation_for(backend),
                        guard: self.guard.clone(),
                        span,
                        expansion: self.invocation_stack.clone(),
                    }));
            }
            return;
        }
        if is_assertion_helper(&name) {
            self.assertions.push(AssertionContract {
                helper: name,
                arguments,
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            });
            return;
        }
        if matches!(name.as_str(), "compare_file" | "compare_verilog") {
            self.comparisons.push(ComparisonContract {
                helper: name,
                arguments,
                guard: self.guard.clone(),
                span: self.span(node),
                expansion: self.invocation_stack.clone(),
            });
            return;
        }
        self.push_unsupported(node, Some(&name), UnsupportedReason::UnsupportedCommand);
    }

    fn lower_workflow_action(&mut self, node: Node<'a>, name: &str, arguments: &[String]) -> bool {
        let guard = self.guard.clone();
        let span = self.span(node);
        let expansion = self.invocation_stack.clone();
        let action = match (name, arguments) {
            ("compile_object_pass", [source]) => {
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: source.clone(),
                    module: None,
                    options: String::new(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("compile_object_pass", [source, module]) => {
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: source.clone(),
                    module: (!module.is_empty()).then(|| module.clone()),
                    options: String::new(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("compile_object_pass", [source, module, options]) => {
                WorkflowAction::CompileObject(CompileObjectAction {
                    source: source.clone(),
                    module: (!module.is_empty()).then(|| module.clone()),
                    options: options.clone(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass", [objects, top]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: String::new(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("link_objects_pass", [objects, top, options]) => {
                WorkflowAction::LinkObjects(LinkObjectsAction {
                    objects: objects.clone(),
                    top: top.clone(),
                    options: options.clone(),
                    guard,
                    span,
                    expansion,
                })
            }
            ("sim_output", [executable]) => WorkflowAction::RunBluesim(RunBluesimAction {
                executable: executable.clone(),
                options: String::new(),
                stdout: format!("{executable}.out"),
                guard,
                span,
                expansion,
            }),
            ("sim_output", [executable, options]) => WorkflowAction::RunBluesim(RunBluesimAction {
                executable: executable.clone(),
                options: options.clone(),
                stdout: format!("{executable}.out"),
                guard,
                span,
                expansion,
            }),
            ("copy", [source, destination]) | ("move", [source, destination]) => {
                let operation = if name == "copy" {
                    ArtifactTransferOperation::Copy
                } else {
                    ArtifactTransferOperation::Move
                };
                WorkflowAction::TransferArtifact(ArtifactTransferAction {
                    operation,
                    source: source.clone(),
                    destination: destination.clone(),
                    guard,
                    span,
                    expansion,
                })
            }
            _ => return false,
        };
        self.workflow_actions.push(action);
        true
    }

    fn lower_procedure_call(
        &mut self,
        node: Node<'a>,
        name: &str,
        arguments: Vec<String>,
        procedure: Procedure<'a>,
    ) {
        if arguments.len() != procedure.parameters.len()
            || self.call_stack.iter().any(|item| item == name)
        {
            self.push_unsupported(node, Some(name), UnsupportedReason::UnsupportedControlFlow);
            return;
        }

        let previous_constants = self.constants.clone();
        for (parameter, value) in procedure.parameters.iter().zip(arguments) {
            self.constants
                .insert(parameter.clone(), StaticValue::Scalar(value));
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
            ("make_bsc_vcomp_output_name", [source]) => Some(format!("{source}.bsc-vcomp-out")),
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

    fn push_unsupported(
        &mut self,
        node: Node<'a>,
        command: Option<&str>,
        reason: UnsupportedReason,
    ) {
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
    let extension = if matches!(
        name,
        "test_c_veri" | "test_c_veri_bs_modules" | "test_c_veri_bs_modules_options"
    ) {
        "bs"
    } else {
        "bsv"
    };
    let source = format!("{module}.{extension}");
    let separate_generation = name.contains("separately")
        || name.starts_with("test_c_only_")
        || name.starts_with("test_veri_only_");
    let backends = if name.starts_with("test_c_only_") {
        vec![SimulationBackend::Bluesim]
    } else if name.starts_with("test_veri_only_") {
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

fn capability_condition(condition: &str) -> Option<Capability> {
    let normalized = condition
        .trim()
        .trim_matches(['{', '}'])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    match normalized.as_str() {
        "$ctest" | "$ctest == 1" | "$ctest != 0" => Some(Capability::Bluesim),
        "$vtest" | "$vtest == 1" => Some(Capability::Verilog),
        _ => None,
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

fn is_compile_helper(name: &str) -> bool {
    matches!(
        name,
        "compile_pass"
            | "compile_fail"
            | "compile_fail_error"
            | "compile_verilog_pass"
            | "compile_verilog_fail"
            | "compile_verilog_fail_error"
            | "compile_verilog_pass_warning"
            | "compile_verilog_schedule_pass"
    )
}

fn is_simulation_helper(name: &str) -> bool {
    matches!(
        name,
        "test_c_veri"
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
    )
}

fn is_assertion_helper(name: &str) -> bool {
    matches!(
        name,
        "find_n_strings"
            | "string_occurs"
            | "string_does_not_occur"
            | "find_regexp"
            | "find_regexp_fail"
            | "find_n_regexp"
            | "find_n_emsg"
    )
}

fn is_builtin_control_or_state(kind: &str) -> bool {
    matches!(
        kind,
        "procedure"
            | "while"
            | "foreach"
            | "try"
            | "catch"
            | "expr_cmd"
            | "global"
            | "namespace"
            | "regexp"
    )
}

fn is_static_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn contains_dynamic_syntax(value: &str) -> bool {
    value.contains('$') || value.contains('[') || value.contains(']')
}

fn strip_delimiters(value: &str, start: char, end: char) -> &str {
    value
        .strip_prefix(start)
        .and_then(|value| value.strip_suffix(end))
        .unwrap_or(value)
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
"#,
        );
        assert_eq!(manifest.contracts.len(), 4);
        assert!(matches!(
            &manifest.contracts[0],
            Contract::Compile(contract) if contract.source == "Demo.bsv"
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
    fn evaluates_allowlisted_output_name_helpers() {
        let manifest = lower("compare_file [make_bsc_output_name Failed.bsv]\n");
        assert_eq!(manifest.comparisons.len(), 1);
        assert_eq!(manifest.comparisons[0].arguments[0], "Failed.bsv.bsc-out");
    }

    #[test]
    fn treats_escaped_dollars_in_quotes_as_literals() {
        let manifest = lower(r#"find_n_strings mkCase.v "abc\$EN = 1'b1" 1"#);
        assert_eq!(manifest.assertions.len(), 1);
        assert_eq!(manifest.assertions[0].arguments[1], "abc$EN = 1'b1");
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
        let manifest = lower_at(
            SCHEDULER_SAT_ORIGIN,
            "set sources [list First Second Third]\n",
        );
        assert!(matches!(
            &manifest.contracts[0],
            Contract::ExternalSet(contract)
                if contract.external_kind == ExternalContractKind::SchedulerSat
                    && contract.cases == ["First", "Second", "Third"]
                    && matches!(
                        contract.guard,
                        Guard::Capability {
                            capability: Capability::Verilog,
                        }
                    )
        ));
        assert!(manifest.unsupported.is_empty());
        assert_eq!(manifest.contracts[0].effective_count(), 3);

        let serialized = serde_json::to_value(&manifest.contracts[0]).unwrap();
        assert_eq!(serialized["kind"], "external_set");
        assert_eq!(serialized["external_kind"], "scheduler_sat");
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
}
