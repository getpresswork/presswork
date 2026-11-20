use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "presswork-cli",
    version,
    about = "Build and check Presswork projects without the desktop application."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum Command {
    #[command(about = "Compile the project and write the files of its export profiles.")]
    Build(Target),
    #[command(about = "Compile the project and find preflight problems. Write no files.")]
    Check(Target),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct Target {
    #[arg(
        long = "profile",
        value_name = "NAME",
        help = "Use only this export profile. Give the option again for more profiles."
    )]
    pub(crate) profiles: Vec<String>,
    #[arg(
        value_name = "FOLDER",
        default_value = ".",
        help = "The project folder."
    )]
    pub(crate) folder: PathBuf,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory as _;

    use super::*;

    fn parse(arguments: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("presswork-cli").chain(arguments.iter().copied()))
            .unwrap()
    }

    #[test]
    fn the_commands_are_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn a_command_uses_the_current_folder_and_all_profiles_by_default() {
        assert_eq!(
            parse(&["build"]).command,
            Command::Build(Target {
                profiles: Vec::new(),
                folder: PathBuf::from("."),
            })
        );
    }

    #[test]
    fn a_command_takes_profiles_and_a_folder() {
        assert_eq!(
            parse(&[
                "check",
                "--profile",
                "print",
                "--profile",
                "screen",
                "books/notes"
            ])
            .command,
            Command::Check(Target {
                profiles: vec!["print".to_owned(), "screen".to_owned()],
                folder: PathBuf::from("books/notes"),
            })
        );
    }
}
