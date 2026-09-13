//! Copy the Neovim adapter and CLI into `~/.evs-delta` during `cargo build`.
//! Shared by `build.rs` and the running binary.
#![allow(dead_code)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn home_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("USERPROFILE") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    std::env::var("HOME")
        .ok()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

pub fn evs_delta_home() -> Option<PathBuf> {
    Some(home_dir()?.join(".evs-delta"))
}

pub fn repo_root_from_cli_manifest(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(manifest_dir)
        .to_path_buf()
}

pub fn install_plugin(repo_root: &Path) -> io::Result<PathBuf> {
    let dest = evs_delta_home()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME/USERPROFILE not set"))?
        .join("delta.nvim");
    let src = repo_root.join("neovim").join("delta.nvim");
    if !src.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("plugin sources missing at {}", src.display()),
        ));
    }
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    copy_dir(&src, &dest)?;
    Ok(dest)
}

pub fn install_cli(exe: &Path) -> io::Result<PathBuf> {
    let dest_dir = evs_delta_home()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME/USERPROFILE not set"))?
        .join("bin");
    fs::create_dir_all(&dest_dir)?;
    let name = if cfg!(windows) {
        "delta-cli.exe"
    } else {
        "delta-cli"
    };
    let dest = dest_dir.join(name);
    if dest.exists() {
        if let (Ok(src), Ok(dst)) = (exe.canonicalize(), dest.canonicalize()) {
            if src == dst {
                return Ok(dest);
            }
        }
        // A running Neovim may lock the dest exe. Rename it, then copy.
        let bak = dest.with_file_name(format!(
            "{}.old",
            dest.file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(name))
                .to_string_lossy()
        ));
        let _ = fs::remove_file(&bak);
        let _ = fs::rename(&dest, &bak);
        fs::copy(exe, &dest)?;
        let _ = fs::remove_file(&bak);
        return Ok(dest);
    }
    fs::copy(exe, &dest)?;
    Ok(dest)
}

/// Called from the built binary: refresh `~/.evs-delta/bin` from this exe.
pub fn install_current_exe() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    install_cli(&exe)
}

pub fn profile_bin_path(out_dir: &Path) -> PathBuf {
    // OUT_DIR = target/<profile>/build/<pkg>/out
    let profile_dir = out_dir.ancestors().nth(3).unwrap_or(out_dir);
    if cfg!(windows) {
        profile_dir.join("delta-cli.exe")
    } else {
        profile_dir.join("delta-cli")
    }
}

pub fn schedule_cli_copy(src: PathBuf, dest: PathBuf) {
    if std::env::var_os("DELTA_SKIP_NVIM_INSTALL").is_some() {
        return;
    }
    let _ = fs::create_dir_all(dest.parent().unwrap_or(Path::new(".")));
    // build.rs finishes before rustc links. Wait for an exe newer than this moment.
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().saturating_sub(2))
        .unwrap_or(0)
        .to_string();

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                "for ($i=0;$i -lt 240;$i++){Start-Sleep -Milliseconds 500; if (-not (Test-Path $env:DELTA_SRC)) { continue }; $after=[DateTimeOffset]::FromUnixTimeSeconds([int64]$env:DELTA_AFTER).LocalDateTime; if ((Get-Item $env:DELTA_SRC).LastWriteTime -lt $after) { continue }; New-Item -Force -ItemType Directory -Path (Split-Path $env:DELTA_DST) | Out-Null; $bak=\"$($env:DELTA_DST).old\"; if (Test-Path $env:DELTA_DST) { Move-Item -Force $env:DELTA_DST $bak -ErrorAction SilentlyContinue }; try { Copy-Item -Force $env:DELTA_SRC $env:DELTA_DST; Remove-Item -Force $bak -ErrorAction SilentlyContinue; exit 0 } catch {} }",
            ])
            .env("DELTA_SRC", &src)
            .env("DELTA_DST", &dest)
            .env("DELTA_AFTER", &after)
            .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
            .spawn();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("sh")
            .args([
                "-c",
                "for i in $(seq 1 240); do sleep 0.5; if [ ! -f \"$DELTA_SRC\" ]; then continue; fi; mtime=$(stat -c %Y \"$DELTA_SRC\" 2>/dev/null || stat -f %m \"$DELTA_SRC\"); if [ \"$mtime\" -lt \"$DELTA_AFTER\" ]; then continue; fi; mkdir -p \"$(dirname \"$DELTA_DST\")\"; cp -f \"$DELTA_SRC\" \"$DELTA_DST\" && exit 0; done",
            ])
            .env("DELTA_SRC", &src)
            .env("DELTA_DST", &dest)
            .env("DELTA_AFTER", &after)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

fn copy_dir(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
