// This build script is used to compile the C++ code in the `cpp` directory.
// It's a bit of a hack, but it's the easiest way to get the C++ code to compile
// with the rest of the Rust code.

use std::env;
use std::process::Command;

fn set_build_date_env() {
    // Try to compute YYYYMMDD in a cross-platform way without external crates.
    // Fall back to "dev" if unavailable.
    let mut date_str: Option<String> = None;

    // Try PowerShell (Windows)
    if date_str.is_none() {
        if let Ok(output) = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Date).ToString('yyyyMMdd')",
            ])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                    date_str = Some(s);
                }
            }
        }
    }

    // Try POSIX date
    if date_str.is_none() {
        if let Ok(output) = Command::new("date").args(["+%Y%m%d"]).output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
                    date_str = Some(s);
                }
            }
        }
    }

    let val = date_str.unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=BUILD_DATE={}", val);
}

fn main() {
    // Always set BUILD_DATE so the version string can concatenate safely.
    set_build_date_env();

    // Only compile C++ if the `buttercomp2` feature is enabled.
    if env::var("CARGO_FEATURE_BUTTERCOMP2").is_err() {
        return;
    }

    // Unset problematic environment variables to prevent cc-rs from auto-reading them
    // and mis-parsing quoted paths. We will add flags and includes manually.
    env::remove_var("CXXFLAGS");
    env::remove_var("CFLAGS");

    let mut build = cc::Build::new();

    build
        .cpp(true)
        .include("cpp")
        .file("cpp/buttercomp2.cpp");

    // Manually parse flags from BINDGEN_EXTRA_CLANG_ARGS.
    // This is to work around a bug in how cc-rs parses CXXFLAGS with quoted paths.
    if let Ok(clang_args) = env::var("BINDGEN_EXTRA_CLANG_ARGS") {
        let parts: Vec<&str> = clang_args.split(" -I").collect();

        if let Some(first_part) = parts.get(0) {
            for flag in first_part.split_whitespace() {
                build.flag(flag);
            }
        }

        for part in &parts[1..] {
            build.include(part.trim_matches('"'));
        }
    }

    // Add platform-specific flags
    let target = env::var("TARGET").unwrap();

    if target.contains("windows-msvc") {
        // MSVC-specific settings
        build.flag("/std:c++17").flag("/EHsc");
    } else if target.contains("windows-gnu") {
        build
            .flag("-static-libgcc")
            .flag("-static-libstdc++");
    } else if target.contains("apple-darwin") {
        build
            .flag("-mmacosx-version-min=10.9")
            .cpp_link_stdlib("c++")
            .cpp_set_stdlib("c++");
    }

    build.compile("buttercomp2");

    // Link the C++ standard library
    if target.contains("windows-msvc") {
        // MSVC uses automatic linking for standard libraries
    } else if target.contains("windows-gnu") {
        println!("cargo:rustc-link-lib=static=stdc++");
    } else if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
