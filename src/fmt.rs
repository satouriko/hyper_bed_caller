use rtdlib::types::*;
use std::convert::TryInto;

const HELP_TEXT: &str = "点击查看帮助。";
const HELP_URL: &str = "https://telegra.ph/%E4%BD%BF%E7%94%A8%E5%B8%AE%E5%8A%A9-11-29";

pub fn build_fmt_message<T>(f: T) -> InputMessageContent
where
    T: Fn(&mut RTDFormattedTextBuilder) -> &mut RTDFormattedTextBuilder,
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

pub fn f_help_message(f: &mut RTDFormattedTextBuilder) -> &mut RTDFormattedTextBuilder {
    f.text(HELP_TEXT);
    let url = TextEntityTypeTextUrl::builder().url(HELP_URL).build();
    let url_entity = TextEntity::builder()
        .type_(TextEntityType::TextUrl(url))
        .offset(0)
        .length(HELP_TEXT.encode_utf16().count().try_into().unwrap())
        .build();
    let entities = vec![url_entity];
    f.entities(entities);
    return f;
}

pub fn f_bad_cron_expression(f: &mut RTDFormattedTextBuilder) -> &mut RTDFormattedTextBuilder {
    let text = "无效的表达式。";
    f.text(format!("{}{}", text, HELP_TEXT));
    let url = TextEntityTypeTextUrl::builder().url(HELP_URL).build();
    let url_entity = TextEntity::builder()
        .type_(TextEntityType::TextUrl(url))
        .offset(text.encode_utf16().count().try_into().unwrap())
        .length(HELP_TEXT.encode_utf16().count().try_into().unwrap())
        .build();
    let entities = vec![url_entity];
    f.entities(entities);
    return f;
}
