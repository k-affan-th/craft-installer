fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Craft Installer");
        res.set("FileDescription", "Craft Installer");
        res.compile().expect("embedding icon failed");
    }
    println!("cargo:rerun-if-changed=assets/icon.ico");
}
