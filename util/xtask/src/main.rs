mod environment;
mod tasks;

use anyhow::Result;
use clap::{Parser, Subcommand};
use environment::PreparedEnvironment;
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
    /// Run the migrated scheduler SAT tests against Z3.
    TestZ3,
    /// Verify Rust declarations against their upstream Tcl origins.
    TestAlignment,
    /// Verify that the remaining-tests inventory is current.
    InventoryCheck,
    /// Regenerate the remaining-tests inventory.
    InventoryUpdate,
    /// Run migrated upstream contracts, optionally restricted by filters.
    TestUpstream {
        #[arg(
            value_name = "ARG",
            allow_hyphen_values = true,
            trailing_var_arg = true
        )]
        arguments: Vec<String>,
    },
    /// Run Rust unit, scheduler, and migrated upstream tests.
    TestRust,
    /// Run the complete default test suite.
    Test,
    /// Run the complete suite with BSC and C++ caches disabled.
    TestCold,
    /// Show Bluesim C++ compiler-cache statistics.
    CcacheStats,
    /// Clear the Bluesim C++ compiler cache.
    CcacheClear,
}

impl Task {
    fn requires_oss(&self) -> bool {
        matches!(
            self,
            Self::TestUpstream { .. } | Self::TestRust | Self::Test | Self::TestCold
        )
    }
}

fn run() -> Result<()> {
    let task = Cli::parse().task;
    let environment = PreparedEnvironment::prepare(task.requires_oss())?;
    let shell = Shell::new()?;
    shell.change_dir(&environment.root);
    let tasks = Tasks::new(&shell, &environment);

    match task {
        Task::TestZ3 => tasks.test_z3(),
        Task::TestAlignment => tasks.test_alignment(),
        Task::InventoryCheck => tasks.inventory_check(),
        Task::InventoryUpdate => tasks.inventory_update(),
        Task::TestUpstream { arguments } => tasks.test_upstream(&arguments),
        Task::TestRust | Task::Test => tasks.test_rust(),
        Task::TestCold => tasks.test_cold(),
        Task::CcacheStats => tasks.ccache_stats(),
        Task::CcacheClear => tasks.ccache_clear(),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error:#}");
        std::process::exit(1);
    }
}
