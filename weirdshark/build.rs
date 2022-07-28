fn main(){
    println!("cargo:rustc-link-search=all=./lib");
    println!("cargo:rustc-link-search=all=./lib/x64");
    println!("cargo:rustc-link-lib=static=packet");
}