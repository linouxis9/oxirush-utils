//! The README and the documentation show what their code prints.
//!
//! Their Rust blocks are documentation tests of the crate: they compile and run, and
//! nothing there looks at what they print. The build script writes each block as a
//! function that prints into a string, and the test compares it with the output that
//! its page shows under it (`build.rs` says how).

#![allow(unused)]

include!(concat!(env!("OUT_DIR"), "/blocks.rs"));

#[test]
fn each_block_prints_what_its_page_shows() {
    let mut differences = String::new();
    for Block {
        page,
        line,
        run,
        shown,
    } in BLOCKS
    {
        let printed = printed(*run);
        if printed != *shown {
            differences += &format!(
                "{page}, the block of line {line} prints:\n{printed}and the page shows:\n{shown}\n"
            );
        }
    }
    assert!(
        !BLOCKS.is_empty() && differences.is_empty(),
        "{differences}"
    );
}

/// The tree module says its example prints this: the order of the members is that in
/// which they were written.
#[test]
fn the_sample_of_the_tree_module_prints_what_its_documentation_shows() {
    use oxirush_utils::tree;

    let mut message = tree!({
        "message-type": "registration-request",
        "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
    });
    tree::set(&mut message, "/requested-nssai/0/sd", tree!("000001")).unwrap();
    tree::remove(&mut message, "/requested-nssai/1").unwrap();
    let shown =
        r#"{"message-type":"registration-request","requested-nssai":[{"sst":1,"sd":"000001"}]}"#;
    assert_eq!(message.to_string(), shown);
    assert!(include_str!("../../src/tree/mod.rs").contains(shown));
}
