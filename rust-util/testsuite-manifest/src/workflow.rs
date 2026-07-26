use std::collections::BTreeSet;

use crate::model::{
    BluesimRun, BluesimWorkflow, Guard, LinkObjectsAction, RunBluesimAction, WorkflowAction,
};

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
                        (transfer.source == action.stdout
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
            link: link.clone(),
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
            (run.stdout == artifact && guard_covers(&run.guard, consumer_guard)).then_some(index)
        })
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
        ArtifactTransferAction, ArtifactTransferOperation, CompileObjectAction, Guard, SourceSpan,
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
    fn leaves_compile_only_and_unmatched_side_artifacts_uncomposed() {
        let (workflows, remaining) = compose_bluesim_workflows(vec![
            generation("SystemC.bsv", Some("mkSystemC")),
            generation("Test.bsv", Some("mkTest")),
            link("mkTest", "mkTest"),
            run("mkTest", "-V dump.vcd"),
            copy("dump.vcd", "saved.vcd"),
        ]);
        assert_eq!(workflows.len(), 1);
        assert_eq!(remaining.len(), 2);
        assert!(matches!(remaining[0], WorkflowAction::CompileObject(_)));
        assert!(matches!(remaining[1], WorkflowAction::TransferArtifact(_)));
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
}
