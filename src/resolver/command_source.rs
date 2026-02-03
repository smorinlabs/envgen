use anyhow::{bail, Result};
use std::time::Duration;
use tokio::process::Command;

use crate::template;

/// Result of executing a source command.
#[derive(Debug)]
#[allow(dead_code)]
pub struct CommandResult {
    pub value: String,
    pub stderr: String,
}

/// Build the resolved command string from a source command template.
pub fn build_command(
    source_command_template: &str,
    var_name: &str,
    source_key: Option<&str>,
    env_name: &str,
    env_config: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    let key = source_key.unwrap_or(var_name);
    let ctx = template::build_context(env_name, env_config, key);
    template::expand_template(source_command_template, &ctx)
}

/// Execute a source command and return the trimmed stdout.
pub async fn execute_command(command: &str, timeout_secs: u64) -> Result<CommandResult> {
    let output = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if !output.status.success() {
                bail!(
                    "Command failed with exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    stderr.trim()
                );
            }
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(CommandResult {
                value: stdout,
                stderr,
            })
        }
        Ok(Err(e)) => bail!("Failed to execute command: {}", e),
        Err(_) => bail!("Command timed out after {} seconds", timeout_secs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_build_command() {
        let template = "firebase functions:secrets:access {key} --project {firebase_project}";
        let mut env_config = BTreeMap::new();
        env_config.insert("firebase_project".to_string(), "my-proj".to_string());

        let cmd = build_command(template, "MY_SECRET", None, "staging", &env_config).unwrap();
        assert_eq!(
            cmd,
            "firebase functions:secrets:access MY_SECRET --project my-proj"
        );
    }

    #[test]
    fn test_build_command_with_source_key() {
        let template = "echo {key}";
        let env_config = BTreeMap::new();

        let cmd =
            build_command(template, "VITE_GOOGLE_ID", Some("GOOGLE_ID"), "local", &env_config)
                .unwrap();
        assert_eq!(cmd, "echo GOOGLE_ID");
    }

    #[tokio::test]
    async fn test_execute_command_success() {
        let result = execute_command("echo hello", 30).await.unwrap();
        assert_eq!(result.value, "hello");
    }

    #[tokio::test]
    async fn test_execute_command_failure() {
        let result = execute_command("exit 1", 30).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_command_timeout() {
        let result = execute_command("sleep 10", 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }
}
