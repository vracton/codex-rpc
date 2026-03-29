use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;

fn main() -> Result<()> {
    if env::var("CARGO_CFG_WINDOWS").is_err() {
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let sdk_root = manifest_dir
        .parent()
        .context("manifest dir has no parent")?
        .parent()
        .context("workspace dir has no parent")?
        .join("third_party/discord-social-sdk/windows");
    let lib_dir = sdk_root.join("lib");
    let bin_dir = sdk_root.join("bin");

    println!("cargo:rustc-link-lib=dylib=discord_partner_sdk");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let profile_dir = target_profile_dir(&out_dir)?;
    let dll_src = bin_dir.join("discord_partner_sdk.dll");
    let dll_dst = profile_dir.join("discord_partner_sdk.dll");
    fs::copy(&dll_src, &dll_dst).with_context(|| {
        format!(
            "failed to copy Discord SDK runtime from {} to {}",
            dll_src.display(),
            dll_dst.display()
        )
    })?;

    Ok(())
}

fn target_profile_dir(out_dir: &Path) -> Result<PathBuf> {
    out_dir
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .context("OUT_DIR did not contain a cargo target profile path")
}
