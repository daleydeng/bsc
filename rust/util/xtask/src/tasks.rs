use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use xshell::{cmd, Shell};

use crate::environment::{save_oss_root, PreparedEnvironment};
use crate::{msys, test_temp, toolchain, z3_bridge};

pub struct Tasks<'a> {
    shell: &'a Shell,
    environment: &'a PreparedEnvironment,
    cargo: OsString,
}

impl<'a> Tasks<'a> {
    pub fn new(shell: &'a Shell, environment: &'a PreparedEnvironment) -> Self {
        Self {
            shell,
            environment,
            cargo: env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")),
        }
    }

    pub fn configure_oss_cad_suite(&self, root: &Path) -> Result<()> {
        save_oss_root(&self.environment.root, root)?;
        Ok(())
    }

    pub fn toolchain(&self) -> Result<()> {
        toolchain::initialize(self.environment)
    }

    pub fn haskell_deps(&self) -> Result<()> {
        toolchain::install_dependencies(self.environment)
    }

    pub fn doctor(&self) -> Result<()> {
        msys::doctor(self.environment)
    }

    pub fn build(&self) -> Result<()> {
        z3_bridge::build(self.environment)?;
        z3_bridge::install_runtime(self.environment)?;
        msys::build(self.environment)?;
        z3_bridge::install_runtime(self.environment)
    }

    pub fn smoke(&self) -> Result<()> {
        msys::smoke(self.environment)
    }

    pub fn clean(&self) -> Result<()> {
        msys::clean(self.environment)?;
        z3_bridge::clean(self.environment)
    }

    pub fn shell(&self) -> Result<()> {
        msys::shell(self.environment)
    }

    pub fn test_z3(&self) -> Result<()> {
        self.test_plans(&["bsc.scheduler/sat/sat".to_owned(), "--exact".to_owned()])
            .context("scheduler SAT Test Plan failed")
    }

    pub fn contracts_parse_check(&self) -> Result<()> {
        let report = bsc_testsuite_manifest::scan_testsuite(&self.environment.root)
            .context("could not parse upstream Tcl testsuite")?;

        let opaque_issues = report
            .issues
            .iter()
            .filter(|issue| {
                issue.scope == bsc_testsuite_manifest::SyntaxIssueScope::OpaqueDataArgument
            })
            .count();
        let opaque_scripts = report
            .issues
            .iter()
            .filter(|issue| {
                issue.scope == bsc_testsuite_manifest::SyntaxIssueScope::OpaqueDataArgument
            })
            .map(|issue| &issue.path)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let mut issues_by_script = BTreeMap::<_, Vec<_>>::new();
        for issue in &report.issues {
            if issue.scope == bsc_testsuite_manifest::SyntaxIssueScope::Structural {
                issues_by_script.entry(&issue.path).or_default().push(issue);
            }
        }
        let mut affected_scripts = issues_by_script.into_iter().collect::<Vec<_>>();
        affected_scripts.sort_by(|(left_path, left), (right_path, right)| {
            right
                .len()
                .cmp(&left.len())
                .then_with(|| left_path.cmp(right_path))
        });

        let structurally_clean_scripts = report.scripts - affected_scripts.len();
        println!(
            "Tree-sitter Tcl parsed {} contract scripts ({} structurally clean, {} with structural issues; {} opaque arguments masked, {} normalization rewrites, {} residual opaque issues in {} scripts; {} bytes, {} node kinds)",
            report.scripts,
            structurally_clean_scripts,
            affected_scripts.len(),
            report.opaque_arguments,
            report.normalization_rewrites,
            opaque_issues,
            opaque_scripts,
            report.bytes,
            report.node_kinds.len(),
        );
        for (path, issues) in affected_scripts.iter().take(30) {
            let first = issues[0];
            eprintln!(
                "{}: {} issues; first at {}:{} ({} {:?}; ancestors: {})",
                path.display(),
                issues.len(),
                first.start_line,
                first.start_column,
                first.kind.label(),
                first.node_kind,
                first.ancestors.join(" <- "),
            );
        }
        if affected_scripts.len() > 30 {
            eprintln!(
                "... {} more scripts with syntax issues",
                affected_scripts.len() - 30
            );
        }
        let structural_issues = affected_scripts
            .iter()
            .map(|(_, issues)| issues.len())
            .sum::<usize>();
        if structural_issues != 0 {
            bail!(
                "Tree-sitter Tcl reported {structural_issues} structural syntax issues in {} scripts",
                affected_scripts.len(),
            );
        }
        Ok(())
    }

    pub fn contracts_ir_check(&self) -> Result<()> {
        let manifest = bsc_testsuite_manifest::build_manifest(&self.environment.root)
            .context("could not lower upstream Tcl testsuite into contract IR")?;
        let summary = manifest.summary();
        println!(
            "contract IR: {} scripts, {} compile contracts, {} simulation contracts, {} external contracts in {} sets ({} unresolved), {} assertions, {} comparisons, {} composed Bluesim sequences ({} contracts), {} composed Bluesim workflows ({} effective contracts), {} uncomposed workflow actions in {} scripts, {} unsupported constructs in {} scripts",
            summary.scripts,
            summary.compile_contracts,
            summary.simulation_contracts,
            summary.external_contracts,
            summary.external_contract_sets,
            summary.unresolved_contracts,
            summary.assertions,
            summary.comparisons,
            summary.bluesim_sequences,
            summary.bluesim_sequence_contracts,
            summary.bluesim_workflows,
            summary.bluesim_workflow_contracts,
            summary.workflow_actions,
            summary.scripts_with_workflow_actions,
            summary.unsupported_constructs,
            summary.scripts_with_unsupported,
        );
        Ok(())
    }

    pub fn contracts_check(&self) -> Result<()> {
        let path = self
            .environment
            .root
            .join("rust/tests/contracts/upstream-contracts.json");
        let manifest = bsc_testsuite_manifest::build_manifest(&self.environment.root)
            .context("could not lower upstream Tcl testsuite into contract IR")?;
        let expected = bsc_testsuite_manifest::render_manifest(&manifest)
            .context("could not render typed contract manifest")?;
        let actual = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        if actual != expected {
            bail!(
                "{} is stale; run `pixi run just contracts-update`",
                path.display()
            );
        }
        let summary = manifest.summary();
        println!(
            "contract manifest ok: {} scripts, {} compile + {} simulation + {} external contracts ({} unresolved), {} composed Bluesim sequences ({} contracts), {} composed Bluesim workflows ({} effective contracts), {} uncomposed workflow actions, {} unsupported constructs",
            summary.scripts,
            summary.compile_contracts,
            summary.simulation_contracts,
            summary.external_contracts,
            summary.unresolved_contracts,
            summary.bluesim_sequences,
            summary.bluesim_sequence_contracts,
            summary.bluesim_workflows,
            summary.bluesim_workflow_contracts,
            summary.workflow_actions,
            summary.unsupported_constructs,
        );
        Ok(())
    }

    pub fn contracts_update(&self) -> Result<()> {
        let path = self
            .environment
            .root
            .join("rust/tests/contracts/upstream-contracts.json");
        let manifest = bsc_testsuite_manifest::build_manifest(&self.environment.root)
            .context("could not lower upstream Tcl testsuite into contract IR")?;
        let rendered = bsc_testsuite_manifest::render_manifest(&manifest)
            .context("could not render typed contract manifest")?;
        fs::write(&path, rendered)
            .with_context(|| format!("could not write {}", path.display()))?;
        let summary = manifest.summary();
        println!(
            "updated {}: {} scripts, {} compile + {} simulation + {} external contracts ({} unresolved), {} composed Bluesim sequences ({} contracts), {} composed Bluesim workflows ({} effective contracts), {} uncomposed workflow actions, {} unsupported constructs",
            path.display(),
            summary.scripts,
            summary.compile_contracts,
            summary.simulation_contracts,
            summary.external_contracts,
            summary.unresolved_contracts,
            summary.bluesim_sequences,
            summary.bluesim_sequence_contracts,
            summary.bluesim_workflows,
            summary.bluesim_workflow_contracts,
            summary.workflow_actions,
            summary.unsupported_constructs,
        );
        Ok(())
    }

    pub fn plans_check(&self) -> Result<()> {
        let generated = bsc_testsuite_manifest::build_test_plans(&self.environment.root)
            .context("could not import upstream Tcl tests into Test Plans")?;
        let expected = render_plan_files(&generated)?;
        let root = self.environment.root.join("rust/tests/plans");
        let actual_paths = collect_relative_files(&root)?;
        let expected_paths = expected.keys().cloned().collect::<BTreeSet<_>>();
        if actual_paths != expected_paths {
            let missing = expected_paths
                .difference(&actual_paths)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            let extra = actual_paths
                .difference(&expected_paths)
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            bail!(
                "committed Test Plan file set is stale; missing: {}; extra: {}; run `pixi run just plans-update`",
                if missing.is_empty() { "none".to_owned() } else { missing.join(", ") },
                if extra.is_empty() { "none".to_owned() } else { extra.join(", ") },
            );
        }
        for (relative, expected) in &expected {
            let path = root.join(relative);
            let actual = fs::read_to_string(&path)
                .with_context(|| format!("could not read {}", path.display()))?;
            if actual != *expected {
                bail!(
                    "{} is stale; run `pixi run just plans-update`",
                    path.display()
                );
            }
        }
        print_plan_summary("Test Plans are current", generated.summary());
        Ok(())
    }

    pub fn plans_update(&self) -> Result<()> {
        let generated = bsc_testsuite_manifest::build_test_plans(&self.environment.root)
            .context("could not import upstream Tcl tests into Test Plans")?;
        let files = render_plan_files(&generated)?;
        let rust_tests = self.environment.root.join("rust/tests");
        let target = rust_tests.join("plans");
        let process = std::process::id();
        let temporary = rust_tests.join(format!(".plans.tmp-{process}"));
        let backup = rust_tests.join(".plans.backup");
        remove_directory_if_present(&temporary)?;
        if backup.exists() {
            if target.exists() {
                remove_directory_if_present(&backup)?;
            } else {
                rename_with_retry(&backup, &target).with_context(|| {
                    format!(
                        "a previous Test Plan update was interrupted and {} could not be restored to {}",
                        backup.display(),
                        target.display()
                    )
                })?;
            }
        }
        write_plan_files(&temporary, &files)?;

        if target.exists() {
            rename_with_retry(&target, &backup).with_context(|| {
                format!(
                    "could not move existing Test Plans from {} to {}",
                    target.display(),
                    backup.display()
                )
            })?;
        }
        if let Err(publish_error) = rename_with_retry(&temporary, &target) {
            if backup.exists() {
                if let Err(rollback_error) = rename_with_retry(&backup, &target) {
                    bail!(
                        "could not publish generated Test Plans from {} to {}: {publish_error}; rollback from {} also failed: {rollback_error}",
                        temporary.display(),
                        target.display(),
                        backup.display()
                    );
                }
            }
            let _ = remove_directory_if_present(&temporary);
            return Err(publish_error).with_context(|| {
                format!(
                    "could not publish generated Test Plans from {} to {} (the previous directory was restored)",
                    temporary.display(),
                    target.display()
                )
            });
        }
        remove_directory_if_present(&backup)?;
        print_plan_summary("updated Test Plans", generated.summary());
        Ok(())
    }

    pub fn plans_audit(&self) -> Result<()> {
        let testsuite = self.environment.root.join("testsuite");
        let mut counts = BTreeMap::<&'static str, usize>::new();
        audit_testsuite_files(&testsuite, &mut counts)?;
        let expected = BTreeMap::from([
            ("contract .exp", 860),
            ("infrastructure .exp", 3),
            ("Makefile", 931),
            (".tcl", 31),
            (".cmd", 23),
            (".pl", 19),
            (".sh", 3),
        ]);
        if counts != expected {
            bail!(
                "testsuite executable-input inventory changed: actual {counts:?}, expected {expected:?}; classify the new or removed files before updating this gate"
            );
        }
        let generated = bsc_testsuite_manifest::build_test_plans(&self.environment.root)
            .context("could not import Test Plans during testsuite audit")?;
        if generated.plans.len() != expected["contract .exp"] {
            bail!(
                "plan importer found {} origins but inventory found {} contract .exp files",
                generated.plans.len(),
                expected["contract .exp"]
            );
        }
        println!(
            "testsuite audit ok: 860 contract .exp + 3 infrastructure .exp; 931 Makefiles; 31 Tcl, 23 command, 19 Perl, and 3 shell auxiliary files"
        );
        print_plan_summary("import coverage", generated.summary());
        Ok(())
    }

    pub fn contracts_cst(&self, script: &Path) -> Result<()> {
        let path = if script.is_absolute() {
            script.to_owned()
        } else {
            self.environment.root.join(script)
        };
        let source = fs::read(&path)
            .with_context(|| format!("could not read Tcl script {}", path.display()))?;
        let mut parser = bsc_testsuite_manifest::TclParser::new()
            .context("could not load Tree-sitter Tcl grammar")?;
        let (tree, adjustments) = parser
            .parse_contract(&source, &path)
            .with_context(|| format!("could not parse Tcl script {}", path.display()))?;
        eprintln!(
            "masked {} opaque data arguments; applied {} normalization rewrites",
            adjustments.opaque_arguments, adjustments.normalization_rewrites
        );
        println!("{}", tree.root_node().to_sexp());
        Ok(())
    }

    pub fn inventory_check(&self) -> Result<()> {
        self.inventory("--check")
    }

    pub fn inventory_update(&self) -> Result<()> {
        self.inventory("--write")
    }

    pub fn test_plans(&self, arguments: &[String]) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin bsc-test --jobs {jobs} -- --jobs {jobs} {arguments...}"
        )
        .run()
        .context("Test Plan execution failed")?;
        Ok(())
    }

    pub fn test_rust(&self) -> Result<()> {
        self.contracts_check()?;
        self.plans_check()?;
        self.plans_audit()?;
        self.inventory_check()?;

        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} test --locked --package bsc-rust-tests --lib --jobs {jobs} -- --test-threads {jobs}"
        )
        .run()
        .context("Rust unit tests failed")?;
        self.test_plans(&[])
    }

    pub fn test_cold(&self) -> Result<()> {
        env::set_var("BSC_TEST_CACHE", "0");
        if self.environment.sccache_managed_cxx {
            env::set_var("CXX", "c++");
        }
        self.test_rust()
    }

    pub fn test_prune(&self) -> Result<()> {
        test_temp::prune(self.environment)
    }

    pub fn sccache_stats(&self) -> Result<()> {
        let sh = self.shell;
        cmd!(sh, "sccache.exe --show-stats")
            .run()
            .context("could not read sccache statistics")?;
        Ok(())
    }

    pub fn sccache_clear(&self) -> Result<()> {
        let sh = self.shell;
        cmd!(sh, "sccache.exe --zero-stats")
            .run()
            .context("could not reset sccache statistics")?;
        cmd!(sh, "sccache.exe --stop-server").run().ok();
        let cache = env::var_os("SCCACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| self.environment.root.join(".pixi/cache/sccache"));
        remove_directory_if_present(&cache)
            .with_context(|| format!("could not clear sccache directory {}", cache.display()))?;
        Ok(())
    }

    fn inventory(&self, mode: &str) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin remaining --jobs {jobs} -- {mode}"
        )
        .run()
        .with_context(|| format!("remaining-tests inventory {mode} failed"))?;
        Ok(())
    }
}

fn render_plan_files(
    generated: &bsc_testsuite_manifest::GeneratedTestPlans,
) -> Result<BTreeMap<PathBuf, String>> {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("schema.json"),
        bsc_test_plan::render_schema().context("could not render Test Plan JSON Schema")?,
    );
    files.insert(
        PathBuf::from("index.json"),
        bsc_test_plan::render_index(&generated.index)
            .context("could not render Test Plan index")?,
    );
    for generated_plan in &generated.plans {
        files.insert(
            generated_plan.relative_path.clone(),
            bsc_test_plan::render_plan(&generated_plan.plan).with_context(|| {
                format!("could not render Test Plan {}", generated_plan.plan.id)
            })?,
        );
    }
    Ok(files)
}

fn write_plan_files(root: &Path, files: &BTreeMap<PathBuf, String>) -> Result<()> {
    for (relative, contents) in files {
        let path = root.join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Test Plan path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
        fs::write(&path, contents)
            .with_context(|| format!("could not write {}", path.display()))?;
    }
    Ok(())
}

fn collect_relative_files(root: &Path) -> Result<BTreeSet<PathBuf>> {
    if !root.exists() {
        return Ok(BTreeSet::new());
    }
    let mut files = BTreeSet::new();
    collect_relative_files_from(root, root, &mut files)?;
    Ok(files)
}

fn collect_relative_files_from(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("could not read {}", directory.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_relative_files_from(root, &entry.path(), files)?;
        } else if file_type.is_file() {
            files.insert(
                entry
                    .path()
                    .strip_prefix(root)
                    .expect("walked path is below root")
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn rename_with_retry(source: &Path, destination: &Path) -> io::Result<()> {
    const ATTEMPTS: u64 = 10;
    for attempt in 0..ATTEMPTS {
        match fs::rename(source, destination) {
            Ok(()) => return Ok(()),
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied && attempt + 1 < ATTEMPTS =>
            {
                thread::sleep(Duration::from_millis(25 * (attempt + 1)));
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the rename retry loop always returns")
}

fn remove_directory_if_present(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("could not remove {}", path.display()))?;
    }
    Ok(())
}

fn audit_testsuite_files(
    testsuite: &Path,
    counts: &mut BTreeMap<&'static str, usize>,
) -> Result<()> {
    audit_testsuite_directory(testsuite, testsuite, counts)
}

fn audit_testsuite_directory(
    testsuite: &Path,
    directory: &Path,
    counts: &mut BTreeMap<&'static str, usize>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("could not audit {}", directory.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            audit_testsuite_directory(testsuite, &path, counts)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(testsuite)
            .expect("audited path is below testsuite")
            .to_string_lossy()
            .replace('\\', "/");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let category = if name == "Makefile" {
            Some("Makefile")
        } else {
            match path.extension().and_then(|extension| extension.to_str()) {
                Some("exp")
                    if matches!(
                        relative.as_str(),
                        "site.exp" | "config/unix.exp" | "lib/bsc.exp"
                    ) =>
                {
                    Some("infrastructure .exp")
                }
                Some("exp") => Some("contract .exp"),
                Some("tcl") => Some(".tcl"),
                Some("cmd") => Some(".cmd"),
                Some("pl") => Some(".pl"),
                Some("sh") => Some(".sh"),
                _ => None,
            }
        };
        if let Some(category) = category {
            *counts.entry(category).or_default() += 1;
        }
    }
    Ok(())
}

fn print_plan_summary(label: &str, summary: bsc_testsuite_manifest::PlanSummary) {
    println!(
        "{label}: {} plans ({} complete, {} disabled, {} blocked), {} scenarios, {} stages, {} operations, {} diagnostics",
        summary.plans,
        summary.complete,
        summary.disabled,
        summary.blocked,
        summary.scenarios,
        summary.stages,
        summary.operations,
        summary.diagnostics,
    );
}
