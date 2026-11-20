use jiff::Timestamp;
use typst::foundations::Datetime;

#[must_use]
pub fn today() -> Option<Datetime> {
    let date = jiff::ZonedDateTime::now().date();
    Datetime::from_ymd(
        i32::from(date.year()),
        u8::try_from(date.month()).ok()?,
        u8::try_from(date.day()).ok()?,
    )
}

pub(crate) fn datetime(timestamp: Timestamp) -> Option<Datetime> {
    let time = timestamp.to_zoned_datetime(jiff::tz::TimeZone::UTC);
    Datetime::from_ymd_hms(
        i32::from(time.year()),
        u8::try_from(time.month()).ok()?,
        u8::try_from(time.day()).ok()?,
        u8::try_from(time.hour()).ok()?,
        u8::try_from(time.minute()).ok()?,
        u8::try_from(time.second()).ok()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_gives_the_date() {
        assert!(today().is_some());
    }

    #[test]
    fn a_timestamp_becomes_a_typst_date_and_time_in_utc() {
        let time = datetime(Timestamp::from_second(1_795_209_243).unwrap()).unwrap();

        assert_eq!(
            (
                time.year(),
                time.month(),
                time.day(),
                time.hour(),
                time.minute()
            ),
            (Some(2026), Some(11), Some(20), Some(21), Some(14))
        );
    }
}
