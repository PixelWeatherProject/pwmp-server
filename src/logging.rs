use crate::error::Error;
use std::time::SystemTime;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::{
    format::{FmtSpan, Writer},
    time::FormatTime,
};

struct DateTimeFormatter;

impl FormatTime for DateTimeFormatter {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = SystemTime::now();
        let datetime: chrono::DateTime<chrono::Local> = now.into();
        write!(w, "{}", datetime.format("%d.%m.%Y/%H:%M:%S"))
    }
}

pub fn setup(debug: bool) -> Result<(), Error> {
    let level = if cfg!(debug_assertions) | debug {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_timer(DateTimeFormatter)
        .with_target(false)
        .with_max_level(level)
        .with_span_events(FmtSpan::CLOSE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
