use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let zone_id = env::var("CF_ZONE_ID")?;
    let api_token = env::var("CF_API_TOKEN")?;

    let record_ids: Vec<String> = env::var("CF_RECORD_IDS")?.split(',').map(|s| s.trim().to_string()).collect();
    let record_names: Vec<String> = env::var("CF_RECORD_NAMES")?.split(',').map(|s| s.trim().to_string()).collect();
    let record_types: Vec<String> = env::var("CF_RECORD_TYPES")?.split(',').map(|s| s.trim().to_uppercase()).collect();
    let record_proxied: Vec<String> = env::var("CF_RECORD_PROXIED")?.split(',').map(|s| s.trim().to_string()).collect();
    let cname_targets: Vec<String> = env::var("CF_CNAME_TARGETS").unwrap_or_default().split(',').map(|s| s.trim().to_string()).collect();

    // Validate record lengths
    if record_ids.len() != record_names.len() || record_names.len() != record_types.len() || record_types.len() != record_proxied.len() {
        eprintln!("\n❌ Error: CF_RECORD_* environment variables must have the same number of comma-separated values.\n");
        std::process::exit(1);
    }

    // Get public IP addresses
    let ipv4 = reqwest::blocking::get("https://api.ipify.org")?.text()?;
    let ipv6 = reqwest::blocking::get("https://api6.ipify.org")?.text().unwrap_or_else(|_| "".to_string());

    // Setup HTTP client with headers
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_token))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let client = Client::builder().default_headers(headers).build()?;

    for i in 0..record_ids.len() {
        let record_id = &record_ids[i];
        let record_name = &record_names[i];
        let record_type = &record_types[i];
        let proxied_flag = &record_proxied[i];

        let proxied = match proxied_flag.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => {
                eprintln!(
                    "\n⚠️  Invalid value for PROXIED flag '{}'. Use 'true' or 'false'. Skipping '{}'.\n",
                    proxied_flag, record_name
                );
                continue;
            }
        };

        // Select content based on record type
        let content = match record_type.as_str() {
            "A" if !ipv4.is_empty() => ipv4.clone(),
            "AAAA" if !ipv6.is_empty() => ipv6.clone(),
            "CNAME" => {
                if i < cname_targets.len() {
                    cname_targets[i].clone()
                } else {
                    eprintln!("\n⚠️  Missing CNAME target for '{}'. Skipping.\n", record_name);
                    continue;
                }
            }
            _ => {
                eprintln!(
                    "\n⚠️  Unsupported record type '{}' or IP not available for '{}'. Skipping.\n",
                    record_type, record_name
                );
                continue;
            }
        };

        println!(
            "\n🔄 Updating record: '{}' (ID: {})\n   ➤ Type: {}\n   ➤ Proxied: {}\n   ➤ Content: {}\n",
            record_name, record_id, record_type, proxied, content
        );

        // Prepare the payload
        let body = json!({
            "type": record_type,
            "name": record_name,
            "content": content,
            "ttl": 1,
            "proxied": proxied
        });

        // Send PATCH request to update record
        let res = client
            .patch(&format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                zone_id, record_id
            ))
            .json(&body)
            .send()?;

        if res.status().is_success() {
            println!("✅ Successfully updated '{}'.\n", record_name);
        } else {
            eprintln!(
                "❌ Failed to update '{}'. Status: {}, Response: {:?}\n",
                record_name,
                res.status(),
                res.text().unwrap_or_else(|_| "No response body".to_string())
            );
        }
    }

    Ok(())
}
