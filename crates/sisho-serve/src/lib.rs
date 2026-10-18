//! Layer 1: `sisho serve`.
//!
//! One document, as HTML, over HTTP on 127.0.0.1, rendered again on every
//! request: edit the file and reload the page. It is the same rendering as
//! `sisho convert --to html`, so what you see is what that writes.
//!
//! Kept in its own crate so that the HTTP stack (axum, tokio) stays out of
//! the build of every other command.

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use sisho_vault::convert::{self, Converted, Direction, Plan, Target};
use sisho_vault::render;

/// What to serve, and how to render it.
#[derive(Debug, Clone)]
pub struct Options {
    /// The document to serve.
    pub file: PathBuf,
    /// Number headings by nesting level and give them slugs.
    pub advanced: bool,
    /// `<html lang="...">` for the page shell.
    pub lang: Option<String>,
}

/// The document as HTML, as it is on disk now.
pub fn render_page(options: &Options) -> Result<String> {
    let html = render::Html {
        advanced: options.advanced,
        lang: options.lang.clone(),
        body: false,
    };
    match convert::convert_file(
        &options.file,
        &Direction::To(Target::Html, html),
        &Plan::default(),
    )? {
        Converted::Text(page) => Ok(page),
        other => unreachable!("no destination was given, so the page is the text: {other:?}"),
    }
}

/// The routes: the document at `/`.
pub fn router(options: Options) -> Router {
    Router::new()
        .route("/", get(page))
        .with_state(Arc::new(options))
}

/// Serve on `127.0.0.1:port` until the process ends. `0` takes a free port.
/// `on_listening` is told the address once it is bound, which is the first
/// moment a request can be made.
pub fn serve(port: u16, options: Options, on_listening: impl FnOnce(SocketAddr)) -> io::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let listener =
            tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
        on_listening(listener.local_addr()?);
        axum::serve(listener, router(options)).await
    })
}

async fn page(State(options): State<Arc<Options>>) -> (StatusCode, Html<String>) {
    // Reading and rendering block, so they are not done on the runtime's own
    // threads.
    let worker = Arc::clone(&options);
    let rendered = tokio::task::spawn_blocking(move || render_page(&worker)).await;
    match rendered {
        Ok(Ok(page)) => (StatusCode::OK, Html(page)),
        Ok(Err(error)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(error_page(&options, &error.to_string())),
        ),
        Err(joined) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(error_page(&options, &joined.to_string())),
        ),
    }
}

/// Why the document could not be shown, as a page: the person editing it is
/// looking at a browser, not at the terminal this runs in.
fn error_page(options: &Options, reason: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"{}\">\n<head><meta charset=\"utf-8\"><title>error</title></head>\n\
         <body>\n<h1>Could not render</h1>\n<p>{}</p>\n<pre>{}</pre>\n</body>\n</html>\n",
        escape_html(options.lang.as_deref().unwrap_or("ja")),
        escape_html(&display(&options.file)),
        escape_html(reason),
    )
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    use super::*;

    /// A document in a fresh temporary place, removed when dropped.
    struct Doc {
        dir: PathBuf,
        file: PathBuf,
    }

    impl Doc {
        fn new(name: &str, text: &str) -> Self {
            // The time, not the process id: `process-door` keeps the whole
            // `process` module out of every crate but `sisho-run`.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("sisho-serve-{nanos}-{name}"));
            fs::create_dir_all(&dir).unwrap();
            let file = dir.join("doc.tmt");
            fs::write(&file, text).unwrap();
            Self { dir, file }
        }

        fn options(&self) -> Options {
            Options {
                file: self.file.clone(),
                advanced: false,
                lang: None,
            }
        }
    }

    impl Drop for Doc {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// Serve `options` on a free port and make one request for `path`.
    async fn get_response(options: Options, path: &str) -> String {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router(options)).await });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    #[test]
    fn the_page_is_what_convert_to_html_writes() {
        let doc = Doc::new("page", "=[ Hello Serve ]\n\nBody.\n");
        let page = render_page(&doc.options()).unwrap();
        assert!(page.contains("<html"), "{page}");
        assert!(page.contains("Hello Serve"), "{page}");
    }

    #[test]
    fn the_options_reach_the_rendering() {
        let doc = Doc::new("options", "=[ Hello ]\n");
        let page = render_page(&Options {
            lang: Some("en".into()),
            ..doc.options()
        })
        .unwrap();
        assert!(page.contains("lang=\"en\""), "{page}");
    }

    #[test]
    fn the_error_page_escapes_what_it_shows() {
        let options = Options {
            file: PathBuf::from("a<b>.tmt"),
            advanced: false,
            lang: None,
        };
        let page = error_page(&options, "expected \"]\" & more <here>");
        assert!(page.contains("a&lt;b&gt;.tmt"), "{page}");
        assert!(
            page.contains("expected &quot;]&quot; &amp; more &lt;here&gt;"),
            "{page}"
        );
        assert!(!page.contains("<here>"), "{page}");
    }

    #[tokio::test]
    async fn the_root_serves_the_document_with_status_200() {
        let doc = Doc::new("root", "=[ Hello Serve ]\n");
        let response = get_response(doc.options(), "/").await;

        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(response.contains("Hello Serve"), "{response}");
    }

    #[tokio::test]
    async fn a_document_that_cannot_be_rendered_is_a_500_that_says_why() {
        let doc = Doc::new("broken", "=[ unterminated\n");
        let response = get_response(doc.options(), "/").await;

        assert!(response.starts_with("HTTP/1.1 500"), "{response}");
        assert!(response.contains("Could not render"), "{response}");
        assert!(response.contains("doc.tmt"), "{response}");
    }

    #[tokio::test]
    async fn any_other_path_is_not_found() {
        let doc = Doc::new("not-found", "=[ Hello ]\n");
        let response = get_response(doc.options(), "/elsewhere").await;
        assert!(response.starts_with("HTTP/1.1 404"), "{response}");
    }

    /// Rendered again on every request: edit the file and reload.
    #[tokio::test]
    async fn the_document_is_read_again_on_every_request() {
        let doc = Doc::new("reload", "=[ First ]\n");
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let options = doc.options();
        tokio::spawn(async move { axum::serve(listener, router(options)).await });

        async fn fetch(addr: SocketAddr) -> String {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let request = format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).await.unwrap();
            response
        }

        assert!(fetch(addr).await.contains("First"));
        fs::write(&doc.file, "=[ Second ]\n").unwrap();
        let after = fetch(addr).await;
        assert!(after.contains("Second"), "{after}");
        assert!(!after.contains("First"), "{after}");
    }
}
