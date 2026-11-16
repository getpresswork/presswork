use clap::{Parser, Subcommand};
use presswork_api::Role;
use sqlx::PgPool;
use uuid::Uuid;

use crate::links::{self, LinkError};

#[derive(Debug, Parser)]
#[command(
    name = "presswork-server",
    version,
    about = "The Presswork collaboration server."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    #[must_use]
    pub fn command(&self) -> Command {
        self.command.clone().unwrap_or(Command::Serve)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    #[command(about = "Run the server. This is the default command.")]
    Serve,
    #[command(about = "Make an organization and an invite link for its first administrator.")]
    CreateOrganization {
        #[arg(long, help = "The name of the organization.")]
        name: String,
        #[arg(long, help = "The email address of the first administrator.")]
        administrator: String,
    },
    #[command(about = "Make an invite link for a person.")]
    Invite {
        #[arg(long, help = "The ID of the organization.")]
        organization: Uuid,
        #[arg(long, help = "The email address of the person.")]
        email: String,
        #[arg(
            long,
            default_value = "member",
            value_parser = parse_role,
            help = "The role in the organization: administrator or member."
        )]
        role: Role,
    },
    #[command(about = "Make a password reset link for a user.")]
    ResetPassword {
        #[arg(long, help = "The email address of the user.")]
        email: String,
    },
}

fn parse_role(text: &str) -> Result<Role, String> {
    Role::parse(text).ok_or_else(|| format!("{text} is not a role. Use administrator or member."))
}

pub async fn execute(pool: &PgPool, command: Command) -> Result<String, LinkError> {
    match command {
        Command::Serve => Ok(String::new()),
        Command::CreateOrganization {
            name,
            administrator,
        } => {
            let organization = links::create_organization(pool, &name).await?;
            let (token, invite) =
                links::invite(pool, organization.id, &administrator, Role::Administrator).await?;
            Ok(format!(
                "Organization: {} ({})\nInvite link for {} (administrator), valid until {}:\n{token}\n",
                organization.name, organization.id, invite.email, invite.expires_at
            ))
        }
        Command::Invite {
            organization,
            email,
            role,
        } => {
            let (token, invite) = links::invite(pool, organization, &email, role).await?;
            Ok(format!(
                "Invite link for {} ({}), valid until {}:\n{token}\n",
                invite.email,
                role.as_str(),
                invite.expires_at
            ))
        }
        Command::ResetPassword { email } => {
            let token = links::reset_link(pool, &email).await?;
            Ok(format!(
                "Password reset link for {email}, valid for 24 hours:\n{token}\n"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::links::accept_invite;
    use crate::testing::test_database;

    fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("presswork-server").chain(arguments.iter().copied()))
    }

    #[test]
    fn no_command_means_serve() {
        assert_eq!(parse(&[]).unwrap().command(), Command::Serve);
        assert_eq!(parse(&["serve"]).unwrap().command(), Command::Serve);
    }

    #[test]
    fn invite_reads_the_organization_the_email_and_the_role() {
        let organization = Uuid::now_v7();

        let cli = parse(&[
            "invite",
            "--organization",
            &organization.to_string(),
            "--email",
            "bob@example.com",
            "--role",
            "administrator",
        ])
        .unwrap();

        assert_eq!(
            cli.command(),
            Command::Invite {
                organization,
                email: "bob@example.com".to_owned(),
                role: Role::Administrator,
            }
        );
    }

    #[test]
    fn the_role_of_an_invite_is_member_by_default() {
        let cli = parse(&[
            "invite",
            "--organization",
            &Uuid::now_v7().to_string(),
            "--email",
            "bob@example.com",
        ])
        .unwrap();

        assert!(matches!(
            cli.command(),
            Command::Invite {
                role: Role::Member,
                ..
            }
        ));
    }

    #[test]
    fn an_unknown_role_is_an_error() {
        let result = parse(&[
            "invite",
            "--organization",
            &Uuid::now_v7().to_string(),
            "--email",
            "bob@example.com",
            "--role",
            "owner",
        ]);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_organization_prints_an_invite_for_the_first_administrator() {
        let Some(database) = test_database().await else {
            return;
        };

        let output = execute(
            &database.pool,
            Command::CreateOrganization {
                name: "Field Press".to_owned(),
                administrator: "ann@example.com".to_owned(),
            },
        )
        .await
        .unwrap();

        assert!(output.contains("Field Press"), "{output}");
        let token = output
            .split_whitespace()
            .find(|word| word.starts_with("pwi_"))
            .unwrap();
        let accepted = accept_invite(
            &database.pool,
            token,
            "Ann",
            "correct horse battery",
            "Laptop",
        )
        .await
        .unwrap();
        assert_eq!(accepted.role, Role::Administrator);
    }

    #[tokio::test]
    async fn reset_password_prints_a_reset_link() {
        let Some(database) = test_database().await else {
            return;
        };
        crate::accounts::create_user(
            &database.pool,
            "ann@example.com",
            "Ann",
            "correct horse battery",
        )
        .await
        .unwrap();

        let output = execute(
            &database.pool,
            Command::ResetPassword {
                email: "ann@example.com".to_owned(),
            },
        )
        .await
        .unwrap();

        assert!(
            output
                .split_whitespace()
                .any(|word| word.starts_with("pwr_")),
            "{output}"
        );
    }
}
