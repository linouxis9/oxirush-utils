/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! `tree!`: a tree written as JSON in the code, its members in the order written.

/// A [`tree::Value`](crate::tree::Value) written as JSON, as `serde_json::json!` writes
/// one, whose objects keep their members in the order in which they are written. A value
/// in it is a literal or any expression that converts into a tree, which it takes as a
/// call takes its argument: a variable that holds a tree moves into it, and is given by
/// reference to stay where it is (`&name`); a name is a string literal or an expression,
/// as a variable, that converts into a `String`.
///
/// ```
/// use oxirush_utils::tree;
///
/// let name = "sd";
/// let slice = tree!({"sst": 1, name: "0000ff", "plmns": ["208-93", null], "default": true});
/// assert_eq!(slice.to_string(), r#"{"sst":1,"sd":"0000ff","plmns":["208-93",null],"default":true}"#);
/// ```
#[macro_export]
macro_rules! tree {
    ($($tree:tt)+) => {
        $crate::__tree!($($tree)+)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __tree {
    // ── The entries of a list: those done, then what is left ──
    (@list [$($done:expr,)*]) => {
        $crate::tree::__vec![$($done,)*]
    };
    (@list [$($done:expr),*]) => {
        $crate::tree::__vec![$($done),*]
    };
    (@list [$($done:expr,)*] null $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!(null)] $($rest)*)
    };
    (@list [$($done:expr,)*] true $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!(true)] $($rest)*)
    };
    (@list [$($done:expr,)*] false $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!(false)] $($rest)*)
    };
    (@list [$($done:expr,)*] [$($list:tt)*] $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!([$($list)*])] $($rest)*)
    };
    (@list [$($done:expr,)*] {$($object:tt)*} $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!({$($object)*})] $($rest)*)
    };
    (@list [$($done:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!($next),] $($rest)*)
    };
    (@list [$($done:expr,)*] $last:expr) => {
        $crate::__tree!(@list [$($done,)* $crate::__tree!($last)])
    };
    (@list [$($done:expr),*] , $($rest:tt)*) => {
        $crate::__tree!(@list [$($done,)*] $($rest)*)
    };

    // ── The members of an object: the name being read, what is left, and a copy ──
    (@object $object:ident () () ()) => {};
    (@object $object:ident [$($name:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($name)+).into(), $value);
        $crate::__tree!(@object $object () ($($rest)*) ($($rest)*));
    };
    (@object $object:ident [$($name:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($name)+).into(), $value);
    };
    (@object $object:ident ($($name:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!(null)) $($rest)*);
    };
    (@object $object:ident ($($name:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!(true)) $($rest)*);
    };
    (@object $object:ident ($($name:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!(false)) $($rest)*);
    };
    (@object $object:ident ($($name:tt)+) (: [$($list:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!([$($list)*])) $($rest)*);
    };
    (@object $object:ident ($($name:tt)+) (: {$($inner:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!({$($inner)*})) $($rest)*);
    };
    (@object $object:ident ($($name:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!($value)) , $($rest)*);
    };
    (@object $object:ident ($($name:tt)+) (: $value:expr) $copy:tt) => {
        $crate::__tree!(@object $object [$($name)+] ($crate::__tree!($value)));
    };
    // A name in parentheses is an expression.
    (@object $object:ident () (($name:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object ($name) (: $($rest)*) (: $($rest)*));
    };
    // Another token of the name.
    (@object $object:ident ($($name:tt)*) ($token:tt $($rest:tt)*) $copy:tt) => {
        $crate::__tree!(@object $object ($($name)* $token) ($($rest)*) ($($rest)*));
    };

    // ── A value ──
    (null) => {
        $crate::tree::Value::Null
    };
    (true) => {
        $crate::tree::Value::Bool(true)
    };
    (false) => {
        $crate::tree::Value::Bool(false)
    };
    ([]) => {
        $crate::tree::Value::Array($crate::tree::__vec![])
    };
    ([ $($list:tt)+ ]) => {
        $crate::tree::Value::Array($crate::__tree!(@list [] $($list)+))
    };
    ({}) => {
        $crate::tree::Value::Object($crate::tree::Map::new())
    };
    ({ $($members:tt)+ }) => {
        $crate::tree::Value::Object({
            let mut object = $crate::tree::Map::new();
            $crate::__tree!(@object object () ($($members)+) ($($members)+));
            object
        })
    };
    ($other:expr) => {
        $crate::tree::Value::from($other)
    };
}
