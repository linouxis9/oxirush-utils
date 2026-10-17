//! The forms of the plain values of a tree, the writer of members and their reader.

use oxirush_utils::tree;
use oxirush_utils::tree::{ErrorKind, Members, Reading, Refused, Tree, Value};

/// Numbers that look random, the same in every run.
struct Numbers(u64);

impl Numbers {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

#[test]
fn octets_are_hexadecimal_in_lower_case_and_are_read_in_either() {
    assert_eq!(tree::hex(&[]), "");
    assert_eq!(
        tree::hex(&[0x00, 0x9a, 0xbc, 0xde, 0xf0, 0xff]),
        "009abcdef0ff"
    );
    assert_eq!(tree::octets(&[0x02, 0xf8, 0x39]), tree!("02f839"));
    for text in ["02f839", "02F839", "02f8 39".replace(' ', "").as_str()] {
        assert_eq!(tree::unhex(text), Some(vec![0x02, 0xf8, 0x39]), "{text}");
    }
    // One spelling: no prefix, no separator, and two digits for each octet.
    for text in [
        "0x02f839",
        "02 f8 39",
        "02:f8:39",
        "2f839",
        "02f83g",
        "０２",
        "02f8\u{0}9",
    ] {
        assert_eq!(tree::unhex(text), None, "{text:?}");
    }
    let mut numbers = Numbers(1);
    for length in 0..300 {
        let octets: Vec<u8> = (0..length).map(|_| numbers.next() as u8).collect();
        let text = tree::hex(&octets);
        assert!(
            text.bytes()
                .all(|digit| digit.is_ascii_digit() || (b'a'..=b'f').contains(&digit))
        );
        assert_eq!(tree::unhex(&text).as_deref(), Some(&octets[..]));
        assert_eq!(
            tree::unhex(&text.to_uppercase()).as_deref(),
            Some(&octets[..])
        );
        assert_eq!(text, hex::encode(&octets));
    }
}

/// oxirush-ngap `a_name_is_the_same_whatever_its_case_and_what_is_between_its_letters`,
/// and the names of oxirush-nas without its spelling of `5g` as `fg`.
#[test]
fn a_name_is_its_letters_and_its_digits() {
    for (a, b) in [
        ("AMF-UE-NGAP-ID", "amf_ue ngap.id"),
        ("pLMNIdentity", "plmnidentity"),
        ("5GMM cause", "5gmm_cause"),
        ("5gmm-cause", "5GMMCause"),
        ("non-3gpp", "Non 3GPP"),
    ] {
        assert!(tree::same_name(a, b) && tree::same_name(b, a), "{a} {b}");
    }
    for (a, b) in [
        ("pLMNIdentity", "plmn"),
        ("-", "_"),
        ("", ""),
        ("a", "ä"),
        ("fgmm-cause", "5gmm-cause"),
        ("tac", "tac2"),
    ] {
        assert!(!tree::same_name(a, b), "{a:?} {b:?}");
    }
    // Among names with the same letters: the name itself, then one written alike.
    let names = ["sNSSAI", "s-nssai", "snssai", "S NSSAI"];
    for (written, position) in [
        ("snssai", 2),
        ("s-nssai", 1),
        ("S_NSSAI", 1),
        ("s nssai", 1),
        ("SNSSAI", 0),
        ("s.n.s.s.a.i", 0),
    ] {
        assert_eq!(tree::named(&names, written), Some(position), "{written}");
    }
    assert_eq!(tree::named(&names, "nssai"), None);
    assert_eq!(tree::named::<&str>(&[], "x"), None);
    let object = tree!({"5gmm-cause": 1, "5GMM Cause": 2});
    let members = object.as_object().unwrap();
    assert_eq!(
        tree::member(members, "5gmm_cause"),
        Some(("5gmm-cause", &tree!(1)))
    );
    assert_eq!(
        tree::member(members, "5GMM Cause"),
        Some(("5GMM Cause", &tree!(2)))
    );
    assert_eq!(tree::member(members, "cause"), None);
}

/// A value with each kind of member.
#[derive(Debug, PartialEq)]
struct Sample {
    count: u64,
    offset: i64,
    flag: bool,
    kind: usize,
    name: String,
    payload: Vec<u8>,
    fixed: [u8; 3],
    mask: (Vec<u8>, usize),
    either: Either,
    list: Vec<u64>,
    maybe: Option<u64>,
}

#[derive(Debug, PartialEq)]
enum Either {
    Nothing,
    Number(u64),
}

const KINDS: &[&str] = &["plmn-not-allowed", "congestion", "5gs-services-not-allowed"];
const MEMBERS: &[&str] = &[
    "count", "offset", "flag", "kind", "name", "payload", "fixed", "mask", "either", "list",
    "maybe",
];

impl Tree for Sample {
    const NAME: &'static str = "Sample";

    fn to_tree(&self) -> Value {
        let mut members = Members::new();
        members.put("count", tree::unsigned(self.count));
        members.put("offset", tree::signed(self.offset));
        members.put("flag", tree!(self.flag));
        members.put("kind", tree::text(KINDS[self.kind]));
        members.put("name", tree::text(&self.name));
        members.put("payload", tree::octets(&self.payload));
        members.put("fixed", tree::octets(&self.fixed));
        members.put("mask", tree::bits(&self.mask.0, self.mask.1));
        members.put(
            "either",
            match self.either {
                Either::Nothing => tree::text("nothing"),
                Either::Number(number) => tree::alternative("number", tree::unsigned(number)),
            },
        );
        members.put(
            "list",
            self.list
                .iter()
                .map(|entry| tree::unsigned(*entry))
                .collect(),
        );
        members.put_optional("maybe", self.maybe.map(tree::unsigned));
        members.finish()
    }

    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let of = reading.members(value, MEMBERS)?;
        Ok(Self {
            count: reading.member(&of, 0, |value, reading| {
                reading.unsigned(value, 0, u64::MAX)
            })?,
            offset: reading.member(&of, 1, |value, reading| reading.signed(value, -8, 7))?,
            flag: reading.member(&of, 2, |value, reading| reading.boolean(value))?,
            kind: reading.member(&of, 3, |value, reading| reading.named(value, KINDS))?,
            name: reading
                .member(&of, 4, |value, reading| reading.text(value))?
                .into(),
            payload: reading.member(&of, 5, |value, reading| reading.octets(value))?,
            fixed: reading.member(&of, 6, |value, reading| reading.octet_array(value))?,
            mask: reading.member(&of, 7, |value, reading| reading.bits(value))?,
            either: reading.member(&of, 8, |value, reading| match value {
                Value::String(_) => reading.named(value, &["nothing"]).map(|_| Either::Nothing),
                value => {
                    let (_, content) = reading.alternative(value, &["number"])?;
                    let read = |value, reading: &mut Reading| reading.unsigned(value, 0, 9);
                    reading.within("number", content, read).map(Either::Number)
                }
            })?,
            list: reading.member(&of, 9, |value, reading| {
                reading.list(value, |entry, reading| reading.unsigned(entry, 0, 255))
            })?,
            maybe: reading.optional(&of, 10, |value, reading| reading.unsigned(value, 0, 255))?,
        })
    }
}

fn sample() -> Sample {
    Sample {
        count: u64::MAX,
        offset: -8,
        flag: true,
        kind: 2,
        name: "C0".into(),
        payload: vec![0xc0, 0xff],
        fixed: [0x02, 0xf8, 0x39],
        mask: (vec![0xc0], 3),
        either: Either::Number(7),
        list: vec![1, 2, 3],
        maybe: None,
    }
}

#[test]
fn a_value_is_written_with_its_members_in_their_order_and_read_back() {
    let written = tree::to_value(&sample());
    assert_eq!(
        written.to_string(),
        r#"{"count":18446744073709551615,"offset":-8,"flag":true,"kind":"5gs-services-not-allowed","name":"C0","payload":"c0ff","fixed":"02f839","mask":{"value":"c0","length":3},"either":{"number":7},"list":[1,2,3]}"#
    );
    assert_eq!(tree::from_value::<Sample>(&written), Ok(sample()));
    // A name however it is written, octets in either case, a member that is null, a
    // string of bits as its octets alone, an alternative without content.
    let other = tree!({
        "Count": 0, "OFFSET": 7, "flag": false, "kind": "PLMN not allowed", "name": "",
        "payload": "C0FF", "fixed": "02F839", "mask": "c0", "either": "Nothing", "list": [],
        "maybe": null,
    });
    let read: Sample = tree::from_value(&other).unwrap();
    assert_eq!((read.count, read.offset, read.kind), (0, 7, 0));
    assert_eq!(
        (read.payload, read.fixed),
        (vec![0xc0, 0xff], [0x02, 0xf8, 0x39])
    );
    assert_eq!(
        (read.mask, read.either, read.maybe),
        ((vec![0xc0], 8), Either::Nothing, None)
    );
    let mut with = written.clone();
    with["maybe"] = tree!(255);
    assert_eq!(tree::from_value::<Sample>(&with).unwrap().maybe, Some(255));
}

#[test]
fn a_refusal_says_the_member_and_why() {
    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut tree = tree::to_value(&sample());
        edit(&mut tree);
        let error = tree::from_value::<Sample>(&tree).unwrap_err();
        assert!(
            error
                .reason()
                .starts_with("this is not a value of Sample: "),
            "{error}"
        );
        (
            error.kind(),
            error.at().to_string(),
            error.reason()[31..].to_string(),
        )
    };
    let value = |at: &str, why: &str| (ErrorKind::Value, at.to_string(), why.to_string());
    // One spelling for each plain value.
    assert_eq!(
        refused(&|tree| tree["count"] = tree!("7")),
        value(
            "count",
            "a number from 0 to 18446744073709551615 is expected, and \"7\" is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["count"] = tree!(1.0)),
        value(
            "count",
            "a number from 0 to 18446744073709551615 is expected, and 1.0 is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["count"] = tree!(-1)),
        value(
            "count",
            "a number from 0 to 18446744073709551615 is expected, and -1 is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["offset"] = tree!(8)),
        value(
            "offset",
            "a number from -8 to 7 is expected, and 8 is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["flag"] = tree!("true")),
        value("flag", "true or false is expected, and \"true\" is written")
    );
    assert_eq!(
        refused(&|tree| tree["payload"] = tree!("0xc0ff")),
        value(
            "payload",
            "octets in hexadecimal, two digits for each, are expected, and \"0xc0ff\" is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["payload"] = tree!([192, 255])),
        value(
            "payload",
            "octets in hexadecimal, two digits for each, are expected, and [192,255] is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["fixed"] = tree!("02f8")),
        value("fixed", "3 octets are expected, and 02f8 has 2")
    );
    assert_eq!(
        refused(&|tree| tree["kind"] = tree!("congestio")),
        value(
            "kind",
            "\"congestio\" is none of its names: plmn-not-allowed, congestion, 5gs-services-not-allowed"
        )
    );
    assert_eq!(
        refused(&|tree| tree["kind"] = tree!(1)),
        value("kind", "a name is expected, and 1 is written")
    );
    // A string of bits has the octets of its length, and zero bits after it.
    assert_eq!(
        refused(&|tree| tree["mask"] = tree!({"value": "c0", "length": 9})),
        value("mask", "9 bits are not 1 octets")
    );
    assert_eq!(
        refused(&|tree| tree["mask"] = tree!({"value": "c1", "length": 3})),
        value("mask", "the bits after the 3 of the string are not zero")
    );
    // Where in a list, and in an alternative.
    assert_eq!(
        refused(&|tree| tree["list"] = tree!([1, 256, 3])),
        value(
            "list/1",
            "a number from 0 to 255 is expected, and 256 is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["either"] = tree!({"number": 10})),
        value(
            "either/number",
            "a number from 0 to 9 is expected, and 10 is written"
        )
    );
    assert_eq!(
        refused(&|tree| tree["either"] = tree!({"number": 1, "other": 2})),
        value(
            "either",
            "an object with one member, the name of an alternative with its value, is expected, \
             and {\"number\":1,\"other\":2} is written"
        )
    );
    // A member that the type has not, one that is missing, one that is there twice.
    let (kind, at, why) = refused(&|tree| tree["cuont"] = tree!(1));
    assert_eq!((kind, at.as_str()), (ErrorKind::NoSuchMember, ""));
    assert!(
        why.starts_with("\"cuont\" is not a member that its type has: it has count, offset, "),
        "{why}"
    );
    assert_eq!(
        refused(&|tree| drop(tree.as_object_mut().unwrap().shift_remove("flag"))),
        value("flag", "the member is missing")
    );
    assert_eq!(
        refused(&|tree| tree["FLAG"] = tree!(true)),
        value("", "the member \"flag\" is written twice")
    );
    let (_, _, why) = refused(&|tree| *tree = tree!([1, 2]));
    assert_eq!(why, "an object is expected, and [1,2] is written");
    // What is written is shown as far as it says which value it is.
    let (_, _, why) = refused(&|tree| tree["name"] = tree!(["x".repeat(100)]));
    assert_eq!(
        why,
        format!(
            "a string is expected, and [\"{}… is written",
            "x".repeat(38)
        )
    );
}

/// The review of the release candidate found a refusal of a misspelled name of an
/// ENUMERATED that listed every name of its 68: a type of more than 12 names says the
/// nearest of them, or its first, and the count.
#[test]
fn a_name_that_a_type_has_not_is_refused_with_the_nearest_of_many() {
    use oxirush_utils::tree::{Reading, Refused, Tree, Value, nearest};

    struct Many(usize);
    const NAMES: [&str; 14] = [
        "unspecified",
        "user-inactivity",
        "radio-connection-with-ue-lost",
        "interaction-with-other-procedure",
        "unknown-PDU-session-ID",
        "handover-cancelled",
        "partial-handover",
        "cell-not-available",
        "radio-resources-not-available",
        "ue-in-rrc-inactive-state-not-reachable",
        "redirection",
        "resources-not-available",
        "unknown-local-UE-NGAP-ID",
        "inconsistent-remote-UE-NGAP-ID",
    ];
    impl Tree for Many {
        const NAME: &'static str = "CauseRadioNetwork";
        fn to_tree(&self) -> Value {
            oxirush_utils::tree::text(NAMES[self.0])
        }
        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            reading.named(value, &NAMES).map(Many)
        }
    }
    let refused = |name: &str| {
        oxirush_utils::tree::from_value::<Many>(&Value::String(name.into()))
            .err()
            .unwrap()
            .to_string()
    };
    assert_eq!(
        refused("user-inactive"),
        "this is not a value of CauseRadioNetwork: \"user-inactive\" is none of its names: \
         the nearest of its 14 is user-inactivity"
    );
    assert_eq!(
        refused("resource-not-available"),
        "this is not a value of CauseRadioNetwork: \"resource-not-available\" is none of its \
         names: the nearest of its 14 are resources-not-available and radio-resources-not-available"
    );
    assert_eq!(
        refused("x"),
        "this is not a value of CauseRadioNetwork: \"x\" is none of its names: unspecified, \
         user-inactivity, radio-connection-with-ue-lost, interaction-with-other-procedure, \
         unknown-PDU-session-ID, handover-cancelled, partial-handover, cell-not-available, and \
         6 more"
    );
    assert_eq!(nearest(&NAMES, "partial handover", 3), ["partial-handover"]);
}
