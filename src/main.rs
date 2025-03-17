use std::env;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    let api_token = env::var("CF_API_TOKEN")?;
    let zone_id = env::var("CF_ZONE_ID")?;

    // Split and parse environment variables properly
    let record_ids_str = env::var("CF_RECORD_IDS")?;
    let record_names_str = env::var("CF_RECORD_NAMES")?;
    let record_types_str = env::var("CF_RECORD_TYPES")?;
    let record_proxied_str = env::var("CF_RECORD_PROXIED")?;
    let cname_targets_str = env::var("CF_CNAME_TARGETS").unwrap_or_default();

    let record_ids: Vec<&str> = record_ids_str.split(',').map(|s| s.trim()).collect();
    let record_names: Vec<&str> = record_names_str.split(',').map(|s| s.trim()).collect();
    let record_types: Vec<&str> = record_types_str.split(',').map(|s| s.trim()).collect();
    let record_proxied: Vec<&str> = record_proxied_str.split(',').map(|s| s.trim()).collect();
    let cname_targets: Vec<&str> = cname_targets_str.split(',').map(|s| s.trim()).collect();

    // Validate lengths of records
    if record_ids.len() != record_names.len() || record_names.len() != record_types.len() || record_types.len() != record_proxied.len() {
        eprintln!("Error: Mismatched number of record IDs, names, types, or proxied flags.");
        std::process::exit(1);
    }

    // Fetch public IPv4
    let ipv4 = reqwest::blocking::get("https://api.ipify.org")
        .and_then(|r| r.text())
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to fetch IPv4 address: {}", e);
            "".to_string()
        });

    // Fetch public IPv6
    let ipv6 = reqwest::blocking::get("https://api6.ipify.org")
        .and_then(|r| r.text())
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to fetch IPv6 address: {}", e);
            "".to_string()
        });

    // Ensure at least one IP address is available
    if ipv4.is_empty() && ipv6.is_empty() {
        eprintln!("Error: Could not retrieve either IPv4 or IPv6 address. Exiting.");
        std::process::exit(1);
    }

    println!("Public IPv4: {}", if ipv4.is_empty() { "None" } else { &ipv4 });
    println!("Public IPv6: {}", if ipv6.is_empty() { "None" } else { &ipv6 });

    // Setup headers for API request
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_token))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = Client::new();

    // Iterate through records and update them
    for (i, record_id) in record_ids.iter().enumerate() {
        let name = record_names[i];
        let record_type = record_types[i];
        let proxied = record_proxied[i].parse::<bool>().unwrap_or(false);

        let content = if record_type == "A" {
            if ipv4.is_empty() {
                eprintln!("Warning: Skipping A record {} due to missing IPv4 address", name);
                continue;
            }
            ipv4.clone()
        } else if record_type == "AAAA" {
            if ipv6.is_empty() {
                eprintln!("Warning: Skipping AAAA record {} due to missing IPv6 address", name);
                continue;
            }
            ipv6.clone()
        } else if record_type == "CNAME" {
            if i >= cname_targets.len() || cname_targets[i].is_empty() {
                eprintln!("Warning: Skipping CNAME record {} due to missing target", name);
                continue;
            }
            cname_targets[i].to_string()
        } else {
            eprintln!("Warning: Unsupported record type {} for record {}", record_type, name);
            continue;
        };

        let url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}", zone_id, record_id);

        let body = json!({
            "type": record_type,
            "name": name,
            "content": content,
            "proxied": proxied
        });

        println!("Updating record {} ({}): {}", name, record_type, content);

        let res = client.put(&url).headers(headers.clone()).json(&body).send();

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    println!("Successfully updated record: {}", name);
                } else {
                    eprintln!("Failed to update record: {}. Response: {:?}", name, response.text().unwrap_or_default());
                }
            }
            Err(e) => {
                eprintln!("Error updating record {}: {}", name, e);
            }
        }
    }

    Ok(())
}
