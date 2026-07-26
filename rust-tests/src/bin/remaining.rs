use bsc_rust_tests::alignment::{
    remaining_inventory, MigrationReadiness, RemainingTestScript, TclCommandCategory,
    UnsupportedTclCommand,
};
use bsc_rust_tests::locate_project_root;
use std::collections::BTreeMap;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("remaining inventory FAILED:\n{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "--check".to_owned());
    if std::env::args().nth(2).is_some() || !matches!(mode.as_str(), "--check" | "--write") {
        return Err("usage: remaining [--check|--write]".to_owned());
    }

    let entries = remaining_inventory()?;
    let rendered = render_inventory(&entries);
    let path = locate_project_root()?.join("rust-tests/REMAINING.md");
    let candidates = entries
        .iter()
        .filter(|entry| entry.readiness == MigrationReadiness::Candidate)
        .count();
    let candidate_contracts = entries
        .iter()
        .filter(|entry| entry.readiness == MigrationReadiness::Candidate)
        .map(|entry| entry.contract_count)
        .sum::<usize>();
    match mode.as_str() {
        "--write" => {
            fs::write(&path, rendered.as_bytes())
                .map_err(|error| format!("write {}: {error}", path.display()))?;
            println!(
                "updated {} with {} remaining scripts; {candidates} typed candidates / {candidate_contracts} contracts",
                path.display(),
                entries.len()
            );
        }
        "--check" => {
            let actual = fs::read_to_string(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            if actual != rendered {
                return Err(format!(
                    "{} is stale; run `pixi run just inventory-update`",
                    path.display()
                ));
            }
            println!(
                "remaining inventory ok: {} scripts, {} contracts; {candidates} typed candidates / {candidate_contracts} contracts",
                entries.len(),
                entries
                    .iter()
                    .map(|entry| entry.contract_count)
                    .sum::<usize>()
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[derive(Default)]
struct AreaSummary {
    scripts: usize,
    contracts: usize,
    candidates: usize,
    candidate_contracts: usize,
    dynamic: usize,
    blocked: usize,
}

struct CommandSummary {
    category: TclCommandCategory,
    calls: usize,
    scripts: usize,
    contracts: usize,
}

fn render_inventory(entries: &[RemainingTestScript]) -> String {
    let static_contracts = entries
        .iter()
        .map(|entry| entry.contract_count)
        .sum::<usize>();
    let mut readiness_summary = BTreeMap::<MigrationReadiness, (usize, usize)>::new();
    let mut suites = BTreeMap::<&str, AreaSummary>::new();
    let mut commands = BTreeMap::<&str, CommandSummary>::new();

    for entry in entries {
        let status = readiness_summary.entry(entry.readiness).or_default();
        status.0 += 1;
        status.1 += entry.contract_count;

        let suite = entry
            .origin
            .strip_prefix("testsuite/")
            .and_then(|path| path.split('/').next())
            .unwrap_or("unknown");
        let area = suites.entry(suite).or_default();
        area.scripts += 1;
        area.contracts += entry.contract_count;
        match entry.readiness {
            MigrationReadiness::Candidate => {
                area.candidates += 1;
                area.candidate_contracts += entry.contract_count;
            }
            MigrationReadiness::Blocked => area.blocked += 1,
            MigrationReadiness::Dynamic => area.dynamic += 1,
            MigrationReadiness::Review => {}
        }

        for command in &entry.unsupported_commands {
            let summary = commands.entry(&command.name).or_insert(CommandSummary {
                category: command.category,
                calls: 0,
                scripts: 0,
                contracts: 0,
            });
            debug_assert_eq!(summary.category, command.category);
            summary.calls += command.count;
            summary.scripts += 1;
            summary.contracts += entry.contract_count;
        }
    }

    let status = |readiness: MigrationReadiness| {
        readiness_summary
            .get(&readiness)
            .copied()
            .unwrap_or_default()
    };
    let (candidate_scripts, candidate_contracts) = status(MigrationReadiness::Candidate);
    let (review_scripts, review_contracts) = status(MigrationReadiness::Review);
    let (blocked_scripts, blocked_contracts) = status(MigrationReadiness::Blocked);
    let (dynamic_scripts, dynamic_contracts) = status(MigrationReadiness::Dynamic);

    let mut candidates = entries
        .iter()
        .filter(|entry| entry.readiness == MigrationReadiness::Candidate)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .contract_count
            .cmp(&left.contract_count)
            .then_with(|| left.origin.cmp(&right.origin))
    });

    let mut command_summaries = commands.into_iter().collect::<Vec<_>>();
    command_summaries.sort_by(|(left_name, left), (right_name, right)| {
        right
            .scripts
            .cmp(&left.scripts)
            .then_with(|| right.contracts.cmp(&left.contracts))
            .then_with(|| right.calls.cmp(&left.calls))
            .then_with(|| left_name.cmp(right_name))
    });

    let mut output = String::new();
    output.push_str("# Remaining testsuite inventory\n\n");
    output.push_str("> Generated by `pixi run just inventory-update`; do not edit manually.\n");
    output.push_str(
        "> `inventory-check` uses the same typed manifest and migration registry as `test-alignment`.\n\n",
    );
    output.push_str("## Summary\n\n");
    output.push_str(&format!(
        "- Remaining test scripts: **{}**\n",
        entries.len()
    ));
    output.push_str(&format!(
        "- Remaining typed contracts: **{static_contracts}**\n"
    ));
    output.push_str(&format!(
        "- Typed migration candidates: **{candidate_scripts} scripts / {candidate_contracts} contracts**\n"
    ));
    output.push_str(&format!(
        "- Typed scripts requiring workflow execution/migration, unsupported-construct review, or new helpers: **{review_scripts} scripts / {review_contracts} contracts**\n"
    ));
    output.push_str(&format!(
        "- Curated known blockers: **{blocked_scripts} scripts / {blocked_contracts} contracts**\n"
    ));
    output.push_str(&format!(
        "- Scripts without typed contracts: **{dynamic_scripts} scripts / {dynamic_contracts} currently recognized contracts**\n\n"
    ));
    output.push_str("`candidate` means that the contract and assertion vocabulary is represented by the typed manifest and the script is not in the curated blocker registry. It is a high-confidence review queue, not permission to skip fixture, option, golden, bug-gate, or runtime validation. `review` rows list composed workflows awaiting Rust execution/migration, ambiguous uncomposed workflow actions, and remaining unsupported constructs; `blocked` reasons are maintained alongside the migration plan and checked against this inventory.\n\n");

    output.push_str("## Ranked typed candidates\n\n");
    output.push_str("| Origin | Typed contracts |\n");
    output.push_str("| --- | ---: |\n");
    for entry in candidates {
        output.push_str(&format!(
            "| `{}` | {} |\n",
            entry.origin, entry.contract_count
        ));
    }

    output.push_str("\n## Highest-leverage workflow and unsupported Tcl commands\n\n");
    output.push_str("The table combines composed workflows awaiting Rust execution/migration, ambiguous uncomposed workflow actions, and unsupported Tcl commands. It is sorted by affected scripts, then affected typed contracts. Contract totals overlap when one script uses multiple commands.\n\n");
    output.push_str(
        "| Command | Category | Calls | Scripts | Typed contracts in affected scripts |\n",
    );
    output.push_str("| --- | --- | ---: | ---: | ---: |\n");
    for (name, summary) in command_summaries.iter().take(40) {
        output.push_str(&format!(
            "| `{name}` | {} | {} | {} | {} |\n",
            summary.category.label(),
            summary.calls,
            summary.scripts,
            summary.contracts
        ));
    }

    output.push_str("\n## By testsuite area\n\n");
    output.push_str("| Area | Remaining scripts | Typed contracts | Candidates | Candidate contracts | Dynamic/custom | Known blockers |\n");
    output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (suite, summary) in suites {
        output.push_str(&format!(
            "| `{suite}` | {} | {} | {} | {} | {} | {} |\n",
            summary.scripts,
            summary.contracts,
            summary.candidates,
            summary.candidate_contracts,
            summary.dynamic,
            summary.blocked
        ));
    }

    output.push_str("\n## Complete remaining list\n\n");
    output.push_str("| Origin | Typed contracts | Readiness | Reason / unsupported Tcl |\n");
    output.push_str("| --- | ---: | --- | --- |\n");
    for entry in entries {
        output.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            entry.origin,
            entry.contract_count,
            entry.readiness.label(),
            render_reason(entry)
        ));
    }
    output
}

fn render_reason(entry: &RemainingTestScript) -> String {
    if let Some(blocker) = &entry.known_blocker {
        return format!("known blocker: {blocker}");
    }
    if entry.unsupported_commands.is_empty() {
        return if entry.contract_count == 0 {
            "no typed contract; inspect unsupported constructs".to_owned()
        } else {
            "supported API vocabulary only; review fixtures, options, goldens, and runtime"
                .to_owned()
        };
    }
    render_commands(&entry.unsupported_commands)
}

fn render_commands(commands: &[UnsupportedTclCommand]) -> String {
    commands
        .iter()
        .map(|command| {
            format!(
                "`{}`×{} ({})",
                command.name,
                command.count,
                command.category.label()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_ranked_candidates_and_command_leverage() {
        let entries = [
            RemainingTestScript {
                origin: "testsuite/bsc.alpha/one.exp".to_owned(),
                contract_count: 2,
                readiness: MigrationReadiness::Candidate,
                unsupported_commands: Vec::new(),
                known_blocker: None,
            },
            RemainingTestScript {
                origin: "testsuite/bsc.beta/two.exp".to_owned(),
                contract_count: 3,
                readiness: MigrationReadiness::Review,
                unsupported_commands: vec![UnsupportedTclCommand {
                    name: "if".to_owned(),
                    count: 2,
                    category: TclCommandCategory::ControlState,
                }],
                known_blocker: None,
            },
            RemainingTestScript {
                origin: "testsuite/bsc.beta/three.exp".to_owned(),
                contract_count: 0,
                readiness: MigrationReadiness::Dynamic,
                unsupported_commands: vec![UnsupportedTclCommand {
                    name: "if".to_owned(),
                    count: 1,
                    category: TclCommandCategory::ControlState,
                }],
                known_blocker: None,
            },
        ];
        let rendered = render_inventory(&entries);
        assert!(rendered.contains("Remaining test scripts: **3**"));
        assert!(rendered.contains("Typed migration candidates: **1 scripts / 2 contracts**"));
        assert!(rendered.contains("| `if` | control/state | 3 | 2 | 3 |"));
        assert!(rendered.contains("| `bsc.alpha` | 1 | 2 | 1 | 2 | 0 | 0 |"));
        assert!(rendered.contains("`if`×2 (control/state)"));
    }
}
