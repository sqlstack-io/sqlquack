//! The module finds the DuckDB library copied next to it, in its package.

fn main() {
    let origin = match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "@loader_path",
        _ => "$ORIGIN",
    };
    println!("cargo:rustc-link-arg-cdylib=-Wl,-rpath,{origin}");
}
