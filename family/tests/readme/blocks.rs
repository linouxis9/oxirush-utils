// The blocks of the README that use other crates of the family, each as a
// function. `tests/readme.rs` writes this file and says how: do not edit it.
#![allow(unused)]

use std::cell::RefCell;

thread_local!(static PRINTED: RefCell<String> = const { RefCell::new(String::new()) });

// What a block prints is kept, to be compared with what the README shows.
macro_rules! println {
    () => {
        PRINTED.with(|printed| printed.borrow_mut().push('\n'))
    };
    // The line is made first, where the block is: its arguments may have a
    // `?` or an `await`.
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        PRINTED.with(|printed| {
            let mut printed = printed.borrow_mut();
            printed.push_str(&line);
            printed.push('\n');
        })
    }};
}

/// What `block` prints.
pub fn printed(block: fn()) -> String {
    PRINTED.with(|printed| printed.borrow_mut().clear());
    block();
    PRINTED.with(|printed| printed.take())
}

pub const BLOCKS: &[fn()] = &[
    block_1,
];

/// The block 1 of the README that uses other crates.
fn block_1() {
    use oxirush_nas::nas_5gs::{
        Nas5gsMessage, NasFGsMobileIdentity, NasFGsRegistrationResult, NasRegistrationAccept,
        RegistrationResult,
    };
    use oxirush_ngap::{helpers, ngap::PLMNIdentity};
    use oxirush_utils::identity::{Guti5g, Plmn};
    use oxirush_utils::tree;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let plmn: Plmn = "310-410".parse()?;
        let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;

        // The 5G-GUTI of a REGISTRATION ACCEPT of oxirush-nas is this value
        let result = NasFGsRegistrationResult::from_result(RegistrationResult::ThreeGppAccess);
        let identity = NasFGsMobileIdentity::from_guti(guti);
        let accept = NasRegistrationAccept::new(result).with_fg_guti(identity);
        let accept = Nas5gsMessage::new_5gmm(accept);
        println!("{}", hex::encode(accept.encode()?));

        // The PLMN identity of a GUAMI of oxirush-ngap has its AP octets
        let guami = helpers::guami(PLMNIdentity(plmn.to_ap_octets()), 0xca, 0x3fe, 0x01)?;
        println!("{:?}: {:02x?}", guami.plmn_identity, guami.plmn_identity.0);

        // The tree of the NAS message is a tree of this crate
        let tree = accept.to_tree();
        println!("{:?}", tree::select(&tree, "/5g-guti/guti/tmsi")?);
        Ok(())
    }
    main().unwrap();
}
