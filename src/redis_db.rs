type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
use reqwest::Url;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

pub fn generate_job_id(url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:08x}", hasher.finish() % 0x1_0000_0000)
}

pub async fn submit_job(con: &mut redis::aio::Connection, raw_url: &str) -> Result<()> {
    let parsed_url = Url::parse(raw_url)?;
    let job_id = generate_job_id(raw_url);
    let meta_key = format!("job:{job_id}:meta");
    let frontier_key = format!("job:{job_id}:frontier");
    let visited_key = format!("job:{job_id}:visited");

    let exists: bool = redis::cmd("EXISTS").arg(&meta_key).query_async(con).await?;
    if exists {
        println!("job {job_id}  {raw_url} (already submitted)");
        return Ok(());
    }

    let _: () = redis::cmd("HSET").arg(&meta_key).arg("base_url").arg(parsed_url.as_str()).query_async(con).await?;
    let _: () = redis::cmd("SADD").arg(&visited_key).arg(parsed_url.as_str()).query_async(con).await?;
    let _: () = redis::cmd("RPUSH").arg(&frontier_key).arg(parsed_url.as_str()).query_async(con).await?;

    println!("job {job_id}  {raw_url}");
    Ok(())
}

pub async fn monitor_status(con: &mut redis::aio::Connection, job_id: &str, follow: bool) -> Result<()> {
    let visited_key = format!("job:{job_id}:visited");
    let frontier_key = format!("job:{job_id}:frontier");
    let inflight_key = format!("job:{job_id}:inflight");

    loop {
        let visited_count: i64 = redis::cmd("SCARD").arg(&visited_key).query_async(con).await.unwrap_or(0);
        let frontier_count: i64 = redis::cmd("LLEN").arg(&frontier_key).query_async(con).await.unwrap_or(0);
        let inflight_count: i64 = redis::cmd("GET").arg(&inflight_key).query_async(con).await.unwrap_or(0);

        let crawled = (visited_count - frontier_count - inflight_count).max(0);
        let state = if frontier_count == 0 && inflight_count == 0 { "done" } else { "running" };

        println!("crawled {crawled:<6} frontier {frontier_count:<6} in flight {inflight_count:<6} {state}");

        if !follow || state == "done" {
            break;
        }

        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}

pub async fn show_stats(con: &mut redis::aio::Connection, job_id: &str) -> Result<()> {
    let stats_key = format!("job:{job_id}:stats");
    let map: HashMap<String, String> = redis::cmd("HGETALL").arg(&stats_key).query_async(con).await?;

    if map.is_empty() {
        return Err("Job stats not found or job hasn't completed.".into());
    }

    let files = map.get("files").cloned().unwrap_or_else(|| "0".to_string());
    let words = map.get("words").cloned().unwrap_or_else(|| "0".to_string());

    let ext_count = map.keys().filter(|k| k.starts_with("ext:")).count();
    println!("files: {files}    extensions: {ext_count}      words: {words}");

    for (k, v) in &map {
        if let Some(ext_name) = k.strip_prefix("ext:") {
            println!("  {ext_name} {v}");
        }
    }

    Ok(())
}