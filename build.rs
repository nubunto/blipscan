// compiles the C shim
fn main() {
    println!("cargo:rerun-if-changed=src/shim.c");
    cc::Build::new().file("src/shim.c").compile("shim");
}
