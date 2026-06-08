use std::env;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();

    if target.contains("windows") || target.contains("win32") || target.contains("win64") {
        // Cross-compilation from Linux: use windres to compile the .rc script and link it
        let out_dir = env::var("OUT_DIR").unwrap();
        let res_obj = std::path::Path::new(&out_dir).join("app_icon.o");

        let rc_status = std::process::Command::new("x86_64-w64-mingw32-windres")
            .args(&["app.rc", "-o"])
            .arg(&res_obj)
            .status()
            .expect("Failed to run windres — is x86_64-w64-mingw32-windres installed?");

        assert!(rc_status.success(), "windres failed to compile app.rc");

        // Link the COFF object file into the binary
        println!("cargo:rustc-link-arg={}", res_obj.display());
        println!("cargo:rerun-if-changed=app-icon.ico");
        println!("cargo:rerun-if-changed=app.rc");
    }
    #[cfg(target_os = "windows")]
    {
        // Native Windows build: use winres
        let mut res = winres::WindowsResource::new();
        res.set_icon("app-icon.ico");
        res.compile()
            .expect("Failed to embed Windows icon resource");
    }
}
