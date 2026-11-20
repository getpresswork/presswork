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

#[must_use]
pub fn now() -> Option<Datetime> {
    let time = jiff::Timestamp::now().to_zoned_datetime(jiff::tz::TimeZone::UTC);
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
    fn the_clock_gives_the_date_and_the_time() {
        assert!(today().is_some());
        assert!(now().is_some_and(|time| time.hour().is_some()));
    }
}
