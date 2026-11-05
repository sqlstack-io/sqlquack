//! The binary finds DuckDB's shared library without a library path: in the directory the build
//! downloaded it to, or next to the binary when it is copied elsewhere with the library.

fn main() {
    let origin = match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => return,
        Ok("macos") => "@loader_path",
        _ => "$ORIGIN",
    };
    println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{origin}");
    if let Ok(library_directory) = std::env::var("DEP_DUCKDB_INCLUDE") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{library_directory}");
    }
}
