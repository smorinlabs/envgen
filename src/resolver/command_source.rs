use anyhow::{bail, Context, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
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

#[cfg(unix)]
fn configure_process_group(cmd: &mut Command) {
    unsafe {
        cmd.pre_exec(|| {
            // Put the child into its own process group so we can terminate the entire group on timeout.
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(unix)]
fn kill_process_group_by_pid(pid: u32) -> std::io::Result<()> {
    let pgid = -(pid as i32);
    let rc = unsafe { libc::kill(pgid, libc::SIGKILL) };
    if rc == 0 {
        return Ok(());
    }

    let err = std::io::Error::last_os_error();
    // Process already exited.
    if err.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(err)
}

/// Execute a source command and return the trimmed stdout.
pub async fn execute_command(command: &str, timeout_secs: u64) -> Result<CommandResult> {
    enum WaitOutcome {
        Completed(std::io::Result<std::process::ExitStatus>),
        TimedOut,
    }

    let timeout = Duration::from_secs(timeout_secs);

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(unix)]
    configure_process_group(&mut cmd);

    let mut child = cmd.spawn().context("Failed to execute command")?;
    let pid = child.id();

    let mut stdout = child
        .stdout
        .take()
        .context("Failed to capture command stdout")?;
    let mut stderr = child
        .stderr
        .take()
        .context("Failed to capture command stderr")?;

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    let mut timed_out = false;
    let mut wait_error: Option<std::io::Error> = None;
    let mut exit_status: Option<std::process::ExitStatus> = None;

    match tokio::select! {
        res = child.wait() => WaitOutcome::Completed(res),
        _ = tokio::time::sleep(timeout) => WaitOutcome::TimedOut,
    } {
        WaitOutcome::Completed(res) => match res {
            Ok(status) => exit_status = Some(status),
            Err(e) => wait_error = Some(e),
        },
        WaitOutcome::TimedOut => {
            timed_out = true;

            if let Some(pid) = pid {
                #[cfg(unix)]
                {
                    let _ = kill_process_group_by_pid(pid);
                }
            }

            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    let stdout_bytes = stdout_task
        .await
        .context("Failed to join stdout reader task")??;
    let stderr_bytes = stderr_task
        .await
        .context("Failed to join stderr reader task")??;

    if let Some(e) = wait_error {
        bail!("Failed to execute command: {}", e);
    }

    if timed_out {
        bail!("Command timed out after {} seconds", timeout_secs);
    }

    let status = exit_status.context("Missing command exit status")?;
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
    if !status.success() {
        bail!(
            "Command failed with exit code {}: {}",
            status.code().unwrap_or(-1),
            stderr.trim()
        );
    }
    let stdout = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
    Ok(CommandResult {
        value: stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

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

        let cmd = build_command(
            template,
            "VITE_GOOGLE_ID",
            Some("GOOGLE_ID"),
            "local",
            &env_config,
        )
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

    #[tokio::test]
    async fn test_execute_command_timeout_terminates_process_group() {
        let tmp = tempdir().unwrap();
        let side_effect_path = tmp.path().join("side_effect.txt");
        let cmd = format!(
            "(sleep 2; echo hi > \"{}\") & sleep 10",
            side_effect_path.display()
        );

        let result = execute_command(&cmd, 1).await;
        assert!(result.is_err());

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        assert!(
            !side_effect_path.exists(),
            "side effect should not run after a timeout"
        );
    }
}
