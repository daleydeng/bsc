use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use xshell::{cmd, Shell};

use crate::environment::{save_oss_root, PreparedEnvironment};
use crate::{msys, test_temp, toolchain};

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
        msys::build(self.environment)
    }

    pub fn smoke(&self) -> Result<()> {
        msys::smoke(self.environment)
    }

    pub fn clean(&self) -> Result<()> {
        msys::clean(self.environment)
    }

    pub fn shell(&self) -> Result<()> {
        msys::shell(self.environment)
    }

    pub fn test_z3(&self) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} test --locked --package bsc-rust-tests --test scheduler_sat --jobs {jobs} -- --test-threads {jobs}"
        )
        .run()
        .context("scheduler SAT tests failed")?;
        Ok(())
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
            "contract IR: {} scripts, {} compile contracts, {} simulation contracts, {} external contracts in {} sets ({} unresolved), {} assertions, {} comparisons, {} unsupported constructs in {} scripts",
            summary.scripts,
            summary.compile_contracts,
            summary.simulation_contracts,
            summary.external_contracts,
            summary.external_contract_sets,
            summary.unresolved_contracts,
            summary.assertions,
            summary.comparisons,
            summary.unsupported_constructs,
            summary.scripts_with_unsupported,
        );
        Ok(())
    }

    pub fn contracts_check(&self) -> Result<()> {
        let path = self
            .environment
            .root
            .join("rust-tests/contracts/upstream-contracts.json");
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
            "contract manifest ok: {} scripts, {} compile + {} simulation + {} external contracts ({} unresolved), {} unsupported constructs",
            summary.scripts,
            summary.compile_contracts,
            summary.simulation_contracts,
            summary.external_contracts,
            summary.unresolved_contracts,
            summary.unsupported_constructs,
        );
        Ok(())
    }

    pub fn contracts_update(&self) -> Result<()> {
        let path = self
            .environment
            .root
            .join("rust-tests/contracts/upstream-contracts.json");
        let manifest = bsc_testsuite_manifest::build_manifest(&self.environment.root)
            .context("could not lower upstream Tcl testsuite into contract IR")?;
        let rendered = bsc_testsuite_manifest::render_manifest(&manifest)
            .context("could not render typed contract manifest")?;
        fs::write(&path, rendered)
            .with_context(|| format!("could not write {}", path.display()))?;
        let summary = manifest.summary();
        println!(
            "updated {}: {} scripts, {} compile + {} simulation + {} external contracts ({} unresolved), {} unsupported constructs",
            path.display(),
            summary.scripts,
            summary.compile_contracts,
            summary.simulation_contracts,
            summary.external_contracts,
            summary.unresolved_contracts,
            summary.unsupported_constructs,
        );
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

    pub fn test_alignment(&self) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin alignment --jobs {jobs}"
        )
        .run()
        .context("upstream alignment check failed")?;
        Ok(())
    }

    pub fn inventory_check(&self) -> Result<()> {
        self.inventory("--check")
    }

    pub fn inventory_update(&self) -> Result<()> {
        self.inventory("--write")
    }

    pub fn test_upstream(&self, arguments: &[String]) -> Result<()> {
        self.test_alignment()?;
        self.inventory_check()?;

        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} run --locked --package bsc-rust-tests --bin upstream --jobs {jobs} -- {arguments...} --test-threads {jobs}"
        )
        .run()
        .context("migrated upstream tests failed")?;
        Ok(())
    }

    pub fn test_rust(&self) -> Result<()> {
        let sh = self.shell;
        let cargo = &self.cargo;
        let jobs = self.environment.jobs.to_string();
        cmd!(
            sh,
            "{cargo} test --locked --package bsc-rust-tests --lib --test scheduler_sat --jobs {jobs} -- --test-threads {jobs}"
        )
        .run()
        .context("Rust unit or scheduler tests failed")?;
        self.test_upstream(&[])
    }

    pub fn test_cold(&self) -> Result<()> {
        env::set_var("BSC_TEST_CACHE", "0");
        env::set_var("CCACHE_DISABLE", "1");
        if self.environment.ccache_managed_cxx {
            env::set_var("CXX", "c++");
        }
        self.test_rust()
    }

    pub fn test_prune(&self) -> Result<()> {
        test_temp::prune(self.environment)
    }

    pub fn ccache_stats(&self) -> Result<()> {
        let sh = self.shell;
        cmd!(sh, "ccache.exe --show-stats")
            .run()
            .context("could not read ccache statistics")?;
        Ok(())
    }

    pub fn ccache_clear(&self) -> Result<()> {
        let sh = self.shell;
        cmd!(sh, "ccache.exe --clear")
            .run()
            .context("could not clear ccache")?;
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
