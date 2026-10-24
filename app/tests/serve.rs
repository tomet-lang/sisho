//! `sisho serve`, started as a real process on a free port, against a fake
//! `tomet html`.
//!
//! The rendering is `tomet`'s and is tested there. What `sisho` owns is the
//! server: running `tomet html` for each request, passing the options on,
//! and turning a failure into a page that says why.
#![cfg(unix)]

mod common;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command, Stdio};

use common::Sandbox;

/// A fake `tomet html FILE [--advanced] [--lang L]` (shell built-ins only: `PATH`
/// holds nothing else): the page is the first line of the file between two tags, in the language asked for, marked when `--advanced` came.
/// A file that says "unterminated" does not render, as a broken document
/// does not.
const TOMET: &str = r#"#!/bin/sh
[ "$1" = html ] || { echo "not html: $*" >&2; exit 2; }
shift
lang=ja; advanced=""
while [ $# -gt 0 ]; do
  case "$1" in
    --lang) lang="$2"; shift;;
    --advanced) advanced="<!-- advanced -->";;
    *) file="$1";;
  esac
  shift
done
IFS= read -r body < "$file"
case "$body" in
  *unterminated*) echo "error: unterminated, expected ']'" >&2; echo "  --> $file:1:1" >&2; exit 1;;
esac
echo "<html lang=\"$lang\">$advanced<body>$body</body></html>"
"#;

/// A running server and the sandbox it serves from; both are cleaned up when
/// this is dropped.
struct Server {
    child: Child,
    sandbox: Sandbox,
    addr: String,
}

impl Server {
    fn start(name: &str, document: &str, extra: &[&str]) -> Self {
        let sandbox = Sandbox::new(&format!("serve-{name}"));
        let path = sandbox.dir.join("tomet");
        fs::write(&path, TOMET).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        Self::start_in(sandbox, document, extra)
    }

    /// Serve from `sandbox` as it is: whatever tools it holds, which may be
    /// none.
    fn start_in(sandbox: Sandbox, document: &str, extra: &[&str]) -> Self {
        fs::write(sandbox.dir.join("doc.tmt"), document).unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_sisho"))
            .args(["serve", "doc.tmt", "--port", "0"])
            .args(extra)
            .current_dir(&sandbox.dir)
            .env_clear()
            .env("PATH", &sandbox.dir)
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
        Self {
            child,
            sandbox,
            addr,
        }
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
        fs::write(self.sandbox.dir.join("doc.tmt"), text).unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
    // tomet's own words, escaped for a page.
    assert!(
        broken.contains("error: unterminated, expected ']'"),
        "{broken}"
    );
    assert!(broken.contains("doc.tmt"), "{broken}");

    server.write("=[ Fixed ]\n");
    let fixed = server.get("/");
    assert!(fixed.starts_with("HTTP/1.1 200"), "{fixed}");
    assert!(fixed.contains("Fixed"), "{fixed}");
}

#[test]
fn the_html_options_reach_tomet() {
    let server = Server::start("options", "=[ Hello ]\n", &["--lang", "en", "--advanced"]);
    let page = server.get("/");
    assert!(page.contains("lang=\"en\""), "{page}");
    assert!(page.contains("<!-- advanced -->"), "{page}");

    let plain = Server::start("no-options", "=[ Hello ]\n", &[]).get("/");
    assert!(plain.contains("lang=\"ja\""), "{plain}");
    assert!(!plain.contains("advanced"), "{plain}");
}

#[test]
fn without_tomet_the_page_says_so_and_it_is_a_500() {
    let server = Server::start_in(Sandbox::new("serve-no-tomet"), "=[ Hello ]\n", &[]);
    let response = server.get("/");
    assert!(response.starts_with("HTTP/1.1 500"), "{response}");
    assert!(response.contains("tomet is not on PATH"), "{response}");
}

#[test]
fn a_port_that_is_taken_is_reported_and_exits_1() {
    let first = Server::start("taken", "=[ Hello ]\n", &[]);
    let port = first.addr.rsplit(':').next().unwrap().to_owned();

    let output = Command::new(env!("CARGO_BIN_EXE_sisho"))
        .args(["serve", "doc.tmt", "--port", &port])
        .current_dir(&first.sandbox.dir)
        .env_clear()
        .env("PATH", &first.sandbox.dir)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("cannot serve on port {port}")),
        "{stderr}"
    );
}
