mod cli;
mod parser;
mod redis_db;
mod worker;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
use clap::Parser;
use cli::{Cli, Commands};
use redis_db::{monitor_status, show_stats, submit_job};
use worker::run_node;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let redis_url = "redis://127.0.0.1:6379";
    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_async_connection().await?;

    match cli.command {
        Commands::Submit { url } => submit_job(&mut con, &url).await?,
        Commands::Node => run_node(client).await?,
        Commands::Status { follow, job_id } => monitor_status(&mut con, &job_id, follow).await?,
        Commands::Stats { job_id } => show_stats(&mut con, &job_id).await?,
        Commands::Clean => {
            redis::cmd("FLUSHALL").query_async::<_, ()>(&mut con).await?;
            println!("Redis flushed successfully");
        }
    }
    Ok(())
}