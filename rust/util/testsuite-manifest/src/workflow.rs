use std::collections::BTreeSet;

use crate::model::{
    ArtifactTransferOperation, AssertionContract, BluesimRun, BluesimSequence,
    BluesimSequenceContract, BluesimWorkflow, Guard, LinkObjectsAction, RunBluesimAction,
    SystemcWorkflow, WorkflowAction, WorkflowOperation,
};
use crate::parse_static_tcl_list;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkflowEvent {
    Action(WorkflowAction),
    Assertion(AssertionContract),
    Boundary,
}

pub(crate) fn compose_bluesim_sequences(
    events: Vec<WorkflowEvent>,
) -> (
    Vec<BluesimSequence>,
    Vec<WorkflowAction>,
    Vec<AssertionContract>,
) {
    let mut sequences = Vec::new();
    let mut actions = Vec::new();
    let mut assertions = Vec::new();
    let mut segment = Vec::new();

    for event in events
        .into_iter()
        .chain(std::iter::once(WorkflowEvent::Boundary))
    {
        if !matches!(event, WorkflowEvent::Boundary) {
            segment.push(event);
            continue;
        }
        if let Some(sequence) = compose_link_sequence(&segment) {
            sequences.push(sequence);
        } else {
            for event in segment.drain(..) {
                match event {
                    WorkflowEvent::Action(action) => actions.push(action),
                    WorkflowEvent::Assertion(assertion) => assertions.push(assertion),
                    WorkflowEvent::Boundary => unreachable!("segments do not contain boundaries"),
                }
            }
        }
        segment.clear();
    }
    (sequences, actions, assertions)
}

fn compose_link_sequence(events: &[WorkflowEvent]) -> Option<BluesimSequence> {
    let links = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::Action(WorkflowAction::LinkObjects(link)) => Some(link),
            _ => None,
        })
        .collect::<Vec<_>>();
    let parallel_links = links.iter().all(|link| {
        parse_static_tcl_list(&link.options)
            .is_ok_and(|options| options.iter().any(|option| option == "-parallel-sim-link"))
    });
    let has_erase = events.iter().any(|event| {
        matches!(
            event,
            WorkflowEvent::Action(WorkflowAction::EraseArtifact(_))
        )
    });
    if links.len() < 2 || (!parallel_links && !has_erase) {
        return None;
    }

    sequence_guard(events)?;

    let mut contracts = Vec::new();
    let mut operations = Vec::new();
    let mut linked = false;
    let mut asserted_after_link = false;
    let mut produced = BTreeSet::new();
    for event in events {
        let starts_next_contract = linked
            && match event {
                WorkflowEvent::Action(
                    WorkflowAction::CompileObject(_)
                    | WorkflowAction::LinkObjects(_)
                    | WorkflowAction::EraseArtifact(_),
                ) => true,
                WorkflowEvent::Action(WorkflowAction::TransferArtifact(_)) => asserted_after_link,
                _ => false,
            };
        if starts_next_contract {
            contracts.push(sequence_contract(&operations, &mut produced)?);
            operations.clear();
            linked = false;
            asserted_after_link = false;
        }
        match event {
            WorkflowEvent::Action(
                action @ (WorkflowAction::CompileObject(_)
                | WorkflowAction::TransferArtifact(_)
                | WorkflowAction::EraseArtifact(_)),
            ) => operations.push(WorkflowOperation::Action(action.clone())),
            WorkflowEvent::Action(action @ WorkflowAction::LinkObjects(_)) => {
                operations.push(WorkflowOperation::Action(action.clone()));
                linked = true;
            }
            WorkflowEvent::Assertion(assertion) => {
                operations.push(WorkflowOperation::Assertion(assertion.clone()));
                asserted_after_link |= linked;
            }
            WorkflowEvent::Action(
                WorkflowAction::BuildCObject(_)
                | WorkflowAction::LinkVerilog(_)
                | WorkflowAction::RunBluesim(_)
                | WorkflowAction::RunVerilog(_)
                | WorkflowAction::ShowRules(_)
                | WorkflowAction::LinkSystemc(_)
                | WorkflowAction::BuildSystemc(_)
                | WorkflowAction::RunSystemc(_)
                | WorkflowAction::BluetclRun(_)
                | WorkflowAction::Bsc2Bsv(_)
                | WorkflowAction::BscParsePretty(_)
                | WorkflowAction::EnsureDirectoryAbsent(_)
                | WorkflowAction::CreateDirectory(_)
                | WorkflowAction::TouchArtifact(_)
                | WorkflowAction::TouchCreateArtifact(_)
                | WorkflowAction::RemoveUserRead(_)
                | WorkflowAction::RewriteDarwinCppIncludePath(_)
                | WorkflowAction::RenderGolden(_)
                | WorkflowAction::RenderM4Curdir(_)
                | WorkflowAction::TextNormalize(_)
                | WorkflowAction::VerilogFilter(_)
                | WorkflowAction::Delay(_)
                | WorkflowAction::DumpIntermediate(_),
            )
            | WorkflowEvent::Boundary => return None,
        }
    }
    contracts.push(sequence_contract(&operations, &mut produced)?);
    (contracts.len() == links.len()).then_some(BluesimSequence { contracts })
}

fn sequence_contract(
    operations: &[WorkflowOperation],
    produced: &mut BTreeSet<String>,
) -> Option<BluesimSequenceContract> {
    let mut links = 0;
    for operation in operations {
        match operation {
            WorkflowOperation::Action(WorkflowAction::CompileObject(_)) => {}
            WorkflowOperation::Action(WorkflowAction::LinkObjects(link)) => {
                links += 1;
                produced.extend(link_sequence_artifacts(link));
            }
            WorkflowOperation::Action(WorkflowAction::TransferArtifact(transfer)) => {
                if !produced.contains(&transfer.source) {
                    return None;
                }
                if transfer.operation == ArtifactTransferOperation::Move {
                    produced.remove(&transfer.source);
                }
                produced.insert(transfer.destination.clone());
            }
            WorkflowOperation::Action(WorkflowAction::EraseArtifact(erase)) => {
                if !produced.remove(&erase.path) {
                    return None;
                }
            }
            WorkflowOperation::Assertion(assertion) => {
                if assertion
                    .arguments
                    .first()
                    .is_none_or(|path| !produced.contains(path))
                {
                    return None;
                }
            }
            WorkflowOperation::Action(
                WorkflowAction::BuildCObject(_)
                | WorkflowAction::LinkVerilog(_)
                | WorkflowAction::RunBluesim(_)
                | WorkflowAction::RunVerilog(_)
                | WorkflowAction::ShowRules(_)
                | WorkflowAction::LinkSystemc(_)
                | WorkflowAction::BuildSystemc(_)
                | WorkflowAction::RunSystemc(_)
                | WorkflowAction::BluetclRun(_)
                | WorkflowAction::Bsc2Bsv(_)
                | WorkflowAction::BscParsePretty(_)
                | WorkflowAction::EnsureDirectoryAbsent(_)
                | WorkflowAction::CreateDirectory(_)
                | WorkflowAction::TouchArtifact(_)
                | WorkflowAction::TouchCreateArtifact(_)
                | WorkflowAction::RemoveUserRead(_)
                | WorkflowAction::RewriteDarwinCppIncludePath(_)
                | WorkflowAction::RenderGolden(_)
                | WorkflowAction::RenderM4Curdir(_)
                | WorkflowAction::TextNormalize(_)
                | WorkflowAction::VerilogFilter(_)
                | WorkflowAction::Delay(_)
                | WorkflowAction::DumpIntermediate(_),
            ) => return None,
        }
    }
    (links == 1).then(|| BluesimSequenceContract {
        operations: operations.to_vec(),
    })
}

fn link_sequence_artifacts(link: &LinkObjectsAction) -> [String; 5] {
    [
        format!("{}.bsc-ccomp-out", link.top),
        format!("{}.cxx", link.top),
        format!("model_{}.cxx", link.top),
        format!("{}.o", link.top),
        format!("model_{}.o", link.top),
    ]
}

fn sequence_guard(events: &[WorkflowEvent]) -> Option<&Guard> {
    let guards = events.iter().filter_map(event_guard).collect::<Vec<_>>();
    if guards.iter().any(|guard| !guard.is_resolved()) {
        return None;
    }
    let selected = guards
        .iter()
        .copied()
        .find(|guard| !matches!(guard, Guard::Always))
        .or_else(|| guards.first().copied())?;
    guards
        .iter()
        .all(|guard| **guard == Guard::Always || *guard == selected)
        .then_some(selected)
}

fn event_guard(event: &WorkflowEvent) -> Option<&Guard> {
    match event {
        WorkflowEvent::Action(action) => Some(action.guard()),
        WorkflowEvent::Assertion(assertion) => Some(&assertion.guard),
        WorkflowEvent::Boundary => None,
    }
}

pub(crate) fn compose_bluesim_workflows(
    actions: Vec<WorkflowAction>,
) -> (Vec<BluesimWorkflow>, Vec<WorkflowAction>) {
    let mut workflows = Vec::new();
    let mut consumed = BTreeSet::new();

    for (link_index, action) in actions.iter().enumerate() {
        let WorkflowAction::LinkObjects(link) = action else {
            continue;
        };
        let generation_indices = generation_indices(&actions, link_index, link);
        if generation_indices.is_empty() {
            continue;
        }

        let pre_link_transfer_indices =
            pre_link_transfer_indices(&actions, link_index, &generation_indices, link);

        let link_transfer_indices = actions
            .iter()
            .enumerate()
            .skip(link_index + 1)
            .filter_map(|(transfer_index, candidate)| {
                let WorkflowAction::TransferArtifact(transfer) = candidate else {
                    return None;
                };
                (link_produces_artifact(link, &transfer.source)
                    && guard_covers(&link.guard, &transfer.guard)
                    && nearest_link_artifact_index(
                        &actions,
                        transfer_index,
                        &transfer.source,
                        &transfer.guard,
                    ) == Some(link_index))
                .then_some(transfer_index)
            })
            .collect::<Vec<_>>();

        let run_indices = actions
            .iter()
            .enumerate()
            .skip(link_index + 1)
            .filter_map(|(run_index, action)| {
                let WorkflowAction::RunBluesim(run) = action else {
                    return None;
                };
                (nearest_link_index(&actions, run_index, run) == Some(link_index))
                    .then_some(run_index)
            })
            .collect::<Vec<_>>();

        let runs = run_indices
            .iter()
            .map(|&run_index| {
                let WorkflowAction::RunBluesim(action) = &actions[run_index] else {
                    unreachable!("run indices only contain Bluesim runs")
                };
                let transfer_indices = actions
                    .iter()
                    .enumerate()
                    .skip(run_index + 1)
                    .filter_map(|(transfer_index, candidate)| {
                        let WorkflowAction::TransferArtifact(transfer) = candidate else {
                            return None;
                        };
                        (run_produces_artifact(action, &transfer.source)
                            && guard_covers(&action.guard, &transfer.guard)
                            && nearest_run_index(
                                &actions,
                                transfer_index,
                                &transfer.source,
                                &transfer.guard,
                            ) == Some(run_index))
                        .then_some(transfer_index)
                    })
                    .collect::<Vec<_>>();
                consumed.extend(transfer_indices.iter().copied());
                BluesimRun {
                    action: action.clone(),
                    transfers: transfer_indices
                        .into_iter()
                        .map(|index| {
                            let WorkflowAction::TransferArtifact(action) = &actions[index] else {
                                unreachable!("transfer indices only contain artifact transfers")
                            };
                            action.clone()
                        })
                        .collect(),
                }
            })
            .collect::<Vec<_>>();

        consumed.insert(link_index);
        consumed.extend(generation_indices.iter().copied());
        consumed.extend(pre_link_transfer_indices.iter().copied());
        consumed.extend(link_transfer_indices.iter().copied());
        consumed.extend(run_indices.iter().copied());
        workflows.push(BluesimWorkflow {
            top: link.top.clone(),
            generations: generation_indices
                .into_iter()
                .map(|index| {
                    let WorkflowAction::CompileObject(action) = &actions[index] else {
                        unreachable!("generation indices only contain object compilations")
                    };
                    action.clone()
                })
                .collect(),
            pre_link_transfers: pre_link_transfer_indices
                .into_iter()
                .map(|index| {
                    let WorkflowAction::TransferArtifact(action) = &actions[index] else {
                        unreachable!("pre-link transfer indices only contain artifact transfers")
                    };
                    action.clone()
                })
                .collect(),
            link: link.clone(),
            link_transfers: link_transfer_indices
                .into_iter()
                .map(|index| {
                    let WorkflowAction::TransferArtifact(action) = &actions[index] else {
                        unreachable!("transfer indices only contain artifact transfers")
                    };
                    action.clone()
                })
                .collect(),
            runs,
        });
    }

    let remaining = actions
        .into_iter()
        .enumerate()
        .filter_map(|(index, action)| (!consumed.contains(&index)).then_some(action))
        .collect();
    (workflows, remaining)
}

/// Compose only a contiguous, source-ordered SystemC pipeline.  The importer
/// never infers C++ commands: every accepted action is one of the fixed
/// SystemC helpers lowered from upstream Tcl.
pub(crate) fn compose_systemc_workflows(
    actions: Vec<WorkflowAction>,
) -> (Vec<SystemcWorkflow>, Vec<WorkflowAction>) {
    let mut workflows = Vec::new();
    let mut remaining = Vec::new();
    let mut segment = Vec::new();

    let finish = |segment: &mut Vec<WorkflowAction>,
                  workflows: &mut Vec<SystemcWorkflow>,
                  remaining: &mut Vec<WorkflowAction>| {
        if is_closed_systemc_segment(segment) {
            workflows.push(SystemcWorkflow {
                operations: std::mem::take(segment),
            });
        } else {
            remaining.append(segment);
        }
    };

    for action in actions {
        if matches!(
            action,
            WorkflowAction::CompileObject(_)
                | WorkflowAction::LinkSystemc(_)
                | WorkflowAction::BuildSystemc(_)
                | WorkflowAction::RunSystemc(_)
        ) {
            segment.push(action);
        } else {
            finish(&mut segment, &mut workflows, &mut remaining);
            remaining.push(action);
        }
    }
    finish(&mut segment, &mut workflows, &mut remaining);
    (workflows, remaining)
}

fn is_closed_systemc_segment(segment: &[WorkflowAction]) -> bool {
    if segment.is_empty()
        || !segment
            .iter()
            .any(|action| matches!(action, WorkflowAction::LinkSystemc(_)))
    {
        return false;
    }
    let Some(guard) = segment.first().map(WorkflowAction::guard) else {
        return false;
    };
    if !guard.is_resolved()
        || !segment
            .iter()
            .all(|action| action.guard() == guard && action.guard().is_resolved())
    {
        return false;
    }

    let mut pending_generation = false;
    let mut linked_modules = BTreeSet::new();
    let mut built = false;
    let mut ran = false;
    for action in segment {
        match action {
            WorkflowAction::CompileObject(_) if !built && !ran => pending_generation = true,
            WorkflowAction::LinkSystemc(link) if pending_generation && !built && !ran => {
                if link.objects.split_whitespace().count() != 1 {
                    return false;
                }
                linked_modules.insert(link.top.as_str());
                pending_generation = false;
            }
            WorkflowAction::BuildSystemc(build) if !pending_generation && !built && !ran => {
                let mut modules = build
                    .top_modules
                    .split_whitespace()
                    .chain(build.other_modules.split_whitespace());
                if modules.clone().next().is_none()
                    || modules.any(|module| !linked_modules.contains(module))
                {
                    return false;
                }
                built = true;
            }
            WorkflowAction::RunSystemc(run) if built && !ran => {
                let Some(WorkflowAction::BuildSystemc(build)) = segment
                    .iter()
                    .rev()
                    .skip_while(|candidate| !matches!(candidate, WorkflowAction::BuildSystemc(_)))
                    .next()
                else {
                    return false;
                };
                if run.executable != build.executable {
                    return false;
                }
                ran = true;
            }
            _ => return false,
        }
    }
    !pending_generation
}

fn pre_link_transfer_indices(
    actions: &[WorkflowAction],
    link_index: usize,
    generation_indices: &[usize],
    link: &LinkObjectsAction,
) -> Vec<usize> {
    let Some(first_index) = generation_indices.iter().copied().max() else {
        return Vec::new();
    };
    let Ok(link_arguments) = parse_static_tcl_list(&link.options) else {
        return Vec::new();
    };
    let positional_inputs = link_arguments
        .into_iter()
        .filter(|argument| !argument.starts_with('-'))
        .collect::<BTreeSet<_>>();
    actions
        .iter()
        .enumerate()
        .skip(first_index + 1)
        .take(link_index.saturating_sub(first_index + 1))
        .filter_map(|(index, action)| {
            let WorkflowAction::TransferArtifact(transfer) = action else {
                return None;
            };
            (transfer.operation == ArtifactTransferOperation::Copy
                && transfer.guard == link.guard
                && positional_inputs.contains(&transfer.destination))
            .then_some(index)
        })
        .collect()
}

fn generation_indices(
    actions: &[WorkflowAction],
    link_index: usize,
    link: &LinkObjectsAction,
) -> Vec<usize> {
    let Some(top_index) = nearest_generation(actions, link_index, &link.top, link)
        .or_else(|| unique_unannotated_generation(actions, link_index, link))
    else {
        return Vec::new();
    };
    let mut indices = BTreeSet::from([top_index]);
    for module in link.objects.split_whitespace().filter_map(module_name) {
        if let Some(index) = nearest_generation(actions, link_index, module, link) {
            indices.insert(index);
        }
    }
    indices.into_iter().collect()
}

fn nearest_generation(
    actions: &[WorkflowAction],
    before: usize,
    module: &str,
    link: &LinkObjectsAction,
) -> Option<usize> {
    actions[..before]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, action)| {
            let WorkflowAction::CompileObject(generation) = action else {
                return None;
            };
            (generation.module.as_deref() == Some(module)
                && guard_covers(&generation.guard, &link.guard))
            .then_some(index)
        })
}

fn unique_unannotated_generation(
    actions: &[WorkflowAction],
    before: usize,
    link: &LinkObjectsAction,
) -> Option<usize> {
    let segment_start = actions[..before]
        .iter()
        .rposition(|action| matches!(action, WorkflowAction::LinkObjects(_)))
        .map_or(0, |index| index + 1);
    let mut candidates = actions[segment_start..before]
        .iter()
        .enumerate()
        .filter_map(|(offset, action)| {
            let WorkflowAction::CompileObject(generation) = action else {
                return None;
            };
            (generation.module.is_none() && guard_covers(&generation.guard, &link.guard))
                .then_some(segment_start + offset)
        });
    let candidate = candidates.next()?;
    candidates.next().is_none().then_some(candidate)
}

fn nearest_link_index(
    actions: &[WorkflowAction],
    before: usize,
    run: &RunBluesimAction,
) -> Option<usize> {
    actions[..before]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, action)| {
            let WorkflowAction::LinkObjects(link) = action else {
                return None;
            };
            (link.top == run.executable && guard_covers(&link.guard, &run.guard)).then_some(index)
        })
}

fn nearest_link_artifact_index(
    actions: &[WorkflowAction],
    before: usize,
    artifact: &str,
    consumer_guard: &Guard,
) -> Option<usize> {
    actions[..before]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, action)| {
            let WorkflowAction::LinkObjects(link) = action else {
                return None;
            };
            (link_produces_artifact(link, artifact) && guard_covers(&link.guard, consumer_guard))
                .then_some(index)
        })
}

fn link_produces_artifact(link: &LinkObjectsAction, artifact: &str) -> bool {
    artifact == format!("{}.bsc-ccomp-out", link.top)
}

fn nearest_run_index(
    actions: &[WorkflowAction],
    before: usize,
    artifact: &str,
    consumer_guard: &Guard,
) -> Option<usize> {
    actions[..before]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, action)| {
            let WorkflowAction::RunBluesim(run) = action else {
                return None;
            };
            (run_produces_artifact(run, artifact) && guard_covers(&run.guard, consumer_guard))
                .then_some(index)
        })
}

fn run_produces_artifact(run: &RunBluesimAction, artifact: &str) -> bool {
    if run.stdout == artifact {
        return true;
    }
    let Ok(options) = parse_static_tcl_list(&run.options) else {
        return false;
    };
    let mut vcd_options = options
        .iter()
        .enumerate()
        .filter(|(_, option)| option.as_str() == "-V");
    let Some((index, _)) = vcd_options.next() else {
        return false;
    };
    if vcd_options.next().is_some() {
        return false;
    }
    let vcd = options
        .get(index + 1)
        .filter(|value| !value.starts_with('-'))
        .map_or("dump.vcd", String::as_str);
    vcd == artifact
}

fn guard_covers(producer: &Guard, consumer: &Guard) -> bool {
    producer == consumer || matches!(producer, Guard::Always)
}

fn module_name(token: &str) -> Option<&str> {
    let token = token.strip_suffix(".ba").unwrap_or(token);
    (!token.is_empty()
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')))
    .then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ArtifactTransferAction, ArtifactTransferOperation, CompileObjectAction,
        EraseArtifactAction, Guard, SourceSpan,
    };

    const SPAN: SourceSpan = SourceSpan {
        start_byte: 0,
        end_byte: 1,
        start_line: 1,
        start_column: 1,
        end_line: 1,
        end_column: 2,
    };

    fn generation(source: &str, module: Option<&str>) -> WorkflowAction {
        WorkflowAction::CompileObject(CompileObjectAction {
            source: source.to_owned(),
            module: module.map(str::to_owned),
            options: String::new(),
            guard: Guard::Always,
            span: SPAN,
            expansion: Vec::new(),
        })
    }

    fn link(objects: &str, top: &str) -> WorkflowAction {
        WorkflowAction::LinkObjects(LinkObjectsAction {
            objects: objects.to_owned(),
            top: top.to_owned(),
            options: String::new(),
            expected_exit: bsc_test_plan::ExpectedExit::Success,
            expectation: bsc_test_plan::OperationExpectation::Required,
            error_diagnostic: None,
            guard: Guard::Always,
            span: SPAN,
            expansion: Vec::new(),
        })
    }

    fn run(top: &str, options: &str) -> WorkflowAction {
        WorkflowAction::RunBluesim(RunBluesimAction {
            executable: top.to_owned(),
            options: options.to_owned(),
            stdout: format!("{top}.out"),
            expected_exits: Vec::new(),
            aarch64_expected_exits: None,
            windows_expected_exits: None,
            guard: Guard::Always,
            span: SPAN,
            expansion: Vec::new(),
        })
    }

    fn copy(source: &str, destination: &str) -> WorkflowAction {
        WorkflowAction::TransferArtifact(ArtifactTransferAction {
            operation: ArtifactTransferOperation::Copy,
            source: source.to_owned(),
            destination: destination.to_owned(),
            guard: Guard::Always,
            span: SPAN,
            expansion: Vec::new(),
        })
    }

    #[test]
    fn composes_multiple_generations_and_runs_by_top_and_artifact_flow() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Design.bsv", Some("mkDesign")),
            generation("Tb.bsv", Some("mkTb")),
            link("mkTb mkDesign", "mkTb"),
            run("mkTb", "-m 5"),
            copy("mkTb.out", "first.out"),
            run("mkTb", "-m 10"),
            copy("mkTb.out", "second.out"),
        ]);
        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].generations.len(), 2);
        assert_eq!(workflows[0].runs.len(), 2);
        assert_eq!(workflows[0].runs[0].transfers[0].destination, "first.out");
        assert_eq!(workflows[0].runs[1].transfers[0].destination, "second.out");
    }

    #[test]
    fn composes_a_static_copy_used_as_a_pre_link_native_input() {
        let mut native_link = link("", "mkTb");
        let WorkflowAction::LinkObjects(action) = &mut native_link else {
            unreachable!("link helper creates a Bluesim link action")
        };
        action.options = "helper.c".to_owned();
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", None),
            copy("helper.c.keep", "helper.c"),
            native_link,
        ]);
        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].pre_link_transfers.len(), 1);
        assert_eq!(workflows[0].pre_link_transfers[0].source, "helper.c.keep");
        assert_eq!(workflows[0].pre_link_transfers[0].destination, "helper.c");
    }

    #[test]
    fn composes_declared_vcd_artifacts_and_leaves_compile_only_actions_uncomposed() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("SystemC.bsv", Some("mkSystemC")),
            generation("Test.bsv", Some("mkTest")),
            link("mkTest", "mkTest"),
            run("mkTest", "-V saved.vcd"),
            copy("saved.vcd", "snapshot.vcd"),
        ]);
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].runs[0].transfers.len(), 1);
        assert_eq!(remaining.len(), 1);
        assert!(matches!(remaining[0], WorkflowAction::CompileObject(_)));
    }

    #[test]
    fn composes_the_default_vcd_artifact_and_rejects_undeclared_side_artifacts() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Test.bsv", Some("mkTest")),
            link("mkTest", "mkTest"),
            run("mkTest", "-V"),
            copy("dump.vcd", "snapshot.vcd"),
            copy("trace.log", "snapshot.log"),
        ]);
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].runs[0].transfers.len(), 1);
        assert_eq!(workflows[0].runs[0].transfers[0].source, "dump.vcd");
        assert_eq!(remaining.len(), 1);
        assert!(matches!(remaining[0], WorkflowAction::TransferArtifact(_)));
    }

    #[test]
    fn composes_a_unique_unannotated_generation_in_the_current_link_segment() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", None),
            link("mkTb mkDesign", "mkTb"),
            run("mkTb", ""),
        ]);
        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].generations[0].source, "Tb.bsv");
    }

    #[test]
    fn composes_an_unconditional_generation_into_a_guarded_workflow() {
        let mut guarded_link = link("mkTb", "mkTb");
        let mut guarded_run = run("mkTb", "");
        let guard = Guard::Capability {
            capability: crate::model::Capability::Bluesim,
        };
        if let WorkflowAction::LinkObjects(action) = &mut guarded_link {
            action.guard = guard.clone();
        }
        if let WorkflowAction::RunBluesim(action) = &mut guarded_run {
            action.guard = guard;
        }
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", Some("mkTb")),
            guarded_link,
            guarded_run,
        ]);
        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].runs.len(), 1);
    }

    #[test]
    fn does_not_guess_between_multiple_unannotated_generations() {
        let actions = vec![
            generation("Design.bsv", None),
            generation("Tb.bsv", None),
            link("mkTb mkDesign", "mkTb"),
            run("mkTb", ""),
        ];
        let (workflows, remaining) = compose_bluesim_workflows(actions.clone());
        assert!(workflows.is_empty());
        assert_eq!(remaining, actions);
    }

    #[test]
    fn does_not_compose_a_link_without_a_static_top_generation() {
        let actions = vec![link("mkMissing", "mkMissing"), run("mkMissing", "")];
        let (workflows, remaining) = compose_bluesim_workflows(actions.clone());
        assert!(workflows.is_empty());
        assert_eq!(remaining, actions);
    }

    #[test]
    fn composes_the_declared_link_log_artifact() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", Some("mkTb")),
            link("mkTb", "mkTb"),
            copy("mkTb.bsc-ccomp-out", "link.snapshot"),
        ]);

        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].link_transfers.len(), 1);
        assert_eq!(workflows[0].link_transfers[0].destination, "link.snapshot");
    }

    #[test]
    fn composes_a_static_fixture_copy_used_as_a_link_input() {
        let mut c_link = link("mkTb", "mkTb");
        let WorkflowAction::LinkObjects(action) = &mut c_link else {
            unreachable!("link helper creates an object link")
        };
        action.options = "helper.c".to_owned();
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", Some("mkTb")),
            copy("helper.c.keep", "helper.c"),
            c_link,
        ]);
        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].pre_link_transfers.len(), 1);
        assert_eq!(workflows[0].pre_link_transfers[0].destination, "helper.c");
    }

    #[test]
    fn binds_link_artifacts_to_the_nearest_matching_link() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", Some("mkTb")),
            link("mkTb", "mkTb"),
            copy("mkTb.bsc-ccomp-out", "first.snapshot"),
            link("mkTb", "mkTb"),
            copy("mkTb.bsc-ccomp-out", "second.snapshot"),
        ]);

        assert!(remaining.is_empty());
        assert_eq!(workflows.len(), 2);
        assert_eq!(workflows[0].link_transfers[0].destination, "first.snapshot");
        assert_eq!(
            workflows[1].link_transfers[0].destination,
            "second.snapshot"
        );
    }

    #[test]
    fn leaves_undeclared_link_side_artifacts_uncomposed() {
        let transfer = copy("mkTb.cxx", "generated.snapshot");
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("Tb.bsv", Some("mkTb")),
            link("mkTb", "mkTb"),
            transfer.clone(),
        ]);

        assert_eq!(workflows.len(), 1);
        assert!(workflows[0].link_transfers.is_empty());
        assert_eq!(remaining, vec![transfer]);
    }

    fn parallel_link(top: &str) -> WorkflowEvent {
        let mut action = link("", top);
        let WorkflowAction::LinkObjects(link) = &mut action else {
            unreachable!("link helper creates a link action")
        };
        link.options = "-v -parallel-sim-link 2".to_owned();
        WorkflowEvent::Action(action)
    }

    fn move_artifact(source: &str, destination: &str) -> WorkflowEvent {
        let mut action = copy(source, destination);
        let WorkflowAction::TransferArtifact(transfer) = &mut action else {
            unreachable!("copy helper creates a transfer action")
        };
        transfer.operation = ArtifactTransferOperation::Move;
        WorkflowEvent::Action(action)
    }

    fn erase_artifact(path: &str) -> WorkflowEvent {
        WorkflowEvent::Action(WorkflowAction::EraseArtifact(EraseArtifactAction {
            path: path.to_owned(),
            guard: Guard::Always,
            span: SPAN,
            expansion: Vec::new(),
        }))
    }

    fn regexp_assertion(path: &str) -> WorkflowEvent {
        WorkflowEvent::Assertion(AssertionContract {
            helper: "find_regexp".to_owned(),
            arguments: vec![path.to_owned(), "exec: make".to_owned()],
            guard: Guard::Always,
            span: SPAN,
            expansion: Vec::new(),
        })
    }

    #[test]
    fn composes_parallel_links_as_ordered_contracts_in_one_sequence() {
        let (sequences, actions, assertions) = compose_bluesim_sequences(vec![
            WorkflowEvent::Action(generation("GCD.bsv", Some("mkGCD"))),
            parallel_link("mkGCD"),
            regexp_assertion("mkGCD.bsc-ccomp-out"),
            WorkflowEvent::Action(generation("TbGCD.bsv", Some("mkTbGCD"))),
            parallel_link("mkTbGCD"),
            move_artifact("mkTbGCD.bsc-ccomp-out", "first.out"),
            regexp_assertion("first.out"),
            parallel_link("mkTbGCD"),
            move_artifact("mkTbGCD.bsc-ccomp-out", "second.out"),
            regexp_assertion("second.out"),
        ]);

        assert!(actions.is_empty());
        assert!(assertions.is_empty());
        assert_eq!(sequences.len(), 1);
        assert_eq!(sequences[0].effective_count(), 3);
        assert_eq!(
            sequences[0]
                .contracts
                .iter()
                .map(|contract| contract.actions().count())
                .collect::<Vec<_>>(),
            vec![2, 3, 2]
        );
        assert!(sequences[0]
            .contracts
            .iter()
            .all(|contract| contract.assertions().count() == 1));
    }

    #[test]
    fn composes_erase_and_relink_with_contract_boundary_snapshots() {
        let (sequences, actions, assertions) = compose_bluesim_sequences(vec![
            WorkflowEvent::Action(generation("Bug.bsv", None)),
            WorkflowEvent::Action(link("", "mkTop")),
            regexp_assertion("mkTop.cxx"),
            move_artifact("mkTop.cxx", "mkTop.A.cxx"),
            regexp_assertion("mkTop.A.cxx"),
            erase_artifact("mkTop.o"),
            WorkflowEvent::Action(link("", "mkTop")),
            move_artifact("mkTop.cxx", "mkTop.0.cxx"),
            regexp_assertion("mkTop.0.cxx"),
            erase_artifact("mkTop.o"),
            WorkflowEvent::Action(link("", "mkTop")),
            move_artifact("mkTop.cxx", "mkTop.1.cxx"),
            regexp_assertion("mkTop.1.cxx"),
        ]);

        assert!(actions.is_empty());
        assert!(assertions.is_empty());
        assert_eq!(sequences.len(), 1);
        assert_eq!(sequences[0].effective_count(), 3);
        assert_eq!(
            sequences[0]
                .contracts
                .iter()
                .map(|contract| contract.actions().count())
                .collect::<Vec<_>>(),
            vec![2, 4, 3]
        );
        assert_eq!(
            sequences[0]
                .contracts
                .iter()
                .map(|contract| contract.assertions().count())
                .collect::<Vec<_>>(),
            vec![1, 2, 1]
        );
        assert!(matches!(
            sequences[0].contracts[1].operations[0],
            WorkflowOperation::Action(WorkflowAction::TransferArtifact(_))
        ));
        assert!(matches!(
            sequences[0].contracts[1].operations[1],
            WorkflowOperation::Assertion(_)
        ));
        assert!(matches!(
            sequences[0].contracts[1].operations[2],
            WorkflowOperation::Action(WorkflowAction::EraseArtifact(_))
        ));
    }

    #[test]
    fn does_not_compose_parallel_links_across_a_boundary() {
        let events = vec![
            WorkflowEvent::Action(generation("GCD.bsv", Some("mkGCD"))),
            parallel_link("mkGCD"),
            regexp_assertion("mkGCD.bsc-ccomp-out"),
            WorkflowEvent::Boundary,
            WorkflowEvent::Action(generation("TbGCD.bsv", Some("mkTbGCD"))),
            parallel_link("mkTbGCD"),
            regexp_assertion("mkTbGCD.bsc-ccomp-out"),
        ];
        let (sequences, actions, assertions) = compose_bluesim_sequences(events);

        assert!(sequences.is_empty());
        assert_eq!(actions.len(), 4);
        assert_eq!(assertions.len(), 2);
    }
}
