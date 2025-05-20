use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    process::Command,
};

const HEADER_WRAPPER: &str = "./vendor/wrapper.h";
const MODULE_DIR: &str = "./vendor/isa-l";

fn main() {
    // Set rerun-if-changed
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor");

    // Set libraries to link
    if cfg!(feature = "link_static") {
        println!("-- Linking static libraries");
        println!("cargo:rustc-link-lib=static=isal");
    } else {
        println!("cargo:rustc-link-lib=isal");
    }

    // Build libraries
    if cfg!(feature = "bundle") {
        if !cfg!(feature = "link_static") {
            println!(
                "cargo::warning=It is discouraged to link shared libraries when bundling. See more: https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-search"
            );
        }
        build_from_source();
    } else {
        // try to link the system libraries
        bindgen_sys();
    }
}

fn bindgen_sys() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let binding_out_dir = out_dir.join("bindings");
    let wrapper_file_path = PathBuf::from(HEADER_WRAPPER);
    let out_file_path = binding_out_dir.join("isal.rs");

    println!(
        "-- Creating bindings directory {}",
        binding_out_dir.display()
    );
    std::fs::create_dir_all(&binding_out_dir).expect("fail to create bindings directory");

    // bindgen
    println!(
        "-- Generate bindings: {} => {}",
        wrapper_file_path.display(),
        out_file_path.display()
    );

    bindgen::Builder::default()
        .header(wrapper_file_path.to_str().unwrap())
        .allowlist_item("gf_.*")
        .allowlist_item("ec_.*")
        .impl_debug(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate_comments(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_file_path)
        .expect("Couldn't write bindings!");
}

fn build_from_source() {
    build_isal();
    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
            .join("lib")
            .canonicalize()
            .unwrap()
            .display()
    );
    // Make bindings
    bindgen_isal();
}

fn build_isal() {
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
        .args(&["-c", "./autogen.sh"])
        .output()
        .unwrap();
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
        .build();

    // cleanup the build dir
    println!("-- Removing build directory {}", build_dir.display());
    std::fs::remove_dir_all(build_dir).unwrap();
}

fn bindgen_isal() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let binding_out_dir = out_dir.join("bindings");
    let local_include_path = out_dir.join("include");
    let header_wrapper = PathBuf::from(HEADER_WRAPPER);
    let out_file = binding_out_dir.join("isal.rs");

    println!(
        "-- Creating bindings directory {}",
        binding_out_dir.display()
    );
    std::fs::create_dir_all(&binding_out_dir).expect("fail to create bindings directory");

    println!(
        "-- Generate bindings: {} => {}",
        header_wrapper.display(),
        out_file.display()
    );
    println!("---- Local include dir: {}", local_include_path.display());
    bindgen::Builder::default()
        .clang_args(["-isystem", local_include_path.to_str().unwrap()])
        .header(header_wrapper.to_str().unwrap())
        .impl_debug(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_item("gf_.*")
        // .allowlist_item("ec_.*")
        .generate_comments(true)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_file)
        .expect("Couldn't write bindings!");
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
