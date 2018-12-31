extern crate ispc;

use std::env;
use std::path::PathBuf;

fn main() {
    let mut cfg = ispc::Config::new();
    let ispc_files = vec!["src/crescent.ispc"];
    for s in &ispc_files[..] {
        cfg.file(*s);
    }
    cfg.compile("crescent");

    if let Ok(e) = env::var("EMBREE_DIR") {
        let mut embree_dir = PathBuf::from(e);
        embree_dir.push("lib");
        println!("cargo:rustc-link-search=native={}", embree_dir.display());
    } else {
        println!("cargo:error=Please set EMBREE_DIR=<path to embree3 root>");
        panic!("Failed to find embree");
    }
    println!("cargo:rerun-if-env-changed=EMBREE_DIR");
    println!("cargo:rustc-link-lib=embree3");
}

