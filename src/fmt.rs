use rtdlib::types::*;
use std::convert::TryInto;

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
    let text = "点击查看帮助。";
    let url = "https://telegra.ph/%E4%BD%BF%E7%94%A8%E5%B8%AE%E5%8A%A9-11-29";
    f.text(text);
    let url = TextEntityTypeTextUrl::builder().url(url).build();
    let url_entity = TextEntity::builder()
        .type_(TextEntityType::TextUrl(url))
        .offset(0)
        .length(text.encode_utf16().count().try_into().unwrap())
        .build();
    let entities = vec![url_entity];
    f.entities(entities);
    return f;
}
