use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use std::io;

mod config;
mod init;
mod repl;
mod api;
mod test_dropdown;

use config::Config;
use init::run_init;
use repl::ShyRepl;

#[derive(Parser)]
#[command(name = "shy")]
#[command(about = "AI-powered shell assistant")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration (API key and model selection)
    Init,
    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Test dropdown completion behavior
    Test,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            run_init()?;
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            eprintln!("Generating completion file for {shell}...");
            print_completions(shell, &mut cmd);
        }
        Some(Commands::Test) => {
            test_dropdown::test_dropdown_behavior().await?;
        }
        None => {
            // No subcommand means start REPL
            if !Config::exists() {
                println!("Welcome to Shy! Let's set up your configuration first.");
                run_init()?;
            }

            let config = Config::load()?;
            let mut repl = ShyRepl::new(config)?;
            repl.run().await?;
        }
    }

    Ok(())
}