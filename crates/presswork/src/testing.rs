use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use iced::Task;
use iced::futures::{Stream, StreamExt};
use iced::keyboard::{self, key};

pub(crate) fn key_press(named: key::Named) -> iced::Event {
    let pressed = keyboard::Key::Named(named);
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: pressed.clone(),
        modified_key: pressed,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct Route {
    request: String,
    status: u16,
    body: Vec<u8>,
}

impl Route {
    pub(crate) fn new(request: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            request: request.to_owned(),
            status: 200,
            body: body.into(),
        }
    }

    pub(crate) fn json(request: &str, value: &serde_json::Value) -> Self {
        Self::new(request, value.to_string())
    }

    pub(crate) fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
}

pub(crate) fn serve(routes: Vec<Route>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            let mut length = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let mut parts = request.split_whitespace();
            let method = parts.next().unwrap_or("GET");
            let target = parts.next().unwrap_or("/");
            let path = target.split('?').next().unwrap_or(target);
            let route = routes
                .iter()
                .find(|route| route.request == format!("{method} {target}"))
                .or_else(|| {
                    routes
                        .iter()
                        .find(|route| route.request == format!("{method} {path}"))
                })
                .cloned()
                .unwrap_or_else(|| Route::new(target, "missing").status(404));
            let head = format!(
                "HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                route.status,
                route.body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&route.body);
        }
    });
    base
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

pub(crate) fn collect<T>(stream: impl Stream<Item = T>) -> Vec<T> {
    runtime().block_on(stream.collect())
}

pub(crate) fn run_task<T: Send + 'static>(task: Task<T>) -> Vec<T> {
    let Some(stream) = iced_runtime::task::into_stream(task) else {
        return Vec::new();
    };
    collect(stream.filter_map(|action| async move {
        match action {
            iced_runtime::Action::Output(output) => Some(output),
            _ => None,
        }
    }))
}
