include!("./src/args/mod.rs");
use clap::CommandFactory;

fn main() {
    if cfg!(windows) {
        println!("cargo:rustc-link-search=all=../weirdshark/lib");
        println!("cargo:rustc-link-search=all=../weirdshark/lib/x64");
        println!("cargo:rustc-link-lib=static=packet");
    }

    // Automatically generate manual
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect(std::io::ErrorKind::NotFound.to_string().as_str())
    );

    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer).unwrap();

    std::fs::write(out_dir.join("weirdshark-cli.1"), buffer).unwrap();
}