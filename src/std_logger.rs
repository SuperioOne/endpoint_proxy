use log::{Record, Metadata, max_level};
use chrono::Local;

pub struct StdLogger;

impl log::Log for StdLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let time_str = Local::now().format("%Y-%m-%dT%H:%M:%S");
            println!("{0} {1:<8}: {2}", time_str, record.level(), record.args())
        }
    }

    fn flush(&self) {}
}
