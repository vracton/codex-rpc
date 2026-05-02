use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;

use anyhow::Context;
use anyhow::Result;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::protocol::HelperCommand;
use crate::protocol::HelperEvent;
use crate::protocol::HelperSnapshot;
use crate::protocol::PetState;

const WINDOWS_HELPER_EXE: &str = "codex-pets-windows.exe";
const WINDOWS_TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";
const ELECTRON_HELPER_DIR: &str = "pets-windows/electron";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetsRuntimeConfig {
    pub selected_pet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetsSnapshot {
    pub state: PetState,
    pub title: String,
    pub subtitle: Option<String>,
    pub detail: Option<String>,
    pub notification_count: u32,
}

enum PetsRequest {
    Toggle,
    Update(PetsSnapshot),
    Shutdown,
}

pub struct PetsClient {
    tx: Option<mpsc::UnboundedSender<PetsRequest>>,
}

impl PetsClient {
    pub fn disabled() -> Self {
        Self { tx: None }
    }

    pub fn new(config: PetsRuntimeConfig, codex_self_exe: Option<&Path>) -> Result<Self> {
        if !cfg!(target_os = "linux") {
            anyhow::bail!("pets overlay is only implemented for WSL-hosted Codex sessions");
        }
        if !is_wsl() {
            anyhow::bail!("pets overlay currently requires WSL so it can bridge to Windows");
        }

        let command = helper_command(codex_self_exe)?;
        let terminal_window_hint = env::var("WT_SESSION").ok();
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            if let Err(err) = bridge_task(command, config, terminal_window_hint, rx).await {
                tracing::warn!(error = %err, "pets overlay bridge exited");
            }
        });
        Ok(Self { tx: Some(tx) })
    }

    pub fn toggle(&self) {
        let Some(tx) = self.tx.as_ref() else {
            return;
        };
        let _ = tx.send(PetsRequest::Toggle);
    }

    pub fn update(&self, snapshot: PetsSnapshot) {
        let Some(tx) = self.tx.as_ref() else {
            return;
        };
        let _ = tx.send(PetsRequest::Update(snapshot));
    }
}

impl Drop for PetsClient {
    fn drop(&mut self) {
        let Some(tx) = self.tx.take() else {
            return;
        };
        let _ = tx.send(PetsRequest::Shutdown);
    }
}

fn helper_command(codex_self_exe: Option<&Path>) -> Result<Command> {
    let script = resolve_helper_script(codex_self_exe)?;
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
    config: PetsRuntimeConfig,
    terminal_window_hint: Option<String>,
    mut rx: mpsc::UnboundedReceiver<PetsRequest>,
) -> Result<()> {
    let mut child = command.spawn().context("failed to spawn pets helper")?;
    let Some(mut stdin) = child.stdin.take() else {
        anyhow::bail!("pets helper stdin was unavailable");
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<HelperEvent>(&line) {
                    Ok(HelperEvent::Ready) => tracing::debug!("pets helper ready"),
                    Ok(HelperEvent::Hidden) => tracing::debug!("pets helper hidden"),
                    Ok(HelperEvent::Error { message }) => {
                        tracing::warn!(%message, "pets helper reported an error");
                    }
                    Err(err) => {
                        tracing::debug!(error = %err, line, "failed to parse pets helper output");
                    }
                }
            }
        });
    }
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(%line, "pets helper stderr");
            }
        });
    }

    let mut last_snapshot = None;
    while let Some(request) = rx.recv().await {
        let command = match request {
            PetsRequest::Toggle => HelperCommand::Show {
                pet: config.selected_pet.clone(),
                terminal_window_hint: terminal_window_hint.clone(),
            },
            PetsRequest::Update(snapshot) => {
                if Some(&snapshot) == last_snapshot.as_ref() {
                    continue;
                }
                last_snapshot = Some(snapshot.clone());
                HelperCommand::SetSnapshot {
                    snapshot: HelperSnapshot {
                        pet: config.selected_pet.clone(),
                        state: snapshot.state,
                        title: snapshot.title,
                        subtitle: snapshot.subtitle,
                        detail: snapshot.detail,
                        notification_count: snapshot.notification_count,
                    },
                }
            }
            PetsRequest::Shutdown => HelperCommand::Shutdown,
        };
        let line = serde_json::to_string(&command)?;
        stdin.write_all(line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        if matches!(command, HelperCommand::Shutdown) {
            break;
        }
    }

    let status = child
        .wait()
        .await
        .context("failed waiting for pets helper")?;
    if !status.success() {
        anyhow::bail!("pets helper exited with status {status}");
    }
    Ok(())
}

fn resolve_helper_script(codex_self_exe: Option<&Path>) -> Result<String> {
    let locations = resolve_helper_locations(codex_self_exe)?;
    let legacy_helper = windows_path_literal(&locations.legacy_helper)?;
    let electron_app_dir = windows_path_literal(&locations.electron_app_dir)?;
    let electron_portable = windows_path_literal(&locations.electron_portable)?;
    let electron_unpacked = windows_path_literal(&locations.electron_unpacked)?;
    let electron_cmd = windows_path_literal(&locations.electron_cmd)?;

    Ok(format!(
        "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; \
         if (Test-Path -LiteralPath {electron_unpacked}) {{ & {electron_unpacked} }} \
         elseif (Test-Path -LiteralPath {electron_portable}) {{ & {electron_portable} }} \
         elseif (Test-Path -LiteralPath {electron_cmd}) {{ & {electron_cmd} {electron_app_dir} }} \
         else {{ & {legacy_helper} }}"
    ))
}

struct HelperLocations {
    legacy_helper: PathBuf,
    electron_app_dir: PathBuf,
    electron_portable: PathBuf,
    electron_unpacked: PathBuf,
    electron_cmd: PathBuf,
}

fn resolve_helper_locations(codex_self_exe: Option<&Path>) -> Result<HelperLocations> {
    let Some(codex_self_exe) = codex_self_exe else {
        anyhow::bail!("Codex executable path is unavailable for pets helper lookup");
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
    let Some(workspace_dir) = target_dir.parent() else {
        anyhow::bail!("failed to derive workspace directory from Codex executable path");
    };
    let electron_app_dir = workspace_dir.join(ELECTRON_HELPER_DIR);
    if !helper_path.exists() && !electron_app_dir.exists() {
        anyhow::bail!(
            "pets helper was not found at {} or {}",
            helper_path.display(),
            electron_app_dir.display()
        );
    }

    Ok(HelperLocations {
        legacy_helper: linux_path_to_windows_launch_path(&helper_path)?,
        electron_portable: linux_path_to_windows_launch_path(
            &electron_app_dir.join("dist").join(WINDOWS_HELPER_EXE),
        )?,
        electron_unpacked: linux_path_to_windows_launch_path(
            &electron_app_dir
                .join("dist")
                .join("win-unpacked")
                .join(WINDOWS_HELPER_EXE),
        )?,
        electron_cmd: linux_path_to_windows_launch_path(
            &electron_app_dir
                .join("node_modules")
                .join(".bin")
                .join("electron.cmd"),
        )?,
        electron_app_dir: linux_path_to_windows_launch_path(&electron_app_dir)?,
    })
}

fn linux_path_to_windows_launch_path(path: &Path) -> Result<PathBuf> {
    let path_str = path
        .to_str()
        .context("path used for pets helper lookup is not valid UTF-8")?;
    if let Some(mapped) = wsl_mount_to_windows_path(path_str) {
        return Ok(PathBuf::from(mapped));
    }

    let distro = env::var("WSL_DISTRO_NAME")
        .context("WSL_DISTRO_NAME is not set; cannot construct Windows path for helper")?;
    Ok(PathBuf::from(wsl_localhost_launch_path(path_str, &distro)))
}

fn windows_path_literal(path: &Path) -> Result<String> {
    let path = path
        .to_str()
        .context("pets helper path is not valid UTF-8")?;
    Ok(pwsh_quote(path))
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
    let path = path.trim_start_matches('/').replace('/', "\\");
    format!("\\\\wsl.localhost\\{distro}\\{path}")
}

fn pwsh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn is_wsl() -> bool {
    env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|release| release.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn wsl_mount_to_windows_path_maps_drive_mounts() {
        assert_eq!(
            wsl_mount_to_windows_path("/mnt/c/Users/vract/codex-pets.exe"),
            Some("C:\\Users\\vract\\codex-pets.exe".to_string())
        );
    }

    #[test]
    fn wsl_localhost_path_maps_linux_paths() {
        assert_eq!(
            wsl_localhost_launch_path("/home/vracto/rpc-codex/target/debug/helper.exe", "Ubuntu"),
            "\\\\wsl.localhost\\Ubuntu\\home\\vracto\\rpc-codex\\target\\debug\\helper.exe"
        );
    }

    #[test]
    fn powershell_quotes_single_quotes() {
        assert_eq!(pwsh_quote("a'b"), "'a''b'");
    }
}
