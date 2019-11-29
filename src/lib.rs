#[derive(Debug, Clone)]
pub struct Command<'a> {
    cmd: &'a str,
    arg: &'a str,
}

impl Command<'_> {
    pub fn cmd(&self) -> &str {
        self.cmd
    }
    pub fn arg(&self) -> &str {
        self.arg
    }
}

pub fn parse_params(input: &str) -> Command<'_> {
    let first_space = input.find(char::is_whitespace);
    match first_space {
        Some(first_space) => Command {
            cmd: &input[..first_space],
            arg: &input[first_space..],
        },
        None => Command {
            cmd: input,
            arg: "",
        },
    }
}
