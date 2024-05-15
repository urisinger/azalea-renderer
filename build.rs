use std::{env, io::Write, process::Command};

fn main() {
    println!(
        "cargo:rustc-env=TEXTURE_PATH={}",
        "/home/uri_singer/Downloads/cobblestone.png"
    );

    println!("cargo:rerun-if-changed=shaders/src/");

    println!("cargo:rerun-if-changed=shaders/Cargo.toml");

    println!("cargo:rerun-if-changed=shaders/.cargo");

    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders")).unwrap();
    let output = Command::new("cargo")
        .args([
            "rustc",
            "--color",
            "always",
            "--",
            "--emit",
            "link=shaders.spv.json",
        ])
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTC")
        .env_remove("CARGO")
        .env_remove("RUSTDOC")
        .env_remove("LD_LIBRARY_PATH")
        .output()
        .unwrap();

    std::io::stdout().write(&output.stdout).unwrap();

    std::io::stdout().flush().unwrap();

    if !output.status.success() {
        std::io::stderr().write(&output.stderr).unwrap();
        panic!(
            "error compiling shader lib, cargo exsited with code: {}",
            output.status
        );
    }

    let shader_path = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders", "/shaders.spv");

    println!("cargo:rustc-env=shaders.spv={}", shader_path);
}
