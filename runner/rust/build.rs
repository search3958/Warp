use std::{env, path::PathBuf, process::Command};

fn run(command: &mut Command, label: &str) {
    let status = command.status().unwrap_or_else(|error| panic!("{label} could not start: {error}"));
    if !status.success() { panic!("{label} failed with status {status}"); }
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let source = root.join("src/scriptvm/mb5_vm.c");
    let object = out.join("mb5_vm.o");
    let archive = out.join("libmb5vm.a");
    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", root.join("src/scriptvm/mb5_vm.h").display());
    run(Command::new(cc).arg("-std=c99").arg("-O2").arg("-fPIC").arg("-Wall").arg("-Wextra").arg("-Werror")
        .arg("-c").arg(&source).arg("-o").arg(&object), "C script VM compiler");
    run(Command::new(ar).arg("crus").arg(&archive).arg(&object), "C script VM archiver");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=mb5vm");
    println!("cargo:rustc-link-lib=m");
}
