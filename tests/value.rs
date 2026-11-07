//! The tree itself: a JSON value whose objects keep the order of their members, which
//! `tree!` writes, and which converts to and from a `serde_json::Value`.

use oxirush_utils::tree;
use oxirush_utils::tree::{Map, Value};
use std::time::{Duration, Instant};

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
    assert_ne!(tree!("7"), tree!(7));
    // Lists and objects of one length and other contents, and of other lengths
    for (a, b) in [
        (tree!([1, 2]), tree!([1, 3])),
        (tree!([1]), tree!([1, 1])),
        (tree!({"a": 1}), tree!({"a": 2})),
        (tree!({"a": 1}), tree!({"a": 1, "b": 1})),
    ] {
        assert_ne!(a, b);
        assert_ne!(b, a);
    }
    // Wide objects in two orders, compared by the order of their names.
    let forward: Map = (0..4096)
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let backward: Map = (0..4096)
        .rev()
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let started = Instant::now();
    assert_eq!(forward, backward);
    assert!(
        started.elapsed() < Duration::from_secs(5),
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
    assert_eq!(map.insert("n".into(), tree!(3)), None);
    assert_eq!(map.keys().collect::<Vec<_>>(), ["z", "a", "m", "n"]);
    assert_eq!(map.shift_remove("z"), Some(tree!("z")));
    assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "m", "n"]);
    *map.entry("b").or_insert(tree!(0)) = tree!(4);
    *map.entry("m").or_insert(Value::Null) = tree!(5);
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
fn an_object_of_many_members_keeps_them_where_they_are() {
    // The calls of the test above, on an object of more members than a value of a
    // protocol has: each member is where it was put, and is found by its name.
    let mut map: Map = (0..100)
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let names = |map: &Map| map.keys().cloned().collect::<Vec<_>>();
    let mut expected = names(&map);
    assert_eq!(map.insert("m7".into(), tree!("seven")), Some(tree!(7)));
    assert_eq!(map["m7"], tree!("seven"));
    assert_eq!(map.shift_remove("m50"), Some(tree!(50)));
    expected.retain(|name| name != "m50");
    assert_eq!(names(&map), expected);
    assert_eq!(map.shift_remove("m10"), Some(tree!(10)));
    expected.retain(|name| name != "m10");
    assert_eq!(names(&map), expected);
    *map.entry("new").or_insert(tree!(0)) = tree!(1);
    expected.push("new".into());
    map.retain(|name, _| name.len() == 2 || name == "new");
    expected.retain(|name| name.len() == 2 || name == "new");
    assert_eq!(names(&map), expected);
    map.extend([
        ("m5".to_string(), tree!(5.5)),
        ("last".to_string(), tree!(1)),
    ]);
    expected.push("last".into());
    assert_eq!(names(&map), expected);
    for (position, name) in expected.iter().enumerate() {
        assert_eq!(map.get_key_value(name).map(|(name, _)| name), Some(name));
        assert!(map.contains_key(name), "{position} {name}");
    }
    assert_eq!((map.get("m10"), map.get("m50")), (None, None));
    assert_eq!((&map["m5"], &map["m7"]), (&tree!(5.5), &tree!("seven")));
    map.retain(|name, _| name.starts_with('m') && name.len() == 2);
    assert_eq!(
        names(&map),
        ["m0", "m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9"]
    );
    assert_eq!(map.insert("m9".into(), tree!(true)), Some(tree!(9)));
    map.clear();
    assert!(map.is_empty() && map.get("m1").is_none());
}

#[test]
fn an_object_that_is_read_has_4096_members_at_most() {
    let object = |count: usize| {
        let members: Vec<String> = (0..count)
            .map(|index| format!("\"m{index}\":{index}"))
            .collect();
        format!("{{{}}}", members.join(","))
    };
    let read: Value = object(4096).parse().unwrap();
    assert_eq!(read.as_object().map(Map::len), Some(4096));
    assert_eq!(read["m4095"], tree!(4095));
    let error = object(4097).parse::<Value>().unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("an object has 4096 members at most"),
        "{error}"
    );
    // In a list, as deep as it is
    let error = format!("[{{\"a\":{}}}]", object(4097))
        .parse::<Value>()
        .unwrap_err();
    assert!(
        error.to_string().starts_with("an object has 4096"),
        "{error}"
    );
}

#[test]
fn a_tree_is_indexed_as_a_value_of_serde_json_is() {
    let mut tree = tree!({"ies": [{"id": 10, "value": "x"}], "a/b": {"~": 1}});
    assert_eq!(tree["ies"][0]["id"], tree!(10));
    assert_eq!(tree["ies"][7]["id"], Value::Null);
    assert_eq!(tree["none"], Value::Null);
    assert_eq!(tree["a/b"]["~"], tree!(1));
    let Value::Object(members) = &tree else {
        unreachable!()
    };
    assert!(members.contains_key("ies") && !members.contains_key("none"));
    assert_eq!(tree.get("ies"), Some(&tree!([{"id": 10, "value": "x"}])));
    assert_eq!((tree.get("none"), tree["ies"].get("id")), (None, None));
    tree["ies"][0]["value"] = tree!("y");
    tree["new"]["member"] = tree!(1);
    assert_eq!(tree["new"], tree!({"member": 1}));
    assert_eq!(tree["ies"][0]["value"], tree!("y"));
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

#[test]
fn a_number_with_a_fraction_is_read_back_from_what_it_prints() {
    // A number is read as the nearest f64, as Rust's own reader reads it, and printed as
    // the shortest text that gives it back.
    for text in [
        "1.000000000000007287",
        "0.1",
        "1e-7",
        "2.2250738585072014e-308",
        "123456789012345678901234567890",
        "-1.5e300",
        "0.30000000000000004",
    ] {
        let read: Value = text.parse().unwrap();
        assert_eq!(read.as_f64(), text.parse::<f64>().ok(), "{text}");
        assert_eq!(read.to_string().parse::<Value>().unwrap(), read, "{text}");
    }
}

#[test]
fn a_value_says_what_it_is_and_gives_what_it_holds() {
    let mut values = [
        tree!(null),
        tree!(true),
        tree!(7),
        tree!(-7),
        tree!(1.5),
        tree!("text"),
        tree!([1]),
        tree!({"a": 1}),
    ];
    // What each one is, in the order of `values`: null, boolean, number, string, list,
    // object
    let kinds = [
        [1, 0, 0, 0, 0, 0],
        [0, 1, 0, 0, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [0, 0, 0, 1, 0, 0],
        [0, 0, 0, 0, 1, 0],
        [0, 0, 0, 0, 0, 1],
    ];
    for (value, kind) in values.iter().zip(kinds) {
        let is = [
            value.is_null(),
            value.is_boolean(),
            value.is_number(),
            value.is_string(),
            value.is_array(),
            value.is_object(),
        ];
        assert_eq!(is, kind.map(|bit| bit == 1), "{value}");
        assert_eq!(value.as_number().is_some(), kind[2] == 1, "{value}");
        assert_eq!(value.as_array().is_some(), kind[4] == 1, "{value}");
    }
    assert_eq!(values[1].as_bool(), Some(true));
    assert_eq!(
        (values[2].as_u64(), values[3].as_i64()),
        (Some(7), Some(-7))
    );
    assert_eq!(values[4].as_f64(), Some(1.5));
    assert_eq!(values[5].as_str(), Some("text"));
    // A list and an object change through what they give
    values[6].as_array_mut().unwrap().push(tree!(2));
    assert_eq!(values[6], tree!([1, 2]));
    assert_eq!(values[5].as_array_mut(), None);
    *values[7].get_mut("a").unwrap() = tree!(2);
    assert_eq!(values[7], tree!({"a": 2}));
    assert_eq!(values[7].get_mut("b"), None);
    values[6][0] = tree!(0);
    assert_eq!(values[6], tree!([0, 2]));
}

#[test]
fn an_object_shows_its_members_in_their_order() {
    let mut map = Map::new();
    map.insert("b".to_string(), tree!(1));
    map.insert("a".to_string(), tree!([true]));
    assert_eq!(
        format!("{map:?}"),
        r#"{"b": Number(1), "a": Array [Bool(true)]}"#
    );
    map["b"] = tree!(2);
    assert_eq!(map.get("b"), Some(&tree!(2)));
}

#[test]
fn a_value_is_made_from_what_it_holds() {
    let number = serde_json::Number::from(5);
    let from = [
        (Value::from(&1.5), tree!(1.5)),
        (Value::from(&true), tree!(true)),
        (Value::from(&"text"), tree!("text")),
        (Value::from(&"text".to_string()), tree!("text")),
        (Value::from(number), tree!(5)),
        (Value::from(&tree!([1])), tree!([1])),
        (Value::from(&[1, 2][..]), tree!([1, 2])),
        (Value::from([3, 4]), tree!([3, 4])),
        (Value::from(()), Value::Null),
        (Value::from(Some(7)), tree!(7)),
        (Value::from(None::<u8>), Value::Null),
    ];
    for (made, expected) in from {
        assert_eq!(made, expected);
    }
    let mut members = Map::new();
    members.insert("a".to_string(), tree!(1));
    assert_eq!(Value::from(members.clone()), tree!({"a": 1}));
    // Through a mutable object, each member changes
    for (_, value) in &mut members {
        *value = tree!(9);
    }
    assert_eq!(members["a"], tree!(9));
}
