use std::{iter::Peekable, str::Chars};

enum Argument {
    NextPositional,
    Index(usize),
    Expression(String),
}

enum Alignment {
    Left,
    Center,
    Right,
}

enum Quantity {
    Int(usize),
    Parameter(Argument),
}

enum Precision {
    Quantity(Quantity),
    Asterisk,
}

enum Sign {
    Plus,
    Minus,
}

#[derive(Default)]
enum FormatType {
    #[default]
    Display,
    Debug,
    DebugLowerHex,
    DebugUpperHex,
    Octal,
    LowerHex,
    UpperHex,
    Pointer,
    Binary,
    LowerExp,
    UpperExp,
}

struct FormatSpec {
    fill: Option<char>,
    align: Option<Alignment>,
    sign: Option<Sign>,
    alternate_form: bool,
    zero_padding: bool,
    width: Option<Quantity>,
    precision: Option<Precision>,
    format_type: FormatType,
}

enum FormatPart {
    Text(String),
    Placeholder { arg: Argument, spec: FormatSpec },
}

pub fn parse_format_string(string: &str) -> Result<Vec<FormatPart>, String> {
    let mut chars = string.chars().peekable();
    let mut format_parts = Vec::with_capacity(2);
    let mut current_text = String::with_capacity(4);
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                current_text.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                current_text.push('}');
            }
            '}' => return Err("unmatched `}` in format string, use `}}` to print `}`".into()),
            '{' => {
                if !current_text.is_empty() {
                    format_parts.push(FormatPart::Text(core::mem::take(&mut current_text)))
                }
            }
            _ => current_text.push(c),
        }
    }
    if !current_text.is_empty() {
        format_parts.push(FormatPart::Text(current_text))
    }
    Ok(format_parts)
}

const fn parse_alignment(c: char) -> Option<Alignment> {
    match c {
        '<' => Some(Alignment::Left),
        '^' => Some(Alignment::Center),
        '>' => Some(Alignment::Right),
        _ => None,
    }
}

fn parse_type(type_str: &str) -> Result<FormatType, String> {
    match type_str {
        "" => Ok(FormatType::Display),
        "?" => Ok(FormatType::Debug),
        "x?" => Ok(FormatType::DebugLowerHex),
        "X?" => Ok(FormatType::DebugUpperHex),
        "o" => Ok(FormatType::Octal),
        "x" => Ok(FormatType::LowerHex),
        "X" => Ok(FormatType::UpperHex),
        "p" => Ok(FormatType::Pointer),
        "b" => Ok(FormatType::Binary),
        "e" => Ok(FormatType::LowerExp),
        "E" => Ok(FormatType::UpperExp),
        other => Err(format!("unknown format type `{other}`")),
    }
}
