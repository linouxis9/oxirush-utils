//! The tree itself: a JSON value whose objects keep the order of their members, which
//! `tree!` writes, and which converts to and from a `serde_json::Value`.

use oxirush_utils::tree;
use oxirush_utils::tree::{Map, Members, Value};
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
fn an_object_of_many_members_keeps_them_where_they_are() {
    // The calls of the test above, on an object of more members than a value of a
    // protocol has: each member is where it was put, and is found by its name.
    let mut map: Map = (0..100)
        .map(|index| (format!("m{index}"), tree!(index)))
        .collect();
    let names = |map: &Map| map.keys().cloned().collect::<Vec<_>>();
    let mut expected = names(&map);
    assert_eq!(map.insert("m7".into(), tree!("seven")), Some(tree!(7)));
    assert_eq!(map["m7"], "seven");
    assert_eq!(
        map.shift_insert(0, "m50".into(), tree!(50)),
        Some(tree!(50))
    );
    expected.retain(|name| name != "m50");
    expected.insert(0, "m50".into());
    assert_eq!(names(&map), expected);
    assert_eq!(map.remove("m10"), Some(tree!(10)));
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
fn an_object_does_what_a_list_of_its_members_does_on_both_sides_of_its_index() {
    // Each call on an object that grows past the 32 members above which it keeps an index
    // of its names and shrinks back, against a list of the members.
    let mut map = Map::new();
    let mut list: Vec<(String, Value)> = Vec::new();
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = |below: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 33) % below
    };
    for step in 0..20_000 {
        let name = format!("m{}", next(80));
        let value = tree!(step);
        let at = list.iter().position(|(known, _)| *known == name);
        match next(if list.len() < 40 { 4 } else { 7 }) {
            0 | 1 => {
                let was = at.map(|at| core::mem::replace(&mut list[at].1, value.clone()));
                if at.is_none() {
                    list.push((name.clone(), value.clone()));
                }
                assert_eq!(map.insert(name, value), was);
            }
            2 => {
                let position = next(list.len() as u64 + 2) as usize;
                let was = at.map(|at| list.remove(at).1);
                list.insert(position.min(list.len()), (name.clone(), value.clone()));
                assert_eq!(map.shift_insert(position, name, value), was);
            }
            3 => {
                if at.is_none() {
                    list.push((name.clone(), value.clone()));
                }
                let expected = list
                    .iter()
                    .find(|(known, _)| *known == name)
                    .unwrap()
                    .1
                    .clone();
                assert_eq!(*map.entry(name).or_insert(value), expected);
            }
            4 | 5 => {
                let was = at.map(|at| list.remove(at).1);
                assert_eq!(map.shift_remove(&name), was);
            }
            _ => {
                let parity = next(2).to_string();
                list.retain(|(known, _)| !known.ends_with(&parity));
                map.retain(|known, _| !known.ends_with(&parity));
            }
        }
        let names: Vec<&String> = list.iter().map(|(name, _)| name).collect();
        assert_eq!(map.keys().collect::<Vec<_>>(), names, "{step}");
        for (name, value) in &list {
            assert_eq!(map.get(name), Some(value), "{step} {name}");
        }
        assert_eq!(map.get("m80"), None);
    }
}

#[test]
fn an_object_of_n_members_is_built_and_read_in_a_time_that_grows_as_n_log_n() {
    // What the reader of a message does with an object that a peer writes: each member is
    // put into another object, one after the other; then each is found, changed and
    // written by its name.
    let count = 100_000;
    let members: Vec<String> = (0..count)
        .map(|index| format!("\"m{index}\":{index}"))
        .collect();
    let text = format!("{{{}}}", members.join(","));
    let read: Value = text.parse().unwrap();
    let started = Instant::now();
    let mut copy = Map::new();
    for (name, value) in read.as_object().unwrap() {
        assert_eq!(copy.insert(name.clone(), value.clone()), None);
    }
    let mut written = Members::new();
    for (name, value) in &copy {
        written.put(name, value.clone());
    }
    let collected: Map = copy.clone().into_iter().collect();
    for index in 0..count {
        let name = format!("m{index}");
        assert_eq!(copy.get(&name), Some(&tree!(index)));
        *copy.get_mut(&name).unwrap() = tree!(index + 1);
        *copy.entry(name).or_default() = tree!(index);
    }
    let mut tree = Value::Object(copy);
    for index in 0..1000 {
        tree::set(&mut tree, &format!("/m{}", count - 1 - index), tree!(true)).unwrap();
    }
    let elapsed = started.elapsed();
    assert_eq!(written.finish(), read);
    assert_eq!(Value::Object(collected), read);
    assert_eq!(tree[format!("m{}", count - 1).as_str()], true);
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
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
