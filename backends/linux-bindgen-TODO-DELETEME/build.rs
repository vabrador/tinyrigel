use std::{env, path::PathBuf};

#[cfg(not(target_os = "linux"))]
compile_error!("linux-v4l2-sys only valid on linux (...or freebsd?)");

#[cfg(target_os = "linux")]
extern crate bindgen;

#[cfg(target_os = "linux")]
fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    // Very standard bindgen setup.
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings.");
    
    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("linux-bindgen.rs"))
        .expect("Couldn't write bindings!");
}
