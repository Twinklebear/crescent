extern crate ispc;

fn main() {
    let mut cfg = ispc::Config::new();
    let ispc_files = vec!["src/crescent.ispc"];
    for s in &ispc_files[..] {
        cfg.file(*s);
    }
    cfg.compile("crescent");
}

