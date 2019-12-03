use crate::store::Alarm;
use chrono::{self, prelude::*};
use cron;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug)]
pub struct AlarmSchedule<'a, Z: TimeZone> {
  inner: Option<(DateTime<Z>, &'a Alarm)>,
}

#[derive(Debug)]
pub struct AlarmScheduleMut<'a, Z: TimeZone> {
  inner: Option<(DateTime<Z>, &'a mut Alarm)>,
}

type AlarmScheduleMutRef<'a, Z> = Option<(&'a DateTime<Z>, &'a Alarm)>;

pub trait AsAlarmScheduleRef<Z>
where
  Z: TimeZone + 'static,
{
  fn as_ref(&self) -> AlarmScheduleMutRef<Z>;
  fn schedule(&self) -> ScheduleRef<'_, Z> {
    match self.as_ref() {
      None => ScheduleRef::default(),
      Some(alarm_schedule) => ScheduleRef::new(&alarm_schedule.0),
    }
  }
  fn alarm(&self) -> Option<&Alarm> {
    match self.as_ref() {
      None => None,
      Some(alarm_schedule) => Some(&alarm_schedule.1),
    }
  }
  fn alarm_title(&self) -> String {
    match self.as_ref() {
      None => String::default(),
      Some(alarm_schedule) => alarm_schedule.1.title.clone(),
    }
  }
}

impl<Z> AsAlarmScheduleRef<Z> for AlarmScheduleMut<'_, Z>
where
  Z: TimeZone + 'static,
{
  fn as_ref(&self) -> AlarmScheduleMutRef<Z> {
    match &self.inner {
      None => None,
      Some(alarm_schedule) => Some((&alarm_schedule.0, alarm_schedule.1)),
    }
  }
}

impl<'a, Z> AlarmScheduleMut<'a, Z>
where
  Z: TimeZone,
{
  pub fn as_immut(self) -> AlarmSchedule<'a, Z> {
    AlarmSchedule {
      inner: match self.inner {
        None => None,
        Some(alarm_schedule) => Some((alarm_schedule.0, alarm_schedule.1)),
      },
    }
  }
  pub fn alarm_mut(&mut self) -> Option<&mut Alarm> {
    match &mut self.inner {
      None => None,
      Some(alarm_schedule) => Some(&mut alarm_schedule.1),
    }
  }
  pub fn default() -> AlarmScheduleMut<'static, Z> {
    AlarmScheduleMut { inner: None }
  }
  pub fn new(schedule: DateTime<Z>, alarm: &mut Alarm) -> AlarmScheduleMut<Z> {
    AlarmScheduleMut {
      inner: Some((schedule, alarm)),
    }
  }
}

impl<Z> AsAlarmScheduleRef<Z> for AlarmSchedule<'_, Z>
where
  Z: TimeZone + 'static,
{
  fn as_ref(&self) -> AlarmScheduleMutRef<Z> {
    match &self.inner {
      None => None,
      Some(alarm_schedule) => Some((&alarm_schedule.0, alarm_schedule.1)),
    }
  }
}

impl<Z> AlarmSchedule<'_, Z>
where
  Z: TimeZone + 'static,
{
  pub fn default() -> AlarmSchedule<'static, Z> {
    AlarmSchedule { inner: None }
  }
  pub fn new(schedule: DateTime<Z>, alarm: &Alarm) -> AlarmSchedule<Z> {
    AlarmSchedule {
      inner: Some((schedule, alarm)),
    }
  }
}

pub fn get_recent_schedule<Z>(alarms: &Vec<Alarm>, timezone: Z, chat_id: i64) -> AlarmSchedule<Z>
where
  Z: TimeZone + 'static,
{
  let next_timestamp = 0;
  let mut recent = AlarmSchedule::default();
  for alarm in alarms.iter() {
    if chat_id < 0 && alarm.chat_id != chat_id {
      continue;
    }
    let next_alarm = get_next_schedule(&alarm.cron, timezone.clone());
    let t = next_alarm.to_timestamp();
    if t >= 0 && (next_timestamp == 0 || t < next_timestamp) {
      recent = AlarmSchedule::new(next_alarm.inner.unwrap(), alarm);
    }
  }
  return recent;
}

pub fn get_recent_schedule_mut<Z>(alarms: &mut Vec<Alarm>, timezone: Z) -> AlarmScheduleMut<Z>
where
  Z: TimeZone + 'static,
{
  let next_timestamp = 0;
  let mut recent = AlarmScheduleMut::default();
  for alarm in alarms.iter_mut() {
    let next_alarm = get_next_schedule(&alarm.cron, timezone.clone());
    let t = next_alarm.to_timestamp();
    if t >= 0 && (next_timestamp == 0 || t < next_timestamp) {
      recent = AlarmScheduleMut::new(next_alarm.inner.unwrap(), alarm);
    }
  }
  return recent;
}

pub trait AsScheduleRef<Z>
where
  Z: TimeZone + 'static,
{
  fn as_ref(&self) -> ScheduleRef<Z>;
  fn has_schedule(&self) -> bool {
    match self.as_ref().inner.as_ref() {
      Some(_) => true,
      None => false,
    }
  }
  fn to_timestamp(&self) -> i64 {
    match self.as_ref().inner.as_ref() {
      Some(schedule) => schedule.timestamp(),
      None => -1,
    }
  }
}

pub trait AsPrintableScheduleRef<Z>: AsScheduleRef<Z>
where
  Z: TimeZone + 'static,
  Z::Offset: Display,
{
  fn to_string(&self) -> Option<String> {
    match self.as_ref().inner.as_ref() {
      Some(schedule) => Some(schedule.format("%F %R%:z").to_string()),
      None => None,
    }
  }
}

#[derive(Debug, Copy)]
pub struct ScheduleRef<'a, Z: TimeZone + 'static> {
  inner: Option<&'a DateTime<Z>>,
}

impl<Z> ScheduleRef<'_, Z>
where
  Z: TimeZone,
{
  pub fn new<'a>(datetime: &'a DateTime<Z>) -> ScheduleRef<'a, Z> {
    ScheduleRef {
      inner: Some(datetime),
    }
  }
  pub fn default() -> ScheduleRef<'static, Z> {
    ScheduleRef { inner: None }
  }
}

impl<'a, Z> Clone for ScheduleRef<'a, Z>
where
  Z: TimeZone + 'static,
{
  fn clone(&self) -> ScheduleRef<'a, Z> {
    match &self.inner {
      None => ScheduleRef::default(),
      Some(datetime) => ScheduleRef::new(datetime),
    }
  }
}

impl<Z> AsScheduleRef<Z> for ScheduleRef<'_, Z>
where
  Z: TimeZone + 'static,
{
  fn as_ref(&self) -> ScheduleRef<Z> {
    self.clone()
  }
}

impl<Z> AsPrintableScheduleRef<Z> for ScheduleRef<'_, Z>
where
  Z: TimeZone + 'static,
  Z::Offset: Display,
{
}

#[derive(Debug)]
pub struct Schedule<Z: TimeZone> {
  inner: Option<DateTime<Z>>,
}

impl<Z> Schedule<Z>
where
  Z: TimeZone,
{
  pub fn new(datetime: DateTime<Z>) -> Schedule<Z> {
    Schedule {
      inner: Some(datetime),
    }
  }
  pub fn default() -> Schedule<Z> {
    Schedule { inner: None }
  }
}

impl<Z> AsScheduleRef<Z> for Schedule<Z>
where
  Z: TimeZone + 'static,
{
  fn as_ref(&self) -> ScheduleRef<Z> {
    match &self.inner {
      None => ScheduleRef::default(),
      Some(datetime) => ScheduleRef::new(&datetime),
    }
  }
}

impl<Z> AsPrintableScheduleRef<Z> for Schedule<Z>
where
  Z: TimeZone + 'static,
  Z::Offset: Display,
{
}

pub fn get_next_schedule<T, Z>(cron: T, timezone: Z) -> Schedule<Z>
where
  T: AsRef<str>,
  Z: TimeZone,
{
  let schedule = cron::Schedule::from_str(cron.as_ref()).unwrap();
  for datetime in schedule.upcoming(timezone).take(1) {
    return Schedule::new(datetime);
  }
  return Schedule::default();
}
