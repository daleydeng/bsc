use std::path::PathBuf;
use std::process::ExitCode;

use bluesim::{Engine, Model};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(about = "Run versioned Bluesim SimIR models")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a model until it calls $finish.
    Run {
        model: PathBuf,
        #[arg(long, default_value_t = 10_000_000)]
        max_cycles: u64,
    },
    /// Advance a model for a fixed number of clock cycles.
    Step {
        model: PathBuf,
        #[arg(long)]
        cycles: u64,
    },
    /// Validate a model without executing it.
    Inspect { model: PathBuf },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("bluesim: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let (model_path, mode) = match cli.command {
        Command::Run { model, max_cycles } => (model, Mode::Run { max_cycles }),
        Command::Step { model, cycles } => (model, Mode::Step { cycles }),
        Command::Inspect { model } => (model, Mode::Inspect),
    };
    let model = Model::read_json(&model_path)?;
    if matches!(mode, Mode::Inspect) {
        println!(
            "SimIR v{}: top {}, {} clocks, {} state cells, {} schedules",
            model.schema_version,
            model.top,
            model.clocks.len(),
            model.state.len(),
            model.schedules.len()
        );
        return Ok(0);
    }

    let mut engine = Engine::new(model)?;
    let result = match mode {
        Mode::Run { max_cycles } => engine.run(max_cycles)?,
        Mode::Step { cycles } => engine.step(cycles)?,
        Mode::Inspect => unreachable!(),
    };
    for line in result.output {
        println!("{line}");
    }
    Ok(result.exit_status.unwrap_or(0))
}

enum Mode {
    Run { max_cycles: u64 },
    Step { cycles: u64 },
    Inspect,
}
