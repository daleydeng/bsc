use bsc_rust_tests::alignment::check_alignment;
use std::process::ExitCode;

fn main() -> ExitCode {
    match check_alignment() {
        Ok(summary) => {
            println!(
                "alignment ok: {} scripts, {} compile cases, {} simulation cases, {} scheduler cases",
                summary.scripts,
                summary.compile_cases,
                summary.simulation_cases,
                summary.scheduler_cases
            );
            println!(
                "migration coverage: {}/{} test scripts migrated, {} remaining",
                summary.migrated_test_scripts,
                summary.total_test_scripts,
                summary.remaining_test_scripts
            );
            println!(
                "contract coverage: {}/{} statically declared contracts migrated, {} remaining",
                summary.migrated_contracts,
                summary.total_statically_declared_contracts,
                summary.remaining_statically_declared_contracts
            );
            println!(
                "contract inventory: {} scripts require dynamic or custom Tcl analysis",
                summary.unclassified_test_scripts
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("alignment FAILED:\n{error}");
            ExitCode::from(1)
        }
    }
}
