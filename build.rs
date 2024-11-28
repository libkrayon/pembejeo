fn main() {
    // Link the IOKit framework
    #[cfg(target_os="macos")]
    println!("cargo:rustc-link-lib=framework=IOKit");
}