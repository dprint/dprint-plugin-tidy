fn main() {
  // tidy's parsers and printers are recursive, and the 1 MiB stack Rust gives a
  // Wasm module overflows at a few hundred levels of nesting in some languages
  // (ex. YAML), which traps and stops the whole `dprint fmt` run
  if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
    println!("cargo:rustc-link-arg-cdylib=-zstack-size=8388608");
  }
}
