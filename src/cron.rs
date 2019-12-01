use crate::store::Alarm;
use chrono::{self, prelude::*};
use cron::Schedule;
use std::fmt::Display;
use std::str::FromStr;

pub struct AlarmSchedule<'a, Z: TimeZone> {
    inner: Option<(DateTime<Z>, &'a Alarm)>,
}

impl<Z: TimeZone> AlarmSchedule<'_, Z> {
    pub fn default() -> AlarmSchedule<'static, Z> {
        AlarmSchedule { inner: None }
    }
    pub fn new(schedule: DateTime<Z>, alarm: &Alarm) -> AlarmSchedule<Z> {
        AlarmSchedule {
            inner: Some((schedule, alarm)),
        }
    }
    pub fn schedule(&self) -> Option<&DateTime<Z>> {
        match &self.inner {
            None => None,
            Some(alarm_schedule) => Some(&alarm_schedule.0),
        }
    }
    pub fn alarm_title(&self) -> String {
        match &self.inner {
            None => String::from(""),
            Some(alarm_schedule) => alarm_schedule.1.title.clone(),
        }
    }
}

pub fn get_recent_schedule<Z>(alarms: &Vec<Alarm>, timezone: Z) -> AlarmSchedule<Z>
where
    Z: TimeZone,
{
    let next_timestamp = 0;
    let mut recent = AlarmSchedule::<Z>::default();
    for alarm in alarms.iter() {
        let next_alarm = get_next_schedule(&alarm.cron, timezone.clone());
        let t = schedule_to_timestamp(next_alarm.as_ref());
        if t >= 0 && (next_timestamp == 0 || t < next_timestamp) {
            recent = AlarmSchedule::new(next_alarm.unwrap(), &alarm);
        }
    }
    return recent;
}

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

pub fn schedule_to_timestamp<Z>(schedule: Option<&DateTime<Z>>) -> i64
where
    Z: TimeZone,
{
    match schedule {
        Some(schedule) => schedule.timestamp(),
        None => -1,
    }
}

pub fn schedule_to_string<Z>(schedule: Option<&DateTime<Z>>) -> Option<String>
where
    Z: TimeZone,
    Z::Offset: Display,
{
    match schedule {
        Some(schedule) => Some(schedule.to_rfc3339()),
        None => None,
    }
}
