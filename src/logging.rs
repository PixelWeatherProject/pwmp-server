use crate::{config::Config, error::Error};
use std::{fs, time::SystemTime};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::{
        format::{FmtSpan, Writer},
        time::FormatTime,
    },
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

struct DateTimeFormatter;

impl FormatTime for DateTimeFormatter {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = SystemTime::now();
        let datetime: chrono::DateTime<chrono::Local> = now.into();
        write!(w, "{}", datetime.format("%d.%m.%Y/%H:%M:%S"))
    }
}

pub fn setup(force_debug: bool, config: &Config) -> Result<(), Error> {
    let level = if cfg!(debug_assertions) | force_debug {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let file_layer = if let Some(path) = &config.logging.file {
        if !path.is_absolute() {
            return Err(Error::IllegalLogfilePath);
        }

        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(config.logging.erase_file_on_start)
            .append(!config.logging.erase_file_on_start)
            .open(path)?;

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_ansi(false)
            .compact()
            .with_timer(DateTimeFormatter)
            .with_target(false)
            .with_span_events(FmtSpan::CLOSE);

        Some(file_layer)
    } else {
        None
    };

    let stdout_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_timer(DateTimeFormatter)
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .with(level)
        .init();

    Ok(())
}
