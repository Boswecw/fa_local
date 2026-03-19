use chrono::{DateTime, Utc};

pub type TimestampUtc = DateTime<Utc>;

pub fn now_utc() -> TimestampUtc {
    Utc::now()
}
