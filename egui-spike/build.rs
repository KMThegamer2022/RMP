// Links the existing engine (src/lib.rs) against libvlc.
// macOS: the libvlc inside VLC.app (override with VLC_APP=/path/to/VLC.app).
// Windows: the VideoLAN SDK import library (VLC_DIR=extracted vlc-3.0.x win64 folder).
fn main() {
    println!("cargo:rerun-if-env-changed=VLC_APP");
    println!("cargo:rerun-if-env-changed=VLC_DIR");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let dir = std::env::var("VLC_DIR").unwrap_or_else(|_| "vlc-3.0.24".into());
        println!("cargo:rustc-link-search=native={dir}/sdk/lib");
        println!("cargo:rustc-link-lib=dylib=libvlc");
        return;
    }
    let app = std::env::var("VLC_APP").unwrap_or_else(|_| "/Applications/VLC.app".into());
    let lib = format!("{app}/Contents/MacOS/lib");
    println!("cargo:rustc-link-search=native={lib}");
    println!("cargo:rustc-link-lib=dylib=vlc");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{lib}");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/lib");
}
