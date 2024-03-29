use crate::alarm::{get_next_schedule, AsScheduleRef};
use crate::store::Alarm;
use chrono::TimeZone;
use rand::prelude::*;
use rtdlib::types::*;
use std::convert::TryInto;

const HELP_TEXT: &str = "点击查看帮助。";
const HELP_URL: &str = "https://telegra.ph/%E4%BD%BF%E7%94%A8%E5%B8%AE%E5%8A%A9-11-29";
const ANSWER_MAP: [&'static str; 4] = [
  "零一二三四五六七八九十",
  "〇一二三四五六七八九十",
  "零壹贰叁肆伍陆柒捌玖拾",
  "洞幺两三四五六拐怕勾叉",
];

pub fn build_fmt_message<T>(f: T) -> InputMessageContent
where
  T: Fn(&mut RTDFormattedTextBuilder) -> (),
{
  let mut builder = FormattedText::builder();
  f(&mut builder);
  InputMessageContent::InputMessageText(
    InputMessageText::builder()
      .text(builder.build())
      .clear_draft(true)
      .build(),
  )
}

pub fn build_plain_message<T>(s: T) -> InputMessageContent
where
  T: AsRef<str>,
{
  InputMessageContent::InputMessageText(
    InputMessageText::builder()
      .text(FormattedText::builder().text(s).build())
      .clear_draft(true)
      .build(),
  )
}

pub fn f_about_message(f: &mut RTDFormattedTextBuilder) {
  let mut text = String::from("是\u{1f980}女仆。");
  let url_text = "点击查看源代码。";
  let href = format!(
    "https://github.com/rikakomoe/hyper_bed_caller/tree/{}",
    env!("COMMIT_SHA")
  );
  let url = TextEntityTypeTextUrl::builder().url(&href).build();
  let url_entity = TextEntity::builder()
    .type_(TextEntityType::TextUrl(url))
    .offset(text.encode_utf16().count().try_into().unwrap())
    .length(url_text.encode_utf16().count().try_into().unwrap())
    .build();
  text += url_text;
  f.text(text);
  let entities = vec![url_entity];
  f.entities(entities);
}

pub fn f_help_message(f: &mut RTDFormattedTextBuilder) {
  f.text(HELP_TEXT);
  let url = TextEntityTypeTextUrl::builder().url(HELP_URL).build();
  let url_entity = TextEntity::builder()
    .type_(TextEntityType::TextUrl(url))
    .offset(0)
    .length(HELP_TEXT.encode_utf16().count().try_into().unwrap())
    .build();
  let entities = vec![url_entity];
  f.entities(entities);
}

pub fn f_bad_arguments<T>(f: &mut RTDFormattedTextBuilder, text: T)
where
  T: AsRef<str>,
{
  f.text(format!("{}{}", text.as_ref(), HELP_TEXT));
  let url = TextEntityTypeTextUrl::builder().url(HELP_URL).build();
  let url_entity = TextEntity::builder()
    .type_(TextEntityType::TextUrl(url))
    .offset(text.as_ref().encode_utf16().count().try_into().unwrap())
    .length(HELP_TEXT.encode_utf16().count().try_into().unwrap())
    .build();
  let entities = vec![url_entity];
  f.entities(entities);
}

pub fn f_list_alarms<'a, Z>(
  f: &'a mut RTDFormattedTextBuilder,
  alarms: &Vec<Alarm>,
  tz: Z,
  chat_id: i64,
) where
  Z: TimeZone + 'static,
{
  let mut text = String::default();
  let mut entities: Vec<TextEntity> = vec![];
  let mut have_expired = false;
  let now = tz.from_utc_datetime(&chrono::Local::now().naive_utc());
  for (i, alarm) in alarms.iter().enumerate() {
    if chat_id < 0 && alarm.chat_id != chat_id {
      continue;
    }
    let num = format!("[{}]", i);
    let bold = TextEntityTypeBold::builder().build();
    let bold_entity = TextEntity::builder()
      .type_(TextEntityType::Bold(bold))
      .offset(text.encode_utf16().count().try_into().unwrap())
      .length(num.encode_utf16().count().try_into().unwrap())
      .build();
    text += &format!("{}  ", num);
    entities.push(bold_entity);
    if alarm.is_informing != 0 {
      text += "#进行中  ";
    } else {
      let next_alarm = get_next_schedule(&alarm.cron, &now);
      if !next_alarm.has_schedule() {
        text += "#已过期  ";
        have_expired = true;
      }
    }
    if alarm.is_disabled {
      text += "#已禁用  ";
    }
    if alarm.title != "" {
      text += &format!("{}  ", alarm.title);
    }
    if alarm.is_strict {
      text += "#严格模式  ";
    }
    let cron = &alarm.cron[2..]; // remove zero for 'second'
    let code = TextEntityTypeCode::builder().build();
    let code_entity = TextEntity::builder()
      .type_(TextEntityType::Code(code))
      .offset(text.encode_utf16().count().try_into().unwrap())
      .length(cron.encode_utf16().count().try_into().unwrap())
      .build();
    text += cron;
    entities.push(code_entity);
    text += "\n";
  }
  if have_expired {
    text += "\nTip：使用命令 #purge 清除所有已过期的闹钟。"
  }
  if text == "" {
    text += "还一个闹钟都没有呢。";
    if alarms.len() > 0 {
      text += "回私聊中试试吧。"
    }
  }
  f.text(text);
  f.entities(entities);
}

pub fn generate_strict_challenge() -> (String, String, String) {
  let mut rng = rand::thread_rng();
  let mut challenge = String::default();
  let map_i = rng.gen_range(0, ANSWER_MAP.len());
  let mut answer = String::default();
  for _ in 0..30 {
    let number = rng.gen_range(0, 11);
    let number_string = number.to_string();
    let num_str = match number_string.as_str() {
      "10" => "X",
      s => s,
    };
    challenge += num_str;
    let char_vec: Vec<char> = ANSWER_MAP[map_i].chars().collect();
    let answer_string = char_vec[number].to_string();
    answer += &answer_string;
  }
  return (challenge, answer, String::from(ANSWER_MAP[map_i]));
}

pub fn f_strict_challenge<T>(f: &mut RTDFormattedTextBuilder, challenge: T, challenge_map: T)
where
  T: AsRef<str>,
{
  let mut text = format!(
    "请用汉字“{}”输入下面的数字以关闭闹钟：\n",
    challenge_map.as_ref()
  );
  let mut entities: Vec<TextEntity> = vec![];
  let code = TextEntityTypeCode::builder().build();
  let code_entity = TextEntity::builder()
    .type_(TextEntityType::Code(code))
    .offset(text.encode_utf16().count().try_into().unwrap())
    .length(
      challenge
        .as_ref()
        .encode_utf16()
        .count()
        .try_into()
        .unwrap(),
    )
    .build();
  text += challenge.as_ref();
  entities.push(code_entity);
  f.text(text);
  f.entities(entities);
}

pub fn f_help_alarm<T>(f: &mut RTDFormattedTextBuilder, name: T, user_id: i64, is_discard: bool)
where
  T: AsRef<str>,
{
  let name = name.as_ref();
  let mut text = if is_discard {
    format!("群友们，妹抖酱叫不醒")
  } else {
    format!("群友们，妹抖酱叫不到")
  };
  let mut entities: Vec<TextEntity> = vec![];
  let mention = TextEntityTypeMentionName::builder()
    .user_id(user_id)
    .build();
  let mention_entity = TextEntity::builder()
    .type_(TextEntityType::MentionName(mention))
    .offset(text.encode_utf16().count().try_into().unwrap())
    .length(name.encode_utf16().count().try_into().unwrap())
    .build();
  entities.push(mention_entity);
  text += name;
  text += if is_discard {
    "了，快来帮忙催一下！"
  } else {
    "了，快想办法用别的方式（比如电话）叫一下！！！"
  };
  f.text(text);
  f.entities(entities);
}
