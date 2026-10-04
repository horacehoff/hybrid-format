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

pub enum Sign {
    Plus,
    Minus,
}

struct FormatSpec {
    align: Option<Alignment>,
    fill: Option<char>,
    sign: Option<Sign>,
    width: Option<Quantity>,
    precision: Option<Precision>,
}

enum FormatPart {
    Text(String),
    Placeholder { arg: Argument, spec: FormatSpec },
}
