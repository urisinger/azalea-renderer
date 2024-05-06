use std::{
    env,
    io::{BufRead, BufReader},
    process::Command,
};

fn main() {
    println!(
        "cargo:rustc-env=TEXTURE_PATH={}",
        "/home/uri_singer/Downloads/cobblestone.png"
    );

    for env in std::env::vars() {
        println!("{}={}\n", env.0, env.1);
    }

    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders")).unwrap();
    let mut child = Command::new("cargo")
        .args(["rustc", "--", "--emit", "link=shaders.json"])
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTC")
        .env_remove("CARGO")
        .env_remove("RUSTDOC")
        .env_remove("LD_LIBRARY_PATH")
        .spawn()
        .expect("failed to spawn command");

    let shader_path = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders", "/shaders");

    println!("cargo:rustc-env=shaders.spv={}", shader_path);
}
