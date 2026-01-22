use inquire::Text;
use std::io::IsTerminal;

pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}

pub fn prompt_text(prompt: &str, help: Option<&str>, placeholder: Option<&str>) -> Option<String> {
    let mut builder = Text::new(prompt);
    if let Some(h) = help {
        builder = builder.with_help_message(h);
    }
    if let Some(p) = placeholder {
        builder = builder.with_placeholder(p);
    }
    builder.prompt().ok().filter(|s| !s.is_empty())
}
