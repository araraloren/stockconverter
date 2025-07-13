fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app.ico");
        res.compile().unwrap();
    }
}
