fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app-icon.ico");
        res.set("ProductName", "Nexus Explorer");
        res.set("FileDescription", "A blazing-fast file explorer");
        res.set("LegalCopyright", "Copyright © 2025");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
