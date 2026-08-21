mod environment;
mod msys;
mod tasks;
mod test_temp;
mod toolchain;
mod z3_bridge;

use anyhow::Result;
use clap::{Parser, Subcommand};
use environment::{EnvironmentRequirements, OssRequirement, PreparedEnvironment};
use std::path::PathBuf;
use tasks::Tasks;
use xshell::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "cargo xtask",
    bin_name = "cargo xtask",
    about = "Cross-platform development tasks for BSC",
    version
)]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    /// Configure an existing OSS CAD Suite installation.
    ConfigureOssCadSuite {
        /// OSS CAD Suite installation root.
        root: PathBuf,
    },
    /// Install and select the project-local GHC and Cabal toolchain.
    Toolchain,
    /// Install BSC's Haskell package dependencies.
    HaskellDeps,
    /// Report the exact tools and platform seen by the native build.
    Doctor,
    /// Build and install BSC into ./inst.
    Build,
    /// Run the upstream smoke test.
    Smoke,
    /// Remove upstream build and installation directories.
    Clean,
    /// Enter the Pixi-managed MSYS2 shell.
    Shell,
    /// Run the migrated scheduler SAT tests against Z3.
    TestZ3,
    /// Parse every upstream Tcl test script without executing Tcl.
    ContractsParseCheck,
    /// Lower every upstream Tcl test script into the typed contract IR.
    ContractsIrCheck,
    /// Verify that the committed typed contract manifest is current.
    ContractsCheck,
    /// Regenerate the committed typed contract manifest.
    ContractsUpdate,
    /// Verify that all committed per-origin Test Plans are current.
    PlansCheck,
    /// Regenerate all per-origin Test Plans and their JSON Schema/index.
    PlansUpdate,
    /// Audit Test Plan coverage and non-exp executable-input inventory.
    PlansAudit,
    /// Execute complete Test Plans with the canonical Rust runner.
    TestPlans {
        #[arg(
            value_name = "ARG",
            allow_hyphen_values = true,
            trailing_var_arg = true
        )]
        arguments: Vec<String>,
    },

    /// Print the Tree-sitter concrete syntax tree for one Tcl test script.
    ContractsCst {
        /// Path relative to the project root, or an absolute path.
        script: PathBuf,
    },

    /// Verify that the remaining-tests inventory is current.
    InventoryCheck,
    /// Regenerate the remaining-tests inventory.
    InventoryUpdate,

    /// Run Rust unit, scheduler, and migrated upstream tests.
    TestRust,
    /// Run the complete default test suite.
    Test,
    /// Run the complete suite with BSC and C++ caches disabled.
    TestCold,
    /// Remove disposable Rust test workspaces and diagnostics from previous runs.
    TestPrune,
    /// Show shared Rust and Bluesim C++ sccache statistics.
    SccacheStats,
    /// Clear the shared Rust and Bluesim C++ sccache.
    SccacheClear,
}

impl Task {
    fn environment_requirements(&self) -> EnvironmentRequirements {
        match self {
            Self::Doctor | Self::Smoke => EnvironmentRequirements::native(OssRequirement::Required),
            Self::Shell => EnvironmentRequirements::native(OssRequirement::Optional),
            Self::Toolchain | Self::HaskellDeps | Self::Build | Self::Clean => {
                EnvironmentRequirements::native(OssRequirement::None)
            }
            Self::TestZ3
            | Self::TestPlans { .. }
            | Self::TestRust
            | Self::Test
            | Self::TestCold => EnvironmentRequirements::basic(OssRequirement::Required),
            _ => EnvironmentRequirements::basic(OssRequirement::None),
        }
    }
}

fn run() -> Result<()> {
    let task = Cli::parse().task;
    let environment = PreparedEnvironment::prepare(task.environment_requirements())?;
    let shell = Shell::new()?;
    shell.change_dir(&environment.root);
    let tasks = Tasks::new(&shell, &environment);

    match task {
        Task::ConfigureOssCadSuite { root } => tasks.configure_oss_cad_suite(&root),
        Task::Toolchain => tasks.toolchain(),
        Task::HaskellDeps => tasks.haskell_deps(),
        Task::Doctor => tasks.doctor(),
        Task::Build => tasks.build(),
        Task::Smoke => tasks.smoke(),
        Task::Clean => tasks.clean(),
        Task::Shell => tasks.shell(),
        Task::TestZ3 => tasks.test_z3(),
        Task::ContractsParseCheck => tasks.contracts_parse_check(),
        Task::ContractsIrCheck => tasks.contracts_ir_check(),
        Task::ContractsCheck => tasks.contracts_check(),
        Task::ContractsUpdate => tasks.contracts_update(),
        Task::PlansCheck => tasks.plans_check(),
        Task::PlansUpdate => tasks.plans_update(),
        Task::PlansAudit => tasks.plans_audit(),
        Task::TestPlans { arguments } => tasks.test_plans(&arguments),
        Task::ContractsCst { script } => tasks.contracts_cst(&script),

        Task::InventoryCheck => tasks.inventory_check(),
        Task::InventoryUpdate => tasks.inventory_update(),

        Task::TestRust | Task::Test => tasks.test_rust(),
        Task::TestCold => tasks.test_cold(),
        Task::TestPrune => tasks.test_prune(),
        Task::SccacheStats => tasks.sccache_stats(),
        Task::SccacheClear => tasks.sccache_clear(),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error:#}");
        std::process::exit(1);
    }
}
