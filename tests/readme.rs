//! The output that the README shows under its first sample is what the crate prints. Its
//! Rust blocks are compiled and run with the documentation tests.

use oxirush_utils::identity::{Guti5g, Plmn};

#[test]
fn the_first_sample_prints_what_the_readme_shows() {
    let readme = include_str!("../README.md");
    let (_, after) = readme.split_once("```text\n").expect("an output block");
    let (shown, _) = after.split_once("\n```").expect("the end of the block");

    let plmn: Plmn = "310-410".parse().unwrap();
    let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef).unwrap();
    let printed = [
        format!(
            "{plmn}: {:02x?} in NAS, {:02x?} in NGAP",
            plmn.to_nas_octets(),
            plmn.to_ap_octets()
        ),
        format!("{guti}: {:02x?}", guti.to_nas_octets()),
    ];
    assert_eq!(printed.join("\n"), shown);
    // The sample itself is in the README as it is here.
    for line in [
        "let plmn: Plmn = \"310-410\".parse()?;",
        "let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;",
        "println!(\"{plmn}: {:02x?} in NAS, {:02x?} in NGAP\", plmn.to_nas_octets(), plmn.to_ap_octets());",
        "println!(\"{guti}: {:02x?}\", guti.to_nas_octets());",
    ] {
        assert!(readme.contains(line), "{line}");
    }
}

/// The tree module says its example prints this: the order of the members is that in
/// which they were written.
#[test]
fn the_sample_of_the_tree_module_prints_what_its_documentation_shows() {
    use oxirush_utils::tree;
    use serde_json::json;

    let mut message = json!({
        "message-type": "registration-request",
        "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
    });
    tree::set(&mut message, "/requested-nssai/0/sd", json!("000001")).unwrap();
    tree::remove(&mut message, "/requested-nssai/1").unwrap();
    let shown =
        r#"{"message-type":"registration-request","requested-nssai":[{"sst":1,"sd":"000001"}]}"#;
    assert_eq!(message.to_string(), shown);
    assert!(include_str!("../src/tree/mod.rs").contains(shown));
}
