use chrono::{self, prelude::*};
use cron::Schedule;
use std::str::FromStr;

pub fn get_next_schedule<Z>(cron: &str, timezone: Z) -> Option<DateTime<Z>>
where
    Z: TimeZone,
{
    let schedule = Schedule::from_str(cron).unwrap();
    for datetime in schedule.upcoming(timezone).take(1) {
        return Some(datetime);
    }
    return None;
}
