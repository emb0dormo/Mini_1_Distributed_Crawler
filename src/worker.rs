type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
use reqwest::Url;
use std::time::Duration;
use tokio::time::sleep;

use crate::parser::{count_words, extract_links, get_extension, is_under_base_path};

pub async fn run_node(client: redis::Client) -> Result<()> {
    println!("Node active. Polling jobs from Redis...");
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    loop {
        let mut con = client.get_async_connection().await?;
        let keys: Vec<String> = redis::cmd("KEYS").arg("job:*:meta").query_async(&mut con).await?;

        if keys.is_empty() {
            sleep(Duration::from_millis(500)).await;
            continue;
        }

        let mut processed_any = false;

        for meta_key in keys {
            let job_id = meta_key.split(':').nth(1).unwrap_or("");
            let base_url_str: String = redis::cmd("HGET")
                .arg(&meta_key)
                .arg("base_url")
                .query_async(&mut con)
                .await
                .unwrap_or_default();

            if base_url_str.is_empty() {
                continue;
            }

            let base_url = Url::parse(&base_url_str)?;
            let frontier_key = format!("job:{job_id}:frontier");
            let visited_key = format!("job:{job_id}:visited");
            let inflight_key = format!("job:{job_id}:inflight");
            let stats_key = format!("job:{job_id}:stats");

            let url_option: Option<String> = redis::cmd("LPOP").arg(&frontier_key).query_async(&mut con).await?;

            if let Some(target_url_str) = url_option {
                processed_any = true;
                let _: () = redis::cmd("INCR").arg(&inflight_key).query_async(&mut con).await?;

                let http_c = http_client.clone();
                let mut con_task = client.get_async_connection().await?;

                tokio::spawn(async move {
                    if let Ok(res) = http_c.get(&target_url_str).send().await {
                        if let Ok(html_text) = res.text().await {
                            let words = count_words(&html_text);
                            let ext = get_extension(&target_url_str);

                            let _: () = redis::cmd("HINCRBY").arg(&stats_key).arg("files").arg(1).query_async(&mut con_task).await.unwrap_or(());
                            let _: () = redis::cmd("HINCRBY").arg(&stats_key).arg("words").arg(words as i64).query_async(&mut con_task).await.unwrap_or(());
                            let _: () = redis::cmd("HINCRBY").arg(&stats_key).arg(format!("ext:{ext}")).arg(1).query_async(&mut con_task).await.unwrap_or(());

                            let links = extract_links(&target_url_str, &html_text);
                            for link in links {
                                if let Ok(parsed) = Url::parse(&link) {
                                    if is_under_base_path(&base_url, &parsed) {
                                        let added: i32 = redis::cmd("SADD")
                                            .arg(&visited_key)
                                            .arg(parsed.as_str())
                                            .query_async(&mut con_task)
                                            .await
                                            .unwrap_or(0);

                                        if added == 1 {
                                            let _: () = redis::cmd("RPUSH")
                                                .arg(&frontier_key)
                                                .arg(parsed.as_str())
                                                .query_async(&mut con_task)
                                                .await
                                                .unwrap_or(());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let _: () = redis::cmd("DECR").arg(&inflight_key).query_async(&mut con_task).await.unwrap_or(());
                });
            }
        }

        if !processed_any {
            sleep(Duration::from_millis(200)).await;
        }
    }
}