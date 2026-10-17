//! Paths on trees: the grammar, the plain rule, and two families that stand for those of
//! the crates of messages.
//!
//! `Nas` has the rules of a view of a NAS message: a root that every path has, each
//! information element as a member that is `null` when the message does not have it,
//! with its `value` and its `octets`. `Ap` has those of a view of an NGAP or S1AP PDU: a
//! root that is short for the list of the IEs, an IE that its name or `@id=N` selects,
//! a segment that is taken in the value of an IE. The tests of oxirush-nas and of
//! oxirush-ngap on paths are ported here on trees that are written out, so that the
//! engine holds them without a codec.

use oxirush_utils::tree;
use oxirush_utils::tree::Value;
use oxirush_utils::tree::{
    Absent, Edit, Error, ErrorKind, Family, Form, Listed, Next, Place, Plain, Removal, Root, Step,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ── A family with the rules of a view of a NAS message ──────────────────────

struct Nas;

/// What `Nas` knows of a place.
#[derive(Clone, Default)]
enum InNas {
    /// The view: the names of the IEs that its message can have, when it says which
    /// message it is of.
    View(Option<&'static [&'static str]>),
    /// An information element: `{"value", "octets"}`, or `null`.
    Ie,
    /// Anything in the value of one.
    #[default]
    Inside,
}

/// The IEs of the one message that the tests know.
const REGISTRATION_REQUEST: &[&str] = &[
    "extended-protocol-discriminator",
    "security-header-type",
    "message-type",
    "5gs-registration-type",
    "ngksi",
    "5gs-mobile-identity",
    "5gmm-capability",
    "ue-security-capability",
    "requested-nssai",
    "mico-indication",
];

impl Family for Nas {
    type Shape = InNas;

    fn root(&self) -> Root<'_> {
        Root::new("nas", &[], true)
    }

    fn start(&self, view: &Value, _rooted: bool) -> InNas {
        let kind = view["message-type"]["value"].as_str();
        InNas::View((kind == Some("registration-request")).then_some(REGISTRATION_REQUEST))
    }

    fn member(&self, shape: &InNas, _name: &str, _value: Option<&Value>) -> InNas {
        match shape {
            InNas::View(_) => InNas::Ie,
            _ => InNas::Inside,
        }
    }

    fn absent(&self, shape: &InNas, name: &str) -> Absent<InNas> {
        match shape {
            InNas::View(Some(names)) if tree::named(names, name).is_none() => {
                let reason = format!("`{name}` is no IE of a message `registration-request`");
                Absent::Refused(Error::new(ErrorKind::NoSuchMember, "", reason))
            }
            InNas::View(Some(_)) => Absent::Optional(InNas::Ie),
            _ => Absent::Unknown,
        }
    }

    fn step<'v>(
        &self,
        place: &Place<'v, InNas>,
        segment: &str,
        next: &mut Next<'v, InNas>,
    ) -> Result<Step<InNas>, Error> {
        let of_an_ie = ["value", "octets"]
            .iter()
            .find(|name| tree::same_name(name, segment));
        let (InNas::Ie, Some(name), true) = (place.shape(), of_an_ie, next.is_last()) else {
            return Ok(Step::Plain);
        };
        match next.edit() {
            Some(Edit::Remove) => {
                let reason = "an IE is removed, not its value or its octets";
                Err(Error::new(ErrorKind::Refused, "", reason))
            }
            // An IE that the message does not have becomes one when its value or its
            // octets are set.
            Some(Edit::Set) if place.value().is_none_or(Value::is_null) => {
                next.absent(name, InNas::Inside);
                Ok(Step::Taken)
            }
            _ => Ok(Step::Plain),
        }
    }

    fn removal(&self, shape: &InNas) -> Removal {
        match shape {
            // The view keeps the name of an IE that is taken out.
            InNas::Ie => Removal::Null,
            _ => Removal::Take,
        }
    }

    fn form(&self, _shape: &InNas, _value: &Value) -> Form {
        Form::Plain
    }
}

/// The view of a REGISTRATION REQUEST with a requested NSSAI of one slice, as
/// oxirush-nas shows `7e004179000d0102f8390000000000000010022e02e0e02f020101`.
fn registration_request() -> Value {
    tree!({
        "extended-protocol-discriminator": {"value": "5gmm"},
        "security-header-type": {"value": "plain"},
        "message-type": {"value": "registration-request"},
        "5gs-registration-type": {"value": {"follow-on-request": true, "type": "initial"}, "octets": "9"},
        "ngksi": {"value": {"tsc": "native", "identifier": 7}, "octets": "7"},
        "5gs-mobile-identity": {
            "value": {"suci": {"imsi": {
                "plmn": "208-93", "routing-indicator": "0", "protection-scheme": "null",
                "home-network-public-key-identifier": 0, "msin": "0000000120",
            }}},
            "octets": "0102f8390000000000000010",
        },
        "5gmm-capability": null,
        "ue-security-capability": {"value": {"ea": ["5g-ea0", "128-5g-ea1", "128-5g-ea2"]}, "octets": "e0e0"},
        "requested-nssai": {"value": [{"sst": 1}], "octets": "0101"},
        "mico-indication": null,
    })
}

/// oxirush-nas `a_path_names_an_ie_however_it_is_written_and_what_is_not_there_is_nothing`.
#[test]
fn a_path_names_an_ie_however_it_is_written_and_what_is_not_there_is_nothing() {
    let tree = registration_request();
    let select = |path: &str| tree::select_in(&Nas, &tree, path);
    for path in [
        "/nas/5gs-mobile-identity/value/suci/imsi/msin",
        "/nas/5GS Mobile Identity/value/SUCI/IMSI/MSIN",
        "/nas/5gs_mobile_identity/value/suci/imsi/msin",
        "/NAS/5gsMobileIdentity/Value/suci/imsi/msin",
    ] {
        assert_eq!(select(path), Ok(vec![&tree!("0000000120")]), "{path}");
    }
    assert_eq!(
        select("/nas/requested-nssai/value/*/sst"),
        Ok(vec![&tree!(1)])
    );
    assert_eq!(select("/nas"), Ok(vec![&tree]));
    // An optional IE that the message does not have, and an entry past the end.
    assert_eq!(select("/nas/5gmm-capability/value/s1-mode"), Ok(vec![]));
    assert_eq!(select("/nas/requested-nssai/value/1/sst"), Ok(vec![]));
    // The IE itself selects nothing either, and no path of it is listed.
    assert_eq!(select("/nas/5gmm-capability"), Ok(vec![]));
    let listed = tree::paths_in(&Nas, &tree);
    assert!(
        !listed
            .iter()
            .any(|(path, _)| path.contains("5gmm-capability"))
    );
    // A view says what message it is of: the name of an IE that the message cannot have
    // is refused when it is set, and the view is as it was.
    let mut edited = tree.clone();
    let timer = tree!({"value": 60});
    let error = tree::set_in(&Nas, &mut edited, "/nas/t3512-value", timer.clone()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("`t3512-value` is no IE of a message"),
        "{error}"
    );
    assert_eq!(edited, tree);
    tree::set_in(
        &Nas,
        &mut edited,
        "/nas/5gmm-capability",
        tree!({"octets": "00"}),
    )
    .unwrap();
    // One that does not say takes any name.
    let mut alone = tree!({});
    tree::set_in(&Nas, &mut alone, "/nas/t3512-value", timer.clone()).unwrap();
    assert_eq!(alone, tree!({"t3512-value": timer}));
    for (path, kind, reason) in [
        (
            "/nas/no-such-ie/value",
            ErrorKind::NoSuchMember,
            "`no-such-ie` is no IE of a message",
        ),
        (
            "/nas/requested-nssai/value/0/misspelled",
            ErrorKind::NoSuchMember,
            "\"misspelled\" is not a member of this value, which has sst",
        ),
        (
            "/nas/requested-nssai/value/first",
            ErrorKind::NotAList,
            "\"first\" is no position in a list",
        ),
        (
            "/nas/requested-nssai/octets/deeper",
            ErrorKind::Scalar,
            "which has none",
        ),
        (
            "/message/requested_nssai",
            ErrorKind::Path,
            "is not a path of this tree: it starts with /nas",
        ),
        (
            "nas/requested-nssai",
            ErrorKind::Path,
            "a path starts with \"/\"",
        ),
    ] {
        let error = select(path).unwrap_err();
        assert_eq!(error.kind(), kind, "{path}: {error}");
        assert!(error.to_string().contains(reason), "{path}: {error}");
    }
}

/// oxirush-nas `a_view_is_edited_at_its_paths`, on the view: what the message makes of
/// an edited view is that crate's.
#[test]
fn a_view_is_edited_at_its_paths() {
    let request = registration_request();
    // A member of a value, an entry of a list, and the octets of an IE.
    let mut tree = request.clone();
    tree::set_in(
        &Nas,
        &mut tree,
        "/nas/Requested NSSAI/value/0/sst",
        tree!(2),
    )
    .unwrap();
    let last = tree!({"sst": 3, "sd": "0000ff"});
    tree::insert_in(&Nas, &mut tree, "/nas/requested-nssai/value/-", last).unwrap();
    tree::insert_in(
        &Nas,
        &mut tree,
        "/nas/requested-nssai/value/0",
        tree!({"sst": 1}),
    )
    .unwrap();
    tree::set_in(
        &Nas,
        &mut tree,
        "/nas/ue-security-capability/octets",
        tree!("f0f0"),
    )
    .unwrap();
    assert_eq!(
        tree["requested-nssai"]["value"],
        tree!([{"sst": 1}, {"sst": 2}, {"sst": 3, "sd": "0000ff"}])
    );
    assert_eq!(tree["ue-security-capability"]["octets"], "f0f0");
    // An optional IE is added by its value or its octets, and one is taken out.
    let mut tree = request.clone();
    tree::set_in(
        &Nas,
        &mut tree,
        "/nas/mico-indication/value",
        tree!({"raai": true}),
    )
    .unwrap();
    tree::set_in(&Nas, &mut tree, "/nas/5gmm-capability/octets", tree!("01")).unwrap();
    tree::remove_in(&Nas, &mut tree, "/nas/requested-nssai").unwrap();
    tree::remove_in(&Nas, &mut tree, "/nas/ue-security-capability").unwrap();
    assert_eq!(tree["mico-indication"], tree!({"value": {"raai": true}}));
    assert_eq!(tree["5gmm-capability"], tree!({"octets": "01"}));
    // The view still names the IE that was taken out, which is added again.
    assert!(tree["requested-nssai"].is_null() && tree["ue-security-capability"].is_null());
    assert_eq!(
        tree.as_object().unwrap().len(),
        request.as_object().unwrap().len()
    );
    tree::set_in(
        &Nas,
        &mut tree,
        "/nas/requested-nssai/value",
        tree!([{"sst": 5}]),
    )
    .unwrap();
    assert_eq!(tree["requested-nssai"], tree!({"value": [{"sst": 5}]}));
    // What selects nothing is refused, and the view is as it was.
    let mut tree = request.clone();
    for (edit, kind, reason) in [
        (
            tree::set_in(&Nas, &mut tree, "/nas/no-such-ie/value", tree!(1)),
            ErrorKind::NoSuchMember,
            "`no-such-ie` is no IE of a message `registration-request`",
        ),
        (
            tree::set_in(
                &Nas,
                &mut tree,
                "/nas/requested-nssai/value/4/sst",
                tree!(1),
            ),
            ErrorKind::NothingSelected,
            "the path selects nothing",
        ),
        (
            tree::set_in(&Nas, &mut tree, "/nas/requested-nssai/value", Value::Null),
            ErrorKind::Refused,
            "an IE is removed",
        ),
        (
            tree::remove_in(&Nas, &mut tree, "/nas/requested-nssai/octets"),
            ErrorKind::Refused,
            "an IE is removed",
        ),
        (
            tree::remove_in(&Nas, &mut tree, "/nas/mico-indication"),
            ErrorKind::NothingSelected,
            "the path selects nothing",
        ),
        (
            tree::insert_in(&Nas, &mut tree, "/nas/requested-nssai", tree!(1)),
            ErrorKind::NotAList,
            "insert adds to a list",
        ),
        (
            tree::set_in(&Nas, &mut tree, "/nas", tree!({})),
            ErrorKind::Path,
            "the path of the tree itself",
        ),
    ] {
        let error = edit.unwrap_err();
        assert_eq!(error.kind(), kind, "{error}");
        assert!(error.to_string().contains(reason), "{error}");
    }
    assert_eq!(tree, request);
}

/// oxirush-nas `a_path_takes_a_name_by_its_letters_and_a_position_as_decimal_writes_it`.
#[test]
fn a_path_takes_a_name_by_its_letters_and_a_position_as_decimal_writes_it() {
    let tree = registration_request();
    let sst = tree::select_in(&Nas, &tree, "/nas/requested-nssai/value/0/sst").unwrap();
    for path in [
        "/NAS/Requested NSSAI/Value/0/SST",
        "/nas/requested_nssai/value/0/sst",
        "/nas/requestednssai/value/0/sst",
        "/nas/requested.nssai/value/0/sst",
    ] {
        assert_eq!(tree::select_in(&Nas, &tree, path).unwrap(), sst, "{path}");
    }
    for path in [
        "/nas/requested-nssai/value/+0/sst",
        "/nas/requested-nssai/value/00/sst",
    ] {
        let error = tree::select_in(&Nas, &tree, path).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotAList, "{path}: {error}");
        assert_eq!(error.at(), path);
    }
    // The spelling of an identifier of Rust for `5g` is no name of the specification.
    let error = tree::select_in(&Nas, &tree, "/nas/fgs_mobile_identity/value").unwrap_err();
    assert!(
        error.to_string().contains("`fgs_mobile_identity` is no IE"),
        "{error}"
    );
}

/// oxirush-nas `every_path_of_every_fixture_selects_its_value`, and the same of
/// oxirush-ngap, on the trees of this file.
#[test]
fn every_path_that_is_listed_selects_its_value() {
    let tree = registration_request();
    let paths = tree::paths_in(&Nas, &tree);
    assert!(paths.len() > 12, "{paths:#?}");
    for (path, value) in &paths {
        assert!(path.starts_with("/nas/"), "{path}");
        assert_eq!(
            tree::select_in(&Nas, &tree, path),
            Ok(vec![value]),
            "{path}"
        );
    }
    // In the order of the view, which is that of the message.
    let first: Vec<&str> = paths
        .iter()
        .take(4)
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(
        first,
        [
            "/nas/extended-protocol-discriminator/value",
            "/nas/security-header-type/value",
            "/nas/message-type/value",
            "/nas/5gs-registration-type/value/follow-on-request",
        ]
    );
    // A list of plain values is one value.
    assert!(paths.contains(&(
        "/nas/ue-security-capability/value/ea".to_string(),
        tree!(["5g-ea0", "128-5g-ea1", "128-5g-ea2"])
    )));
    let pdu = initial_ue_message();
    let paths = tree::paths_in(&Ap, &pdu);
    for (path, value) in &paths {
        assert_eq!(tree::select_in(&Ap, &pdu, path), Ok(vec![value]), "{path}");
    }
    let plain = tree!({"a": [{"b": 1}, [2, 3]], "c/d": {"~": true}, "e": {}, "f": []});
    let paths = tree::paths(&plain);
    assert_eq!(
        paths,
        [
            ("/a/0/b".to_string(), tree!(1)),
            ("/a/1".to_string(), tree!([2, 3])),
            ("/c~1d/~0".to_string(), tree!(true)),
            ("/e".to_string(), tree!({})),
            ("/f".to_string(), tree!([])),
        ]
    );
    for (path, value) in &paths {
        assert_eq!(tree::select(&plain, path), Ok(vec![value]), "{path}");
    }
    // A member without a name is at "/", as a JSON pointer has it, and a tree that is one
    // value is at the empty path: what the `paths` fuzz target found unsaid.
    let unnamed = tree!({"": 1});
    assert_eq!(tree::paths(&unnamed), [("/".to_string(), tree!(1))]);
    assert_eq!(tree::select(&unnamed, "/"), Ok(vec![&tree!(1)]));
    assert_eq!(tree::paths(&tree!(5)), [(String::new(), tree!(5))]);
    assert_eq!(tree::select(&tree!(5), ""), Ok(vec![&tree!(5)]));
    assert!(tree::paths(&Value::Null).is_empty());
    let with_nulls = tree!([null, {"a": null, "b": 1}]);
    assert_eq!(tree::paths(&with_nulls), [("/1/b".to_string(), tree!(1))]);
    // A family with a root has no path without it.
    let error = tree::select_in(&Nas, &unnamed, "").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path);
}

// ── A family with the rules of a view of an NGAP or S1AP PDU ────────────────

struct Ap;

/// What `Ap` knows of a place.
#[derive(Clone, Default, PartialEq)]
enum InAp {
    #[default]
    Unknown,
    /// The view of a PDU, and whether the path starts with the root.
    Pdu(bool),
    Message(bool),
    /// The list of the IEs of a message, whose entries a name selects when the path
    /// starts with the root.
    Ies(bool),
    /// An entry of that list, `{"id", "criticality", "value"}`, with its identifier.
    Ie(bool, Option<u64>),
    /// The value of the NAS-PDU IE: octets.
    Octets,
}

/// The IEs that the tests know, by the name that ASN.1 gives them after `id-`.
const IES: &[(&str, u64)] = &[
    ("AMF-UE-NGAP-ID", 10),
    ("Cause", 15),
    ("NAS-PDU", 38),
    ("RAN-UE-NGAP-ID", 85),
    ("UserLocationInformation", 121),
];

fn ie_id(segment: &str) -> Option<u64> {
    match segment.strip_prefix("@id=") {
        Some(id) => id.parse::<u16>().ok().map(u64::from),
        None => {
            let names: Vec<&str> = IES.iter().map(|(name, _)| *name).collect();
            tree::named(&names, segment).map(|position| IES[position].1)
        }
    }
}

impl Family for Ap {
    type Shape = InAp;

    fn root(&self) -> Root<'_> {
        Root::new("ngap", &["message", "protocolIEs"], false)
    }

    fn start(&self, _view: &Value, rooted: bool) -> InAp {
        InAp::Pdu(rooted)
    }

    fn member(&self, shape: &InAp, name: &str, _value: Option<&Value>) -> InAp {
        match (shape, name) {
            (InAp::Pdu(rooted), "message") => InAp::Message(*rooted),
            (InAp::Message(rooted), "protocolIEs") => InAp::Ies(*rooted),
            // The value of the NAS-PDU IE is octets.
            (InAp::Ie(_, Some(38)), "value") => InAp::Octets,
            _ => InAp::Unknown,
        }
    }

    fn entry(&self, shape: &InAp, value: Option<&Value>) -> InAp {
        match shape {
            InAp::Ies(rooted) => InAp::Ie(*rooted, value.and_then(|entry| entry["id"].as_u64())),
            _ => InAp::Unknown,
        }
    }

    fn step<'v>(
        &self,
        place: &Place<'v, InAp>,
        segment: &str,
        next: &mut Next<'v, InAp>,
    ) -> Result<Step<InAp>, Error> {
        match (place.shape(), place.value()) {
            // An IE of the list, by its name or by `@id=N`.
            (InAp::Ies(named), Some(Value::Array(entries))) => {
                let by_id = segment.starts_with("@id=");
                let Some(id) = ie_id(segment).filter(|_| *named || by_id) else {
                    return match by_id {
                        true => {
                            let reason = format!("{segment:?} is not the identifier of an IE");
                            Err(Error::new(ErrorKind::Path, "", reason))
                        }
                        false => Ok(Step::Plain),
                    };
                };
                let mut found = false;
                for (position, entry) in entries.iter().enumerate() {
                    if entry["id"].as_u64() == Some(id) {
                        next.entry(position, entry, InAp::Ie(*named, Some(id)));
                        found = true;
                    }
                }
                // An IE that is not there: what follows is still one of its paths.
                if !found {
                    next.absent(segment, InAp::Ie(*named, Some(id)));
                }
                Ok(Step::Taken)
            }
            (InAp::Ie(named, _), Some(Value::Object(entry))) => {
                if segment == "*" || tree::member(entry, segment).is_some() {
                    return Ok(Step::Plain);
                }
                // The name of the IE that the entry is stays on the entry.
                if *named && ie_id(segment).is_some() {
                    return match ie_id(segment) == entry["id"].as_u64() {
                        true => {
                            next.stay(place, place.shape().clone());
                            Ok(Step::Taken)
                        }
                        false => {
                            let reason = format!("{segment:?} is not the IE that this entry is");
                            Err(Error::new(ErrorKind::NoSuchMember, "", reason))
                        }
                    };
                }
                // A segment that names no member of an IE is taken in its value.
                let of_value = match entry["id"].as_u64() {
                    Some(38) => InAp::Octets,
                    _ => InAp::Unknown,
                };
                Ok(Step::Through("value".into(), of_value))
            }
            _ => Ok(Step::Plain),
        }
    }

    fn listed(&self, shape: &InAp, name: &str) -> Listed {
        match (shape, name) {
            (_, name) if name.starts_with('_') => Listed::Hidden,
            (InAp::Message(_), "protocolIEs") => Listed::Root,
            _ => Listed::Member,
        }
    }

    fn entry_names(&self, shape: &InAp, list: &[Value]) -> Option<Vec<Option<String>>> {
        if !matches!(shape, InAp::Ies(_)) {
            return None;
        }
        // One pass counts each identifier, and one names the entries.
        let mut counts: HashMap<u64, usize> = HashMap::new();
        for entry in list {
            *counts
                .entry(entry["id"].as_u64().unwrap_or(u64::MAX))
                .or_default() += 1;
        }
        let name = |entry: &Value| {
            let id = entry["id"].as_u64()?;
            let (name, _) = IES.iter().find(|(_, known)| *known == id)?;
            (counts[&id] == 1).then(|| name.to_string())
        };
        Some(list.iter().map(name).collect())
    }

    fn form(&self, shape: &InAp, _value: &Value) -> Form {
        match shape {
            InAp::Octets => Form::Octets,
            _ => Form::Plain,
        }
    }

    fn edited(&self, view: &mut Value, at: &[tree::Key], _shape: &InAp, _edit: Edit) {
        // What a view keeps of what was received is no longer what it says.
        if let [
            tree::Key::Name(message),
            tree::Key::Name(ies),
            tree::Key::Position(position),
            ..,
        ] = at
            && let Some(entry) = view[message][ies][position].as_object_mut()
        {
            entry.shift_remove("_raw_value");
        }
    }
}

/// The view of an INITIAL UE MESSAGE, as oxirush-ngap shows one.
fn initial_ue_message() -> Value {
    tree!({
        "kind": "initiatingMessage",
        "procedureCode": 15,
        "criticality": "ignore",
        "message": {"protocolIEs": [
            {"id": 85, "criticality": "reject", "value": 7, "_raw_value": "40010007"},
            {"id": 38, "criticality": "reject", "value": "7e004179000d0102f839", "_raw_value": "0a7e004179000d0102f839"},
            {"id": 121, "criticality": "reject", "value": {"userLocationInformationNR": {
                "nR-CGI": {"pLMNIdentity": "02f839", "nRCellIdentity": "000000010"},
                "tAI": {"pLMNIdentity": "02f839", "tAC": "000001"},
            }}},
            {"id": 65000, "criticality": "ignore", "value": "00"},
        ]},
    })
}

/// oxirush-ngap `the_ies_of_a_message_have_paths_by_their_names`,
/// `a_segment_that_names_no_member_of_an_ie_is_taken_in_its_value` and
/// `the_name_of_the_ie_that_an_entry_is_stays_on_the_entry`.
#[test]
fn an_ie_is_selected_by_its_name_and_a_segment_is_taken_in_its_value() {
    let pdu = initial_ue_message();
    let select = |path: &str| tree::select_in(&Ap, &pdu, path);
    // The root is short for the list of the IEs, where a name selects an IE.
    for path in [
        "/ngap/RAN-UE-NGAP-ID/value",
        "/ngap/ran_ue_ngap_id/value",
        "/NGAP/RanUeNgapId/Value",
        "/ngap/@id=85/value",
        "/ngap/0/value",
        "/message/protocolIEs/0/value",
        "/message/protocolIEs/@id=85/value",
        // The name of the IE that an entry is stays on the entry.
        "/ngap/0/RAN-UE-NGAP-ID/value",
    ] {
        assert_eq!(select(path), Ok(vec![&tree!(7)]), "{path}");
    }
    // A segment that names no member of an IE is taken in its value.
    for path in [
        "/ngap/UserLocationInformation/value/userLocationInformationNR/tAI/tAC",
        "/ngap/UserLocationInformation/userLocationInformationNR/tAI/tAC",
        "/ngap/user location information/user_location_information_nr/tai/tac",
        "/ngap/2/userLocationInformationNR/tAI/tAC",
    ] {
        assert_eq!(select(path), Ok(vec![&tree!("000001")]), "{path}");
    }
    assert_eq!(select("/ngap/*/id").unwrap().len(), 4);
    assert_eq!(
        select("/ngap/UserLocationInformation/value/*/*/pLMNIdentity")
            .unwrap()
            .len(),
        2
    );
    // An IE that the message does not have selects nothing, and what follows is still a
    // path of it.
    assert_eq!(select("/ngap/Cause"), Ok(vec![]));
    assert_eq!(select("/ngap/Cause/value/radioNetwork"), Ok(vec![]));
    assert_eq!(select("/ngap/@id=7"), Ok(vec![]));
    // Without the root, a name is no IE: the list has positions.
    let error = select("/message/protocolIEs/RAN-UE-NGAP-ID").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotAList, "{error}");
    let error = select("/ngap/0/Cause").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("\"Cause\" is not the IE that this entry is"),
        "{error}"
    );
    let error = select("/ngap/@id=x").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Path, "{error}");
    let error =
        select("/ngap/UserLocationInformation/userLocationInformationNR/tAI/tAc/x").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Scalar, "{error}");
}

/// oxirush-ngap `the_ies_of_a_message_have_paths_by_their_names`: an IE goes by its name
/// under the root, and by its position when it has no name or its identifier is there
/// twice. What a view keeps of what was received is not listed.
#[test]
fn the_ies_of_a_message_are_listed_by_their_names_under_the_root() {
    let pdu = initial_ue_message();
    let paths: Vec<String> = tree::paths_in(&Ap, &pdu)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        paths,
        [
            "/kind",
            "/procedureCode",
            "/criticality",
            "/ngap/RAN-UE-NGAP-ID/id",
            "/ngap/RAN-UE-NGAP-ID/criticality",
            "/ngap/RAN-UE-NGAP-ID/value",
            "/ngap/NAS-PDU/id",
            "/ngap/NAS-PDU/criticality",
            "/ngap/NAS-PDU/value",
            "/ngap/UserLocationInformation/id",
            "/ngap/UserLocationInformation/criticality",
            "/ngap/UserLocationInformation/value/userLocationInformationNR/nR-CGI/pLMNIdentity",
            "/ngap/UserLocationInformation/value/userLocationInformationNR/nR-CGI/nRCellIdentity",
            "/ngap/UserLocationInformation/value/userLocationInformationNR/tAI/pLMNIdentity",
            "/ngap/UserLocationInformation/value/userLocationInformationNR/tAI/tAC",
            "/ngap/3/id",
            "/ngap/3/criticality",
            "/ngap/3/value",
        ]
    );
    // An identifier that is there twice names no entry alone.
    let mut twice = pdu.clone();
    let again = tree!({"id": 85, "criticality": "reject", "value": 8});
    tree::insert_in(&Ap, &mut twice, "/ngap/-", again).unwrap();
    let paths = tree::paths_in(&Ap, &twice);
    assert!(paths.contains(&("/ngap/0/value".to_string(), tree!(7))));
    assert!(paths.contains(&("/ngap/4/value".to_string(), tree!(8))));
    assert_eq!(
        tree::select_in(&Ap, &twice, "/ngap/RAN-UE-NGAP-ID/value")
            .unwrap()
            .len(),
        2
    );
    // A message without an IE says so.
    let empty = tree!({"message": {"protocolIEs": []}});
    assert_eq!(
        tree::paths_in(&Ap, &empty),
        [("/ngap".to_string(), tree!([]))]
    );
}

#[test]
fn a_family_edits_what_it_selects_and_says_what_else_changes() {
    let mut pdu = initial_ue_message();
    tree::set_in(&Ap, &mut pdu, "/ngap/RAN-UE-NGAP-ID/value", tree!(9)).unwrap();
    // The octets that were received are no longer those of the IE.
    assert_eq!(
        pdu["message"]["protocolIEs"][0],
        tree!({"id": 85, "criticality": "reject", "value": 9})
    );
    // A member of the value of an IE, through the IE.
    tree::set_in(
        &Ap,
        &mut pdu,
        "/ngap/UserLocationInformation/userLocationInformationNR/tAI/tAC",
        tree!("000002"),
    )
    .unwrap();
    assert_eq!(
        tree::select_in(&Ap, &pdu, "/ngap/2/value/userLocationInformationNR/tAI/tAC"),
        Ok(vec![&tree!("000002")])
    );
    tree::remove_in(&Ap, &mut pdu, "/ngap/NAS-PDU").unwrap();
    let ids = tree::select_in(&Ap, &pdu, "/ngap/*/id").unwrap();
    assert_eq!(ids, [&tree!(85), &tree!(121), &tree!(65000)]);
    // An IE that is not there is nothing to set a member of.
    let before = pdu.clone();
    let error = tree::set_in(&Ap, &mut pdu, "/ngap/Cause/value", tree!(1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NothingSelected, "{error}");
    assert_eq!(pdu, before);
}

// ── The plain rule ──────────────────────────────────────────────────────────

#[test]
fn a_path_is_the_segments_of_a_json_pointer_within_its_limits() {
    assert_eq!(
        tree::segments("/a/b~1c/~0~1/").unwrap(),
        ["a", "b/c", "~/", ""]
    );
    assert_eq!(tree::segments("/").unwrap(), [""]);
    // The empty path is that of the tree itself.
    assert!(tree::segments("").unwrap().is_empty());
    for (path, kind, reason) in [
        (
            "a",
            ErrorKind::Path,
            "\"a\" is no path: a path starts with \"/\"",
        ),
        ("a/b", ErrorKind::Path, "\"a/b\" is no path"),
        ("/a~2b", ErrorKind::Path, "\"~\" is followed by 0 or 1"),
        ("/a~", ErrorKind::Path, "\"~\" is followed by 0 or 1"),
    ] {
        let error = tree::segments(path).unwrap_err();
        assert_eq!((error.kind(), error.at()), (kind, path));
        assert!(error.to_string().contains(reason), "{error}");
    }
    // 4096 octets and 64 segments are taken, and one more is not.
    let longest = format!("/{}", "a".repeat(4095));
    assert_eq!(tree::segments(&longest).unwrap().len(), 1);
    let error = tree::segments(&format!("{longest}a")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Limit);
    assert_eq!(error.reason(), "a path has 4096 octets at most");
    assert_eq!(tree::segments(&"/a".repeat(64)).unwrap().len(), 64);
    let error = tree::segments(&"/a".repeat(65)).unwrap_err();
    assert_eq!(
        (error.kind(), error.reason()),
        (ErrorKind::Limit, "a path has 64 segments at most")
    );
    for (segment, position) in [
        ("0", Some(0)),
        ("10", Some(10)),
        ("01", None),
        ("-1", None),
        ("1.0", None),
        ("", None),
        ("१", None),
    ] {
        assert_eq!(tree::position(segment), position, "{segment:?}");
    }
    assert_eq!(tree::position("99999999999999999999999999"), None);
}

#[test]
fn the_plain_rule_selects_members_entries_and_each_of_them() {
    let tree = tree!({
        "list": [{"a": 1, "b": null}, {"a": 2, "c": [10, 20]}, 5],
        "name/with": {"~tilde": "x"},
        "nothing": null,
    });
    let select = |path: &str| tree::select(&tree, path);
    assert_eq!(select(""), Ok(vec![&tree]));
    assert_eq!(select("/list/1/a"), Ok(vec![&tree!(2)]));
    assert_eq!(select("/list/1/c/1"), Ok(vec![&tree!(20)]));
    assert_eq!(select("/name~1with/~0tilde"), Ok(vec![&tree!("x")]));
    // `*` is each member or each entry: what one does not have, another may.
    assert_eq!(select("/list/*/a"), Ok(vec![&tree!(1), &tree!(2)]));
    assert_eq!(select("/list/*/c/*"), Ok(vec![&tree!(10), &tree!(20)]));
    assert_eq!(select("/*/0/a"), Ok(vec![&tree!(1)]));
    // A name that none of them has is an error, and so is a path into a plain value.
    assert_eq!(
        select("/list/*/z").unwrap_err().kind(),
        ErrorKind::NoSuchMember
    );
    assert_eq!(select("/list/2/a").unwrap_err().kind(), ErrorKind::Scalar);
    assert_eq!(select("/list/2/*").unwrap_err().kind(), ErrorKind::Scalar);
    // What is null is not there: it selects nothing, and nothing under it.
    assert_eq!(select("/nothing"), Ok(vec![]));
    assert_eq!(select("/nothing/any/thing"), Ok(vec![]));
    assert_eq!(select("/list/0/b"), Ok(vec![]));
    assert_eq!(select("/list/7"), Ok(vec![]));
    assert_eq!(select("/list/*/b"), Ok(vec![]));
    // `-` is the end of a list for an insertion alone.
    assert_eq!(select("/list/-").unwrap_err().kind(), ErrorKind::NotAList);
}

#[test]
fn an_edit_is_done_whole_or_not_at_all() {
    let before = tree!({"list": [{"a": 1}, {"a": 2}, {"a": 3}], "kept": true});
    let mut tree = before.clone();
    // Each entry, the last first: positions stay those of the path.
    tree::remove(&mut tree, "/list/*/a").unwrap();
    assert_eq!(tree["list"], tree!([{}, {}, {}]));
    let mut tree = before.clone();
    tree::remove(&mut tree, "/list/*").unwrap();
    assert_eq!(tree["list"], tree!([]));
    let mut tree = before.clone();
    tree::insert(&mut tree, "/list/*", tree!(0)).unwrap();
    assert_eq!(tree["list"], tree!([0, {"a": 1}, 0, {"a": 2}, 0, {"a": 3}]));
    let mut tree = before.clone();
    tree::set(&mut tree, "/list/*/b", tree!("new")).unwrap();
    assert_eq!(tree["list"][2], tree!({"a": 3, "b": "new"}));
    // `null` takes a member or an entry out, as `remove` does.
    tree::set(&mut tree, "/list/0", Value::Null).unwrap();
    tree::set(&mut tree, "/kept", Value::Null).unwrap();
    assert_eq!(
        tree,
        tree!({"list": [{"a": 2, "b": "new"}, {"a": 3, "b": "new"}]})
    );
    // What an edit cannot do leaves the tree as it was.
    let mut tree = before.clone();
    for (edit, kind) in [
        (
            tree::set(&mut tree, "/list/3/a", tree!(1)),
            ErrorKind::NothingSelected,
        ),
        (
            tree::set(&mut tree, "/list/x", tree!(1)),
            ErrorKind::NotAList,
        ),
        (tree::set(&mut tree, "/kept/x", tree!(1)), ErrorKind::Scalar),
        (tree::remove(&mut tree, "/missing"), ErrorKind::NoSuchMember),
        (
            tree::remove(&mut tree, "/list/9"),
            ErrorKind::NothingSelected,
        ),
        (
            tree::insert(&mut tree, "/list/3", tree!(1)),
            ErrorKind::NothingSelected,
        ),
        (
            tree::insert(&mut tree, "/kept", tree!(1)),
            ErrorKind::NotAList,
        ),
        (
            tree::insert(&mut tree, "/kept/-", tree!(1)),
            ErrorKind::Scalar,
        ),
        (tree::set(&mut tree, "", tree!(1)), ErrorKind::Path),
        (tree::remove(&mut tree, ""), ErrorKind::Path),
        (tree::set(&mut tree, "list", tree!(1)), ErrorKind::Path),
    ] {
        assert_eq!(edit.unwrap_err().kind(), kind);
        assert_eq!(tree, before);
    }
}

/// A family that refuses an edit of an entry whose `locked` is true, in its step,
/// before anything of the tree changes.
struct Locked;

impl Family for Locked {
    type Shape = ();

    fn step<'v>(
        &self,
        place: &Place<'v, ()>,
        _segment: &str,
        next: &mut Next<'v, ()>,
    ) -> Result<Step<()>, Error> {
        let locked = place.value().and_then(|value| value.get("locked"));
        match (locked, next.edit()) {
            (Some(Value::Bool(true)), Some(_)) => {
                Err(Error::new(ErrorKind::Refused, "", "this entry is locked").with_code(7))
            }
            _ => Ok(Step::Plain),
        }
    }
}

/// A family with a fault: `*` on a list also selects a member that a list cannot have,
/// before its entries, which the edit takes last and cannot do once the entries took it.
struct Faulty;

impl Family for Faulty {
    type Shape = ();

    fn step<'v>(
        &self,
        place: &Place<'v, ()>,
        segment: &str,
        next: &mut Next<'v, ()>,
    ) -> Result<Step<()>, Error> {
        let Some(entries) = place.value().and_then(Value::as_array) else {
            return Ok(Step::Plain);
        };
        if segment != "*" {
            return Ok(Step::Plain);
        }
        next.absent("extra", ());
        for (position, entry) in entries.iter().enumerate() {
            next.entry(position, entry, ());
        }
        Ok(Step::Taken)
    }
}

#[test]
fn an_edit_in_the_place_of_the_tree_is_put_back_whole() {
    let before = tree!({"list": [
        {"a": 1, "b": 1},
        {"a": 2, "b": 2},
        {"a": 3, "b": 3, "locked": false},
    ], "last": {"x": 0, "y": 0}});
    // Each place of an edit, the last first, and a removal that keeps the order.
    for (edit, after) in [
        (
            "remove /list/*/a",
            tree!([{"b": 1}, {"b": 2}, {"b": 3, "locked": false}]),
        ),
        (
            "set /list/*/c",
            tree!([{"a": 1, "b": 1, "c": 0}, {"a": 2, "b": 2, "c": 0}, {"a": 3, "b": 3, "locked": false, "c": 0}]),
        ),
        (
            "insert /list/*",
            tree!([0, {"a": 1, "b": 1}, 0, {"a": 2, "b": 2}, 0, {"a": 3, "b": 3, "locked": false}]),
        ),
    ] {
        let mut tree = before.clone();
        let (operation, path) = edit.split_once(' ').unwrap();
        match operation {
            "remove" => tree::remove_in(&Locked, &mut tree, path).unwrap(),
            "set" => tree::set_in(&Locked, &mut tree, path, tree!(0)).unwrap(),
            _ => tree::insert_in(&Locked, &mut tree, path, tree!(0)).unwrap(),
        }
        assert_eq!(tree["list"], after, "{edit}");
    }
    let mut tree = before.clone();
    tree::remove(&mut tree, "/last/x").unwrap();
    tree::set(&mut tree, "/last/x", tree!(1)).unwrap();
    assert_eq!(tree["last"].to_string(), r#"{"y":0,"x":1}"#);

    // An entry that the family refuses: nothing changes, and the code that the family
    // gave its error is carried as it is.
    let mut locked = before.clone();
    locked["list"][1]["locked"] = tree!(true);
    for edit in ["remove /list/*/a", "set /list/*/a"] {
        let mut tree = locked.clone();
        let (operation, path) = edit.split_once(' ').unwrap();
        let error = match operation {
            "remove" => tree::remove_in(&Locked, &mut tree, path),
            _ => tree::set_in(&Locked, &mut tree, path, tree!(9)),
        }
        .unwrap_err();
        assert_eq!(
            (error.kind(), error.code()),
            (ErrorKind::Refused, Some(7)),
            "{edit}"
        );
        assert_eq!(tree.to_string(), locked.to_string(), "{edit}");
    }

    // A place that cannot take the edit once the places after it took it: what they
    // changed is put back, the members in their order and the entries in their places.
    // The entries took the value, the last first, before the member that a list cannot
    // have; a removal and an insertion leave out a place without a value.
    let mut tree = before.clone();
    let error = tree::set_in(&Faulty, &mut tree, "/list/*", tree!(9)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NothingSelected);
    assert_eq!(tree.to_string(), before.to_string());
}

/// A family that takes a segment that names no member of an entry in the `value` of
/// the entry, with its own rule there, and words what the value refuses as its own
/// error; and that selects by its types where a list has no entry.
struct InValue;

impl Family for InValue {
    type Shape = u8;

    fn step<'v>(
        &self,
        place: &Place<'v, u8>,
        segment: &str,
        next: &mut Next<'v, u8>,
    ) -> Result<Step<u8>, Error> {
        match place.value() {
            Some(Value::Object(entry))
                if entry.contains_key("value") && tree::member(entry, segment).is_none() =>
            {
                next.through(place, "value", 1, |inside, next| {
                    match inside.value().and_then(|value| value.get(segment)) {
                        Some(value) => {
                            next.member(segment, value, 2);
                            Ok(Step::Taken)
                        }
                        None => {
                            let reason = format!(
                                "{segment:?} is a member of neither the entry nor its value"
                            );
                            Err(Error::new(ErrorKind::NoSuchMember, "", reason))
                        }
                    }
                })
            }
            Some(Value::Array(_)) if segment == "?" => {
                // Two types, each once whatever names it.
                next.absent_once("a", 9);
                next.absent_once("b", 9);
                next.absent_once("c", 8);
                Ok(Step::Taken)
            }
            _ => Ok(Step::Plain),
        }
    }
}

#[test]
fn a_family_takes_a_segment_in_a_member_with_its_own_rule() {
    let mut tree = tree!({"ie": {"id": 5, "value": {"cause": 3}}, "list": []});
    assert_eq!(
        tree::select_in(&InValue, &tree, "/ie/cause"),
        Ok(vec![&tree!(3)])
    );
    assert_eq!(
        tree::select_in(&InValue, &tree, "/ie/id"),
        Ok(vec![&tree!(5)])
    );
    let error = tree::select_in(&InValue, &tree, "/ie/timer").unwrap_err();
    assert_eq!(
        error.to_string(),
        "\"timer\" is a member of neither the entry nor its value, at /ie/timer"
    );
    // An edit there is under the member.
    tree::set_in(&InValue, &mut tree, "/ie/cause", tree!(4)).unwrap();
    assert_eq!(tree["ie"], tree!({"id": 5, "value": {"cause": 4}}));
    let places = tree::places_in(&InValue, &tree, "/list/?").unwrap();
    let shapes: Vec<u8> = places.iter().map(|place| *place.shape()).collect();
    assert_eq!(shapes, [9, 8]);
}

/// A family whose texts hold numbers, which it opens: `open_in` shows each as an object
/// with the number, on the way of the path, and puts every one back after an error.
struct Numbers;

impl Family for Numbers {
    type Shape = ();

    fn step<'v>(
        &self,
        place: &Place<'v, ()>,
        segment: &str,
        next: &mut Next<'v, ()>,
    ) -> Result<Step<()>, Error> {
        let text = place.value().and_then(Value::as_str);
        match (text, segment, next.edit()) {
            (Some(text), "number", Some(Edit::Open)) => match text.parse::<u64>() {
                Ok(number) => {
                    next.open(place, tree!({"number": number}));
                    Ok(Step::Taken)
                }
                Err(_) => Err(Error::new(ErrorKind::Refused, "", "these are no digits")),
            },
            (Some(_), "number", _) => {
                Err(Error::new(ErrorKind::Refused, "", "the text is not opened"))
            }
            _ => Ok(Step::Plain),
        }
    }
}

#[test]
fn a_family_opens_the_values_on_the_way_of_a_path() {
    let before = tree!({"timers": ["60", "3600", {"number": 1}], "name": "t3512"});
    let mut tree = before.clone();
    tree::open_in(&Numbers, &mut tree, "/timers/*/number").unwrap();
    assert_eq!(
        tree,
        tree!({"timers": [{"number": 60}, {"number": 3600}, {"number": 1}], "name": "t3512"})
    );
    assert_eq!(
        tree::select_in(&Numbers, &tree, "/timers/*/number").unwrap(),
        [&tree!(60), &tree!(3600), &tree!(1)]
    );
    // A path into a value that was not opened says so, and opening it again changes
    // nothing.
    let error = tree::select_in(&Numbers, &before, "/timers/0/number").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Refused);
    let opened = tree.clone();
    tree::open_in(&Numbers, &mut tree, "/timers/*/number").unwrap();
    assert_eq!(tree, opened);
    // A path that selects nothing opens nothing, and is no error.
    let mut tree = before.clone();
    tree::open_in(&Numbers, &mut tree, "/timers/7/number").unwrap();
    assert_eq!(tree, before);
    // What cannot be opened leaves every value as it was, those opened before it too.
    let mut tree = tree!({"timers": ["60", "x"]});
    let error = tree::open_in(&Numbers, &mut tree, "/timers/*/number").unwrap_err();
    assert_eq!(error.reason(), "these are no digits");
    assert_eq!(tree, tree!({"timers": ["60", "x"]}));
    // A family that opens a value where the edit is no opening has a fault.
    let mut tree = before.clone();
    struct Opens;
    impl Family for Opens {
        type Shape = ();
        fn step<'v>(
            &self,
            place: &Place<'v, ()>,
            _: &str,
            next: &mut Next<'v, ()>,
        ) -> Result<Step<()>, Error> {
            next.open(place, tree!(0));
            Ok(Step::Taken)
        }
    }
    let error = tree::set_in(&Opens, &mut tree, "/name", tree!(1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Refused);
    assert_eq!(tree, before);
}

/// The fourth finding of the review of the move of the paths to oxirush-utils: `open_in`
/// opened one value a turn and took the path again from the tree for the next, a time that
/// grew as the square of the values (10 s for the 4,096 containers of a HANDOVER REQUIRED).
/// A turn opens every place of a segment now: the family is asked of each place once a
/// turn, and a path has a turn for each segment that opens, and one more.
#[test]
fn open_asks_of_each_place_once_a_turn() {
    struct Counted(std::cell::Cell<usize>);
    impl Family for Counted {
        type Shape = ();
        fn step<'v>(
            &self,
            place: &Place<'v, ()>,
            segment: &str,
            next: &mut Next<'v, ()>,
        ) -> Result<Step<()>, Error> {
            self.0.set(self.0.get() + 1);
            Numbers.step(place, segment, next)
        }
    }
    for count in [1024, 4096, 65_536] {
        let texts: Vec<String> = (0..count).map(|number| number.to_string()).collect();
        let mut tree = tree!({"name": "t3512", "timers": texts});
        let counted = Counted(std::cell::Cell::new(0));
        let started = Instant::now();
        tree::open_in(&counted, &mut tree, "/timers/*/number").unwrap();
        let elapsed = started.elapsed();
        // Two turns: the one that opens, and the one that finds nothing more to open; each
        // asks of the root, of the list and of each entry.
        assert_eq!(counted.0.get(), 2 * (count + 2));
        assert!(elapsed < Duration::from_secs(5), "{count}: {elapsed:?}");
        assert_eq!(tree["timers"][count - 1], tree!({"number": count - 1}));
        assert_eq!(
            tree::select_in(&Numbers, &tree, "/timers/*/number").map(|values| values.len()),
            Ok(count)
        );
    }
}

// ── NULL ────────────────────────────────────────────────────────────────────

/// What a family knows of the type of a place: `pLMNWide` and `noReporting` are of the type
/// NULL of ASN.1, alternatives of a CHOICE whose one value is `null`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Type {
    #[default]
    Other,
    Null,
}

struct Typed;

impl Family for Typed {
    type Shape = Type;

    fn member(&self, _: &Type, name: &str, _: Option<&Value>) -> Type {
        match name {
            "pLMNWide" | "noReporting" => Type::Null,
            _ => Type::Other,
        }
    }

    fn null_is_value(&self, shape: &Type) -> bool {
        *shape == Type::Null
    }
}

/// The first finding of the review of the move of the paths to oxirush-utils: an
/// alternative of the type NULL (`{"pLMNWide": null}`) was read as absent, so that no path
/// listed or selected it. It is a value where the family says that the type of the place is
/// NULL, and `null` is still read as absent elsewhere.
#[test]
fn null_is_a_value_where_the_type_of_the_place_is_null() {
    let mut tree = tree!({
        "areaScope": {"pLMNWide": null},
        "reporting": {"noReporting": null},
        "t3512": null,
    });
    // Listed and selected, by its name and by `*`.
    assert_eq!(
        tree::paths_in(&Typed, &tree),
        [
            ("/areaScope/pLMNWide".to_string(), Value::Null),
            ("/reporting/noReporting".to_string(), Value::Null),
        ]
    );
    assert_eq!(
        tree::select_in(&Typed, &tree, "/areaScope/pLMNWide"),
        Ok(vec![&Value::Null])
    );
    assert_eq!(
        tree::select_in(&Typed, &tree, "/*/*"),
        Ok(vec![&Value::Null, &Value::Null])
    );
    // A null of another type is absent: it selects nothing, and nothing lists it.
    assert_eq!(tree::select_in(&Typed, &tree, "/t3512"), Ok(vec![]));
    assert_eq!(
        tree::select_in(&Typed, &tree, "/*").map(|values| values.len()),
        Ok(2)
    );
    // The plain family knows no type, and reads every null as absent.
    assert_eq!(tree::paths(&tree), []);
    assert_eq!(tree::select(&tree, "/areaScope/pLMNWide"), Ok(vec![]));
    // A null that is set where the type is NULL is the value; elsewhere it takes the
    // member out, as it did.
    tree::set_in(&Typed, &mut tree, "/areaScope", tree!({"cellBased": 1})).unwrap();
    tree::set_in(&Typed, &mut tree, "/areaScope/pLMNWide", Value::Null).unwrap();
    assert_eq!(tree["areaScope"], tree!({"cellBased": 1, "pLMNWide": null}));
    tree::set_in(&Typed, &mut tree, "/areaScope/cellBased", Value::Null).unwrap();
    assert_eq!(tree["areaScope"], tree!({"pLMNWide": null}));
    // And `remove` takes it out.
    tree::remove_in(&Typed, &mut tree, "/reporting/noReporting").unwrap();
    assert_eq!(tree["reporting"], tree!({}));
}

/// The places of a path, with what the family knows of each: a family that reads its
/// own tables there takes those where a value could be and is not.
#[test]
fn the_places_of_a_path_have_what_the_family_knows_of_them() {
    let mut view = registration_request();
    view.as_object_mut()
        .unwrap()
        .shift_remove("mico-indication");
    // An IE that the message can have and the view has not: a place without a value.
    let places = tree::places_in(&Nas, &view, "/nas/mico-indication").unwrap();
    assert_eq!(places.len(), 1);
    assert!(places[0].value().is_none());
    assert!(matches!(places[0].shape(), InNas::Ie));
    assert_eq!(
        tree::select_in(&Nas, &view, "/nas/mico-indication"),
        Ok(vec![])
    );
    let places = tree::places_in(&Nas, &view, "/nas/ngksi").unwrap();
    assert!(matches!(places[0].shape(), InNas::Ie));
    assert_eq!(places[0].value(), Some(&view["ngksi"]));
}

// ── The order of the members ────────────────────────────────────────────────

/// The members of a tree keep the order in which they were written, whatever their
/// names, without a feature of `serde_json`: a tree is a [`Value`] of the crate, and an
/// edit keeps the order.
#[test]
fn the_members_keep_the_order_in_which_they_are_written() {
    let mut members = tree::Members::new();
    for name in ["startTimeStamp", "endTimeStamp", "message", "ies", "a"] {
        members.put(name, tree!(1));
    }
    let mut tree = members.finish();
    let order =
        |tree: &Value| -> Vec<String> { tree.as_object().unwrap().keys().cloned().collect() };
    assert_eq!(
        order(&tree),
        ["startTimeStamp", "endTimeStamp", "message", "ies", "a"]
    );
    assert_eq!(
        tree.to_string(),
        r#"{"startTimeStamp":1,"endTimeStamp":1,"message":1,"ies":1,"a":1}"#
    );
    let listed: Vec<String> = tree::paths(&tree)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        listed,
        ["/startTimeStamp", "/endTimeStamp", "/message", "/ies", "/a"]
    );
    // A member that goes leaves the others where they were, and a new one is the last.
    tree::remove(&mut tree, "/endTimeStamp").unwrap();
    tree::set(&mut tree, "/b", tree!(2)).unwrap();
    tree::set(&mut tree, "/message", tree!(3)).unwrap();
    assert_eq!(order(&tree), ["startTimeStamp", "message", "ies", "a", "b"]);
    // Text that is read keeps the order in which it is written, too.
    let read: Value = serde_json::from_str(r#"{"z":1,"y":{"b":1,"a":2},"x":3}"#).unwrap();
    assert_eq!(read.to_string(), r#"{"z":1,"y":{"b":1,"a":2},"x":3}"#);
    assert_eq!(tree::select(&read, "/*").unwrap().len(), 3);
    assert_eq!(tree::paths(&read)[1].0, "/y/b");
}

/// The third finding of the review of the move of the paths to oxirush-utils: the feature
/// `tree` turned on `serde_json/preserve_order`, which changed every `serde_json::Map` of
/// the program that used the crate. It turns on no feature of `serde_json` now: in this
/// build, which has every feature of the crate, a `serde_json::Map` is still in the order
/// of its names, and a tree keeps its own order when it goes through one.
#[test]
fn the_crate_leaves_the_order_of_the_json_of_the_program_alone() {
    let mut map = serde_json::Map::new();
    for name in ["z", "a", "m"] {
        map.insert(name.into(), serde_json::Value::Null);
    }
    assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "m", "z"]);
    let tree = tree!({"z": 1, "a": 2});
    let json = serde_json::Value::from(tree.clone());
    assert_eq!(json.to_string(), r#"{"a":2,"z":1}"#);
    assert_eq!(tree.to_string(), r#"{"z":1,"a":2}"#);
    assert_eq!(json, tree);
    assert_eq!(serde_json::to_string(&tree).unwrap(), r#"{"z":1,"a":2}"#);
}

/// The seventh finding of the review of the move of the paths to oxirush-utils: a name
/// written twice in an object of JSON was read as its last value. A tree that is read
/// refuses it, and an object that is collected gives the name its last value at the place
/// of the first.
#[test]
fn a_name_written_twice_in_an_object_is_refused() {
    let error =
        serde_json::from_str::<Value>(r#"{"sst": 1, "sd": "000001", "sst": 2}"#).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the member \"sst\" is written twice in an object at line 1 column 36"
    );
    let error = serde_json::from_str::<Value>(r#"[{"a": {"b": 1, "b": 1}}]"#).unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("the member \"b\" is written twice"),
        "{error}"
    );
    // Twice in two objects is no repetition.
    let read: Value = serde_json::from_str(r#"[{"a": 1}, {"a": 2}]"#).unwrap();
    assert_eq!(read, tree!([{"a": 1}, {"a": 2}]));
    // A wide object takes a time that grows as n log n.
    let wide = (0..65_536)
        .map(|index| format!("\"m{index}\":{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let started = Instant::now();
    let read: Value = serde_json::from_str::<Value>(&format!("{{{wide},\"m7\":0}}"))
        .map_or_else(|error| tree!(error.to_string()), |_| tree!("read"));
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(
        read.as_str()
            .map(|text| text.starts_with("the member \"m7\"")),
        Some(true)
    );
    let members: tree::Map = [("a", 1), ("b", 2), ("a", 3)]
        .into_iter()
        .map(|(name, value)| (name.to_string(), tree!(value)))
        .collect();
    assert_eq!(Value::Object(members).to_string(), r#"{"a":3,"b":2}"#);
}

// ── Size, depth and time ────────────────────────────────────────────────────

/// The second review's finding on `paths()`: 65,535 IEs of one identifier took minutes,
/// as each was counted for each. Here a family names the entries of a list once, and
/// each operation takes a time that grows with the tree and no faster.
#[test]
fn the_operations_are_linear_in_the_size_of_the_tree() {
    const MOST: usize = 65_535;
    let ie = |id: u64, value: usize| tree!({"id": id, "criticality": "reject", "value": value});
    let same: Vec<Value> = (0..MOST).map(|index| ie(10, index)).collect();
    let mut pdu = tree!({"message": {"protocolIEs": same}});
    let started = Instant::now();
    let paths = tree::paths_in(&Ap, &pdu);
    assert_eq!(paths.len(), 3 * MOST);
    assert_eq!(
        paths[3 * MOST - 1],
        (format!("/ngap/{}/value", MOST - 1), tree!(MOST - 1))
    );
    let last = format!("/ngap/{}/value", MOST - 1);
    assert_eq!(
        tree::select_in(&Ap, &pdu, &last),
        Ok(vec![&tree!(MOST - 1)])
    );
    tree::set_in(&Ap, &mut pdu, &last, tree!(0)).unwrap();
    tree::remove_in(&Ap, &mut pdu, "/ngap/0").unwrap();
    tree::insert_in(&Ap, &mut pdu, "/ngap/-", ie(15, 0)).unwrap();
    assert_eq!(
        tree::select_in(&Ap, &pdu, "/ngap/Cause/value"),
        Ok(vec![&tree!(0)])
    );
    // A path selects as many values as the longest list of the protocols has (the 65,536
    // connections of an NG RESET), and more are an error, which is told at once.
    let selected = tree::select_in(&Ap, &pdu, "/ngap/AMF-UE-NGAP-ID/value").unwrap();
    assert_eq!(selected.len(), MOST - 1);
    assert_eq!(
        tree::select(&pdu, "/message/protocolIEs/*/value").map(|values| values.len()),
        Ok(MOST)
    );
    tree::insert_in(&Ap, &mut pdu, "/ngap/-", ie(10, 0)).unwrap();
    assert_eq!(
        tree::select(&pdu, "/message/protocolIEs/*/value").map(|values| values.len()),
        Ok(tree::MAX_SELECTED)
    );
    tree::insert_in(&Ap, &mut pdu, "/ngap/-", ie(10, 0)).unwrap();
    let error = tree::select(&pdu, "/message/protocolIEs/*/value").unwrap_err();
    assert_eq!(
        (error.kind(), error.reason()),
        (ErrorKind::Limit, "a path selects 65536 values at most")
    );
    // A plain tree of as many members.
    let members: tree::Map = (0..MOST)
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let wide = Value::Object(members);
    assert_eq!(tree::paths(&wide).len(), MOST);
    assert_eq!(tree::select(&wide, "/M65534"), Ok(vec![&tree!(65534)]));
    let elapsed = started.elapsed();
    // A count for each entry would be four thousand million steps: minutes.
    assert!(elapsed < Duration::from_secs(20), "{elapsed:?}");
}

/// A path has 64 segments at most: a tree that is nested deeper is one value at the
/// deepest path, and every path that is listed is one that is taken.
#[test]
fn a_tree_is_listed_as_deep_as_a_path_goes() {
    let mut deep = tree!("leaf");
    for _ in 0..200 {
        deep = tree!({"a": deep});
    }
    let paths = tree::paths(&deep);
    assert_eq!(paths.len(), 1);
    let (path, value) = &paths[0];
    assert_eq!(path, &"/a".repeat(64));
    assert!(value.is_object());
    assert_eq!(tree::select(&deep, path), Ok(vec![value]));
    // With a root, the root is one of the 64.
    let view = tree!({"x": deep});
    let paths = tree::paths_in(&Nas, &view);
    assert_eq!(paths[0].0, format!("/nas/x{}", "/a".repeat(62)));
    assert_eq!(
        tree::select_in(&Nas, &view, &paths[0].0),
        Ok(vec![&paths[0].1])
    );
    // Lists nest as objects do.
    let mut lists = tree!(1);
    for _ in 0..100 {
        lists = tree!([[lists]]);
    }
    let paths = tree::paths(&lists);
    assert_eq!(paths[0].0, "/0".repeat(64));
}

// ── Comparing with a value that someone wrote ───────────────────────────────

/// A tree shows octets in lower case and reads them in either: a comparison with a value
/// that someone wrote takes that value as the tree shows it at that place. `C0` is the
/// octets `c0` at a place of octets, and other text than `c0` at a place of text.
#[test]
fn a_written_value_is_compared_as_the_tree_shows_it_at_its_place() {
    let pdu = tree!({"message": {"protocolIEs": [
        {"id": 38, "criticality": "reject", "value": "c0"},
        {"id": 65000, "criticality": "ignore", "value": "c0"},
    ]}});
    let same = |path: &str, written: Value| {
        let selected = tree::select_forms_in(&Ap, &pdu, path).unwrap();
        let [(value, form)] = selected[..] else {
            panic!("{path} selects one value");
        };
        *value == tree::written_as(form, &written)
    };
    // The value of the NAS-PDU is octets.
    assert!(same("/ngap/NAS-PDU/value", tree!("C0")));
    assert!(same("/ngap/NAS-PDU/value", tree!("c0")));
    assert!(!same("/ngap/NAS-PDU/value", tree!("c1")));
    // The family says nothing of the other value: it is text, and `C0` is other text.
    assert!(!same("/ngap/1/value", tree!("C0")));
    assert!(same("/ngap/1/value", tree!("c0")));
    // One of several, each as the tree shows it.
    let one_of = tree::written_as(Form::Octets, &tree!(["C0", "FF", "not octets"]));
    assert_eq!(one_of, tree!(["c0", "ff", "not octets"]));
    // A name of a set is the one that the tree shows.
    let names = Form::Named(&["plmn-not-allowed", "congestion"]);
    assert_eq!(
        tree::written_as(names, &tree!("PLMN not allowed")),
        tree!("plmn-not-allowed")
    );
    assert_eq!(
        tree::written_as(names, &tree!("no such cause")),
        tree!("no such cause")
    );
    assert_eq!(tree::written_as(Form::Plain, &tree!(3)), tree!(3));
    let plain = tree::select_forms_in(&Plain, &pdu, "/message/protocolIEs/0/value").unwrap();
    assert_eq!(plain, [(&tree!("c0"), Form::Plain)]);
}
