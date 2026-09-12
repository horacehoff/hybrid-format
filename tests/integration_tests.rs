use hybrid_format::hformat;

#[test]
fn simple_test() {
    let z = 9;
    let val = hformat!("{z}");
    assert_eq!(val, "9");
}
