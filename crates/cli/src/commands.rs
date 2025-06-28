use std::path::PathBuf;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub(crate) enum Commands {
    Write(Write),
}

#[derive(clap::Args, Debug)]
pub(crate) struct Write {
    #[clap(short, long)]
    pub output: PathBuf,
}
