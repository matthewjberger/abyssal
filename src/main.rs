#![warn(clippy::all, rust_2018_idioms)]
// #![windows_subsystem = "windows"] // uncomment this to suppress terminal on windows

mod cli;
mod context;
mod ecs;
mod run;

use cli::{Command, Options};
use structopt::StructOpt;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let Options { command } = Options::from_args();
    if matches!(command, Some(Command::Run) | None) {
        let mut context = context::Context::default();
        run::run(&mut context);
    }
    Ok(())
}
