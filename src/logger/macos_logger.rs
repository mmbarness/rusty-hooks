
#[cfg(feature="mac")]
pub trait MacOsLogger {
    fn load() {
        OsLogger::new("com.example.test")
            .level_filter(LevelFilter::Debug)
            .category_level_filter("Settings", LevelFilter::Trace)
            .init()
            .unwrap();
    }
}