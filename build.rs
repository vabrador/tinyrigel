
// OS-dependent build-steps.
// ===
    
// Windows
// ---
//
// On Windows, we generate bindings using a subproject with its own build step. The subproject then exports Windows API bindings generated via windows-rs.
#[cfg(target_os = "windows")]
fn main() { }

// macOS / iOS
// ---
//
// On macOS, we need to include Framework dependencies for accessing video devices.
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
//
// On Linux, we generate bindings using a subproject with its own build step.
#[cfg(target_os = "linux")]
fn main() { }
