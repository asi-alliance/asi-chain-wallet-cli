use anyhow::{Result, anyhow};
use node_cli::utils::{CompressedDeployStatus, DeployCompressedInfo};

pub fn parse_deploy_status(deploy_info: &DeployCompressedInfo) -> Result<()> {
    match *deploy_info.status() {
        CompressedDeployStatus::Deploying => {
            println!("⚠ Deploy is sent but not yet included in block");
            println!("  This may indicate network delay or insufficient wait time");
            println!(
                "  Consider increasing the wait time or checking the network/deploy_id status"
            );
            Ok(())
        }
        CompressedDeployStatus::Finalizing => {
            println!("⚠ Deploy is sent and included in block, but not finalized yet");
            println!(
                "  Consider increasing the wait time or checking the network/deploy_id status"
            );
            Ok(())
        }
        CompressedDeployStatus::Finalized => {
            println!("✔ Deploy completed successfully");
            Ok(())
        }
        _ => {
            let error_msg = deploy_info.msg().unwrap_or("Unknown error");
            Err(anyhow!(
                "✖ Deploy status doesn't match any known status: {}",
                error_msg
            ))
        }
    }
}
