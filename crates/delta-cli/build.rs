#[path = "src/nvim_install.rs"]
mod nvim_install;

use std::path::PathBuf;

fn main() {
    if std::env::var_os("DOCS_RS").is_some()
        || std::env::var_os("DELTA_SKIP_NVIM_INSTALL").is_some()
    {
        return;
    }

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo = nvim_install::repo_root_from_cli_manifest(&manifest);
    let plugin = repo.join("neovim").join("delta.nvim");
    rerun_if_changed(&plugin);
    rerun_if_changed(&manifest.join("src"));

    match nvim_install::install_plugin(&repo) {
        Ok(dest) => eprintln!("delta: plugin -> {}", dest.display()),
        Err(err) => eprintln!("delta: plugin install skipped ({err})"),
    }

    if let Some(home) = nvim_install::evs_delta_home() {
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let src = nvim_install::profile_bin_path(&out);
        let dest = home.join("bin").join(if cfg!(windows) {
            "delta-cli.exe"
        } else {
            "delta-cli"
        });
        nvim_install::schedule_cli_copy(src, dest);
    }
}

fn rerun_if_changed(path: &std::path::Path) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                rerun_if_changed(&entry.path());
            }
        }
        return;
    }
    println!("cargo:rerun-if-changed={}", path.display());
}
