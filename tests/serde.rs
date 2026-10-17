//! `Serialize` and `Deserialize` through the tree: the same form in JSON and in another
//! format that describes itself.

use oxirush_utils::tree::{self, Members, Reading, Refused, Tree, Value};
use serde_json::json;

#[derive(Debug, PartialEq)]
struct Slice {
    sst: u8,
    sd: Option<[u8; 3]>,
}

impl Tree for Slice {
    const NAME: &'static str = "S-NSSAI";

    fn to_tree(&self) -> Value {
        let mut members = Members::new();
        members.put("sst", tree::unsigned(self.sst.into()));
        members.put_optional("sd", self.sd.map(|sd| tree::octets(&sd)));
        members.finish()
    }

    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let of = reading.members(value, &["sst", "sd"])?;
        Ok(Self {
            sst: reading.member(&of, 0, |value, reading| reading.unsigned(value, 0, 255))? as u8,
            sd: reading.optional(&of, 1, |value, reading| reading.octet_array(value))?,
        })
    }
}

#[derive(Debug, PartialEq)]
struct Request {
    slices: Vec<Slice>,
    cause: String,
}

impl Tree for Request {
    const NAME: &'static str = "Request";

    fn to_tree(&self) -> Value {
        let mut members = Members::new();
        // Not the order of the alphabet: that of the type.
        members.put("slices", self.slices.iter().map(tree::to_value).collect());
        members.put("cause", tree::text(&self.cause));
        members.finish()
    }

    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        let of = reading.members(value, &["slices", "cause"])?;
        Ok(Self {
            slices: reading.member(&of, 0, |value, reading| {
                reading.list(value, Slice::from_tree)
            })?,
            cause: reading
                .member(&of, 1, |value, reading| reading.text(value))?
                .into(),
        })
    }
}

oxirush_utils::impl_serde!(Slice, Request);

/// A name that borrows its text, as the bindings of a protocol borrow their octets: a
/// value of any lifetime is deserialized, which owns its text.
#[derive(Debug, PartialEq)]
struct Name<'a>(std::borrow::Cow<'a, str>);

impl Tree for Name<'_> {
    const NAME: &'static str = "Name";

    fn to_tree(&self) -> Value {
        tree::text(&self.0)
    }

    fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
        Ok(Self(reading.text(value)?.to_string().into()))
    }
}

oxirush_utils::impl_serde!(Name<'a>);

#[test]
fn a_type_with_a_lifetime_is_written_and_read_through_its_tree() {
    let text = String::from("gnb-1");
    let name = Name(text.as_str().into());
    let written = serde_json::to_string(&name).unwrap();
    assert_eq!(written, r#""gnb-1""#);
    let read: Name<'static> = serde_json::from_str(&written).unwrap();
    assert_eq!(read, name);
}

fn request() -> Request {
    Request {
        slices: vec![
            Slice { sst: 1, sd: None },
            Slice {
                sst: 2,
                sd: Some([0, 0, 0xff]),
            },
        ],
        cause: "congestion".into(),
    }
}

#[test]
fn json_carries_the_tree() {
    let text = serde_json::to_string(&request()).unwrap();
    assert_eq!(
        text,
        r#"{"slices":[{"sst":1},{"sst":2,"sd":"0000ff"}],"cause":"congestion"}"#
    );
    assert_eq!(serde_json::from_str::<Request>(&text).unwrap(), request());
    assert_eq!(
        serde_json::to_value(request()).unwrap(),
        tree::to_value(&request())
    );
    let read: Request =
        serde_json::from_value(json!({"Cause": "x", "SLICES": [{"SD": "0000FF", "sst": 9}]}))
            .unwrap();
    assert_eq!(
        read.slices,
        [Slice {
            sst: 9,
            sd: Some([0, 0, 0xff])
        }]
    );
}

/// YAML is what a scenario is written in: it has the same members in the same order,
/// octets as the same text, and the refusals of the tree.
#[test]
fn yaml_carries_the_same_tree() {
    let text = serde_yaml::to_string(&request()).unwrap();
    assert_eq!(
        text,
        "slices:\n- sst: 1\n- sst: 2\n  sd: 0000ff\ncause: congestion\n"
    );
    assert_eq!(serde_yaml::from_str::<Request>(&text).unwrap(), request());
    // A number may be written in hexadecimal in YAML, which reads it as a number.
    let read: Slice = serde_yaml::from_str("{SST: 0x10, sd: '0000FF'}").unwrap();
    assert_eq!(
        read,
        Slice {
            sst: 16,
            sd: Some([0, 0, 0xff])
        }
    );
    let error = serde_yaml::from_str::<Request>("{slices: [{sst: 256}], cause: x}").unwrap_err();
    assert_eq!(
        error.to_string(),
        "this is not a value of Request: a number from 0 to 255 is expected, and 256 is written, \
         at slices/0/sst"
    );
    let error = serde_yaml::from_str::<Slice>("{sst: 1, sd: 255}").unwrap_err();
    assert!(
        error.to_string().contains("octets in hexadecimal"),
        "{error}"
    );
    let error = serde_json::from_str::<Slice>(r#"{"sst":1,"sd":"0000ff","x":1}"#).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("\"x\" is not a member that its type has: it has sst, sd"),
        "{error}"
    );
}
