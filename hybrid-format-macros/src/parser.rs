use core::{iter::Peekable, str::Chars};

#[derive(PartialEq, Eq)]
pub enum Argument {
    NextPositional,
    Index(usize),
    Expression(String),
}

#[derive(PartialEq, Eq)]
enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(PartialEq, Eq)]
enum Quantity {
    Int(usize),
    Parameter(Argument),
}

#[derive(PartialEq, Eq)]
enum Precision {
    Quantity(Quantity),
    Asterisk,
}

#[derive(PartialEq, Eq)]
enum Sign {
    Plus,
    Minus,
}

#[derive(Default, PartialEq, Eq)]
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

#[derive(Default, PartialEq, Eq)]
pub struct FormatSpec {
    fill: Option<char>,
    align: Option<Alignment>,
    sign: Option<Sign>,
    alternate_form: bool,
    zero_padding: bool,
    width: Option<Quantity>,
    precision: Option<Precision>,
    format_type: FormatType,
}

pub enum FormatPart {
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
                    format_parts.push(FormatPart::Text(core::mem::take(&mut current_text)));
                }
                format_parts.push(parse_formatting_argument(&mut chars)?);
            }
            _ => current_text.push(c),
        }
    }
    if !current_text.is_empty() {
        format_parts.push(FormatPart::Text(current_text));
    }
    Ok(format_parts)
}

fn parse_formatting_argument(chars: &mut Peekable<Chars>) -> Result<FormatPart, String> {
    let (expr, has_spec) = parse_argument(chars)?;
    Ok(FormatPart::Placeholder {
        arg: if expr.is_empty() {
            Argument::NextPositional
        } else {
            Argument::Expression(expr)
        },
        spec: if has_spec {
            parse_spec(chars)?
        } else {
            FormatSpec::default()
        },
    })
}

/// Result<(expr, `has_spec`), Error>
fn parse_argument(chars: &mut Peekable<Chars>) -> Result<(String, bool), String> {
    let mut expr = String::with_capacity(4);
    let mut expr_depth: usize = 0;
    while let Some(c) = chars.next() {
        match c {
            // '::'
            ':' if expr_depth == 0 && chars.next_if_eq(&':').is_some() => expr.push(':'),
            '}' | ':' if expr_depth == 0 => return Ok((expr.trim().to_owned(), c == ':')),
            '(' | '[' | '{' => expr_depth += 1,
            ')' | ']' | '}' => expr_depth = expr_depth.saturating_sub(1),
            // skip strings
            '"' => {
                expr.push(c);
                while let Some(c) = chars.next().filter(|&c| c != '"') {
                    expr.push(c);
                    if c == '\\' {
                        expr.extend(chars.next());
                    }
                }
            }
            _ => {}
        }
        expr.push(c);
    }
    Err("unclosed `{` in format string, use `{{` to print `{`".into())
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
fn parse_quantity(chars: &mut Peekable<Chars>) -> Result<Option<Quantity>, String> {
    let mut chars_dup = chars.clone();
    let is_int = chars_dup.peek().is_some_and(char::is_ascii_digit);
    let word: String = core::iter::from_fn(|| {
        chars_dup.next_if(|&c| {
            if is_int {
                c.is_ascii_digit()
            } else {
                c.is_alphanumeric() || c == '_'
            }
        })
    })
    .collect();
    let is_parameter = chars_dup.next_if_eq(&'$').is_some();
    let int = || {
        word.parse::<usize>()
            .map_err(|_| format!("`{word}` is not a valid `usize`"))
    };
    let quantity = match (is_int, is_parameter) {
        (true, true) => Quantity::Parameter(Argument::Index(int()?)),
        (true, false) => Quantity::Int(int()?),
        (false, true) if !word.is_empty() => Quantity::Parameter(Argument::Expression(word)),
        _ => return Ok(None),
    };
    *chars = chars_dup;
    Ok(Some(quantity))
}

fn parse_spec(chars: &mut Peekable<Chars>) -> Result<FormatSpec, String> {
    let mut chars_dup = chars.clone().take(2);
    let (fst, snd) = (chars_dup.next(), chars_dup.next());
    let (fill, align) = match (snd.and_then(parse_alignment), fst.and_then(parse_alignment)) {
        (Some(align), _) => (fst, Some(align)),
        (None, align) => (None, align),
    };
    if fill.is_some() {
        chars.next();
    }
    if align.is_some() {
        chars.next();
    }
    let sign = chars
        .next_if(|&c| c == '+' || c == '-')
        .map(|c| if c == '+' { Sign::Plus } else { Sign::Minus });
    let alternate_form = chars.next_if_eq(&'#').is_some();

    let mut chars_dup = chars.clone().take(2);
    let zero_padding = chars_dup.next() == Some('0') && chars_dup.next() != Some('$');
    if zero_padding {
        chars.next();
    }
    let width = parse_quantity(chars)?;
    let precision = if chars.next_if_eq(&'.').is_some() {
        if chars.next_if_eq(&'*').is_some() {
            Some(Precision::Asterisk)
        } else {
            match parse_quantity(chars) {
                Ok(Some(q)) => Some(Precision::Quantity(q)),
                _ => None,
            }
        }
    } else {
        None
    };

    let type_str: String =
        core::iter::from_fn(|| chars.next_if(|&c| c != '}' && !c.is_whitespace())).collect();
    let format_type = parse_type(&type_str)?;
    while chars.next_if(|c| c.is_whitespace()).is_some() {}
    match chars.next() {
        Some('}') => Ok(FormatSpec {
            fill,
            align,
            sign,
            alternate_form,
            zero_padding,
            width,
            precision,
            format_type,
        }),
        Some(unexpected) => Err(format!("unexpected `{unexpected}` in format spec")),
        None => Err("unclosed `{` in format string, use `{{` to print `{`".into()),
    }
}
