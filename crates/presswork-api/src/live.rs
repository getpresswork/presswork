use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROTOCOL: u32 = 1;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol: u32,
        token: String,
    },
    Subscribe {
        document: Uuid,
        #[serde(with = "serde_bytes")]
        version: Vec<u8>,
    },
    Updates {
        document: Uuid,
        id: u64,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        user: Uuid,
    },
    Subscribed {
        document: Uuid,
        #[serde(with = "serde_bytes")]
        version: Vec<u8>,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    Updates {
        document: Uuid,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    Ack {
        document: Uuid,
        id: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(message)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(bytes)
}

struct Bytes(usize);

impl fmt::Debug for Bytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} bytes", self.0)
    }
}

impl fmt::Debug for ClientMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hello { protocol, .. } => formatter
                .debug_struct("Hello")
                .field("protocol", protocol)
                .field("token", &"[removed]")
                .finish(),
            Self::Subscribe { document, version } => formatter
                .debug_struct("Subscribe")
                .field("document", document)
                .field("version", &Bytes(version.len()))
                .finish(),
            Self::Updates { document, id, data } => formatter
                .debug_struct("Updates")
                .field("document", document)
                .field("id", id)
                .field("data", &Bytes(data.len()))
                .finish(),
        }
    }
}

impl fmt::Debug for ServerMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Welcome { user } => formatter
                .debug_struct("Welcome")
                .field("user", user)
                .finish(),
            Self::Subscribed {
                document,
                version,
                data,
            } => formatter
                .debug_struct("Subscribed")
                .field("document", document)
                .field("version", &Bytes(version.len()))
                .field("data", &Bytes(data.len()))
                .finish(),
            Self::Updates { document, data } => formatter
                .debug_struct("Updates")
                .field("document", document)
                .field("data", &Bytes(data.len()))
                .finish(),
            Self::Ack { document, id } => formatter
                .debug_struct("Ack")
                .field("document", document)
                .field("id", id)
                .finish(),
            Self::Error { code, message } => formatter
                .debug_struct("Error")
                .field("code", code)
                .field("message", message)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use uuid::Uuid;

    use super::*;

    fn document() -> Uuid {
        Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0001)
    }

    #[test]
    fn client_messages_survive_a_round_trip() {
        for message in [
            ClientMessage::Hello {
                protocol: PROTOCOL,
                token: "pws_secret".to_owned(),
            },
            ClientMessage::Subscribe {
                document: document(),
                version: vec![1, 2, 3],
            },
            ClientMessage::Updates {
                document: document(),
                id: 7,
                data: vec![200; 40],
            },
        ] {
            let bytes = encode(&message).unwrap();

            assert_eq!(decode::<ClientMessage>(&bytes).unwrap(), message);
        }
    }

    #[test]
    fn server_messages_survive_a_round_trip() {
        for message in [
            ServerMessage::Welcome { user: document() },
            ServerMessage::Subscribed {
                document: document(),
                version: vec![9],
                data: vec![1, 2],
            },
            ServerMessage::Updates {
                document: document(),
                data: vec![3, 4],
            },
            ServerMessage::Ack {
                document: document(),
                id: 7,
            },
            ServerMessage::Error {
                code: "forbidden".to_owned(),
                message: "You do not have the permission for this function.".to_owned(),
            },
        ] {
            let bytes = encode(&message).unwrap();

            assert_eq!(decode::<ServerMessage>(&bytes).unwrap(), message);
        }
    }

    #[test]
    fn updates_are_binary_values() {
        let message = ServerMessage::Updates {
            document: document(),
            data: vec![200; 100],
        };

        let bytes = encode(&message).unwrap();

        assert!(bytes.len() < 150, "{} bytes", bytes.len());
    }

    #[test]
    fn fields_have_their_names() {
        let bytes = encode(&ClientMessage::Hello {
            protocol: PROTOCOL,
            token: "pws_secret".to_owned(),
        })
        .unwrap();

        assert!(bytes.windows(5).any(|window| window == b"token"));
    }

    #[test]
    fn a_newer_peer_can_add_fields() {
        #[derive(Serialize)]
        #[serde(rename_all = "snake_case")]
        enum Newer {
            Hello {
                protocol: u32,
                token: String,
                client: String,
            },
        }
        let bytes = rmp_serde::to_vec_named(&Newer::Hello {
            protocol: PROTOCOL,
            token: "pws_secret".to_owned(),
            client: "Presswork 0.2".to_owned(),
        })
        .unwrap();

        assert_eq!(
            decode::<ClientMessage>(&bytes).unwrap(),
            ClientMessage::Hello {
                protocol: PROTOCOL,
                token: "pws_secret".to_owned(),
            }
        );
    }

    #[test]
    fn the_debug_text_hides_the_token() {
        let hello = ClientMessage::Hello {
            protocol: PROTOCOL,
            token: "pws_secret".to_owned(),
        };

        let text = format!("{hello:?}");

        assert!(!text.contains("pws_secret"), "{text}");
        assert!(text.contains("[removed]"), "{text}");
    }

    #[test]
    fn bytes_that_are_not_a_message_give_an_error() {
        assert!(decode::<ClientMessage>(b"\x01\x02\x03").is_err());
    }
}
