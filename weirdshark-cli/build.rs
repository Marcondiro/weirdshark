fn main() {
    if cfg!(windows) {
        println!("cargo:rustc-link-search=all=../weirdshark/lib");
        println!("cargo:rustc-link-search=all=../weirdshark/lib/x64");
        println!("cargo:rustc-link-lib=static=packet");
    }
}