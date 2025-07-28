use std::env;
use std::path::PathBuf;

fn main() {
    // Get the target directory
    let _out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    
    // Build ButterComp2 C++ wrapper with cross-compilation support
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("cpp/buttercomp2.cpp")
        .include("cpp")
        .flag_if_supported("-std=c++11")
        .flag_if_supported("-O3")
        .flag_if_supported("-ffast-math");
    
    // Configure cross-compilation settings
    if target.contains("windows") {
        if target.contains("gnu") {
            // MinGW cross-compilation
            build.compiler("x86_64-w64-mingw32-g++");
        }
        // Windows-specific flags
        build.flag_if_supported("-static-libgcc");
        build.flag_if_supported("-static-libstdc++");
    } else if target.contains("apple") {
        // macOS-specific flags
        if target.contains("aarch64") {
            // Apple Silicon
            build.flag_if_supported("-arch").flag_if_supported("arm64");
        } else {
            // Intel Mac
            build.flag_if_supported("-arch").flag_if_supported("x86_64");
        }
        build.flag_if_supported("-mmacosx-version-min=10.9");
    } else if target.contains("linux") {
        // Linux-specific flags (already handled by default)
    }
    
    build.compile("buttercomp2");
    
    // Tell cargo to link the library
    println!("cargo:rustc-link-lib=static=buttercomp2");
    
    // Tell cargo to rerun build script if cpp files change
    println!("cargo:rerun-if-changed=cpp/buttercomp2.cpp");
    println!("cargo:rerun-if-changed=cpp/buttercomp2.h");
    println!("cargo:rerun-if-changed=build.rs");
}