use jizifin_domain::Cents;

#[test]
fn zero_and_new_roundtrip() {
    assert_eq!(Cents::ZERO.value(), 0);
    assert_eq!(Cents::new(150).value(), 150);
    assert_eq!(Cents::new(-42).value(), -42);
}

#[test]
fn predicates_behave_correctly() {
    assert!(Cents::ZERO.is_zero());
    assert!(!Cents::new(10).is_zero());
    assert!(Cents::new(10).is_positive());
    assert!(!Cents::new(-10).is_positive());
    assert!(Cents::new(-10).is_negative());
    assert!(!Cents::new(10).is_negative());
}

#[test]
fn abs_and_negation() {
    assert_eq!(Cents::new(50).abs(), Cents::new(50));
    assert_eq!(Cents::new(-50).abs(), Cents::new(50));
    assert_eq!(-Cents::new(50), Cents::new(-50));
    assert_eq!(-Cents::new(-50), Cents::new(50));
}

#[test]
fn arithmetic_and_assignment() {
    let mut balance = Cents::new(100);
    balance += Cents::new(50);
    assert_eq!(balance, Cents::new(150));
    balance -= Cents::new(200);
    assert_eq!(balance, Cents::new(-50));
    assert_eq!(Cents::new(10) + Cents::new(20), Cents::new(30));
    assert_eq!(Cents::new(10) - Cents::new(25), Cents::new(-15));
}

#[test]
fn sum_iterator() {
    let items = vec![Cents::new(33), Cents::new(33), Cents::new(34)];
    let total: Cents = items.into_iter().sum();
    assert_eq!(total, Cents::new(100));
}

#[test]
fn large_boundary_limits_without_loss() {
    let js_max = Cents::new(9_007_199_254_740_991);
    assert_eq!(js_max.value(), 9_007_199_254_740_991);
    let delta = Cents::new(1);
    assert_eq!((js_max + delta).value(), 9_007_199_254_740_992);
}

#[test]
fn serde_roundtrip() {
    let original = Cents::new(12345);
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, "12345");
    let parsed: Cents = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, original);
}
