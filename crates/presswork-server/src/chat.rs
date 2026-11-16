use jiff_sqlx::Timestamp as SqlTimestamp;
use presswork_api::live::ChatMessage;
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) const RECENT: i64 = 100;
const MAX_TEXT_CHARACTERS: usize = 4000;

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("a message must have 1 to 4000 characters")]
    InvalidText,
    #[error("the database failed: {0}")]
    Database(#[from] sqlx::Error),
}

type MessageRow = (Uuid, Uuid, Option<Uuid>, String, String, SqlTimestamp);

fn message((id, organization, author, author_name, text, sent_at): MessageRow) -> ChatMessage {
    ChatMessage {
        id,
        organization,
        author: author.unwrap_or_default(),
        author_name,
        text,
        sent_at: sent_at.to_jiff(),
    }
}

pub(crate) fn parse_text(text: &str) -> Result<&str, ChatError> {
    let text = text.trim();
    if (1..=MAX_TEXT_CHARACTERS).contains(&text.chars().count()) {
        Ok(text)
    } else {
        Err(ChatError::InvalidText)
    }
}

pub async fn post(
    pool: &PgPool,
    organization_id: Uuid,
    author_id: Uuid,
    text: &str,
) -> Result<ChatMessage, ChatError> {
    let text = parse_text(text)?;
    let row: MessageRow = sqlx::query_as(
        "WITH inserted AS (\
             INSERT INTO chat_messages (id, organization_id, author_id, text) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, organization_id, author_id, text, sent_at) \
         SELECT inserted.id, inserted.organization_id, inserted.author_id, \
                COALESCE(users.name, ''), inserted.text, inserted.sent_at \
         FROM inserted LEFT JOIN users ON users.id = inserted.author_id",
    )
    .bind(Uuid::now_v7())
    .bind(organization_id)
    .bind(author_id)
    .bind(text)
    .fetch_one(pool)
    .await?;
    Ok(message(row))
}

pub async fn recent(
    pool: &PgPool,
    organization_id: Uuid,
    limit: i64,
) -> Result<Vec<ChatMessage>, ChatError> {
    let rows: Vec<MessageRow> = sqlx::query_as(
        "SELECT id, organization_id, author_id, name, text, sent_at FROM (\
             SELECT chat_messages.id, chat_messages.organization_id, chat_messages.author_id, \
                    COALESCE(users.name, '') AS name, chat_messages.text, chat_messages.sent_at \
             FROM chat_messages LEFT JOIN users ON users.id = chat_messages.author_id \
             WHERE chat_messages.organization_id = $1 \
             ORDER BY chat_messages.sent_at DESC, chat_messages.id DESC LIMIT $2) AS recent \
         ORDER BY sent_at, id",
    )
    .bind(organization_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(message).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::create_user;
    use crate::links::create_organization;
    use crate::testing::test_database;

    #[test]
    fn a_message_has_1_to_4000_characters() {
        assert_eq!(parse_text("  Ready.  ").unwrap(), "Ready.");
        assert!(matches!(parse_text("   "), Err(ChatError::InvalidText)));
        assert!(matches!(
            parse_text(&"x".repeat(4001)),
            Err(ChatError::InvalidText)
        ));
        assert_eq!(parse_text(&"ü".repeat(4000)).unwrap().chars().count(), 4000);
    }

    #[tokio::test]
    async fn a_posted_message_has_its_author_and_its_time() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let ann = create_user(pool, "ann@example.com", "Ann", "correct horse battery")
            .await
            .unwrap();

        let message = post(pool, press.id, ann.id, " The proofs are ready. ")
            .await
            .unwrap();

        assert_eq!(message.organization, press.id);
        assert_eq!(message.author, ann.id);
        assert_eq!(message.author_name, "Ann");
        assert_eq!(message.text, "The proofs are ready.");
        assert_eq!(recent(pool, press.id, 100).await.unwrap(), [message]);
    }

    #[tokio::test]
    async fn the_recent_messages_are_the_last_ones_in_their_sequence() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let press = create_organization(pool, "Field Press").await.unwrap();
        let guild = create_organization(pool, "Binders Guild").await.unwrap();
        let ann = create_user(pool, "ann@example.com", "Ann", "correct horse battery")
            .await
            .unwrap();
        for number in 1..=5 {
            post(pool, press.id, ann.id, &format!("Message {number}"))
                .await
                .unwrap();
        }
        post(pool, guild.id, ann.id, "Elsewhere").await.unwrap();

        let texts: Vec<String> = recent(pool, press.id, 3)
            .await
            .unwrap()
            .into_iter()
            .map(|message| message.text)
            .collect();

        assert_eq!(texts, ["Message 3", "Message 4", "Message 5"]);
    }
}
