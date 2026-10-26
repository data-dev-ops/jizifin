use jizifin_domain::Ciphertext;

#[test]
fn base64url_valid_and_invalid() {
    assert!(Ciphertext::parse("valid-Base64URL_123").is_ok());
    assert!(Ciphertext::parse("").is_err());
    assert!(Ciphertext::parse("invalid+character").is_err());
    assert!(Ciphertext::parse("invalid/slash").is_err());
    assert!(Ciphertext::parse("invalid=padding").is_err());
    assert!(Ciphertext::parse("has space").is_err());
}

#[test]
fn equality_and_serde() {
    let c1 = Ciphertext::parse("token123").unwrap();
    let c2 = Ciphertext::parse("token123").unwrap();
    let c3 = Ciphertext::parse("token456").unwrap();
    assert_eq!(c1, c2);
    assert_ne!(c1, c3);
    assert_eq!(c1.as_str(), "token123");

    let json = serde_json::to_string(&c1).unwrap();
    assert_eq!(json, "\"token123\"");
    let back: Ciphertext = serde_json::from_str(&json).unwrap();
    assert_eq!(back, c1);
}
