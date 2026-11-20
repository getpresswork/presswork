mod cli;
mod i18n;
mod location;
mod run;

use std::io;
use std::process::ExitCode;

use clap::Parser as _;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    ExitCode::from(run::run(
        &cli,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}
