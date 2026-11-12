fn main() {
    // Embed the app icon as Windows resource ID 1; GPUI uses it for the
    // window and taskbar icon.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("failed to embed the Windows icon");
    }
}
