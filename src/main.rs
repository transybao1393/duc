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
    let record_proxied = env::var("CF_RECORD_PROXIED")?;

    let client = Client::new();

    // Split comma-separated lists
    let record_ids: Vec<&str> = record_ids.split(',').map(|s| s.trim()).collect();
    let record_names: Vec<&str> = record_names.split(',').map(|s| s.trim()).collect();
    let record_proxied: Vec<&str> = record_proxied.split(',').map(|s| s.trim()).collect();

    // Validation
    if record_ids.len() != record_names.len() || record_names.len() != record_proxied.len() {
        eprintln!("\n❌ Mismatch in number of RECORD IDs, NAMES, and PROXIED flags!\n");
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
    println!("\n🌐 Current Public IP: {}\n", current_ip);

    // Step 2: Loop and update each DNS record
    for ((record_id, record_name), proxied_str) in record_ids.iter().zip(record_names.iter()).zip(record_proxied.iter()) {
        let proxied = match proxied_str.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!(
                    "\n⚠️  Invalid value for PROXIED flag '{}'. Use 'true' or 'false'. Skipping '{}'.\n",
                    proxied_str, record_name
                );
                continue;
            }
        };

        println!(
            "🔄 Updating record: '{}' (ID: {})\n   ➤ Proxied: {}\n   ➤ New IP: {}\n",
            record_name, record_id, proxied, current_ip
        );

        let dns_update = DnsRecordUpdate {
            r#type: "A".to_string(),
            name: record_name.to_string(),
            content: current_ip.clone(),
            ttl: 120,
            proxied,
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
            println!("✅ Successfully updated '{}'\n", record_name);
        } else {
            eprintln!("❌ Failed to update '{}': {:?}\n", record_name, res.errors);
        }
    }

    println!("🎉 DNS records update completed!\n");
    Ok(())
}
