use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crawl")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Submit { url: String },
    Node,
    Status {
        #[arg(short, long)]
        follow: bool,
        job_id: String,
    },
    Stats { job_id: String },
    Clean,
}