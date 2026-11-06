//! With DuckDB compiled in, Linux links must keep DuckDB's static extension loader.

fn main() {
    // libduckdb-sys places the archive that calls the loader after the one that defines it, and GNU
    // ld does not look back, so the link fails. Marking the loader undefined up front keeps it.
    if std::env::var_os("CARGO_FEATURE_BUNDLED").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
    {
        println!("cargo:rustc-link-arg=-Wl,--undefined=duckdb_register_static_extensions");
    }
}
