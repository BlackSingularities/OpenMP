//! Wbudowuje katalogi tłumaczeń `lang/<kod>/LC_MESSAGES/openmp.po` (te same co dla Slint).
use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("lang");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut langs: Vec<(String, String)> = Vec::new();
    if let Ok(dirs) = std::fs::read_dir(&root) {
        for d in dirs.flatten() {
            let po = d.path().join("LC_MESSAGES").join("openmp.po");
            if po.is_file() {
                println!("cargo:rerun-if-changed={}", po.display());
                let code = d.file_name().to_string_lossy().into_owned();
                let abs = po.canonicalize().unwrap_or(po);
                langs.push((code, abs.to_string_lossy().trim_start_matches(r"\\?\").to_string()));
            }
        }
    }
    langs.sort();
    let mut out = String::from("/// Wbudowane katalogi: (kod języka, treść pliku .po).\npub static CATALOGS: &[(&str, &str)] = &[\n");
    for (code, path) in &langs {
        out.push_str(&format!("    ({code:?}, include_str!({path:?})),\n"));
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("catalogs.rs");
    std::fs::write(dest, out).expect("catalogs.rs");
}
