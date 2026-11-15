use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Health {
    pub status: Status,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_has_a_lowercase_status_and_the_version() {
        let health = Health {
            status: Status::Unavailable,
            version: "0.1.0".to_owned(),
        };

        assert_eq!(
            serde_json::to_value(&health).unwrap(),
            serde_json::json!({ "status": "unavailable", "version": "0.1.0" })
        );
    }
}
