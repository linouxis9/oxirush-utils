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

//! The macro that writes the scope of a table.

/// A message or the content of a grouped IE, from its table: a struct with a field for
/// each row, `new()` with the mandatory IEs, a `with_` builder for each row, the codec,
/// the check and the tree.
///
/// The first argument names the protocol and its modules, which a protocol's crate passes
/// in a wrapper: `{proto: P, table: m, ie: m, check: m, paths: m, error: E, ctors: c}`, the
/// modules of `crate` that have its table items (`Presence`, `Interfaces`, `Content`,
/// `emit_field`), its IE items (`IeType`, `Codec`, `Listed`, `RawIe`, `encode_value`), its
/// checks and its trees, and `c` its macro of constructors by interface.
///
/// A row is `field: "Name" P TYPE [INTERFACES]`, or `field: "Name" P TYPE INSTANCE
/// [INTERFACES]` for a protocol whose IEs are keyed by their type and instance, where `P`
/// is `M`, `C`, `CO` or `O` (as the protocol's presences name them), followed by
/// `*` for a list or `*N` for a list of at most N, and `TYPE` the type of its value. `Cb`
/// and `Ob` are `C` and `O` of a grouped IE that the field boxes, as it is large and most
/// scopes lack it. A field that several rows share, which the wire does not tell apart,
/// lists them after `roles`, each with its presence and its interfaces, which are the
/// row's; its own are those where a constructor takes it: where one of its rows is
/// mandatory. A row whose condition the scope alone shows ends with
/// `absent(|scope| condition)`, where the check reports it there, and `if |scope|
/// condition`, where it reports it missing. A table whose rows are the scope above rows
/// with a condition on its IEs begins with `@ function;`, which checks those conditions
/// from it. `by [INTERFACES]` after its table's number gives a table whose mandatory IEs
/// differ between its interfaces a constructor for each, which the protocol's macro of
/// constructors writes, and no `new()`.
// `crate` in the expansion is the protocol's crate, whose modules the first argument names.
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! table {
    // A table whose mandatory IEs differ between its interfaces has a constructor for each
    // interface that it lists, and no `new()`.
    (
        $fam:tt
        $(#[$meta:meta])*
        $kind:ident $ty:ident = $code:literal, $name:literal, $id:literal by [$($by:ident)*] { $($rows:tt)* }
    ) => {
        $crate::table!(@table $fam [$($by)*] $(#[$meta])* $kind $ty = $code, $name, $id { $($rows)* });
    };
    (
        $fam:tt
        $(#[$meta:meta])*
        $kind:ident $ty:ident = $code:literal, $name:literal, $id:literal { $($rows:tt)* }
    ) => {
        $crate::table!(@table $fam [] $(#[$meta])* $kind $ty = $code, $name, $id { $($rows)* });
    };
    (@table {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [$($by:ident)*]
        $(#[$meta:meta])*
        $kind:ident $ty:ident = $code:literal, $name:literal, $id:literal {
            $(@ $across:path ;)?
            $( $field:ident : $row:literal $p:ident $(* $($max:literal)?)? $vty:ident $($instance:literal)?
                [$($interface:ident)*] $(roles [$($role:literal $rp:ident $(* $($rmax:literal)?)? [$($ri:ident)*]),*])?
                $(absent(|$ascope:ident| $absent:expr))? $(if |$scope:ident| $condition:expr)? , )*
        }
    ) => { $crate::fields::__pastey::paste! {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $ty {
            $(
                #[doc = concat!("The ", $row, " IE.")]
                pub $field: $crate::table!(@field {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] $p ($vty)),
            )*
            /// The IEs that the scope keeps as octets, in the order of the wire.
            pub raw: alloc::vec::Vec<crate::$im::RawIe>,
            /// The order of the wire, when it is not that of the table: each row, its values
            /// then the IEs of its type in `raw`, and the IEs of no row last.
            pub order: alloc::vec::Vec<crate::$im::Placed>,
        }

        const _: () = assert!(
            <[&str]>::len(&[$($row),*]) < crate::$tm::MAX_ROWS,
            "a table of more rows than the encoder counts"
        );
        // A field holds as many IEs as its roles allow.
        const _: () = {$(
            let roles: &[crate::$tm::Role] = $crate::table!(@roles {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $row $p [$(* $($max)?)?] [$($interface)*] $([$($role $rp [$(* $($rmax)?)?] [$($ri)*]),*])?);
            assert!(
                crate::$tm::capacity(roles) == crate::$tm::capacity(&[$crate::table!(@role {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $row $p [$(* $($max)?)?] [])]),
                concat!("the field of ", $row, " holds other than its roles")
            );
        )*};

        pub(crate) static [<$ty:snake:upper _TABLE>]: crate::$tm::Table = crate::$tm::Table {
            id: $id,
            name: $name,
            code: $code,
            constructors: crate::$tm::Interfaces(0 $(| crate::$tm::Interfaces::$by.0)*),
            rows: &[$(
                crate::$tm::Row {
                    name: $row,
                    ie_type: $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?),
                    roles: $crate::table!(@roles {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $row $p [$(* $($max)?)?] [$($interface)*] $([$($role $rp [$(* $($rmax)?)?] [$($ri)*]),*])?),
                    content: <$vty as crate::$tm::Content>::TABLE,
                    checked: $crate::table!(@checked {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $($scope)? $($ascope)?),
                },
            )*],
        };

        impl $ty {
            /// The table of the scope.
            pub fn table() -> &'static crate::$tm::Table {
                &[<$ty:snake:upper _TABLE>]
            }

            $(
                #[doc = concat!("Builder: add the ", $row, " IE.")]
                pub fn [<with_ $field>](mut self, value: $vty) -> Self {
                    crate::$tm::FieldValues::<$vty>::add(&mut self.$field, value);
                    self
                }
            )*
        }

        $crate::table!(@ctors {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$($by)*] $ty { $( $field : $row $p $(* $($max)?)? $vty [$($interface)*] , )* });

        // A table without rows leaves its arguments unused.
        #[allow(unused_variables, unused_mut, unused_assignments)]
        impl $crate::tables::Scope for $ty {
            type P = $proto;
            fn table() -> &'static crate::$tm::Table {
                &[<$ty:snake:upper _TABLE>]
            }

            #[allow(unused_assignments, unused_variables, unused_mut)]
            fn position(ie_type: <$proto as $crate::tables::Protocol>::Key) -> Option<usize> {
                let mut at = 0;
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return Some(at);
                    }
                    at += 1;
                )*
                None
            }

            const PRESIZE: bool = false $( || $crate::table!(@large {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] ($vty)) )*;

            // Only the IEs that can be values are counted: not one of a null length, nor
            // one whose IEs run past it, which the scope keeps as octets.
            #[allow(unused_variables)]
            fn presize(&mut self, octets: &[u8]) {
                $( let mut [<count_ $field>] = 0usize; )*
                for (ie_type, content) in <$proto as $crate::tables::Protocol>::ies(octets).flatten() {
                    $(
                        if $crate::table!(@large {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] ($vty))
                            && ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?)
                        {
                            if !content.is_empty() && <$proto as $crate::tables::Protocol>::ies(content).all(|inner| inner.is_ok()) {
                                [<count_ $field>] += 1;
                            }
                            continue;
                        }
                    )*
                }
                $( $crate::table!(@reserve {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] self.$field, [<count_ $field>]); )*
            }

            fn place(&mut self, ie_type: <$proto as $crate::tables::Protocol>::Key, body: &[u8]) -> crate::$im::Placing {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return $crate::table!(@place {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] $p ($vty) self.$field, body);
                    }
                )*
                crate::$im::Placing::Unexpected
            }

            fn emit(&self, ie_type: <$proto as $crate::tables::Protocol>::Key, nth: usize, out: &mut alloc::vec::Vec<u8>) -> Result<bool, $error> {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return crate::$tm::emit_field::<$vty, _>(ie_type, &self.$field, nth, out);
                    }
                )*
                Ok(false)
            }

            fn encoded_len(&self) -> usize {
                let mut length = 0;
                $( $crate::table!(@each {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] ($vty) self.$field, |value| {
                    length += 4 + crate::$im::Codec::encoded_len(value);
                }); )*
                for ie in &self.raw {
                    length += $crate::tables::Raw::encoded_len(ie);
                }
                length
            }

            fn encode_fields(&self, out: &mut alloc::vec::Vec<u8>) -> Result<(), $error> {
                $( $crate::table!(@each {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] ($vty) self.$field, |value| {
                    crate::$im::encode_value($crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?), value, out)?;
                }); )*
                Ok(())
            }

            #[cfg(feature = "endpoint")]
            fn held(&self) -> usize {
                let mut held = self.raw.capacity() * core::mem::size_of::<crate::$im::RawIe>()
                    + self.order.capacity() * core::mem::size_of::<crate::$im::Placed>();
                $( held += $crate::table!(@held {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] $p ($vty) self.$field); )*
                held
            }

            fn count(&self, ie_type: <$proto as $crate::tables::Protocol>::Key) -> usize {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return crate::$tm::FieldValues::<$vty>::values(&self.$field).len();
                    }
                )*
                0
            }

            fn clear(&mut self, ie_type: <$proto as $crate::tables::Protocol>::Key) {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        self.$field = Default::default();
                        return;
                    }
                )*
            }

            fn decodes(&self, ie_type: <$proto as $crate::tables::Protocol>::Key, octets: &[u8]) -> bool {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return <$vty as crate::$im::Codec>::decode(octets).is_some();
                    }
                )*
                false
            }

            fn plain(&self) -> bool {
                self.raw.is_empty() $( && $crate::table!(@plain {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] ($vty) self.$field) )*
            }

            fn is_empty(&self) -> bool {
                self.raw.is_empty() $( && crate::$tm::FieldValues::<$vty>::values(&self.$field).is_empty() )*
            }

            fn raw(&self) -> &[crate::$im::RawIe] { &self.raw }
            fn raw_mut(&mut self) -> &mut alloc::vec::Vec<crate::$im::RawIe> { &mut self.raw }
            fn order(&self) -> &[crate::$im::Placed] { &self.order }
            fn order_mut(&mut self) -> &mut alloc::vec::Vec<crate::$im::Placed> { &mut self.order }
        }

        // A table without rows leaves its arguments unused.
        #[allow(unused_variables, unused_mut, unused_assignments)]
        impl crate::$cm::Checked for $ty {
            fn check_values(&self, path: &crate::$cm::Path, context: crate::$cm::Context, found: &mut alloc::vec::Vec<crate::$cm::Deviation>) {
                // The conditions read the scope as its receiver on the interface takes it:
                // without the values of the rows that the interface does not have.
                const CONDITIONS: bool = false $( || $crate::table!(@checked {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $($scope)? $($ascope)?) )*;
                let mut off = 0u128;
                if CONDITIONS {
                    let (rows, mut row) = (<Self as $crate::tables::Scope>::table().rows, 0);
                    $(
                        if !crate::$tm::FieldValues::<$vty>::values(&self.$field).is_empty() && !rows[row].roles.iter().any(|role| $crate::tables::Interfaces::has(role.interfaces, <$proto as $crate::tables::Checking>::interfaces(context))) {
                            off |= 1 << row;
                        }
                        row += 1;
                    )*
                }
                let taken = crate::$cm::taken(self, off);
                let view: &Self = taken.as_ref().unwrap_or(self);
                let mut row = 0;
                $(
                    crate::$cm::check_field_values::<$vty, _, _>(
                        self, row, &self.$field, path, context, found,
                    );
                    $({
                        let $scope: &Self = view;
                        if $condition {
                            crate::$cm::condition_missing(self, row, path, context, found);
                        }
                    })?
                    $({
                        let $ascope: &Self = view;
                        if $absent {
                            crate::$cm::condition_absent(self, row, path, context, found);
                        }
                    })?
                    row += 1;
                )*
                $( $across(self, path, context, found); )?
            }
        }

        $crate::table!(@kind {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $kind $ty);

        #[cfg(feature = "tree")]
        // A table without rows leaves its arguments unused.
        #[allow(unused_variables, unused_mut, unused_assignments)]
        impl crate::$pm::TreeScope for $ty {
            fn field_tree(&self, ie_type: <$proto as $crate::tables::Protocol>::Key, nth: usize) -> Option<$crate::tree::Value> {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return crate::$pm::field_tree::<$vty, _>(&self.$field, nth);
                    }
                )*
                None
            }

            fn read_field(
                &mut self,
                ie_type: <$proto as $crate::tables::Protocol>::Key,
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> Result<(), $crate::tree::Refused> {
                $(
                    if ie_type == $crate::table!(@key {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $vty $($instance)?) {
                        return crate::$pm::read_field::<$vty, _>(&mut self.$field, value, reading);
                    }
                )*
                Ok(())
            }
        }
    } };

    // ── The parts of a row ──────────────────────────────────────────────────

    // A row that is there once is a field of its value, in place, but a grouped IE of `Cb`
    // or `Ob`, large and rare, which is boxed.
    // The key of the IEs of a row: its IE type, or its IE type and its instance.
    (@key {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $vty:ident) => { <$vty as crate::$im::IeType>::IE_TYPE };
    (@key {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $vty:ident $instance:literal) => { (<$vty as crate::$im::IeType>::IE_TYPE, $instance) };
    (@field $fam:tt [] Cb ($t:ty)) => { Option<alloc::boxed::Box<$t>> };
    (@field $fam:tt [] Ob ($t:ty)) => { Option<alloc::boxed::Box<$t>> };
    (@field $fam:tt [] $p:ident ($t:ty)) => { Option<$t> };
    (@field $fam:tt [* $($max:literal)?] $p:ident ($t:ty)) => { alloc::vec::Vec<$t> };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} M) => { crate::$tm::Presence::Mandatory };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} C) => { crate::$tm::Presence::Conditional };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} O) => { crate::$tm::Presence::Optional };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} CO) => { crate::$tm::Presence::ConditionalOptional };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} Cb) => { crate::$tm::Presence::Conditional };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} Ob) => { crate::$tm::Presence::Optional };
    // The roles of a row: the row itself, or the rows that share its field.
    (@roles $fam:tt $row:literal $p:ident [$($repeat:tt)*] [$($interface:ident)*]) => {
        &[$crate::table!(@role $fam $row $p [$($repeat)*] [$($interface)*])]
    };
    (@roles $fam:tt $row:literal $p:ident $repeat:tt $interfaces:tt [$($role:literal $rp:ident [$($rrepeat:tt)*] [$($ri:ident)*]),*]) => {
        &[$($crate::table!(@role $fam $role $rp [$($rrepeat)*] [$($ri)*])),*]
    };
    (@role {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $name:literal $p:ident [$($repeat:tt)*] [$($interface:ident)*]) => {
        crate::$tm::Role {
            name: $name,
            presence: $crate::table!(@presence {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $p),
            repeat: $crate::table!(@repeat {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $($repeat)*),
            interfaces: crate::$tm::Interfaces(0 $(| crate::$tm::Interfaces::$interface.0)*),
        }
    };
    (@repeat $fam:tt) => { $crate::tables::Repeat::Once };
    (@repeat {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} *) => { $crate::tables::Repeat::Any };
    (@repeat {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} * $max:literal) => { $crate::tables::Repeat::AtMost($max) };
    // Each value of a field: that of a field there once, or those of a list.
    (@each {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [] ($t:ty) $f:expr, |$value:ident| $body:block) => {
        if let Some($value) = crate::$tm::FieldValues::<$t>::first(&$f) $body
    };
    (@each {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [* $($max:literal)?] ($t:ty) $f:expr, |$value:ident| $body:block) => {
        for $value in crate::$tm::FieldValues::<$t>::values(&$f) $body
    };
    (@checked $fam:tt) => { false };
    (@checked $fam:tt $($scope:ident)+) => { true };
    (@place $fam:tt [] $p:ident ($t:ty) $f:expr, $body:expr) => {
        $crate::table!(@place_once $fam $p ($t) $f, $body)
    };
    (@place_once {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} Cb ($t:ty) $f:expr, $body:expr) => {
        $crate::table!(@placing {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $f, crate::$tm::decode_boxed(&mut $f, $body))
    };
    (@place_once {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} Ob ($t:ty) $f:expr, $body:expr) => {
        $crate::table!(@placing {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $f, crate::$tm::decode_boxed(&mut $f, $body))
    };
    (@place_once {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $p:ident ($t:ty) $f:expr, $body:expr) => {
        $crate::table!(@placing {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $f, crate::$im::Listed::decode_into(&mut $f, $body))
    };
    // A row there once that has its value repeats it; one that has none takes it when it
    // decodes.
    (@placing {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $f:expr, $decoded:expr) => {
        match $f.is_some() {
            true => crate::$im::Placing::Repeated,
            false if $decoded => crate::$im::Placing::Placed,
            false => crate::$im::Placing::Malformed,
        }
    };
    (@place {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [* $($max:literal)?] $p:ident ($t:ty) $f:expr, $body:expr) => {
        if $crate::table!(@full {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$($max)?] $f) {
            crate::$im::Placing::Repeated
        } else if <$t as crate::$im::Listed>::decode_push(&mut $f, $body) {
            crate::$im::Placing::Placed
        } else {
            crate::$im::Placing::Malformed
        }
    };
    // A list of values larger than this is sized before it is filled.
    (@large $fam:tt [] ($t:ty)) => { false };
    (@large $fam:tt [* $($max:literal)?] ($t:ty)) => { core::mem::size_of::<$t>() > 64 };
    (@reserve $fam:tt [] $f:expr, $n:expr) => { let _ = $n; };
    (@reserve $fam:tt [* $($max:literal)?] $f:expr, $n:expr) => { $f.reserve_exact($n) };
    // What a field takes beyond its own size: a list its entries, a box its value.
    (@held {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [* $($max:literal)?] $p:ident ($t:ty) $f:expr) => {
        $f.capacity() * core::mem::size_of::<$t>() + $f.iter().map(crate::$im::Listed::held).sum::<usize>()
    };
    (@held $fam:tt [] Cb ($t:ty) $f:expr) => { $crate::table!(@held_box $fam ($t) $f) };
    (@held $fam:tt [] Ob ($t:ty) $f:expr) => { $crate::table!(@held_box $fam ($t) $f) };
    (@held {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [] $p:ident ($t:ty) $f:expr) => { $f.as_ref().map_or(0, crate::$im::Listed::held) };
    (@held_box $fam:tt ($t:ty) $f:expr) => {
        $f.as_ref().map_or(0, |value| core::mem::size_of::<$t>() + $crate::tables::Scope::held(&**value))
    };
    // Whether a field reads as it is: as many values as the row takes, each plain.
    (@plain {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [] ($t:ty) $f:expr) => { crate::$tm::FieldValues::<$t>::values(&$f).iter().all(crate::$im::Listed::plain) };
    (@plain {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [* $($max:literal)?] ($t:ty) $f:expr) => { !$crate::table!(@over {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$($max)?] $f) && $f.iter().all(crate::$im::Listed::plain) };
    (@over $fam:tt [] $f:expr) => { false };
    (@over $fam:tt [$max:literal] $f:expr) => { $f.len() > $max };
    (@full $fam:tt [] $f:expr) => { false };
    (@full $fam:tt [$max:literal] $f:expr) => { $f.len() >= $max };

    // ── A message, or the content of a grouped IE ───────────────────────────

    (@kind {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} group $ty:ident) => {
        impl crate::$im::Codec for $ty {
            fn decode(octets: &[u8]) -> Option<Self> {
                crate::$tm::decode_scope(octets).ok()
            }

            fn encode(&self, out: &mut alloc::vec::Vec<u8>) -> Result<(), $error> {
                crate::$tm::encode_scope(self, out)
            }

            fn encoded_len(&self) -> usize {
                $crate::tables::Scope::encoded_len(self)
            }
        }

        impl crate::$im::Listed for $ty {
            // A grouped IE is large: it is decoded in its place, and not moved there.
            fn decode_into(slot: &mut Option<Self>, octets: &[u8]) -> bool {
                let scope = slot.insert(Self::default());
                if crate::$tm::decode_scope_into(scope, octets).is_err() {
                    *slot = None;
                    return false;
                }
                true
            }

            fn decode_push(list: &mut alloc::vec::Vec<Self>, octets: &[u8]) -> bool {
                list.push(Self::default());
                let last = list.len() - 1;
                if crate::$tm::decode_scope_into(&mut list[last], octets).is_err() {
                    list.pop();
                    return false;
                }
                true
            }

            #[cfg(feature = "endpoint")]
            fn held(&self) -> usize {
                $crate::tables::Scope::held(self)
            }

            fn check_in(&self, path: &crate::$cm::Path, context: crate::$cm::Context, found: &mut alloc::vec::Vec<crate::$cm::Deviation>) {
                crate::$cm::check_scope(self, path, context, found);
            }

            /// Of a grouped IE that has an IE: one of a null length reads as none.
            fn plain(&self) -> bool {
                $crate::tables::Scope::plain(self) && !$crate::tables::Scope::is_empty(self)
            }
        }

        impl crate::$cm::Rules for $ty {
            /// Its table on the interfaces that it has, as the receiver of a response: the
            /// IEs of any, and those that all need; as its receiver reads it
            /// ([`crate::$cm::as_read`]).
            fn check(&self, path: &crate::$cm::Path, found: &mut alloc::vec::Vec<crate::$cm::Deviation>) {
                let interfaces = <Self as $crate::tables::Scope>::table().interfaces();
                let context = crate::$cm::Context::on(interfaces);
                let read = crate::$cm::as_read(self, path, context, found);
                crate::$cm::check_scope(read.as_ref().unwrap_or(self), path, context, found);
            }
        }

        #[cfg(feature = "tree")]
        impl $crate::tree::Tree for $ty {
            const NAME: &'static str = <$ty as crate::$im::IeType>::TYPE_NAME;

            fn to_tree(&self) -> $crate::tree::Value {
                crate::$pm::scope_to_tree(self)
            }

            fn from_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> Result<Self, $crate::tree::Refused> {
                crate::$pm::scope_from_tree(value, reading)
            }
        }
    };
    (@kind $fam:tt message $ty:ident) => {};

    // The constructors: `new()` with the mandatory rows of every interface, or one for each
    // interface that the table lists, with the mandatory rows of that interface.
    (@ctors {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} [$($by:ident)+] $ty:ident $rows:tt) => {
        $ctors!(@ctors {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$($by)+] $ty $rows);
    };
    (@ctors $fam:tt [] $ty:ident { $($rows:tt)* }) => {
        $crate::table!(@new $fam new * $ty "The scope with its mandatory IEs, which each interface of its table has alike." [] [] $($rows)*);
    };
    // A constructor, with the fields of the mandatory rows of `$on` (`*` for any interface),
    // which a muncher collects.
    (@new {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $name:ident $on:tt $ty:ident $doc:literal [$($arg:tt)*] [$( ($f:ident [$($m:tt)*] ($ft:ty)) )*]) => {
        impl $ty {
            #[doc = $doc]
            #[allow(clippy::too_many_arguments)]
            pub fn $name($($arg)*) -> Self {
                #[allow(unused_mut)]
                let mut scope = Self::default();
                $( crate::$tm::FieldValues::<$ft>::add(&mut scope.$f, $f); )*
                scope
            }
        }
    };
    (@new $fam:tt $name:ident $on:tt $ty:ident $doc:literal [$($arg:tt)*] [$($set:tt)*] $field:ident : $row:literal M $(* $($max:literal)?)? $vty:ident [$($i:ident)*] , $($rest:tt)*) => {
        $crate::table!(@in $fam $on [$($i)*] $name $ty $doc [$($arg)*] [$($set)*] ($field [$(* $($max)?)?] ($vty)) $($rest)*);
    };
    (@new $fam:tt $name:ident $on:tt $ty:ident $doc:literal [$($arg:tt)*] [$($set:tt)*] $field:ident : $row:literal $p:ident $(* $($max:literal)?)? $vty:ident [$($i:ident)*] , $($rest:tt)*) => {
        $crate::table!(@new $fam $name $on $ty $doc [$($arg)*] [$($set)*] $($rest)*);
    };
    // A mandatory row is an argument where its interfaces have `$on`.
    (@in $fam:tt * [$($i:ident)*] $name:ident $ty:ident $doc:literal [$($arg:tt)*] [$($set:tt)*] ($field:ident [$($m:tt)*] ($t:ty)) $($rest:tt)*) => {
        $crate::table!(@new $fam $name * $ty $doc [$($arg)* $field: $t,] [$($set)* ($field [$($m)*] ($t))] $($rest)*);
    };
    // A mandatory row of an interface of the protocol: the protocol's macro of
    // constructors tells whether the row has it.
    (@in {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} $on:ident $($rest:tt)*) => {
        $ctors!(@in {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} $on $($rest)*);
    };

}

/// The IE types of a protocol's table of IE types, one line each: the type, its name,
/// its form and the number of fixed octets that the table gives it, the clause or the
/// table that defines it, and the types of the crate that are its values. The first
/// argument names the protocol and the modules of its crate that hold its table items
/// (`Content`) and its IE items (`IeType`, `Codec`): `{proto: P, table: m, ie: m}`. It
/// writes `IE_TYPES`, `takes_of`, `value_tree`, `ie_type_layout` (with `spec`) and the
/// protocol's [`RegistryTree`](crate::tables::RegistryTree).
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! ies {
    ({proto: $proto:ty, table: $tm:ident, ie: $im:ident} $( $num:literal $name:literal $form:ident $(($fixed:literal))? $clause:literal $(=> $($ty:ident),+)? ; )*) => {
        /// Every IE type of the table of IE types, in the order of their numbers.
        pub static IE_TYPES: &[$crate::tables::IeTypeInfo<$proto>] = &[
            $( $crate::tables::IeTypeInfo::<$proto> {
                ie_type: $num,
                name: $name,
                form: $crate::ies!(@form $form),
                fixed: $crate::ies!(@fixed $($fixed)?),
                clause: $clause,
                members: $crate::ies!(@members $form $($($ty),+)?),
                scalar: $crate::fields::one($crate::ies!(@fields $form $($($ty),+)?)),
                tables: $crate::ies!(@tables $form $($($ty),+)?),
            }, )*
        ];

        /// The names of the flags and fields of the figure of `ie_type` that the crate lays
        /// out, and how it lays them out, which the test `spec` compares with the figure:
        /// for that test alone.
        #[cfg(feature = "spec")]
        #[doc(hidden)]
        pub fn ie_type_layout(ie_type: u16) -> (&'static [&'static str], Option<&'static $crate::fields::Layout>) {
            match ie_type {
                $( $num => ($crate::ies!(@fields $form $($($ty),+)?), $crate::ies!(@layout $form $($($ty),+)?)), )*
                _ => (&[], None),
            }
        }
        $( $crate::ies!(@impls {proto: $proto, table: $tm, ie: $im} $form $num $name [$crate::ies!(@length $form $(($fixed))?)] $($($ty),+)?); )*

        #[cfg(feature = "tree")]
        impl $crate::tables::RegistryTree for $proto {
            fn takes_of(ie_type: u16) -> Option<&'static $crate::tree::Takes> {
                takes_of(ie_type)
            }
            fn value_tree(
                ie_type: u16,
                table: Option<&'static $crate::tables::Table<$proto>>,
                octets: &[u8],
            ) -> Option<$crate::tree::Value> {
                value_tree(ie_type, table, octets)
            }
        }

        /// What the tree of a value of `ie_type` takes: none for a grouped IE.
        #[cfg(feature = "tree")]
        pub(crate) fn takes_of(ie_type: u16) -> Option<&'static $crate::tree::Takes> {
            match ie_type {
                $( $num => $crate::ies!(@takes $form $($($ty),+)?), )*
                _ => None,
            }
        }

        /// The tree of the value of `ie_type` that `octets` are, of the content of the table
        /// `table` for a grouped IE, or of its first: `None` for octets that are no such
        /// value, and for a table of another type.
        #[cfg(feature = "tree")]
        pub(crate) fn value_tree(
            ie_type: u16,
            table: Option<&'static $crate::tables::Table<$proto>>,
            octets: &[u8],
        ) -> Option<$crate::tree::Value> {
            match ie_type {
                $( $num => $crate::ies!(@value_tree {proto: $proto, table: $tm, ie: $im} $form table octets $($($ty),+)?), )*
                _ => None,
            }
        }
    };
    (@impls {proto: $proto:ty, table: $tm:ident, ie: $im:ident} $form:ident $num:literal $name:literal [$fixed:expr] $($ty:ident),*) => {
        $(
            impl crate::$im::IeType for $ty {
                const IE_TYPE: u16 = $num;
                const TYPE_NAME: &'static str = $name;
                const FIXED: Option<u16> = $fixed;
            }

            impl crate::$tm::Content for $ty {
                const TABLE: Option<&'static $crate::tables::Table<$proto>> = $crate::ies!(@content {proto: $proto, table: $tm, ie: $im} $form $ty);
            }
        )*
    };
    (@content {proto: $proto:ty, table: $tm:ident, ie: $im:ident} grouped $ty:ident) => { $crate::fields::__pastey::paste! { Some(&[<$ty:snake:upper _TABLE>]) } };
    (@content {proto: $proto:ty, table: $tm:ident, ie: $im:ident} $form:ident $ty:ident) => { None };
    (@form fixed) => { $crate::tables::Form::Fixed };
    (@form variable) => { $crate::tables::Form::Variable };
    (@form extendable) => { $crate::tables::Form::Extendable };
    (@form grouped) => { $crate::tables::Form::Grouped };
    (@fixed) => { None };
    (@fixed $fixed:literal) => { Some($fixed) };
    (@length fixed ($fixed:literal)) => { Some($fixed) };
    (@length $form:ident $(($fixed:literal))?) => { None };
    (@fields grouped $($ty:ident),+) => { &[] };
    (@fields $form:ident $ty:ident) => { $ty::FIELDS };
    (@takes grouped $($ty:ident),+) => { None };
    (@takes $form:ident $ty:ident) => { Some($ty::TAKES) };
    (@value_tree {proto: $proto:ty, table: $tm:ident, ie: $im:ident} grouped $table:ident $octets:ident $($ty:ident),+) => { $crate::fields::__pastey::paste! {{
        $(
            if $table.is_none_or(|table| core::ptr::eq(table, &[<$ty:snake:upper _TABLE>])) {
                return <$ty as crate::$im::Codec>::decode($octets).map(|value| $crate::tree::Tree::to_tree(&value));
            }
        )+
        None
    }} };
    (@value_tree {proto: $proto:ty, table: $tm:ident, ie: $im:ident} $form:ident $table:ident $octets:ident $ty:ident) => {
        match $table {
            Some(_) => None,
            None => <$ty as crate::$im::Codec>::decode($octets).map(|value| $crate::tree::Tree::to_tree(&value)),
        }
    };
    (@members grouped $($ty:ident),+) => { &[] };
    (@members $form:ident $ty:ident) => { $ty::MEMBERS };
    (@layout grouped $($ty:ident),+) => { None };
    (@layout $form:ident $ty:ident) => { $ty::LAYOUT };
    (@layout $form:ident) => { None };
    (@tables grouped $($ty:ident),+) => { $crate::fields::__pastey::paste! { &[$(&[<$ty:snake:upper _TABLE>]),+] } };
    (@tables $form:ident $($ty:ident),*) => { &[] };
}

/// The messages of a protocol's table of message types, one each: their type, the struct
/// of their table and their interfaces. It writes `MESSAGE_TYPES` and the body of a message,
/// and what is done with it by its type. The first argument names the protocol and the
/// modules of its crate: `{proto: P, table: m, ie: m, check: m, paths: m, message: m,
/// registry: m, error: E}`.
#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! messages {
    ({proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, message: $mm:ident, registry: $rm:ident, error: $error:ty} $( $code:literal $ty:ident [$($interface:ident)*] ),* $(,)?) => { $crate::fields::__pastey::paste! {
        /// Every message type of the table of message types, in the order of their numbers.
        pub static MESSAGE_TYPES: &[crate::$mm::MessageTypeInfo] = &[$(
            crate::$mm::MessageTypeInfo {
                message_type: $code,
                interfaces: crate::$tm::Interfaces(0 $(| crate::$tm::Interfaces::$interface.0)*),
                table: &crate::$rm::[<$ty:snake:upper _TABLE>],
            },
        )*];

        /// The body of a message: one type of the table of message types each.
        #[derive(Debug, Clone, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Body {
            $(
                #[doc = concat!("Message type ", stringify!($code), ".")]
                $ty(alloc::boxed::Box<crate::$rm::$ty>),
            )*
            /// A message type that the table of message types does not have, with the
            /// octets after its header.
            Other {
                /// The message type.
                message_type: u8,
                /// The octets of its IEs.
                octets: $crate::fields::Octets,
            },
        }

        $(
            impl From<crate::$rm::$ty> for Body {
                fn from(body: crate::$rm::$ty) -> Self {
                    Self::$ty(alloc::boxed::Box::new(body))
                }
            }
        )*

        impl Body {
            /// The message type.
            pub fn message_type(&self) -> u8 {
                match self {
                    $( Self::$ty(_) => $code, )*
                    Self::Other { message_type, .. } => *message_type,
                }
            }

            /// The table of the message.
            pub fn table(&self) -> Option<&'static crate::$tm::Table> {
                match self {
                    $( Self::$ty(_) => Some(crate::$rm::$ty::table()), )*
                    Self::Other { .. } => None,
                }
            }

            /// The name that the table of message types gives the message.
            pub fn name(&self) -> Option<&'static str> {
                self.table().map(|table| table.name)
            }

            /// The names of the messages, in the order of their types.
            #[cfg(feature = "tree")]
            pub(crate) fn names() -> [&'static str; [$($code),*].len()] {
                [$( crate::$rm::$ty::table().name ),*]
            }

            pub(crate) fn decode(message_type: u8, octets: &[u8]) -> Result<Self, crate::$im::IeFault> {
                Ok(match message_type {
                    $( $code => {
                        // The message is decoded in its box, and not moved there.
                        let mut body = alloc::boxed::Box::<crate::$rm::$ty>::default();
                        crate::$tm::decode_scope_sized(&mut *body, octets, true)?;
                        Self::$ty(body)
                    } )*
                    other => Self::Other { message_type: other, octets: $crate::fields::Octets::copied(octets) },
                })
            }

            pub(crate) fn encode(&self, out: &mut alloc::vec::Vec<u8>) -> Result<(), $error> {
                match self {
                    $( Self::$ty(body) => crate::$tm::encode_scope(&**body, out), )*
                    Self::Other { octets, .. } => {
                        out.extend_from_slice(octets);
                        Ok(())
                    }
                }
            }

            pub(crate) fn encoded_len(&self) -> usize {
                match self {
                    $( Self::$ty(body) => $crate::tables::Scope::encoded_len(&**body), )*
                    Self::Other { octets, .. } => octets.len(),
                }
            }

            /// The octets that the body takes beyond its own size: its box, and what its
            /// IEs hold ([`Listed::held`](crate::$im::Listed::held)).
            #[cfg(feature = "endpoint")]
            pub(crate) fn held(&self) -> usize {
                match self {
                    $( Self::$ty(body) => core::mem::size_of::<crate::$rm::$ty>() + $crate::tables::Scope::held(&**body), )*
                    Self::Other { .. } => 0,
                }
            }

            /// The body as its receiver reads its octets ([`crate::$cm::as_read`]).
            pub(crate) fn as_read(&self, context: crate::$cm::Context, found: &mut alloc::vec::Vec<crate::$cm::Deviation>) -> Option<Self> {
                let path = crate::$cm::Path::Root(crate::$cm::ROOT);
                match self {
                    $( Self::$ty(body) => crate::$cm::as_read(&**body, &path, context, found).map(Self::from), )*
                    Self::Other { .. } => None,
                }
            }

            pub(crate) fn check(&self, path: &crate::$cm::Path, context: crate::$cm::Context, found: &mut alloc::vec::Vec<crate::$cm::Deviation>) {
                match self {
                    $( Self::$ty(body) => crate::$cm::check_scope(&**body, path, context, found), )*
                    Self::Other { message_type, .. } => {
                        let _ = context;
                        found.push(crate::$cm::unknown_message(*message_type));
                    }
                }
            }

            #[cfg(feature = "tree")]
            pub(crate) fn ies_tree(&self) -> $crate::tree::Value {
                match self {
                    $( Self::$ty(body) => crate::$pm::scope_to_tree(&**body), )*
                    Self::Other { octets, .. } => $crate::tree::alternative("octets", $crate::tree::octets(octets)),
                }
            }

            /// The body of the message of `kind`, a position among the names or a message
            /// type that none has, from the tree of its IEs.
            #[cfg(feature = "tree")]
            pub(crate) fn from_ies_tree(
                kind: Result<usize, u8>,
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> Result<Self, $crate::tree::Refused> {
                const CODES: &[u8] = &[$($code),*];
                let code = match kind {
                    Ok(position) => CODES[position],
                    Err(code) => code,
                };
                Ok(match code {
                    $( $code => Self::$ty(alloc::boxed::Box::new(crate::$pm::scope_from_tree(value, reading)?)), )*
                    other => {
                        let (_, octets) = reading.alternative(value, &["octets"])?;
                        let octets = reading.within("octets", octets, |value, reading| reading.octets(value))?;
                        Self::Other { message_type: other, octets: octets.into() }
                    }
                })
            }
        }
    }};
}
