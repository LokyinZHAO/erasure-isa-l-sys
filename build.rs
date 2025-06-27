use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    process::Command,
};

const ISAL_HEADERS: [&str; 7] = [
    // #include <crc.h>
    // #include <crc64.h>
    // #include <erasure_code.h>
    // #include <gf_vect_mul.h>
    // #include <igzip_lib.h>
    // #include <mem_routines.h>
    // #include <raid.h>
    "crc.h",
    "crc64.h",
    "erasure_code.h",
    "gf_vect_mul.h",
    "igzip_lib.h",
    "mem_routines.h",
    "raid.h",
];
// const HEADER_WRAPPER: &str = "./vendor/wrapper.h";
const MODULE_DIR: &str = "./vendor/isa-l";

fn main() {
    // Set rerun-if-changed
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor");

    // Build libraries
    #[cfg(feature = "from_system")]
    // try to link the system libraries
    match bindgen_sys() {
        Ok(_) => return,
        Err(e) => {
            println!(
                "cargo::warning=Failed to link from system libisal: {e}, falling back to building from source",
            );
        }
    }
    build_from_source().expect("Failed to build from source");
}

#[cfg(feature = "from_system")]
fn bindgen_sys() -> Result<(), String> {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let binding_out_dir = out_dir.join("bindings");
    let out_file_path = binding_out_dir.join("isal.rs");

    println!(
        "-- Creating bindings directory {}",
        binding_out_dir.display()
    );
    std::fs::create_dir_all(&binding_out_dir).expect("fail to create bindings directory");

    let libisal = pkg_config::Config::new()
        .atleast_version("2.0.0")
        .probe("libisal")
        .map_err(|e| format!("Failed to find libisal: {e}"))?;
    println!(
        "-- Found libisal: {} (version: {})",
        libisal.libs.first().unwrap(),
        libisal.version
    );

    let headers = libisal
        .include_paths
        .iter()
        .map(|p| p.join("isa-l.h"))
        .filter(|p| p.exists())
        .map(|p| p.to_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    println!(
        "-- Generate system bindings: [{}] => {}",
        headers.join(", "),
        out_file_path.display()
    );

    bindgen::Builder::default()
        .headers(headers)
        .allowlist_item("gf_.*")
        .allowlist_item("ec_.*")
        .impl_debug(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate_comments(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_file_path)
        .map_err(|e| format!("Couldn't write bindings: {e}"))?;

    // Link the library
    libisal.link_paths.iter().for_each(|path| {
        println!(
            "cargo:rustc-link-search=native={}",
            path.canonicalize().unwrap().display()
        );
    });
    if cfg!(feature = "link_static") {
        println!("-- Linking static libraries");
        println!("cargo:rustc-link-lib=static=isal");
    } else {
        println!("-- Linking dynamic libraries");
        println!("cargo:rustc-link-lib=isal");
    }

    Ok(())
}

fn build_from_source() -> Result<(), String> {
    if !cfg!(feature = "link_static") {
        println!(
            "cargo::warning=Linking to libisal.a instead of libisal.so when building from source. Consider add feature: link_static. See more: https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-search"
        );
    }
    build_isal()?;
    // Make bindings
    bindgen_isal()?;
    // Link the library
    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("lib")
            .canonicalize()
            .unwrap()
            .display()
    );
    println!("cargo:rustc-link-lib=static=isal");
    Ok(())
}

fn build_isal() -> Result<(), String> {
    const LIB_NAME: &str = "isal";

    // Submodule directory containing upstream source files (readonly)
    let module_dir = std::fs::canonicalize(MODULE_DIR).expect("isa-l directory not found");

    // Copy source files to writable directory in `OUT_DIR`
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let src_dir = out_dir.join("src").join(LIB_NAME);
    let build_dir = out_dir.join("build");
    create_dir_all(&src_dir).unwrap_or_else(|_| panic!("Failed to create {}", src_dir.display()));
    println!("-- Copying isa-l source files to {}", src_dir.display());
    cp_r(module_dir, src_dir.clone());

    // Run `autoreconf`
    println!("sh: [./autogen.sh] in {}", src_dir.display());
    let output = Command::new("sh")
        .current_dir(src_dir.clone())
        .args(["-c", "./autogen.sh"])
        .output()
        .map_err(|e| format!("Failed to run autogen.sh: {e}"))?;
    println!("autogen.sh: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("autogen.sh: {}", String::from_utf8_lossy(&output.stderr));

    // Build using auto-tools
    println!(
        "sh: [./configure --prefix={}] in {}",
        build_dir.display(),
        src_dir.display()
    );
    let _install_root_dir = autotools::Config::new(src_dir)
        .enable_shared()
        .enable_static()
        .cflag("-O2")
        .try_build()
        .map_err(|e| format!("Failed to configure build: {e}"))?;

    // cleanup the build dir
    println!("-- Removing build directory {}", build_dir.display());
    std::fs::remove_dir_all(build_dir).unwrap();
    Ok(())
}

fn bindgen_isal() -> Result<(), String> {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let binding_out_dir = out_dir.join("bindings");
    let out_file = binding_out_dir.join("isal.rs");

    println!(
        "-- Creating bindings directory {}",
        binding_out_dir.display()
    );
    std::fs::create_dir_all(&binding_out_dir).expect("fail to create bindings directory");

    // let local_include_path = out_dir.join("include");
    // let header_wrapper = PathBuf::from(HEADER_WRAPPER);
    let isa_l_include_path = PathBuf::from(MODULE_DIR).join("include");
    let headers = ISAL_HEADERS
        .iter()
        .map(|header| isa_l_include_path.join(header))
        .map(|header| header.canonicalize().unwrap())
        .map(|header| header.to_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    println!(
        "-- Generate bindings: [{}] => {}",
        headers.join(", "),
        out_file.display()
    );
    bindgen::Builder::default()
        // .clang_args(["-isystem", local_include_path.to_str().unwrap()])
        .headers(headers)
        .impl_debug(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_item("gf_.*")
        .allowlist_item("ec_.*")
        .generate_comments(true)
        .generate()
        .map_err(|e| format!("Unable to generate bindings: {e}"))?
        .write_to_file(out_file)
        .map_err(|e| format!("Couldn't write bindings: {e}"))?;
    Ok(())
}

fn cp_r(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    for e in from.as_ref().read_dir().unwrap() {
        let e = e.unwrap();
        let from = e.path();
        let to = to.as_ref().join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&to).unwrap();
            cp_r(&from, &to);
        } else {
            println!("cp: {} => {}", from.display(), to.display());
            std::fs::copy(&from, &to).unwrap();
        }
    }
}
