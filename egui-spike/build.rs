// Links the existing engine (src/lib.rs) against the libvlc that ships inside VLC.app,
// the same runtime the shipping build uses. Override with VLC_APP=/path/to/VLC.app.
fn main() {
    println!("cargo:rerun-if-env-changed=VLC_APP");
    let app = std::env::var("VLC_APP").unwrap_or_else(|_| "/Applications/VLC.app".into());
    let lib = format!("{app}/Contents/MacOS/lib");
    println!("cargo:rustc-link-search=native={lib}");
    println!("cargo:rustc-link-lib=dylib=vlc");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{lib}");
}
