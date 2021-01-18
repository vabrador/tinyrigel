fn main() {
    // macOS / iOS backend-specific dependencies.
    if std::env::var("TARGET").unwrap().contains("-apple") {
        // Link in AVFoundation.
        println!("cargo:rustc-link-lib=framework=AVFoundation");

        // Referenced this build.rs from tts-rs on github - that app also has an AppKit dependency, which I believe TinyRigel does not have. If at some point we need AppKit or other dependencies based on the configured target, we can add them similarly.
        // if !std::env::var("CARGO_CFG_TARGET_OS").unwrap().contains("ios") {
        //     println!("cargo:rustc-link-lib=framework=AppKit")
        // }
    }
}
