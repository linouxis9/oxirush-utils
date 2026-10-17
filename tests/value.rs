//! The tree itself: a JSON value whose objects keep the order of their members, which
//! `tree!` writes, and which converts to and from a `serde_json::Value`.

use oxirush_utils::tree;
use oxirush_utils::tree::{Map, Value};

#[test]
fn tree_writes_json_in_the_order_written() {
    let name = "sd";
    let plmns = vec!["208-93", "001-01"];
    let tree = tree!({
        "sst": 1,
        name: "0000ff",
        ("de".to_string() + "fault"): true,
        "plmns": plmns,
        "nested": [null, false, [], {}, {"b": 2, "a": [1, -1, 1.5]}],
        "absent": None::<u8>,
    });
    assert_eq!(
        tree.to_string(),
        r#"{"sst":1,"sd":"0000ff","default":true,"plmns":["208-93","001-01"],"nested":[null,false,[],{},{"b":2,"a":[1,-1,1.5]}],"absent":null}"#
    );
    assert_eq!(tree!([1, 2,]), tree!([1, 2]));
    assert_eq!(tree!({"a": 1,}), tree!({"a": 1}));
    assert_eq!(
        format!("{:#}", tree!({"a": [1]})),
        "{\n  \"a\": [\n    1\n  ]\n}"
    );
}

#[test]
fn two_trees_are_equal_whatever_the_order_of_their_members() {
    assert_eq!(
        tree!({"a": 1, "b": {"c": 2, "d": 3}}),
        tree!({"b": {"d": 3, "c": 2}, "a": 1})
    );
    assert_ne!(tree!({"a": 1, "b": 2}), tree!({"a": 1, "c": 2}));
    assert_ne!(tree!({"a": 1}), tree!({"a": 1, "b": 2}));
    assert_ne!(tree!([1, 2]), tree!([2, 1]));
    assert_ne!(tree!(1), tree!(1.0));
    // With a value of serde_json, and with a plain value.
    assert_eq!(
        tree!({"b": 1, "a": [true]}),
        serde_json::json!({"a": [true], "b": 1})
    );
    assert_eq!(serde_json::json!({"a": null}), tree!({"a": null}));
    assert_eq!(tree!(7), 7);
    assert_eq!(tree!(-7), -7i64);
    assert_eq!(tree!("x"), "x");
    assert_eq!(tree!(true), true);
    assert_ne!(tree!("7"), 7);
    // Wide objects in two orders, compared in a time that grows as n log n.
    let forward: Map = (0..65_536)
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let backward: Map = (0..65_536)
        .rev()
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let started = std::time::Instant::now();
    assert_eq!(forward, backward);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn an_object_keeps_its_members_where_they_are() {
    let mut map = Map::new();
    for name in ["z", "a", "m"] {
        assert_eq!(map.insert(name.into(), tree!(name)), None);
    }
    assert_eq!(map.insert("a".into(), tree!(1)), Some(tree!("a")));
    assert_eq!(map.keys().collect::<Vec<_>>(), ["z", "a", "m"]);
    assert_eq!(map.shift_insert(0, "m".into(), tree!(2)), Some(tree!("m")));
    assert_eq!(map.shift_insert(9, "n".into(), tree!(3)), None);
    assert_eq!(map.keys().collect::<Vec<_>>(), ["m", "z", "a", "n"]);
    assert_eq!(map.remove("z"), Some(tree!("z")));
    assert_eq!(map.keys().collect::<Vec<_>>(), ["m", "a", "n"]);
    *map.entry("b").or_insert(tree!(0)) = tree!(4);
    *map.entry("m").or_default() = tree!(5);
    map.retain(|name, _| name != "a");
    assert_eq!(
        Value::Object(map.clone()).to_string(),
        r#"{"m":5,"n":3,"b":4}"#
    );
    map.extend([("m".to_string(), tree!(6)), ("c".to_string(), tree!(7))]);
    assert_eq!(
        Value::Object(map).to_string(),
        r#"{"m":6,"n":3,"b":4,"c":7}"#
    );
}

#[test]
fn a_tree_is_indexed_as_a_value_of_serde_json_is() {
    let mut tree = tree!({"ies": [{"id": 10, "value": "x"}], "a/b": {"~": 1}});
    assert_eq!(tree["ies"][0]["id"], 10);
    assert_eq!(tree["ies"][7]["id"], Value::Null);
    assert_eq!(tree["none"], Value::Null);
    assert_eq!(
        tree.get("ies").and_then(|ies| ies.get(0)),
        Some(&tree!({"id": 10, "value": "x"}))
    );
    assert_eq!(tree.pointer("/ies/0/value"), Some(&tree!("x")));
    assert_eq!(tree.pointer("/a~1b/~0"), Some(&tree!(1)));
    assert_eq!(tree.pointer(""), Some(&tree.clone()));
    assert_eq!(tree.pointer("ies"), None);
    *tree.pointer_mut("/ies/0/value").unwrap() = tree!("y");
    tree["new"]["member"] = tree!(1);
    assert_eq!(tree["new"], tree!({"member": 1}));
    assert_eq!(tree["ies"][0]["value"].take(), "y");
    assert_eq!(tree["ies"][0]["value"], Value::Null);
}

#[test]
fn a_tree_goes_through_serde_json_and_back() {
    let tree = tree!({"z": [1, {"y": null}], "a": 18446744073709551615u64, "m": -9223372036854775808i64, "f": 0.5});
    let json = serde_json::Value::from(tree.clone());
    assert_eq!(Value::from(json.clone()), tree);
    assert_eq!(Value::from(&json), tree);
    assert_eq!(serde_json::Value::from(&tree), json);
    // Text is read in the order written, and written in the order of the tree.
    let text = r#"{"z":[1,{"y":null}],"a":18446744073709551615,"m":-9223372036854775808,"f":0.5}"#;
    assert_eq!(tree.to_string(), text);
    let read: Value = text.parse().unwrap();
    assert_eq!(read.to_string(), text);
    assert_eq!(serde_json::to_value(&tree).unwrap(), json);
    assert_eq!(serde_json::from_value::<Value>(json).unwrap(), tree);
}
