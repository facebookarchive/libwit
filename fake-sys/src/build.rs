fn main() {
    // TODO: compile the library here using the cross-compiling toolchain
    println!("cargo:rustc-flags=-L lib/arm -l fake:static");
}
