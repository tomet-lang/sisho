//! `sisho serve`, started as a real process on a free port.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// A running server and the directory it serves from; both are cleaned up
/// when this is dropped.
struct Server {
    child: Child,
    dir: PathBuf,
    addr: String,
}

impl Server {
    fn start(name: &str, document: &str, extra: &[&str]) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sisho-app-serve-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("doc.tmt"), document).unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
            .args(["serve", "doc.tmt", "--port", "0"])
            .args(extra)
            .current_dir(&dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        // The first line says where it is listening, once it is.
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let addr = line
            .strip_prefix("Serving at http://")
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_else(|| panic!("not a ready line: {line:?}"))
            .to_owned();
        Self { child, dir, addr }
    }

    fn get(&self, path: &str) -> String {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.addr
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    fn write(&self, text: &str) {
        fs::write(self.dir.join("doc.tmt"), text).unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn it_says_where_it_listens_and_serves_the_document() {
    let server = Server::start("page", "=[ Hello Serve ]\n", &[]);
    assert!(server.addr.starts_with("127.0.0.1:"), "{}", server.addr);

    let response = server.get("/");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("Hello Serve"), "{response}");
}

#[test]
fn editing_the_file_and_reloading_shows_the_change() {
    let server = Server::start("reload", "=[ First ]\n", &[]);
    assert!(server.get("/").contains("First"));

    server.write("=[ Second ]\n");
    let response = server.get("/");
    assert!(response.contains("Second"), "{response}");
    assert!(!response.contains("First"), "{response}");
}

#[test]
fn a_document_that_breaks_is_a_500_with_the_reason_and_recovers_when_fixed() {
    let server = Server::start("breaks", "=[ Fine ]\n", &[]);
    assert!(server.get("/").starts_with("HTTP/1.1 200"));

    server.write("=[ unterminated\n");
    let broken = server.get("/");
    assert!(broken.starts_with("HTTP/1.1 500"), "{broken}");
    assert!(broken.contains("Could not render"), "{broken}");
    assert!(broken.contains("doc.tmt"), "{broken}");

    server.write("=[ Fixed ]\n");
    let fixed = server.get("/");
    assert!(fixed.starts_with("HTTP/1.1 200"), "{fixed}");
    assert!(fixed.contains("Fixed"), "{fixed}");
}

#[test]
fn the_html_options_reach_the_page() {
    let server = Server::start("options", "=[ Hello ]\n", &["--lang", "en"]);
    assert!(server.get("/").contains("lang=\"en\""));
}

#[test]
fn a_port_that_is_taken_is_reported_and_exits_1() {
    let first = Server::start("taken", "=[ Hello ]\n", &[]);
    let port = first.addr.rsplit(':').next().unwrap().to_owned();

    let output = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["serve", "doc.tmt", "--port", &port])
        .current_dir(&first.dir)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("cannot serve on port {port}")),
        "{stderr}"
    );
}
