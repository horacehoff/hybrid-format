use crate::ConstFormattedString;

#[test]
fn int_const_literal() {
    assert_eq!(const { hformat!("{}", 42) }, format!("{}", 42));
}
#[test]
fn int_const_variable() {
    const MY_INT: i32 = 42;
    assert_eq!(const { hformat!("{}", MY_INT) }, format!("{MY_INT}"));
}
#[test]
fn int_dynamic() {
    let i = 2 + 2;
    assert_eq!(hformat!("{}", i), format!("{i}"));
}
#[test]
fn float_const_literal() {
    assert_eq!(
        const { hformat!("Float: {}", 4.2) },
        format!("Float: {}", 4.2)
    );
}
#[test]
fn float_const_variable() {
    const MY_FLOAT: f64 = 4.2;
    assert_eq!(
        const { hformat!("Float: {}", MY_FLOAT) },
        format!("Float: {MY_FLOAT}")
    );
}
#[test]
fn float_dynamic() {
    let f = 4.2 + 6.7;
    assert_eq!(hformat!("Float: {}", f), format!("Float: {f}"));
}
#[test]
fn bool_const_literal() {
    assert_eq!(
        const { hformat!("Bool: {}", true) },
        format!("Bool: {}", true)
    );
}
#[test]
fn bool_const_variable() {
    const MY_BOOL: bool = true;
    assert_eq!(
        const { hformat!("Bool: {}", MY_BOOL) },
        format!("Bool: {MY_BOOL}")
    );
}
#[test]
fn bool_dynamic() {
    let b = true;
    assert_eq!(hformat!("Bool: {}", b), format!("Bool: {b}"));
}
#[test]
fn char_const_literal() {
    assert_eq!(
        const { hformat!("Char: {}", 'a') },
        format!("Char: {}", 'a')
    );
}
#[test]
fn char_const_variable() {
    const MY_CHAR: char = 'a';
    assert_eq!(
        const { hformat!("Char: {}", MY_CHAR) },
        format!("Char: {MY_CHAR}")
    );
}
#[test]
fn char_dynamic() {
    let c = 'a';
    assert_eq!(hformat!("Char: {}", c), format!("Char: {c}"));
}
#[test]
fn str_const_literal() {
    assert_eq!(
        const { hformat!("String: {}", "Hello, world!") },
        format!("String: {}", "Hello, world!")
    );
}
#[test]
fn str_const_variable() {
    const MY_STR: &str = "Hello, world!";
    assert_eq!(
        const { hformat!("String: {}", MY_STR) },
        format!("String: {MY_STR}")
    );
}
#[test]
fn str_dynamic() {
    let s = "Hello, world!";
    assert_eq!(hformat!("String: {}", s), format!("String: {s}"));
}
#[test]
fn hybrid() {
    const MY_INT: i32 = 42;
    const MY_BOOL: bool = true;
    const MY_STR: &str = "Hello, world!";
    let f = 4.2 + 6.7;
    let c = 'a';
    assert_eq!(
        hformat!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}"),
        format!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}")
    );
}
#[test]
fn hybrid_hwrite() {
    const MY_INT: i32 = 42;
    const MY_BOOL: bool = true;
    const MY_STR: &str = "Hello, world!";
    let f = 4.2 + 6.7;
    let c = 'a';
    let mut buf = String::with_capacity(10);
    hwrite!(buf, "{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}");
    assert_eq!(buf, format!("{MY_INT}:{MY_BOOL}:{MY_STR}:{f}:{c}"));
}
#[test]
fn custom_compiletime_type() {
    struct Person {
        first_name: &'static str,
        last_name: &'static str,
        age: u8,
        balance: f64,
    }
    impl Person {
        const fn format(&self) -> ConstFormattedString<64> {
            ConstFormattedString::new()
                .push_str(self.last_name)
                .push_str(", ")
                .push_str(self.first_name)
                .push_str(" | Age: ")
                .push_u8(self.age)
                .push_str(" | $")
                .push_f64(self.balance)
        }
    }
    const RANDOM_GUY: Person = Person {
        first_name: "John",
        last_name: "Doe",
        age: 40,
        balance: 32.0,
    };
    const RANDOM_GUY_STR: ConstFormattedString<64> = RANDOM_GUY.format();
    const GREETING: &str = hformat!("Hello, {RANDOM_GUY_STR}!");
    const SAME_GREETING: &str = hformat!("Hello, {}!", const { RANDOM_GUY.format() });
    assert_eq!(GREETING, "Hello, Doe, John | Age: 40 | $32.0!");
    assert_eq!(GREETING, SAME_GREETING);
    let id = core::hint::black_box(0);
    assert_eq!(
        hformat!("#{id} - {RANDOM_GUY_STR}"),
        "#0 - Doe, John | Age: 40 | $32.0"
    );
}
#[test]
fn custom_compiletime_type_hwrite() {
    struct Person {
        first_name: &'static str,
        last_name: &'static str,
        age: u8,
        balance: f64,
    }
    impl Person {
        const fn format(&self) -> ConstFormattedString<64> {
            ConstFormattedString::new()
                .push_str(self.last_name)
                .push_str(", ")
                .push_str(self.first_name)
                .push_str(" | Age: ")
                .push_u8(self.age)
                .push_str(" | $")
                .push_f64(self.balance)
        }
    }
    const RANDOM_GUY: Person = Person {
        first_name: "John",
        last_name: "Doe",
        age: 40,
        balance: 32.0,
    };
    const RANDOM_GUY_STR: ConstFormattedString<64> = RANDOM_GUY.format();
    const GREETING: &str = hformat!("Hello, {RANDOM_GUY_STR}!");
    const SAME_GREETING: &str = hformat!("Hello, {}!", const { RANDOM_GUY.format() });
    assert_eq!(GREETING, "Hello, Doe, John | Age: 40 | $32.0!");
    assert_eq!(GREETING, SAME_GREETING);
    let id = core::hint::black_box(0);
    let mut buffer = String::new();
    hwrite!(buffer, "#{id} - {RANDOM_GUY_STR}");
    assert_eq!(buffer, "#0 - Doe, John | Age: 40 | $32.0");
}
