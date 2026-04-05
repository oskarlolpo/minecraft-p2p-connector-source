use std::{env, fs, path::{Path, PathBuf}, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(embedded_ygg)");
    println!("cargo:rerun-if-env-changed=MC_EMBED_YGG");
    println!("cargo:rerun-if-env-changed=MC_YGGSTACK_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=MC_YGGSTACK_BRIDGE_CC");
    println!("cargo:rerun-if-env-changed=MC_YGGSTACK_BRIDGE_ARCHIVE_DIR");
    println!("cargo:rerun-if-env-changed=MC_YGGSTACK_BINARY");

    tauri_build::build();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    if !should_embed_ygg() {
        sync_sidecar_resource_if_needed(&manifest_dir);
        return;
    }

    remove_sidecar_resource_if_present(&manifest_dir);

    if let Some(archive_dir) = resolve_prebuilt_bridge_dir() {
        emit_embedded_bridge_linking(&archive_dir);
        println!(
            "cargo:warning=using prebuilt embedded yggstack bridge from {}",
            archive_dir.display()
        );
        return;
    }

    let source_dir = resolve_default_source_dir(&manifest_dir);
    let capi_dir = source_dir.join("contrib").join("capi");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("yggffi");

    println!("cargo:warning=MC_EMBED_YGG=1: preparing embedded yggstack bridge from {}", source_dir.display());
    if !capi_dir.exists() {
        println!(
            "cargo:warning=embedded yggstack bridge skipped: {} not found",
            capi_dir.display()
        );
        return;
    }

    if let Err(error) = std::fs::create_dir_all(&out_dir) {
        println!(
            "cargo:warning=embedded yggstack bridge skipped: cannot create {}: {}",
            out_dir.display(),
            error
        );
        return;
    }

    let archive_path = out_dir.join("yggstackbridge.a");
    let mut command = Command::new("go");
    command
        .arg("build")
        .arg("-buildmode=c-archive")
        .arg("-o")
        .arg(&archive_path)
        .arg("./contrib/capi")
        .current_dir(&source_dir)
        .env("CGO_ENABLED", "1");

    if let Some((cc, extra_path)) = resolve_c_compiler() {
        command.env("CC", cc);
        if let Some(extra_path) = extra_path {
            let existing_path = env::var_os("PATH").unwrap_or_default();
            let mut joined = std::ffi::OsString::new();
            joined.push(extra_path);
            joined.push(";");
            joined.push(existing_path);
            command.env("PATH", joined);
        }
    }

    match command.status() {
        Ok(status) if status.success() => {
            emit_embedded_bridge_linking(&out_dir);
            println!(
                "cargo:warning=embedded yggstack bridge archive built at {}",
                archive_path.display()
            );
        }
        Ok(status) => {
            println!(
                "cargo:warning=embedded yggstack bridge build failed with exit status {}",
                status
            );
        }
        Err(error) => {
            println!(
                "cargo:warning=embedded yggstack bridge build failed to start: {}",
                error
            );
        }
    }
}

fn sync_sidecar_resource_if_needed(manifest_dir: &Path) {
    let target = sidecar_resource_target(manifest_dir);

    if let Some(source) = resolve_sidecar_binary_source(manifest_dir) {
        if source == target {
            println!(
                "cargo:warning=using existing bundled yggstack sidecar at {}",
                target.display()
            );
            return;
        }

        if let Some(parent) = target.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                println!(
                    "cargo:warning=failed to create bundled yggstack resource dir {}: {}",
                    parent.display(),
                    error
                );
                return;
            }
        }

        match fs::copy(&source, &target) {
            Ok(_) => println!(
                "cargo:warning=bundled yggstack sidecar copied to {}",
                target.display()
            ),
            Err(error) => println!(
                "cargo:warning=failed to copy bundled yggstack sidecar {} -> {}: {}",
                source.display(),
                target.display(),
                error
            ),
        }
        return;
    }

    println!(
        "cargo:warning=no bundled yggstack sidecar source found, relying on embedded bridge only"
    );
}

fn remove_sidecar_resource_if_present(manifest_dir: &Path) {
    let target = sidecar_resource_target(manifest_dir);
    if !target.exists() {
        return;
    }

    match fs::remove_file(&target) {
        Ok(_) => println!(
            "cargo:warning=removed bundled yggstack sidecar {} because embedded bridge is enabled",
            target.display()
        ),
        Err(error) => println!(
            "cargo:warning=failed to remove bundled yggstack sidecar {}: {}",
            target.display(),
            error
        ),
    }
}

fn sidecar_resource_target(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .join("resources")
        .join("yggstack")
        .join(if cfg!(target_os = "windows") {
            "yggstack.exe"
        } else {
            "yggstack"
        })
}

fn resolve_sidecar_binary_source(manifest_dir: &Path) -> Option<PathBuf> {
    if let Ok(binary) = env::var("MC_YGGSTACK_BINARY") {
        let source = PathBuf::from(binary);
        if source.exists() {
            return Some(source);
        }

        println!(
            "cargo:warning=MC_YGGSTACK_BINARY points to missing file: {}",
            source.display()
        );
    }

    let target = sidecar_resource_target(manifest_dir);
    if target.exists() {
        return Some(target);
    }

    let source_dir = resolve_default_source_dir(manifest_dir);
    let command_dir = source_dir.join("cmd").join("yggstack");
    if !command_dir.exists() {
        return None;
    }

    let build_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("yggsidecar");
    if let Err(error) = fs::create_dir_all(&build_dir) {
        println!(
            "cargo:warning=failed to create temporary yggstack sidecar build dir {}: {}",
            build_dir.display(),
            error
        );
        return None;
    }

    let sidecar_binary = build_dir.join(if cfg!(target_os = "windows") {
        "yggstack.exe"
    } else {
        "yggstack"
    });

    if build_sidecar_from_source(&source_dir, &sidecar_binary).is_ok() {
        return Some(sidecar_binary);
    }

    None
}

fn build_sidecar_from_source(source_dir: &Path, binary_path: &Path) -> Result<(), ()> {
    println!(
        "cargo:warning=building bundled yggstack sidecar from {}",
        source_dir.display()
    );

    let mut command = Command::new("go");
    command
        .arg("build")
        .arg("-o")
        .arg(binary_path)
        .arg("./cmd/yggstack")
        .current_dir(source_dir);

    if let Some((cc, extra_path)) = resolve_c_compiler() {
        command.env("CC", cc);
        if let Some(extra_path) = extra_path {
            let existing_path = env::var_os("PATH").unwrap_or_default();
            let mut joined = std::ffi::OsString::new();
            joined.push(extra_path);
            joined.push(";");
            joined.push(existing_path);
            command.env("PATH", joined);
        }
    }

    match command.status() {
        Ok(status) if status.success() && binary_path.exists() => Ok(()),
        Ok(status) => {
            println!(
                "cargo:warning=failed to build bundled yggstack sidecar from {}: exit status {}",
                source_dir.display(),
                status
            );
            Err(())
        }
        Err(error) => {
            println!(
                "cargo:warning=failed to start go build for bundled yggstack sidecar from {}: {}",
                source_dir.display(),
                error
            );
            Err(())
        }
    }
}

fn resolve_default_source_dir(manifest_dir: &Path) -> PathBuf {
    env::var("MC_YGGSTACK_SOURCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            manifest_dir
                .parent()
                .and_then(|dir| dir.parent())
                .unwrap_or(manifest_dir)
                .join("newrepo")
                .join("yggstack-develop")
        })
}

fn resolve_prebuilt_bridge_dir() -> Option<PathBuf> {
    if let Ok(value) = env::var("MC_YGGSTACK_BRIDGE_ARCHIVE_DIR") {
        let path = PathBuf::from(value);
        if path.join("yggstackbridge.a").exists() {
            return Some(path);
        }
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let candidates = [
        manifest_dir.join("..").join(".runtime").join("ygg-ffi"),
        manifest_dir.join("ygg-ffi"),
    ];

    candidates.into_iter().find(|dir| dir.join("yggstackbridge.a").exists())
}

fn emit_embedded_bridge_linking(dir: &Path) {
    println!("cargo:rustc-cfg=embedded_ygg");
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=static=yggstackbridge");
    println!("cargo:rerun-if-changed={}", dir.join("yggstackbridge.a").display());
    if dir.join("yggstackbridge.h").exists() {
        println!("cargo:rerun-if-changed={}", dir.join("yggstackbridge.h").display());
    }
    for native_lib in ["ws2_32", "iphlpapi", "bcrypt", "userenv", "crypt32", "advapi32", "ntdll"] {
        println!("cargo:rustc-link-lib={native_lib}");
    }
}

fn should_embed_ygg() -> bool {
    match env::var("MC_EMBED_YGG") {
        Ok(value) if value == "0" => false,
        Ok(value) if value == "1" => true,
        _ => cfg!(target_os = "windows"),
    }
}

fn resolve_c_compiler() -> Option<(String, Option<String>)> {
    if let Ok(cc) = env::var("MC_YGGSTACK_BRIDGE_CC") {
        if !cc.trim().is_empty() {
            return Some((cc, None));
        }
    }

    let mingw_bin = PathBuf::from(r"C:\msys64\mingw64\bin");
    if mingw_bin.join("gcc.exe").exists() {
        return Some(("gcc".into(), Some(mingw_bin.display().to_string())));
    }

    None
}
