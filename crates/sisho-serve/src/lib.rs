//! Layer 1: `sisho serve`.
//!
//! One document, as HTML, over HTTP on 127.0.0.1, rendered again on every
//! request: edit the file and reload the page. The rendering is `tomet html`,
//! run for each request, so what you see is what `sisho convert --to html`
//! writes.
//!
//! Kept in its own crate so that the HTTP stack (axum, tokio) stays out of
//! the build of every other command.

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use sisho_run::{Outcome, Status};
use sisho_tools::{Conversion, Direction, Target};

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
///
/// `Err` says why it could not be shown, in `tomet`'s own words.
pub fn render_page(options: &Options) -> Result<String> {
    let mut conversion = Conversion::new(&options.file, Direction::To(Target::Html));
    conversion.advanced = options.advanced;
    conversion.lang = options.lang.as_deref();
    page_of(conversion.step().run())
}

/// The page `tomet html` wrote, or why it did not.
fn page_of(outcome: Outcome) -> Result<String> {
    let said = outcome.stderr.trim();
    let reason = match &outcome.status {
        Status::Passed => return Ok(outcome.stdout),
        Status::NotFound => "tomet is not on PATH".to_owned(),
        Status::CouldNotStart { reason } => format!("could not start tomet: {reason}"),
        // What `tomet` said is the reason, in its own words.
        Status::Failed { .. } | Status::Signalled { .. } if !said.is_empty() => said.to_owned(),
        Status::Failed { code } => format!("`{}` exited with code {code}", outcome.command_line),
        Status::Signalled { signal } => {
            format!("`{}` was ended by signal {signal}", outcome.command_line)
        }
    };
    bail!(reason)
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
    // Running `tomet` blocks, so it is not done on the runtime's own threads.
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
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    use super::*;

    fn outcome(status: Status, stdout: &str, stderr: &str) -> Outcome {
        Outcome::in_process("tomet html", status, stdout, stderr, Duration::ZERO)
    }

    #[test]
    fn the_page_is_what_tomet_html_wrote() {
        let page = page_of(outcome(Status::Passed, "<html>page</html>\n", "")).unwrap();
        assert_eq!(page, "<html>page</html>\n");
    }

    #[test]
    fn a_failure_is_shown_in_tomets_own_words() {
        let error = page_of(outcome(
            Status::Failed { code: 1 },
            "",
            "error: expected ']'\n  --> doc.tmt:1:3\n",
        ))
        .unwrap_err();
        assert_eq!(error.to_string(), "error: expected ']'\n  --> doc.tmt:1:3");
    }

    #[test]
    fn a_failure_that_said_nothing_still_says_what_ran() {
        let error = page_of(outcome(Status::Failed { code: 3 }, "", "")).unwrap_err();
        assert!(error.to_string().contains("tomet html"), "{error}");
        assert!(error.to_string().contains("code 3"), "{error}");
    }

    #[test]
    fn a_tomet_that_is_not_installed_is_said_so() {
        let error = page_of(outcome(Status::NotFound, "", "")).unwrap_err();
        assert_eq!(error.to_string(), "tomet is not on PATH");
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

    /// Only `/` is a route, so nothing else runs `tomet`.
    #[tokio::test]
    async fn any_other_path_is_not_found() {
        let options = Options {
            file: PathBuf::from("doc.tmt"),
            advanced: false,
            lang: None,
        };
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router(options)).await });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request =
            format!("GET /elsewhere HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 404"), "{response}");
    }
}
