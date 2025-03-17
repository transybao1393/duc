use dotenv::dotenv;
use std::env;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct IpResponse {
    ip: String,
}

#[derive(Serialize)]
struct DnsRecordUpdate {
    r#type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Deserialize)]
struct CloudflareResponse {
    success: bool,
    errors: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_token = env::var("CF_API_TOKEN")?;
    let zone_id = env::var("CF_ZONE_ID")?;
    let record_ids = env::var("CF_RECORD_IDS")?;
    let record_names = env::var("CF_RECORD_NAMES")?;

    let client = Client::new();

    // Split multiple IDs and names
    let record_ids: Vec<&str> = record_ids.split(',').map(|s| s.trim()).collect();
    let record_names: Vec<&str> = record_names.split(',').map(|s| s.trim()).collect();

    if record_ids.len() != record_names.len() {
        eprintln!("Mismatch between number of RECORD IDs and RECORD NAMES!");
        return Ok(());
    }

    // Step 1: Fetch current public IP
    let ip_resp: IpResponse = client
        .get("https://api.ipify.org?format=json")
        .send()
        .await?
        .json()
        .await?;

    let current_ip = ip_resp.ip;
    println!("Current Public IP: {}", current_ip);

    // Step 2: Loop through each DNS record and update it
    for (record_id, record_name) in record_ids.iter().zip(record_names.iter()) {
        println!("Updating record '{}' (ID: {})", record_name, record_id);

        let dns_update = DnsRecordUpdate {
            r#type: "A".to_string(),
            name: record_name.to_string(),
            content: current_ip.clone(),
            ttl: 120,
            proxied: false,
        };

        let res: CloudflareResponse = client
            .put(format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                zone_id, record_id
            ))
            .bearer_auth(&api_token)
            .json(&dns_update)
            .send()
            .await?
            .json()
            .await?;

        if res.success {
            println!("✅ Successfully updated '{}'", record_name);
        } else {
            eprintln!("❌ Failed to update '{}': {:?}", record_name, res.errors);
        }
    }

    Ok(())
}
