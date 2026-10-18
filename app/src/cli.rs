use clap::Parser;

#[derive(Parser)]
#[command(
    name = "sisho",
    version,
    about = "Orchestrate the Tomet ecosystem (tomet, twrit, tandr)",
    arg_required_else_help = true
)]
pub struct Cli {}
