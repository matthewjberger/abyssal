use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "Abyssal", about = "A 3D game engine written in Rust")]
pub struct Options {
    #[structopt(subcommand)]
    pub command: Option<Command>,
}

#[derive(Default, Debug, StructOpt)]
pub enum Command {
    /// Launches the standalone desktop app
    #[structopt(about = "Run the editor")]
    #[default]
    Run,
}
