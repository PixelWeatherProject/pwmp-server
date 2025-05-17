use crate::error::Error;
use std::time::SystemTime;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

struct DateTimeFormatter;

impl FormatTime for DateTimeFormatter {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = SystemTime::now();
        let datetime: chrono::DateTime<chrono::Local> = now.into();
        write!(w, "{}", datetime.format("%d.%m.%Y/%H:%M:%S"))
    }
}

pub fn setup() -> Result<(), Error> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_timer(DateTimeFormatter)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
