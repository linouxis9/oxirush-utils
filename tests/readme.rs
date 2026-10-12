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
