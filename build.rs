use std::path::PathBuf;

fn main() {
    let _ = embed_resource::compile("icon.rc", embed_resource::NONE);

    // Copy BASS DLLs to the output directory so the binary can find them at runtime.
    // The DLLs can be downloaded via `download-bass.ps1` (which puts them in target/release/).
    //
    // We look for DLLs in the project root first, then in target/release/ as fallback.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // Navigate: OUT_DIR = target/debug/build/hm-<hash>/out -> target/debug/
    let target_dir = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());

    let dll_names = ["bass.dll", "bass_fx.dll", "bassmidi.dll", "basswasapi.dll"];

    if let Some(target) = target_dir {
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let release_dir = manifest_dir.join("target").join("release");

        for name in &dll_names {
            // Check project root first
            let src = manifest_dir.join(name);
            // Fallback to target/release/
            let src = if src.exists() { src } else { release_dir.join(name) };

            if src.exists() {
                let dest = target.join(name);
                if !dest.exists() {
                    match std::fs::copy(&src, &dest) {
                        Ok(_) => println!("cargo:warning=Copied {} to output dir", name),
                        Err(e) => println!("cargo:warning=Failed to copy {}: {}", name, e),
                    }
                }
            }
        }
    }
}