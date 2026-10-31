fn main() {
    // Ikona i opis pliku .exe (Eksplorator, pasek zadań, „Aplikacje i funkcje”).
    println!("cargo:rerun-if-changed=icon.ico");
    println!("cargo:rerun-if-env-changed=OPENMP_VERSION");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::path::Path::new("icon.ico").exists() {
        let mut res = winresource::WindowsResource::new();
        let ver = std::env::var("OPENMP_VERSION").unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").unwrap());
        res.set_icon("icon.ico")
            .set("FileDescription", "OpenMP")
            .set("ProductName", "OpenMP")
            .set("CompanyName", "Adam Rędzikowski")
            .set("LegalCopyright", "Adam Rędzikowski · redzikowski.com")
            .set("ProductVersion", &ver)
            .set("FileVersion", &ver);
        if let Err(e) = res.compile() {
            println!("cargo:warning=ikona .exe pominięta: {e}");
        }
    }
    slint_build::compile_with_config("ui/main.slint", slint_build::CompilerConfiguration::new().with_style("fluent-dark".into())).expect("slint");
}
