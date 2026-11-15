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
/// A row is `field: "Name" P TYPE [INTERFACES]`, where `P` is `M`, `C` or `O`, followed by
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
            $( $field:ident : $row:literal $p:ident $(* $($max:literal)?)? $vty:ident
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
                    ie_type: <$vty as crate::$im::IeType>::IE_TYPE,
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
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
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
                            && ie_type == <$vty as crate::$im::IeType>::IE_TYPE
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
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
                        return $crate::table!(@place {proto: $proto, table: $tm, ie: $im, check: $cm, paths: $pm, error: $error, ctors: $ctors} [$(* $($max)?)?] $p ($vty) self.$field, body);
                    }
                )*
                crate::$im::Placing::Unexpected
            }

            fn emit(&self, ie_type: <$proto as $crate::tables::Protocol>::Key, nth: usize, out: &mut alloc::vec::Vec<u8>) -> Result<bool, $error> {
                $(
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
                        return crate::$tm::emit_field::<$vty, _>(&self.$field, nth, out);
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
                    crate::$im::encode_value(value, out)?;
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
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
                        return crate::$tm::FieldValues::<$vty>::values(&self.$field).len();
                    }
                )*
                0
            }

            fn clear(&mut self, ie_type: <$proto as $crate::tables::Protocol>::Key) {
                $(
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
                        self.$field = Default::default();
                        return;
                    }
                )*
            }

            fn decodes(&self, ie_type: <$proto as $crate::tables::Protocol>::Key, octets: &[u8]) -> bool {
                $(
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
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
                let mut off = 0u64;
                if CONDITIONS {
                    let (rows, mut row) = (<Self as $crate::tables::Scope>::table().rows, 0);
                    $(
                        if !crate::$tm::FieldValues::<$vty>::values(&self.$field).is_empty() && !rows[row].roles.iter().any(|role| role.interfaces.has(context.interface)) {
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
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
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
                    if ie_type == <$vty as crate::$im::IeType>::IE_TYPE {
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
    (@field $fam:tt [] Cb ($t:ty)) => { Option<alloc::boxed::Box<$t>> };
    (@field $fam:tt [] Ob ($t:ty)) => { Option<alloc::boxed::Box<$t>> };
    (@field $fam:tt [] $p:ident ($t:ty)) => { Option<$t> };
    (@field $fam:tt [* $($max:literal)?] $p:ident ($t:ty)) => { alloc::vec::Vec<$t> };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} M) => { crate::$tm::Presence::Mandatory };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} C) => { crate::$tm::Presence::Conditional };
    (@presence {proto: $proto:ty, table: $tm:ident, ie: $im:ident, check: $cm:ident, paths: $pm:ident, error: $error:ty, ctors: $ctors:ident} O) => { crate::$tm::Presence::Optional };
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
