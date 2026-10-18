//! `sisho serve`.

use std::io;
use std::process::ExitCode;

use anyhow::Result;

use super::{Sink, finish, with_standard_streams};
use crate::cli::ServeArgs;

pub fn serve(args: &ServeArgs) -> ExitCode {
    finish(with_standard_streams(|out, err| run_serve(args, out, err)))
}

pub fn run_serve(args: &ServeArgs, out: Sink<'_>, _err: Sink<'_>) -> Result<()> {
    let options = sisho_serve::Options {
        file: args.file.clone(),
        advanced: args.advanced,
        lang: args.lang.clone(),
    };
    // Said once the port is bound, which is when it can be used. Flushed, so
    // that whoever started this through a pipe sees it now and not at exit.
    let mut announced: io::Result<()> = Ok(());
    let served = sisho_serve::serve(args.port, options, |addr| {
        announced =
            writeln!(out, "Serving at http://{addr} (Ctrl+C to stop)").and_then(|()| out.flush());
    });
    served.map_err(|error| anyhow::anyhow!("cannot serve on port {}: {error}", args.port))?;
    announced?;
    Ok(())
}
