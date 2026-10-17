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

//! `Serialize` and `Deserialize` for a type through its tree, so that every format of
//! serde carries the form that the tree has.
//!
//! [`serialize`] and [`deserialize`] are the two functions that an implementation
//! calls, and [`impl_serde!`](crate::impl_serde) writes the two implementations for a
//! type that is a [`Tree`]. A format that is read has to describe itself, as JSON and
//! YAML do: the tree is read first, then the value from it.
//!
//! ```
//! use oxirush_utils::{tree, tree::{Members, Reading, Refused, Tree, Value}};
//!
//! #[derive(Debug, PartialEq)]
//! struct Tai {
//!     plmn: String,
//!     tac: u32,
//! }
//!
//! impl Tree for Tai {
//!     const NAME: &'static str = "TAI";
//!
//!     fn to_tree(&self) -> Value {
//!         let mut members = Members::new();
//!         members.put("plmn", tree::text(&self.plmn));
//!         members.put("tac", tree::unsigned(self.tac.into()));
//!         members.finish()
//!     }
//!
//!     fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
//!         let members = reading.members(value, &["plmn", "tac"])?;
//!         Ok(Self {
//!             plmn: reading.member(&members, 0, |value, reading| reading.text(value))?.into(),
//!             tac: reading.member(&members, 1, |value, reading| reading.unsigned(value, 0, 0xff_ffff))? as u32,
//!         })
//!     }
//! }
//!
//! oxirush_utils::impl_serde!(Tai);
//!
//! let tai = Tai { plmn: "208-93".into(), tac: 1 };
//! let text = serde_json::to_string(&tai).unwrap();
//! assert_eq!(text, r#"{"plmn":"208-93","tac":1}"#);
//! assert_eq!(serde_json::from_str::<Tai>(&text).unwrap(), tai);
//!
//! // What the tree refuses, the format refuses with the same words
//! let error = serde_json::from_str::<Tai>(r#"{"plmn":"208-93","tac":"1"}"#).unwrap_err();
//! assert!(error.to_string().starts_with("this is not a value of TAI: a number from 0 to 16777215"));
//! ```

use super::value::Value;
use super::{Tree, from_value, to_value};
use serde_core::de::Error as _;

// The traits of serde, which `impl_serde!` names here: a crate that calls it need not
// depend on serde itself.
#[doc(hidden)]
pub use serde_core::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialize `value` as its tree.
///
/// # Errors
///
/// Those of the serializer.
pub fn serialize<T: Tree, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
    to_value(value).serialize(serializer)
}

/// Deserialize a value from its tree, which the format has to describe.
///
/// # Errors
///
/// Those of the deserializer, and why the tree is not a value of the type.
pub fn deserialize<'de, T: Tree, D: Deserializer<'de>>(deserializer: D) -> Result<T, D::Error> {
    let tree = Value::deserialize(deserializer)?;
    from_value(&tree).map_err(D::Error::custom)
}

/// `Serialize` and `Deserialize` for each of the types named, which are
/// [`Tree`](crate::tree::Tree): through their tree, with
/// [`serialize`](crate::tree::serde::serialize) and
/// [`deserialize`](crate::tree::serde::deserialize).
///
/// A type is named by its name, with its lifetime after it, as `Name<'a>`, when it has
/// one: the value that is deserialized is one of any lifetime. A type with generic
/// parameters writes the two implementations itself, in three lines each.
#[macro_export]
macro_rules! impl_serde {
    ($($name:ident $(<$life:lifetime>)?),+ $(,)?) => {$(
        impl$(<$life>)? $crate::tree::serde::Serialize for $name$(<$life>)? {
            fn serialize<S: $crate::tree::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                $crate::tree::serde::serialize(self, serializer)
            }
        }

        impl<'de $(, $life)?> $crate::tree::serde::Deserialize<'de> for $name$(<$life>)? {
            fn deserialize<D: $crate::tree::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                $crate::tree::serde::deserialize(deserializer)
            }
        }
    )+};
    ($($name:ty),+ $(,)?) => {$(
        impl $crate::tree::serde::Serialize for $name {
            fn serialize<S: $crate::tree::serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                $crate::tree::serde::serialize(self, serializer)
            }
        }

        impl<'de> $crate::tree::serde::Deserialize<'de> for $name {
            fn deserialize<D: $crate::tree::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                $crate::tree::serde::deserialize(deserializer)
            }
        }
    )+};
}
