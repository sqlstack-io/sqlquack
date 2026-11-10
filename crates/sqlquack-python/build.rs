//! The module finds the DuckDB library copied next to it, in its package.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS");
    let origin = match target_os.as_deref() {
        Ok("macos") => "@loader_path",
        _ => "$ORIGIN",
    };
    println!("cargo:rustc-link-arg-cdylib=-Wl,-rpath,{origin}");
    // With DuckDB compiled in, libduckdb-sys places the archive that calls DuckDB's static extension
    // loader after the one that defines it. GNU ld does not look back, so the module would load with
    // the loader unresolved; marking it undefined up front keeps it.
    if std::env::var_os("CARGO_FEATURE_BUNDLED").is_some() && target_os.as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,--undefined=duckdb_register_static_extensions");
        // The extensions the loader registers sit before it as well. Remove these once a duckdb-rs
        // release includes https://github.com/duckdb/duckdb-rs/pull/945, which fixes the order.
        for extension in ["core_functions", "parquet", "json", "icu"] {
            println!("cargo:rustc-link-arg=-Wl,--undefined=duckdb_extension_{extension}_describe");
        }
    }
}
