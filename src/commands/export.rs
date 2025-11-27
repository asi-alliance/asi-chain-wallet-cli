use crate::config::Config;
use crate::utils::deployment::{
    fetch_recent_deployments, format_deployment_timestamp, get_deployment_field,
    get_deployment_status, get_transfer_direction,
};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportRecord {
    deploy_id: String,
    block_number: String,
    status: String,
    created_at: String,
    transfer_direction: Option<String>,
    transfer_amount: Option<String>,
    transfer_from: Option<String>,
    transfer_to: Option<String>,
}

pub async fn handle_export(
    config: &Config,
    format: &ExportFormat,
    output: Option<&str>,
    limit: Option<u32>,
) -> Result<()> {
    let wallet = config.get_wallet()?;
    let limit = limit.unwrap_or(100);

    println!("\n>>> Exporting wallet data...");
    println!("Wallet: {}", wallet.address);
    println!("Format: {:?}", format);
    println!("Limit: {}", limit);

    let deployments = fetch_recent_deployments(
        &config.graphql_url,
        &wallet.address,
        &wallet.public_key,
        limit,
    )
    .await?;

    let records = convert_to_records(deployments, &wallet.address)?;
    let output_path = determine_output_path(output, format)?;

    match format {
        ExportFormat::Json => export_json(&records, &output_path)?,
        ExportFormat::Csv => export_csv(&records, &output_path)?,
    }

    println!("✓ Data exported successfully to: {}", output_path);
    println!("  Total records: {}", records.len());

    Ok(())
}

fn convert_to_records(
    deployments: Vec<serde_json::Value>,
    wallet_address: &str,
) -> Result<Vec<ExportRecord>> {
    let mut records = Vec::new();

    for deployment in deployments {
        let deploy_id = get_deployment_field(&deployment, "deploy_id", "unknown");
        let block_number = get_deployment_field(&deployment, "block_number", "0");
        let final_status = get_deployment_status(&deployment);
        let formatted_date = format_deployment_timestamp(&deployment);

        if let Some(transfers) = deployment.get("transfers").and_then(|v| v.as_array()) {
            if transfers.is_empty() {
                records.push(ExportRecord {
                    deploy_id: deploy_id.clone(),
                    block_number: block_number.clone(),
                    status: final_status.clone(),
                    created_at: formatted_date.clone(),
                    transfer_direction: None,
                    transfer_amount: None,
                    transfer_from: None,
                    transfer_to: None,
                });
            } else {
                for transfer in transfers {
                    let from_address = get_deployment_field(transfer, "from_address", "unknown");
                    let to_address = get_deployment_field(transfer, "to_address", "unknown");
                    let amount_dust = get_deployment_field(transfer, "amount_dust", "0");
                    let direction =
                        get_transfer_direction(&from_address, wallet_address).to_string();

                    records.push(ExportRecord {
                        deploy_id: deploy_id.clone(),
                        block_number: block_number.clone(),
                        status: final_status.clone(),
                        created_at: formatted_date.clone(),
                        transfer_direction: Some(direction),
                        transfer_amount: Some(amount_dust),
                        transfer_from: Some(from_address),
                        transfer_to: Some(to_address),
                    });
                }
            }
        } else {
            records.push(ExportRecord {
                deploy_id,
                block_number,
                status: final_status,
                created_at: formatted_date,
                transfer_direction: None,
                transfer_amount: None,
                transfer_from: None,
                transfer_to: None,
            });
        }
    }

    Ok(records)
}

fn determine_output_path(output: Option<&str>, format: &ExportFormat) -> Result<String> {
    if let Some(path) = output {
        Ok(path.to_string())
    } else {
        let exports_dir = "exports";
        fs::create_dir_all(exports_dir).context("Failed to create exports directory")?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let extension = match format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
        };

        Ok(format!(
            "{}/wallet_export_{}.{}",
            exports_dir, timestamp, extension
        ))
    }
}

fn export_json(records: &[ExportRecord], output_path: &str) -> Result<()> {
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let json_data =
        serde_json::to_string_pretty(&records).context("Failed to serialize data to JSON")?;

    fs::write(output_path, json_data).context("Failed to write JSON file")?;

    Ok(())
}

fn export_csv(records: &[ExportRecord], output_path: &str) -> Result<()> {
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let mut wtr = csv::Writer::from_path(output_path).context("Failed to create CSV writer")?;

    for record in records {
        wtr.serialize(record)
            .context("Failed to write CSV record")?;
    }

    wtr.flush().context("Failed to flush CSV writer")?;

    Ok(())
}
