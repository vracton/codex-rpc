use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::protocol::HelperCommand;
use crate::protocol::HelperEvent;

const WINDOWS_HELPER_EXE: &str = "codex-discord-presence-windows.exe";
const WINDOWS_TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordPresenceRuntimeConfig {
    pub application_id: String,
    pub large_image: Option<String>,
    pub large_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordPresenceSnapshot {
    pub details: String,
    pub state: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
}

enum PresenceRequest {
    Update(Option<DiscordPresenceSnapshot>),
    Shutdown,
}

pub struct DiscordPresenceClient {
    tx: Option<mpsc::UnboundedSender<PresenceRequest>>,
}

impl DiscordPresenceClient {
    pub fn disabled() -> Self {
        Self { tx: None }
    }

    pub fn new(
        config: DiscordPresenceRuntimeConfig,
        codex_self_exe: Option<&Path>,
    ) -> Result<Self> {
        if !cfg!(target_os = "linux") {
            anyhow::bail!("Discord presence is only implemented for WSL-hosted Codex sessions");
        }
        if !is_wsl() {
            anyhow::bail!(
                "Discord presence currently requires WSL so it can bridge to Windows Discord"
            );
        }

        let command = helper_command(codex_self_exe, &config)?;
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            if let Err(err) = bridge_task(command, rx).await {
                tracing::warn!(error = %err, "discord presence bridge exited");
            }
        });
        Ok(Self { tx: Some(tx) })
    }

    pub fn update(&self, snapshot: Option<DiscordPresenceSnapshot>) {
        let Some(tx) = self.tx.as_ref() else {
            return;
        };
        let _ = tx.send(PresenceRequest::Update(snapshot));
    }
}

impl Drop for DiscordPresenceClient {
    fn drop(&mut self) {
        let Some(tx) = self.tx.take() else {
            return;
        };
        let _ = tx.send(PresenceRequest::Shutdown);
    }
}

fn helper_command(
    codex_self_exe: Option<&Path>,
    config: &DiscordPresenceRuntimeConfig,
) -> Result<Command> {
    let helper_path = resolve_helper_path(codex_self_exe)?;
    let helper_path = helper_path
        .to_str()
        .context("Discord presence helper path is not valid UTF-8")?;
    let application_id = pwsh_quote(&config.application_id);
    let helper_path = pwsh_quote(helper_path);
    let mut script = format!(
        "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; \
         & {helper_path} --application-id {application_id}"
    );
    if let Some(large_image) = config.large_image.as_deref() {
        let large_image = pwsh_quote(large_image);
        script.push_str(&format!(" --large-image {large_image}"));
    }
    if let Some(large_text) = config.large_text.as_deref() {
        let large_text = pwsh_quote(large_text);
        script.push_str(&format!(" --large-text {large_text}"));
    }

    let mut command = Command::new("powershell.exe");
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["-NoProfile", "-Command", script.as_str()]);
    Ok(command)
}

async fn bridge_task(
    mut command: Command,
    mut rx: mpsc::UnboundedReceiver<PresenceRequest>,
) -> Result<()> {
    let session_started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_secs();
    let mut child = command
        .spawn()
        .context("failed to spawn discord presence helper")?;
    let Some(mut stdin) = child.stdin.take() else {
        anyhow::bail!("discord presence helper stdin was unavailable");
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<HelperEvent>(&line) {
                    Ok(HelperEvent::Ready) => {
                        tracing::debug!("discord presence helper ready");
                    }
                    Ok(HelperEvent::Error { message }) => {
                        tracing::warn!(%message, "discord presence helper reported an error");
                    }
                    Err(err) => {
                        tracing::debug!(error = %err, line, "failed to parse discord helper output");
                    }
                }
            }
        });
    }
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(%line, "discord presence helper stderr");
            }
        });
    }

    let mut last_snapshot = None;
    while let Some(request) = rx.recv().await {
        match request {
            PresenceRequest::Update(snapshot) => {
                if snapshot == last_snapshot {
                    continue;
                }
                last_snapshot = snapshot.clone();
                let command = match snapshot {
                    Some(snapshot) => HelperCommand::SetPresence {
                        details: snapshot.details,
                        state: snapshot.state,
                        small_image: snapshot.small_image,
                        small_text: snapshot.small_text,
                        start_timestamp_seconds: session_started_at,
                    },
                    None => HelperCommand::ClearPresence,
                };
                let line = serde_json::to_string(&command)?;
                stdin.write_all(line.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
            }
            PresenceRequest::Shutdown => {
                let line = serde_json::to_string(&HelperCommand::Shutdown)?;
                stdin.write_all(line.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
                break;
            }
        }
    }

    let status = child
        .wait()
        .await
        .context("failed waiting for discord presence helper")?;
    if !status.success() {
        anyhow::bail!("discord presence helper exited with status {status}");
    }
    Ok(())
}

fn resolve_helper_path(codex_self_exe: Option<&Path>) -> Result<PathBuf> {
    let Some(codex_self_exe) = codex_self_exe else {
        anyhow::bail!("Codex executable path is unavailable for Discord presence helper lookup");
    };
    let Some(profile_dir) = codex_self_exe.parent() else {
        anyhow::bail!("Codex executable path has no parent directory");
    };
    let Some(profile_name) = profile_dir.file_name().and_then(|name| name.to_str()) else {
        anyhow::bail!("failed to derive build profile from Codex executable path");
    };
    let Some(target_dir) = profile_dir.parent() else {
        anyhow::bail!("failed to derive target directory from Codex executable path");
    };
    let helper_path = target_dir
        .join(WINDOWS_TARGET_TRIPLE)
        .join(profile_name)
        .join(WINDOWS_HELPER_EXE);
    if !helper_path.exists() {
        anyhow::bail!(
            "Discord presence helper was not found at {}",
            helper_path.display()
        );
    }
    linux_path_to_windows_launch_path(&helper_path)
}

fn linux_path_to_windows_launch_path(path: &Path) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .context("path used for Discord helper lookup is not valid UTF-8")?;
    if let Some(mapped) = wsl_mount_to_windows_path(path_str) {
        return Ok(PathBuf::from(mapped));
    }

    let distro = env::var("WSL_DISTRO_NAME")
        .context("WSL_DISTRO_NAME is not set; cannot construct Windows path for helper")?;
    Ok(PathBuf::from(wsl_localhost_launch_path(path_str, &distro)))
}

fn wsl_mount_to_windows_path(path: &str) -> Option<String> {
    let stripped = path.strip_prefix("/mnt/")?;
    let mut parts = stripped.splitn(2, '/');
    let drive = parts.next()?;
    if drive.len() != 1 {
        return None;
    }
    let tail = parts.next().unwrap_or_default().replace('/', "\\");
    let drive_letter = drive.chars().next()?.to_ascii_uppercase();
    Some(if tail.is_empty() {
        format!("{drive_letter}:\\")
    } else {
        format!("{drive_letter}:\\{tail}")
    })
}

fn wsl_localhost_launch_path(path: &str, distro: &str) -> String {
    format!("//wsl.localhost/{distro}{path}")
}

fn pwsh_quote(input: &str) -> String {
    format!("'{}'", input.replace('\'', "''"))
}

fn is_wsl() -> bool {
    env::var_os("WSL_DISTRO_NAME").is_some() || env::var_os("WSL_INTEROP").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn mount_paths_convert_to_windows_paths() {
        assert_eq!(
            wsl_mount_to_windows_path("/mnt/c/Users/alice/app.exe"),
            Some("C:\\Users\\alice\\app.exe".to_string())
        );
    }

    #[test]
    fn non_mount_paths_convert_to_wsl_localhost_launch_path() {
        assert_eq!(
            wsl_localhost_launch_path("/home/alice/app.exe", "Ubuntu"),
            "//wsl.localhost/Ubuntu/home/alice/app.exe".to_string()
        );
    }

    #[test]
    fn powershell_quotes_single_quotes() {
        assert_eq!(pwsh_quote("Alice's Codex"), "'Alice''s Codex'");
    }
}
