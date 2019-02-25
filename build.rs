extern crate ispc;

use std::env;
use std::path::PathBuf;

fn main() {
    let mut embree_include;
    if let Ok(e) = env::var("EMBREE_DIR") {
        embree_include = PathBuf::from(e);
        embree_include.push("include");
    } else {
        println!("cargo:error=Please set EMBREE_DIR=<path to embree3 root>");
        panic!("Failed to find embree");
    }
    println!("cargo:rerun-if-env-changed=EMBREE_DIR");

    let mut cfg = ispc::Config::new();
    let ispc_files = vec!["src/crescent.ispc"];
    for s in &ispc_files[..] {
        cfg.file(*s);
    }
    cfg.include_path(embree_include)
        .optimization_opt(ispc::OptimizationOpt::FastMath)
        .opt_level(2)
        .compile("crescent");
}

