use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use presswork_api::live::{self, ClientMessage, DecodeError, EncodeError, PROTOCOL, ServerMessage};
use rustls::ClientConfig;
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
};
use uuid::Uuid;

const LIVE_PATH: &str = "/api/v1/live";
const CONNECT_TIME: Duration = Duration::from_secs(15);
const WELCOME_TIME: Duration = Duration::from_secs(15);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("the server address must start with http:// or https://")]
    Address,
    #[error("the connection to the server failed")]
    Connection(#[from] tungstenite::Error),
    #[error("TLS cannot start")]
    Tls(#[from] rustls::Error),
    #[error("the server did not answer in time")]
    TimedOut,
    #[error("the server refused the connection: {message}")]
    Refused { code: String, message: String },
    #[error("the server sent an unexpected message")]
    Unexpected,
    #[error("the connection closed")]
    Closed,
    #[error("the message cannot be encoded")]
    Encode(#[from] EncodeError),
    #[error("the server sent a message that is not valid")]
    Decode(#[from] DecodeError),
}

pub struct Live {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    user: Uuid,
}

impl Live {
    pub async fn connect(server: &str, token: &str) -> Result<Self, ClientError> {
        let url = live_url(server)?;
        let connector = Connector::Rustls(Arc::new(tls()?));
        let (socket, _) = timeout(
            CONNECT_TIME,
            connect_async_tls_with_config(url, None, true, Some(connector)),
        )
        .await
        .map_err(|_| ClientError::TimedOut)??;
        let mut live = Self {
            socket,
            user: Uuid::nil(),
        };
        live.send(&ClientMessage::Hello {
            protocol: PROTOCOL,
            token: token.to_owned(),
        })
        .await?;
        let answer = timeout(WELCOME_TIME, live.receive())
            .await
            .map_err(|_| ClientError::TimedOut)??;
        match answer {
            Some(ServerMessage::Welcome { user }) => {
                live.user = user;
                Ok(live)
            }
            Some(ServerMessage::Error { code, message }) => {
                Err(ClientError::Refused { code, message })
            }
            Some(_) => Err(ClientError::Unexpected),
            None => Err(ClientError::Closed),
        }
    }

    #[must_use]
    pub fn user(&self) -> Uuid {
        self.user
    }

    pub async fn send(&mut self, message: &ClientMessage) -> Result<(), ClientError> {
        let bytes = live::encode(message)?;
        self.socket.send(Message::Binary(bytes.into())).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<ServerMessage>, ClientError> {
        while let Some(message) = self.socket.next().await {
            match message? {
                Message::Binary(bytes) => return Ok(Some(live::decode(&bytes)?)),
                Message::Close(_) => return Ok(None),
                Message::Text(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            }
        }
        Ok(None)
    }
}

impl fmt::Debug for Live {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Live")
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}

pub fn live_url(server: &str) -> Result<String, ClientError> {
    let server = server.trim_end_matches('/');
    let (scheme, rest) = if let Some(rest) = server.strip_prefix("https://") {
        ("wss", rest)
    } else if let Some(rest) = server.strip_prefix("http://") {
        ("ws", rest)
    } else {
        return Err(ClientError::Address);
    };
    if rest.is_empty() {
        return Err(ClientError::Address);
    }
    Ok(format!("{scheme}://{rest}{LIVE_PATH}"))
}

fn tls() -> Result<ClientConfig, ClientError> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    Ok(ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_platform_verifier()?
        .with_no_client_auth())
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use futures::{SinkExt, StreamExt};
    use presswork_api::live::{self, ClientMessage, PROTOCOL, ServerMessage};
    use tokio::net::{TcpListener, TcpStream};
    use tokio_tungstenite::WebSocketStream;
    use tokio_tungstenite::tungstenite::Message;
    use uuid::Uuid;

    use super::*;

    type Socket = WebSocketStream<TcpStream>;

    const ANN: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_00a1);
    const DOCUMENT: Uuid = Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_00d1);

    async fn server<F, Fut>(peer: F) -> String
    where
        F: FnOnce(Socket) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            peer(socket).await;
        });
        format!("http://{address}")
    }

    async fn read(socket: &mut Socket) -> ClientMessage {
        loop {
            if let Message::Binary(bytes) = socket.next().await.unwrap().unwrap() {
                return live::decode(&bytes).unwrap();
            }
        }
    }

    async fn write(socket: &mut Socket, message: &ServerMessage) {
        let bytes = live::encode(message).unwrap();
        socket.send(Message::Binary(bytes.into())).await.unwrap();
    }

    async fn welcome(socket: &mut Socket) {
        let hello = read(socket).await;
        assert_eq!(
            hello,
            ClientMessage::Hello {
                protocol: PROTOCOL,
                token: "pws_ann".to_owned(),
            }
        );
        write(socket, &ServerMessage::Welcome { user: ANN }).await;
    }

    #[test]
    fn the_live_address_comes_from_the_server_address() {
        for (server, live) in [
            (
                "https://press.example.com",
                "wss://press.example.com/api/v1/live",
            ),
            (
                "https://example.com/presswork/",
                "wss://example.com/presswork/api/v1/live",
            ),
            ("http://127.0.0.1:8080", "ws://127.0.0.1:8080/api/v1/live"),
        ] {
            assert_eq!(live_url(server).unwrap(), live);
        }
        for server in ["press.example.com", "ftp://press.example.com", ""] {
            assert!(
                matches!(live_url(server), Err(ClientError::Address)),
                "{server}"
            );
        }
    }

    #[tokio::test]
    async fn connect_says_hello_with_the_token_and_learns_the_user() {
        let address = server(|mut socket| async move { welcome(&mut socket).await }).await;

        let live = Live::connect(&address, "pws_ann").await.unwrap();

        assert_eq!(live.user(), ANN);
    }

    #[tokio::test]
    async fn a_refused_hello_gives_the_reason() {
        let address = server(|mut socket| async move {
            read(&mut socket).await;
            write(
                &mut socket,
                &ServerMessage::Error {
                    code: "unauthenticated".to_owned(),
                    message: "Sign in to use this function.".to_owned(),
                },
            )
            .await;
        })
        .await;

        let refused = Live::connect(&address, "pws_ann").await;

        assert!(
            matches!(&refused, Err(ClientError::Refused { code, .. }) if code == "unauthenticated"),
            "{refused:?}"
        );
    }

    #[tokio::test]
    async fn messages_go_in_both_directions() {
        let address = server(|mut socket| async move {
            welcome(&mut socket).await;
            let ClientMessage::Subscribe { document, version } = read(&mut socket).await else {
                panic!("the client did not subscribe");
            };
            write(
                &mut socket,
                &ServerMessage::Subscribed {
                    document,
                    version: version.clone(),
                    data: vec![7; 3],
                },
            )
            .await;
        })
        .await;
        let mut live = Live::connect(&address, "pws_ann").await.unwrap();

        live.send(&ClientMessage::Subscribe {
            document: DOCUMENT,
            version: vec![1, 2],
        })
        .await
        .unwrap();

        assert_eq!(
            live.receive().await.unwrap(),
            Some(ServerMessage::Subscribed {
                document: DOCUMENT,
                version: vec![1, 2],
                data: vec![7; 3],
            })
        );
    }

    #[tokio::test]
    async fn receive_gives_nothing_after_the_server_closes() {
        let address = server(|mut socket| async move {
            welcome(&mut socket).await;
            socket.close(None).await.unwrap();
        })
        .await;
        let mut live = Live::connect(&address, "pws_ann").await.unwrap();

        assert_eq!(live.receive().await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_server_that_is_not_there_gives_a_connection_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);

        let failed = Live::connect(&address, "pws_ann").await;

        assert!(
            matches!(failed, Err(ClientError::Connection(_))),
            "{failed:?}"
        );
    }
}
