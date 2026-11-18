fn main() {
    // CI images may export the deprecated STATIC_VCRUNTIME, which makes tauri-build
    // warn. The setting lives in tauri.conf.json (default: static), so drop the variable.
    std::env::remove_var("STATIC_VCRUNTIME");

    // On Linux the tray daemon ships as a sidecar. tauri-build would fail with a
    // terse message when it is missing, so check first and say what to do.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        let target = std::env::var("TARGET").unwrap_or_default();
        let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let sidecar = dir.join("binaries").join(format!("decoy-daemon-{target}"));
        println!("cargo:rerun-if-changed={}", sidecar.display());
        // src/vblank.rs replaces libdrm's drmWaitVBlank for this process; that only
        // works if the executable exports the symbol.
        println!("cargo:rustc-link-arg-bins=-Wl,--export-dynamic-symbol=drmWaitVBlank");
        assert!(
            sidecar.is_file(),
            "missing {}\nrun `bash scripts/prepare-daemon.sh` first (add --debug for a quick debug build)",
            sidecar.display()
        );
    }

    tauri_build::build()
}
