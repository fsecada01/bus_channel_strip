use std::env;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Build ButterComp2 C++ wrapper
    cc::Build::new()
        .cpp(true)
        .file("cpp/buttercomp2.cpp")
        .include("cpp")
        .flag_if_supported("-std=c++11")
        .flag_if_supported("-O3")
        .flag_if_supported("-ffast-math")
        .compile("buttercomp2");
    
    // Tell cargo to link the library
    println!("cargo:rustc-link-lib=static=buttercomp2");
    
    // Tell cargo to rerun build script if cpp files change
    println!("cargo:rerun-if-changed=cpp/buttercomp2.cpp");
    println!("cargo:rerun-if-changed=cpp/buttercomp2.h");
    println!("cargo:rerun-if-changed=build.rs");
}