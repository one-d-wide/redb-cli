use std::path::PathBuf;

fn main() {
    let src = "src/process_multimap.rs";

    println!("cargo::rerun-if-changed=src/grammar.pest");
    println!("cargo::rerun-if-changed={src}");

    let mut buf = String::new();
    for mut l in std::fs::read_to_string(src).unwrap().lines() {
        if let Some((_, r)) = l.split_once("// or") {
            l = r;
        }
        buf.push_str(l);
        buf.push('\n');
    }

    let s = buf
        .replace("_multimap", "")
        .replace("Multimap", "")
        .replace(
            "table.remove_all(&k)?.is_empty()",
            "table.remove(&k)?.is_none()",
        )
        .replace("v.is_empty()", "v.is_none()")
        // ...
        ;

    let out: PathBuf = std::env::var("OUT_DIR").unwrap().into();
    std::fs::write(out.join("process.rs"), s).unwrap();
}
