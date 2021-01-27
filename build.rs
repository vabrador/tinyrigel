
// Windows
// ---

// This space is intentionally left blank.
//
// On Windows, we generate bindings using a subproject with its own build step. The subproject then exports Windows API bindings generated via windows-rs.

// macOS / iOS
// ---

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn main() {
    // macOS / iOS backend-specific Framework dependencies.
    if std::env::var("TARGET").unwrap().contains("-apple") {
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
        println!("cargo:rustc-link-lib=framework=CoreVideo");

        // Referenced this build.rs from tts-rs on github - that app also has an AppKit dependency, which I believe TinyRigel does not have. If at some point we need AppKit or other dependencies based on the configured target, we can add them similarly.
        // if !std::env::var("CARGO_CFG_TARGET_OS").unwrap().contains("ios") {
        //     println!("cargo:rustc-link-lib=framework=AppKit")
        // }
    }
}

// Linux
// ---

// This space is intentionally left blank.
//
// On Linux, we generate bindings using a subproject with its own build step.

fn main() { }
