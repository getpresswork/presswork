use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use crate::platform::Platform;

#[derive(Debug, Clone)]
pub(crate) struct Route {
    path: String,
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Route {
    pub(crate) fn new(path: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.to_owned(),
            status: 200,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    pub(crate) fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub(crate) fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

pub(crate) fn serve(routes: impl FnOnce(&str) -> Vec<Route>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let routes = routes(&base);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            reader.read_line(&mut request).unwrap();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let path = request.split_whitespace().nth(1).unwrap_or("/");
            let route = routes
                .iter()
                .find(|route| route.path == path)
                .cloned()
                .unwrap_or_else(|| Route::new(path, "missing").status(404));
            let mut head = format!(
                "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n",
                route.status,
                route.body.len()
            );
            for (name, value) in &route.headers {
                let _ = write!(head, "{name}: {value}\r\n");
            }
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&route.body);
        }
    });
    base
}

pub(crate) fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

pub(crate) fn platform_archive(folder: &Path) -> PathBuf {
    let platform = Platform::current();
    let program = format!("typst-x/{}", platform.executable());
    if Path::new(&platform.release_file().unwrap())
        .extension()
        .is_some_and(|extension| extension == "zip")
    {
        zip_file(folder, &[(program.as_str(), b"binary")])
    } else {
        tar_xz_file(
            folder,
            &[(program.as_str(), tar::EntryType::Regular, b"binary")],
        )
    }
}

pub(crate) fn zip_file(folder: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = folder.join("typst.zip");
    let mut writer = zip::ZipWriter::new(File::create(&path).unwrap());
    for (name, data) in entries {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default().unix_permissions(0o755),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
    path
}

fn tar_bytes(entries: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, kind, data) in entries {
        let mut header = tar::Header::new_gnu();
        header.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
        header.set_entry_type(*kind);
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append(&header, *data).unwrap();
    }
    builder.into_inner().unwrap()
}

pub(crate) fn tar_xz_file(folder: &Path, entries: &[(&str, tar::EntryType, &[u8])]) -> PathBuf {
    let path = folder.join("typst.tar.xz");
    let mut writer = lzma_rust2::XzWriter::new(
        File::create(&path).unwrap(),
        lzma_rust2::XzOptions::with_preset(1),
    )
    .unwrap();
    writer.write_all(&tar_bytes(entries)).unwrap();
    writer.finish().unwrap();
    path
}

pub(crate) fn tar_gz_bytes(entries: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&tar_bytes(entries)).unwrap();
    encoder.finish().unwrap()
}

pub(crate) fn tar_gz_file(
    folder: &Path,
    name: &str,
    entries: &[(&str, tar::EntryType, &[u8])],
) -> PathBuf {
    let path = folder.join(name);
    std::fs::write(&path, tar_gz_bytes(entries)).unwrap();
    path
}
