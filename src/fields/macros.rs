/// A value of octets of bits: flags of one bit and fields of a few, each at its octet
/// (from 0), its lowest bit (from 1) and its width, in at most `N` octets of which the
/// first `MIN` are always there; then the octets that a later release adds. The other
/// bits are spare and kept, and so is how many of the `N` octets a decoded value had. A
/// value of a table that only a flag gives meaning to, `if flag`, is checked where the
/// flag is set. The rules of its clause on which flags go together follow it.
///
/// `over [Row, row]`, then `impl Type (field)`, writes no value: the accessors of the fields
/// over the octets of a value that holds them, `field` an octet or an
/// [`Octets`](crate::fields::Octets) ([`Storage`](crate::fields::Storage)): for each
/// field its getter and its setter, which returns the value, and for a field of more
/// than one bit its getter and setter of bits (`_raw`). The setter of a field of more
/// than one bit refuses a value that its bits do not hold
/// ([`OutOfRange`](crate::fields::OutOfRange)); the setter of bits writes those of the
/// value that the field has. Then `MASKS`, `spare_bits_are_zero()`,
/// `FIELDS`, a `Row` of the protocol for each field, which its macro `row!` writes of
/// `(field, name, octet, low, width, [octet2 low2 width2], [Kind])`, the brackets empty
/// for a field of one part and one of no table, and `LAYOUT`.
/// A field `as` a table's values is read and written through
/// [`Narrowable`](crate::fields::Narrowable).
///
/// Otherwise, the first argument names the protocol's items that the value is written with: its
/// codec trait, its trait of rules, its trait of IE types, its deviation, its error, and
/// the function that makes a [`FieldError`](crate::fields::FieldError) its error. A
/// protocol's crate wraps the macro with them, as PFCP's Apply Action (TS 29.244 §8.2.26)
/// is written:
///
/// ```text
/// pub struct ApplyAction "8.2.26" octets 2 min 1 {
///     drop "DROP": 0 1 1, forw "FORW": 0 2 1, edrt "EDRT": 1 1 1,
/// }
/// check(action, path, found) { ... }
/// ```
#[macro_export]
macro_rules! bits {
    // The accessors of the fields of bits over the octets of a value that holds them, an
    // octet or `Octets`: no codec, no tree. The protocol names its row type and the macro
    // that writes a row of it.
    (
        over [$row:ty, $row_macro:ident]
        $(#[$meta:meta])*
        impl $ty:ident ($storage:tt) $clause:literal octets $n:literal {
            $( $field:ident $name:literal : $octet:literal $low:literal $width:tt $(+ $octet2:literal $low2:literal $width2:tt)? $(as $kind:ident)? ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        impl $ty {
            /// The bits of each octet that a field has: the others are spare.
            pub const MASKS: [u8; $n] = {
                let mut masks = [0u8; $n];
                $( $crate::bits!(@mask masks, $octet, $low, $width [$($kind)? $(+ $octet2 $low2 $width2)?]); )*
                masks
            };
            /// Its fields, in the order of the figure.
            #[allow(dead_code)]
            pub(crate) const FIELDS: &'static [$row] = &[$(
                $row_macro!($field, $name, $octet, $low, $width, [$($octet2 $low2 $width2)?], [$($kind)?]),
            )*];
            /// Its layout, which a test compares with the figure of its clause.
            #[doc(hidden)]
            pub const LAYOUT: $crate::fields::Layout = $crate::fields::Layout {
                head: &[],
                bits: &[$(
                    $crate::fields::Bits { name: $name, ident: stringify!($field), octet: $octet, bit: $low, width: $width },
                    $( $crate::fields::Bits { name: $name, ident: stringify!($field), octet: $octet2, bit: $low2, width: $width2 }, )?
                )*],
                fields: &[],
                read: |_| Some($crate::__alloc::vec::Vec::new()),
            };

            /// Whether the bits that no field has are 0 in each of its octets.
            pub fn spare_bits_are_zero(&self) -> bool {
                (0..$n).all(|at| $crate::fields::Storage::octet(&self.$storage, at) & !Self::MASKS[at] == 0)
            }

            $( $crate::bits!(@over $storage $field $name [$octet $low $width] [$($kind)? $(+ $octet2 $low2 $width2)?]); )*
        }
    };
    // A flag: its getter and its setter.
    (@over $storage:tt $field:ident $name:literal [$octet:literal $low:literal 1] []) => { $crate::fields::__pastey::paste! {
        #[doc = concat!("The ", $name, " flag.")]
        pub fn $field(&self) -> bool {
            $crate::fields::get_bits(&self.$storage, $octet, $low, 1) == 1
        }
        #[doc = concat!("Set or clear the ", $name, " flag.")]
        pub fn [<set_ $field>](&mut self, value: bool) -> &mut Self {
            $crate::fields::set_bits(&mut self.$storage, $octet, $low, 1, u8::from(value));
            self
        }
    }};
    // A number, or a value of a table: its getter and setter, and those of its bits.
    (@over $storage:tt $field:ident $name:literal [$octet:literal $low:literal $width:tt] [$($kind:ident)?]) => { $crate::fields::__pastey::paste! {
        #[doc = concat!("The ", $name, " field.")]
        pub fn $field(&self) -> $crate::bits!(@type $width [$($kind)?]) {
            $crate::bits!(@from [$($kind)?] self.[<$field _raw>]())
        }
        #[doc = concat!("The bits of the ", $name, " field.")]
        pub fn [<$field _raw>](&self) -> u8 {
            $crate::fields::get_bits(&self.$storage, $octet, $low, $width)
        }
        #[doc = concat!("Set the ", $name, " field: a value that its bits do not hold is refused.")]
        pub fn [<set_ $field>](
            &mut self,
            value: $crate::bits!(@type $width [$($kind)?]),
        ) -> core::result::Result<&mut Self, $crate::fields::OutOfRange> {
            let bits: u8 = $crate::bits!(@into [$($kind)?] value);
            $crate::fields::OutOfRange::check($name, u32::from(bits), $width)?;
            core::result::Result::Ok(self.[<set_ $field _raw>](bits))
        }
        #[doc = concat!("Set the bits of the ", $name, " field: those of `value` that it has.")]
        pub fn [<set_ $field _raw>](&mut self, value: u8) -> &mut Self {
            $crate::fields::set_bits(&mut self.$storage, $octet, $low, $width, value);
            self
        }
    }};
    // A number of two parts, the high bits in the first.
    (@over $storage:tt $field:ident $name:literal [$octet:literal $low:literal $width:tt] [+ $octet2:literal $low2:literal $width2:tt]) => { $crate::fields::__pastey::paste! {
        #[doc = concat!("The ", $name, " field.")]
        pub fn $field(&self) -> u16 {
            self.[<$field _raw>]()
        }
        #[doc = concat!("The bits of the ", $name, " field, the high ones first.")]
        pub fn [<$field _raw>](&self) -> u16 {
            (u16::from($crate::fields::get_bits(&self.$storage, $octet, $low, $width)) << $width2)
                | u16::from($crate::fields::get_bits(&self.$storage, $octet2, $low2, $width2))
        }
        #[doc = concat!("Set the ", $name, " field: a value that its bits do not hold is refused.")]
        pub fn [<set_ $field>](
            &mut self,
            value: u16,
        ) -> core::result::Result<&mut Self, $crate::fields::OutOfRange> {
            $crate::fields::OutOfRange::check($name, u32::from(value), $width + $width2)?;
            core::result::Result::Ok(self.[<set_ $field _raw>](value))
        }
        #[doc = concat!("Set the bits of the ", $name, " field: those of `value` that it has.")]
        pub fn [<set_ $field _raw>](&mut self, value: u16) -> &mut Self {
            $crate::fields::set_bits(&mut self.$storage, $octet, $low, $width, (value >> $width2) as u8);
            $crate::fields::set_bits(&mut self.$storage, $octet2, $low2, $width2, (value & ((1u16 << $width2) - 1)) as u8);
            self
        }
    }};
    (@from [] $raw:expr) => { $raw };
    (@from [$kind:ident] $raw:expr) => { <$kind as $crate::fields::Narrowable>::from_value($raw) };
    (@into [] $value:expr) => { $value };
    (@into [$kind:ident] $value:expr) => { <$kind as $crate::fields::Narrowable>::value($value) };
    (
        [$codec:path, $rules_trait:path, $ie_type:path, $deviation:ty, $error:ty, $error_of:path]
        $(#[$meta:meta])*
        pub struct $ty:ident $clause:literal octets $n:literal min $min:literal {
            $( $field:ident $name:literal : $octet:literal $low:literal $width:tt $(+ $octet2:literal $low2:literal $width2:tt)? $(as $kind:ident)? $(if $flag:ident)? ),* $(,)?
        }
        $( check($this:ident, $path:ident, $found:ident) $rules:block )?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $ty {
            $(
                #[doc = concat!("The ", $name, " field.")]
                pub $field: $crate::bits!(@type $width [$($kind)? $(+ $octet2 $low2 $width2)?]),
            )*
            /// The bits of each octet that no field has, which are spare.
            pub spare: [u8; $n],
            /// How many of the octets of the fields the value has: the first ones of a
            /// later release are absent from a sender of an earlier one.
            pub present: u8,
            /// The octets after them, which a later release adds and a receiver ignores,
            /// `"unknown octets"` in a tree.
            pub extra: $crate::fields::Octets,
        }

        impl core::default::Default for $ty {
            fn default() -> Self {
                Self { $( $field: core::default::Default::default(), )* spare: [0; $n], present: $n, extra: core::default::Default::default() }
            }
        }

        $crate::bits!(@new $ty $([$field $name $width [$($kind)? $(+ $octet2 $low2 $width2)?]])*);

        impl $ty {
            /// The names of its flags and fields.
            pub(crate) const FIELDS: &'static [&'static str] = &[$($name),*];
            /// The members that its tree may have: its flags and fields, its spare bits, the
            /// number of the octets of its fields that it has, and its unknown octets.
            pub(crate) const MEMBERS: &'static [&'static str] = &[$($name,)* "Spare", "length", $crate::fields::UNKNOWN_OCTETS];
            /// What its tree takes: its flags and fields, or the one field that is no flag.
            #[cfg(feature = "tree")]
            pub(crate) const TAKES: &'static $crate::tree::Takes = &$crate::tree::Takes::Members(
                if Self::BARE { $crate::fields::one(Self::FIELDS) } else { None },
                &[
                    $( ($name, $crate::bits!(@takes $width [$($kind)? $(+ $octet2 $low2 $width2)?])), )*
                    ("Spare", &$crate::tree::Takes::Octets(Some($n))),
                    ("length", &$crate::tree::Takes::Number($n)),
                    ($crate::fields::UNKNOWN_OCTETS, &$crate::tree::Takes::Octets(None)),
                ],
            );
            /// Its layout.
            #[doc(hidden)]
            #[cfg(feature = "spec")]
            pub const LAYOUT: Option<&'static $crate::fields::Layout> = Some(&$crate::fields::Layout {
                head: &[],
                bits: &[$(
                    $crate::fields::Bits { name: $name, ident: stringify!($field), octet: $octet, bit: $low, width: $width },
                    $( $crate::fields::Bits { name: $name, ident: stringify!($field), octet: $octet2, bit: $low2, width: $width2 }, )?
                )*],
                fields: &[],
                read: |octets| <Self as $codec>::decode(octets).map(|_| $crate::__alloc::vec::Vec::new()),
            });
            /// Whether the tree is the value of the field alone: one field, not a flag.
            #[cfg(feature = "tree")]
            const BARE: bool = { let widths: &[u32] = &[$($width),*]; widths.len() == 1 && widths[0] > 1 };
            /// The bits of each octet that a field has.
            const MASKS: [u8; $n] = {
                let mut masks = [0u8; $n];
                $( $crate::bits!(@mask masks, $octet, $low, $width [$($kind)? $(+ $octet2 $low2 $width2)?]); )*
                masks
            };

            /// Whether no bit of a field is set: "at least one bit shall be set to 1" of a
            /// value of flags alone.
            #[allow(dead_code)]
            pub(crate) fn none_set(&self) -> bool {
                let (octets, _) = self.octets();
                octets.iter().zip(Self::MASKS).all(|(octet, mask)| octet & mask == 0)
            }

            /// The octets of the fields, and how many are written.
            fn octets(&self) -> ([u8; $n], usize) {
                let mut octets = [0u8; $n];
                for (octet, (spare, mask)) in octets.iter_mut().zip(self.spare.iter().zip(Self::MASKS)) {
                    *octet = spare & !mask;
                }
                $( $crate::bits!(@store octets, $width [$($kind)? $(+ $octet2 $low2 $width2)?] self.$field, $octet, $low); )*
                (octets, usize::from(self.present).min($n))
            }

            /// Whether its length is not one that it may have, from `MIN` to `N` octets, or a
            /// field, a spare bit or unknown octets are past the octets that the value has,
            /// which its length would not write: unknown octets follow all the octets of the
            /// fields.
            fn past_length(&self) -> bool {
                let (octets, used) = self.octets();
                !($min..=$n).contains(&usize::from(self.present))
                    || octets[used..].iter().any(|octet| *octet != 0)
                    || used < $n && !self.extra.is_empty()
            }
        }

        impl $codec for $ty {
            fn decode(octets: &[u8]) -> Option<Self> {
                if octets.len() < $min {
                    return None;
                }
                let present = octets.len().min($n);
                let at = |octet: usize| if octet < present { octets[octet] } else { 0 };
                let mut spare = [0u8; $n];
                for octet in 0..present {
                    spare[octet] = octets[octet] & !Self::MASKS[octet];
                }
                Some(Self {
                    $( $field: $crate::bits!(@read $width [$($kind)? $(+ $octet2 $low2 $width2)?] at($octet), $low), )*
                    spare,
                    present: present as u8,
                    extra: $crate::fields::Octets::copied(&octets[present..]),
                })
            }

            fn encode(&self, out: &mut $crate::__alloc::vec::Vec<u8>) -> core::result::Result<(), $error> {
                let ie = <Self as $ie_type>::TYPE_NAME;
                if let Some(field) = self.wide() {
                    return Err($error_of($crate::fields::FieldError::Length { ie, field }));
                }
                if self.past_length() {
                    return Err($error_of($crate::fields::FieldError::Inconsistent { ie, field: "length", clause: $clause }));
                }
                let (octets, used) = self.octets();
                out.extend_from_slice(&octets[..used]);
                out.extend_from_slice(&self.extra);
                Ok(())
            }

            fn encoded_len(&self) -> usize {
                self.octets().1 + self.extra.len()
            }
        }

        impl $ty {
            /// The first field whose value is wider than its bits: encode() refuses it.
            #[allow(unused_comparisons)]
            fn wide(&self) -> Option<&'static str> {
                $(
                    if $crate::bits!(@wide $width [$($kind)? $(+ $octet2 $low2 $width2)?] self.$field) {
                        return Some($name);
                    }
                )*
                None
            }
        }

        impl $rules_trait for $ty {
            fn check(&self, path: &$crate::fields::Path<'_>, found: &mut $crate::__alloc::vec::Vec<$deviation>) {
                if self.wide().is_some() {
                    found.push(<$deviation as $crate::fields::Deviate>::found(path, Some($clause), $crate::fields::Finding::Rule("a value wider than its bits")));
                }
                if self.past_length() {
                    found.push(<$deviation as $crate::fields::Deviate>::found(path, Some($clause), $crate::fields::Finding::Inconsistent { field: "length" }));
                }
                if self.spare.iter().any(|spare| *spare != 0) {
                    found.push(<$deviation as $crate::fields::Deviate>::found(path, None, $crate::fields::Finding::SpareBits));
                }
                $( $crate::bits!(@when [$(self.$flag)?] $crate::bits!(@check $width [$($kind)? $(+ $octet2 $low2 $width2)?] self.$field, path, found, $clause)); )*
                $(
                    let ($this, $path, $found) = (self, path, found);
                    $rules
                )?
            }
        }

        /// The flags that are set and the fields, with `Spare`, `length` (the octets of
        /// the fields that the value has, when it has not all of them, which a tree reads
        /// from `MIN` to `N`) and `extra` where they are not what a sender of this release
        /// writes.
        #[cfg(feature = "tree")]
        impl $crate::tree::Tree for $ty {
            const NAME: &'static str = <Self as $ie_type>::TYPE_NAME;

            fn to_tree(&self) -> $crate::tree::Value {
                if Self::BARE && self.spare.iter().all(|spare| *spare == 0) && usize::from(self.present) == $n && self.extra.is_empty() {
                    $( if Self::BARE { return $crate::bits!(@tree $width [$($kind)? $(+ $octet2 $low2 $width2)?] self.$field); } )*
                }
                let mut members = $crate::tree::Members::new();
                $( $crate::bits!(@put $width [$($kind)? $(+ $octet2 $low2 $width2)?] members, $name, self.$field); )*
                if self.spare.iter().any(|spare| *spare != 0) {
                    members.put("Spare", $crate::tree::octets(&self.spare));
                }
                if usize::from(self.present) != $n {
                    members.put("length", $crate::tree::unsigned(self.present.into()));
                }
                if !self.extra.is_empty() {
                    members.put($crate::fields::UNKNOWN_OCTETS, $crate::tree::octets(&self.extra));
                }
                members.finish()
            }

            fn from_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> core::result::Result<Self, $crate::tree::Refused> {
                let mut bits = Self::default();
                if Self::BARE && !value.is_object() {
                    $( bits.$field = $crate::bits!(@get $width [$($kind)? $(+ $octet2 $low2 $width2)?] value, reading)?; )*
                    return Ok(bits);
                }
                let read = reading.members(value, Self::MEMBERS)?;
                $(
                    if let Some(value) = reading.optional(&read, $name, |value, reading| $crate::bits!(@get $width [$($kind)? $(+ $octet2 $low2 $width2)?] value, reading))? {
                        bits.$field = value;
                    }
                )*
                if let Some(spare) = reading.optional(&read, "Spare", |value, reading| reading.octet_array::<$n>(value))? {
                    bits.spare = spare;
                }
                if let Some(present) = reading.optional(&read, "length", |value, reading| reading.unsigned(value, $min, $n))? {
                    bits.present = present as u8;
                }
                bits.extra = reading
                    .optional(&read, $crate::fields::UNKNOWN_OCTETS, |value, reading| reading.octets(value))?
                    .map($crate::fields::Octets::from)
                    .unwrap_or_default();
                if bits.spare.iter().zip(Self::MASKS).any(|(spare, mask)| spare & mask != 0) {
                    return Err(reading.refuse($crate::tree::ErrorKind::Value, "Spare has a bit that a field has"));
                }
                if bits.past_length() {
                    return Err(reading.refuse(
                        $crate::tree::ErrorKind::Value,
                        $crate::__alloc::format!("a field or a spare bit is set past the length, or unknown octets follow a length short of the fields ({})", $clause),
                    ));
                }
                Ok(bits)
            }
        }
    };
    // A field of one bit is a flag; of a few, a number or a value of its table (`as`); of
    // two parts (`+`), a number whose high bits are in the first and low bits in the second.
    (@type $width:tt [+ $octet2:literal $low2:literal $width2:tt]) => { u16 };
    // What its tree takes: a flag, a number of its bits or a name of its table.
    (@takes $width:tt [+ $octet2:literal $low2:literal $width2:tt]) => {
        &$crate::tree::Takes::Number((1 << ($width + $width2)) - 1)
    };
    (@takes 1 []) => { &$crate::tree::Takes::Flag };
    (@takes $width:tt []) => { &$crate::tree::Takes::Number((1 << $width) - 1) };
    (@takes $width:tt [$kind:ident]) => {
        &$crate::tree::Takes::OneOf(&[
            &$crate::tree::Takes::Named($kind::NAMED),
            &$crate::tree::Takes::Number((1 << $width) - 1),
        ])
    };
    (@wide 1 [] $value:expr) => { false };
    (@wide $width:tt [] $value:expr) => { u16::from($value) >= (1u16 << $width) };
    (@wide $width:tt [$kind:ident] $value:expr) => { u16::from($value.value()) >= (1u16 << $width) };
    (@wide $width:tt [+ $octet2:literal $low2:literal $width2:tt] $value:expr) => { u32::from($value) >= (1u32 << ($width + $width2)) };
    // A value of one field that is no flag is built from it.
    (@new $ty:ident [$field:ident $name:literal 1 []]) => {};
    (@new $ty:ident [$field:ident $name:literal $width:tt [$($kind:tt)*]]) => {
        impl $ty {
            #[doc = concat!("A value of its ", $name, " field, without spare bits.")]
            pub fn new($field: $crate::bits!(@type $width [$($kind)*])) -> Self {
                Self { $field, ..Self::default() }
            }
        }
    };
    (@new $ty:ident $($fields:tt)*) => {};
    (@type 1 []) => { bool };
    (@type $width:tt []) => { u8 };
    (@type $width:tt [$kind:ident]) => { $kind };
    (@mask $masks:ident, $octet:literal, $low:literal, $width:tt [+ $octet2:literal $low2:literal $width2:tt]) => {
        $crate::bits!(@mask $masks, $octet, $low, $width []);
        $crate::bits!(@mask $masks, $octet2, $low2, $width2 []);
    };
    (@mask $masks:ident, $octet:literal, $low:literal, $width:tt [$($kind:ident)?]) => {
        $masks[$octet] |= ($crate::bits!(@ones $width [] $low) & 0xff) as u8;
    };
    (@read $width:tt [+ $octet2:literal $low2:literal $width2:tt] $at:ident($octet:literal), $low:literal) => {
        (u16::from($crate::bits!(@read $width [] $at($octet), $low)) << $width2) | u16::from($crate::bits!(@read $width2 [] $at($octet2), $low2))
    };
    (@store $octets:ident, $width:tt [+ $octet2:literal $low2:literal $width2:tt] $value:expr, $octet:literal, $low:literal) => {
        $octets[$octet] |= $crate::bits!(@write $width [] u8, ($value >> $width2) as u8, $low);
        $octets[$octet2] |= $crate::bits!(@write $width2 [] u8, ($value & ((1u16 << $width2) - 1)) as u8, $low2);
    };
    (@store $octets:ident, $width:tt [$($kind:ident)?] $value:expr, $octet:literal, $low:literal) => {
        $octets[$octet] |= $crate::bits!(@write $width [$($kind)?] u8, $value, $low);
    };
    // A field of `width` bits from bit `low` (from 1) of an integer, of an octet here and of
    // the flags of a layout: its bits, its value read from the integer, and its value
    // written as an integer of type `int`.
    (@width $width:tt [$($kind:ident)?]) => { $width };
    (@ones $width:tt [$($kind:ident)?] $low:expr) => { ((1u32 << $width) - 1) << ($low - 1) };
    (@read 1 [] $integer:expr, $low:expr) => { $integer >> ($low - 1) & 1 == 1 };
    (@read $width:tt [] $integer:expr, $low:expr) => { (u32::from($integer >> ($low - 1)) & ((1u32 << $width) - 1)) as u8 };
    (@read $width:tt [$kind:ident] $integer:expr, $low:expr) => { $kind::from_value($crate::bits!(@read $width [] $integer, $low)) };
    (@write 1 [] $int:ty, $value:expr, $low:expr) => { <$int>::from($value) << ($low - 1) };
    (@write $width:tt [] $int:ty, $value:expr, $low:expr) => { (<$int>::from($value) & (((1u32 << $width) - 1) as $int)) << ($low - 1) };
    (@write $width:tt [$kind:ident] $int:ty, $value:expr, $low:expr) => { $crate::bits!(@write $width [] $int, $value.value(), $low) };
    (@when [] $($check:tt)*) => { $($check)*; };
    (@when [$flag:expr] $($check:tt)*) => { if $flag { $($check)*; } };
    (@check $width:tt [+ $($part:tt)*] $value:expr, $path:ident, $found:ident, $clause:literal) => {};
    (@check $width:tt [] $value:expr, $path:ident, $found:ident, $clause:literal) => {};
    (@check $width:tt [$kind:ident] $value:expr, $path:ident, $found:ident, $clause:literal) => {
        $crate::fields::Field::check(&$value, $path, $clause, $found)
    };
    (@tree $width:tt [+ $($part:tt)*] $value:expr) => { $crate::tree::unsigned($value.into()) };
    (@tree 1 [] $value:expr) => { $crate::tree::Value::Bool($value) };
    (@tree $width:tt [] $value:expr) => { $crate::tree::unsigned($value.into()) };
    (@tree $width:tt [$kind:ident] $value:expr) => { $value.tree() };
    (@put 1 [] $members:ident, $name:literal, $value:expr) => {
        if $value { $members.put($name, $crate::tree::Value::Bool(true)); }
    };
    (@put $width:tt [$($part:tt)*] $members:ident, $name:literal, $value:expr) => {
        $members.put($name, $crate::bits!(@tree $width [$($part)*] $value));
    };
    (@get $width:tt [+ $octet2:literal $low2:literal $width2:tt] $value:expr, $reading:expr) => {
        $reading.unsigned($value, 0, (1u64 << ($width + $width2)) - 1).map(|value| value as u16)
    };
    (@get 1 [] $value:expr, $reading:expr) => { $reading.boolean($value) };
    (@get $width:tt [] $value:expr, $reading:expr) => {
        $reading.unsigned($value, 0, (1u64 << $width) - 1).map(|value| value as u8)
    };
    (@get $width:tt [$kind:ident] $value:expr, $reading:expr) => {
        $kind::read_tree($value, $reading, ((1u16 << $width) - 1) as u8)
    };
}

/// A value of a layout of its clause: fields that are always there, then an octet of
/// flags and fields each present when its condition over the flags holds, then the octets
/// that a later release adds ("present only if explicitly specified"), which the value
/// keeps. A value of fields alone has no flags, and the tree of a value of one field
/// alone is that field while it has no unknown octets. A field of a length that the flags
/// give, `[length]` after its kind, is read from that many octets. Each field is written
/// by [`put`](crate::fields::put), which refuses one that its bits cannot say. A layout
/// that is a part of a value, of a second figure of its clause, and no IE, says `part` and
/// its name: it is a field of that value after its length of two octets. A `check`
/// block adds the rules of its clause, and a `placed` block those that its place in a
/// table gives, which the protocol's trait of rules has as `check_placed`. The first
/// argument names the protocol's items, as for [`bits!`](crate::bits). PFCP's F-TEID and
/// MBR (TS 29.244 §8.2.3, §8.2.8):
///
/// ```text
/// pub struct FTeid "8.2.3" {
///     flags(u8) { chid "CHID": 4, ch "CH": 3, v6 "V6": 2, v4 "V4": 1 }
///     teid "TEID": u32 if !ch,
///     ipv4 "IPv4 address": Ipv4Addr if v4 && !ch,
/// }
/// pub struct Mbr "8.2.8" { fields { ul_mbr "UL MBR": U40, dl_mbr "DL MBR": U40 } }
/// ```
#[macro_export]
macro_rules! layout {
    ([$codec:path, $rules_trait:path, $ie_type:path, $deviation:ty, $error:ty, $error_of:path] $(
        $(#[$meta:meta])*
        pub struct $ty:ident $clause:literal $(part $part:literal)? {
            $( fields { $( $head:ident $head_name:literal : $hkind:ty ),* $(,)? } )?
            $(
                flags($fk:ty) { $( $flag:ident $flag_name:literal : $bit:literal $(/ $width:literal $(as $fkind:ident)?)? ),* $(,)? }
                $( $field:ident $field_name:literal : $kind:ident $(<$param:ident>)? $([$length:expr])? $(if $cond:expr)? ),* $(,)?
            )?
        }
        $( check($this:ident, $path:ident, $found:ident) $rules:block )?
        $( placed($pthis:ident, $table:ident, $ppath:ident, $pfound:ident) $placed:block )?
    )*) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $ty {
            $($(
                #[doc = concat!("The ", $head_name, " field.")]
                pub $head: $hkind,
            )*)?
            $(
                $(
                    #[doc = concat!("The ", $flag_name, " field of the octet of flags, from bit ", stringify!($bit), ".")]
                    pub $flag: $crate::layout!(@bits type [$($width $(as $fkind)?)?]),
                )*
                /// The other bits of the octet of flags, which are spare: zero from a sender.
                pub spare: $fk,
                $(
                    #[doc = concat!("The ", $field_name, " field.")]
                    pub $field: $crate::layout!(@type [$($cond)?] $kind $(<$param>)?),
                )*
            )?
            /// The octets after the fields, which a later release adds and a receiver
            /// ignores, `"unknown octets"` in a tree.
            pub extra: $crate::fields::Octets,
        }

        impl $ty {
            /// Its name in an error and in a refusal of its tree: that of its IE type, or
            /// of the part of a value that it is.
            const NAMED: &'static str = $crate::layout!(@named $ie_type, [$($part)?] $ty);
            /// What its tree takes: each member, with what it takes.
            #[cfg(feature = "tree")]
            const MEMBERS_TAKE: &'static [(&'static str, &'static $crate::tree::Takes)] = &[
                $($( ($head_name, <$hkind as $crate::fields::FieldTree>::TAKES), )*)?
                $(
                    $( ($flag_name, $crate::layout!(@bits takes [$($width $(as $fkind)?)?])), )*
                    ($crate::layout!(@spare $fk), &$crate::tree::Takes::Number(<$fk>::MAX as u64)),
                    $( ($field_name, <$kind $(<$param>)? as $crate::fields::FieldTree>::TAKES), )*
                )?
                ($crate::fields::UNKNOWN_OCTETS, &$crate::tree::Takes::Octets(None)),
            ];
            $(
                /// The bits of the octet of flags that are flags.
                const FLAGS: $fk = 0 $( | $crate::layout!(@bits ones [$($width $(as $fkind)?)?] $bit) as $fk )*;
            )?
            /// The names of its fields and flags.
            #[allow(dead_code)]
            pub(crate) const FIELDS: &'static [&'static str] = &[$($($head_name,)*)? $($($flag_name,)* $($field_name,)*)?];
            /// The members that its tree may have: its fields and flags, the spare bits of its
            /// flags and its unknown octets.
            #[allow(dead_code)]
            pub(crate) const MEMBERS: &'static [&'static str] = &[$($($head_name,)*)? $($($flag_name,)* $crate::layout!(@spare $fk), $($field_name,)*)? $crate::fields::UNKNOWN_OCTETS];
            /// What its tree takes: an object of its members, or the tree of its one field.
            #[cfg(feature = "tree")]
            #[allow(dead_code)]
            pub(crate) const TAKES: &'static $crate::tree::Takes =
                &$crate::tree::Takes::Members($crate::fields::one(Self::FIELDS), Self::MEMBERS_TAKE);
            /// The octets of the fields before its flags, where its flags are.
            #[cfg(feature = "spec")]
            #[allow(dead_code)]
            const HEAD_OCTETS: usize = 0 $($( + $crate::fields::octets_of(<$hkind as $crate::fields::Field>::OCTETS) )*)?;
            /// Its layout.
            #[doc(hidden)]
            #[cfg(feature = "spec")]
            pub const LAYOUT: Option<&'static $crate::fields::Layout> = Some(&$crate::fields::Layout {
                head: &[$($($crate::fields::Member { name: $head_name, octets: <$hkind as $crate::fields::Field>::OCTETS, cells: <$hkind as $crate::fields::Field>::CELLS, condition: "true" }),*)?],
                bits: &[$($($crate::fields::Bits {
                    name: $flag_name,
                    ident: stringify!($flag),
                    octet: Self::HEAD_OCTETS + (core::mem::size_of::<$fk>() - 1 - ($bit - 1) / 8),
                    bit: ($bit - 1) % 8 + 1,
                    width: $crate::layout!(@bits width [$($width $(as $fkind)?)?]),
                }),*)?],
                fields: &[$($($crate::fields::Member { name: $field_name, octets: <$kind $(<$param>)? as $crate::fields::Field>::OCTETS, cells: <$kind $(<$param>)? as $crate::fields::Field>::CELLS, condition: $crate::layout!(@condition [$($cond)?]) }),*)?],
                read: |octets| {
                    let read = <Self as $codec>::decode(octets)?;
                    #[allow(unused_mut)]
                    let mut names = $crate::__alloc::vec![$($($head_name),*)?];
                    $($(
                        if $crate::layout!(@get [$($cond)?] read.$field).is_some() {
                            names.push($field_name);
                        }
                    )*)?
                    let _ = read;
                    Some(names)
                },
            });

            /// The first field of the octet of flags whose value is wider than its bits.
            fn wide(&self) -> Option<&'static str> {
                $($(
                    if $crate::layout!(@bits wide [$($width $(as $fkind)?)?] self.$flag) {
                        return Some($flag_name);
                    }
                )*)?
                None
            }

            /// Whether each field is there as its flags say, and its octets read as it: the
            /// name of the first that is not. A field of the length that its flags give has
            /// no more octets, and no fewer where octets follow it, and a field that runs to
            /// the end of the IE has no unknown octets after it: they would read as part of it.
            #[allow(unused_variables, unused_mut, unused_assignments)]
            pub(crate) fn inconsistent(&self) -> Option<&'static str> {
                let mut ends = false;
                // A field shorter than the length that its flags give.
                let mut short = None;
                $($( ends = <$hkind as $crate::fields::Field>::TO_THE_END; )*)?
                $(
                    $( let $flag = self.$flag; )*
                    $(
                        $(
                            if self.$field.is_some() != ($cond) {
                                return Some($field_name);
                            }
                        )?
                        let field = $crate::layout!(@get [$($cond)?] self.$field);
                        if short.is_some() && field.is_some_and(|field| $crate::fields::Field::size(field) > 0) {
                            return short;
                        }
                        $(
                            let size = field.map_or(0, $crate::fields::Field::size);
                            if size > $length {
                                return Some($field_name);
                            }
                            short = short.or((field.is_some() && size < $length).then_some($field_name));
                        )?
                        ends = field.is_some() && $crate::layout!(@ends [$($length)?] $kind $(<$param>)?);
                    )*
                )?
                if short.is_some() && !self.extra.is_empty() {
                    return short;
                }
                (ends && !self.extra.is_empty()).then_some($crate::fields::UNKNOWN_OCTETS)
            }
        }

        impl $codec for $ty {
            #[allow(unused_variables)]
            fn decode(octets: &[u8]) -> Option<Self> {
                let mut rest = octets;
                $($( let $head = <$hkind as $crate::fields::Field>::read(&mut rest)?; )*)?
                $(
                    let first = <$fk as $crate::fields::Field>::read(&mut rest)?;
                    $( let $flag = $crate::layout!(@bits read [$($width $(as $fkind)?)?] first, $bit); )*
                    $(
                        let $field = $crate::layout!(@decode [$($cond)?] $crate::layout!(@read rest, [$kind $(<$param>)?] $(, $length)?));
                    )*
                )?
                Some(Self {
                    $($( $head, )*)?
                    $( $( $flag, )* spare: first & !Self::FLAGS, $( $field, )* )?
                    extra: $crate::fields::Octets::copied(rest),
                })
            }

            fn encode(&self, out: &mut $crate::__alloc::vec::Vec<u8>) -> core::result::Result<(), $error> {
                if let Some(field) = self.inconsistent() {
                    return Err($error_of($crate::fields::FieldError::Inconsistent {
                        ie: Self::NAMED,
                        field,
                        clause: $clause,
                    }));
                }
                let ie = Self::NAMED;
                if let Some(field) = self.wide() {
                    return Err($error_of($crate::fields::FieldError::Length { ie, field }));
                }
                $($( $crate::fields::put(&self.$head, ie, $head_name, out).map_err($error_of)?; )*)?
                $(
                    // The octet of flags, whose fields wide() checked.
                    $crate::fields::Field::write(&(self.spare & !Self::FLAGS $( | $crate::layout!(@bits write [$($width $(as $fkind)?)?] $fk, self.$flag, $bit) )*), out);
                    $(
                        if let Some(field) = $crate::layout!(@get [$($cond)?] self.$field) {
                            $crate::fields::put(field, ie, $field_name, out).map_err($error_of)?;
                        }
                    )*
                )?
                out.extend_from_slice(&self.extra);
                Ok(())
            }

            fn encoded_len(&self) -> usize {
                0 $($( + $crate::fields::Field::size(&self.$head) )*)?
                    $( + core::mem::size_of::<$fk>() $( + $crate::layout!(@get [$($cond)?] self.$field).map_or(0, $crate::fields::Field::size) )* )?
                    + self.extra.len()
            }
        }

        impl $rules_trait for $ty {
            #[allow(unused_variables)]
            fn check(&self, path: &$crate::fields::Path<'_>, found: &mut $crate::__alloc::vec::Vec<$deviation>) {
                if let Some(field) = self.inconsistent() {
                    found.push(<$deviation as $crate::fields::Deviate>::found(path, Some($clause), $crate::fields::Finding::Inconsistent { field }));
                }
                if self.wide().is_some() {
                    found.push(<$deviation as $crate::fields::Deviate>::found(path, Some($clause), $crate::fields::Finding::Rule("a value wider than its bits")));
                }
                $(
                    if self.spare != <$fk>::default() {
                        found.push(<$deviation as $crate::fields::Deviate>::found(path, None, $crate::fields::Finding::SpareBits));
                    }
                )?
                $($( $crate::fields::Field::check(&self.$head, path, $clause, found); )*)?
                $(
                    $( $crate::layout!(@bits check [$($width $(as $fkind)?)?] self.$flag, path, found, $clause); )*
                    $(
                        if let Some(field) = $crate::layout!(@get [$($cond)?] self.$field) {
                            $crate::fields::Field::check(field, path, $clause, found);
                        }
                    )*
                )?
                $(
                    let ($this, $path, $found) = (self, path, found);
                    $rules
                )?
            }

            $(
                fn check_placed(&self, $table: &'static str, $ppath: &$crate::fields::Path<'_>, $pfound: &mut $crate::__alloc::vec::Vec<$deviation>) {
                    let $pthis = self;
                    $placed
                }
            )?
        }

        $( $crate::layout!(@part $codec, $ty $part); )?

        #[cfg(feature = "tree")]
        impl $crate::tree::Tree for $ty {
            const NAME: &'static str = Self::NAMED;

            fn to_tree(&self) -> $crate::tree::Value {
                // A value of one field, without flags, is that field, while it has no
                // unknown octets.
                $($(
                    if Self::FIELDS.len() == 1 && self.extra.is_empty() {
                        return $crate::fields::FieldTree::to_tree(&self.$head);
                    }
                )*)?
                let mut members = $crate::tree::Members::new();
                $($( $crate::fields::FieldTree::put_member(&self.$head, &mut members, $head_name); )*)?
                $(
                    $( $crate::layout!(@bits put [$($width $(as $fkind)?)?] members, $flag_name, self.$flag); )*
                    if self.spare != <$fk>::default() {
                        members.put("Spare", $crate::tree::unsigned(self.spare.into()));
                    }
                    $(
                        if let Some(field) = $crate::layout!(@get [$($cond)?] self.$field) {
                            $crate::fields::FieldTree::put_member(field, &mut members, $field_name);
                        }
                    )*
                )?
                if !self.extra.is_empty() {
                    members.put($crate::fields::UNKNOWN_OCTETS, $crate::tree::octets(&self.extra));
                }
                members.finish()
            }

            fn from_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> core::result::Result<Self, $crate::tree::Refused> {
                $($(
                    if Self::FIELDS.len() == 1 && !value.is_object() {
                        let $head = <$hkind as $crate::fields::FieldTree>::from_tree(value, reading)?;
                        return Ok(Self { $head, ..Self::default() });
                    }
                )*)?
                let read = reading.members(value, Self::MEMBERS)?;
                $($(
                    let $head = <$hkind as $crate::fields::FieldTree>::read_member(&read, $head_name, reading)?;
                )*)?
                $(
                    $(
                        let $flag = reading
                            .optional(&read, $flag_name, |value, reading| $crate::layout!(@bits get [$($width $(as $fkind)?)?] value, reading))?
                            .unwrap_or_default();
                    )*
                    let spare = reading
                        .optional(&read, "Spare", |value, reading| reading.unsigned(value, 0, <$fk>::MAX.into()))?
                        .unwrap_or(0) as $fk;
                    if spare & Self::FLAGS != 0 {
                        return Err(reading.refuse(
                            $crate::tree::ErrorKind::Value,
                            "Spare has a bit that is a flag",
                        ));
                    }
                    $(
                        let $field = $crate::layout!(@from_tree [$($cond)?] reading, &read, $field_name, $kind $(<$param>)?)?;
                    )*
                )?
                let extra = reading.optional(&read, $crate::fields::UNKNOWN_OCTETS, |value, reading| reading.octets(value))?;
                let value = Self {
                    $($( $head, )*)?
                    $( $( $flag, )* spare, $( $field, )* )?
                    extra: extra.map($crate::fields::Octets::from).unwrap_or_default(),
                };
                match value.inconsistent() {
                    None => Ok(value),
                    Some(field) => Err(reading.refuse(
                        $crate::tree::ErrorKind::Value,
                        $crate::__alloc::format!(
                            "{field} is written where its flags or its other fields do not have it, or is missing where they do, or its octets would read as another value ({})",
                            $clause
                        ),
                    )),
                }
            }
        }
    )*};
    (@spare $fk:ty) => { "Spare" };
    (@named $ie_type:path, [] $ty:ident) => { <$ty as $ie_type>::TYPE_NAME };
    (@named $ie_type:path, [$part:literal] $ty:ident) => { $part };
    // A part of a value, which no IE is: a field of that value after its length of two
    // octets, an entry of a list of them. Its value checks each at its place in the list.
    (@part $codec:path, $ty:ident $part:literal) => {
        impl $crate::fields::Field for $ty {
            fn read(octets: &mut &[u8]) -> Option<Self> {
                let length = usize::from(<u16 as $crate::fields::Field>::read(octets)?);
                let (within, rest) = octets.split_at_checked(length)?;
                *octets = rest;
                <Self as $codec>::decode(within)
            }
            fn write(&self, out: &mut $crate::__alloc::vec::Vec<u8>) {
                // put() refuses one that does not encode, or is longer than its length says.
                let mut within = $crate::__alloc::vec::Vec::new();
                if <Self as $codec>::encode(self, &mut within).is_ok() {
                    out.extend_from_slice(&(within.len() as u16).to_be_bytes());
                    out.extend_from_slice(&within);
                }
            }
            fn size(&self) -> usize {
                2 + <Self as $codec>::encoded_len(self)
            }
            fn fits(&self) -> bool {
                let length = <Self as $codec>::encoded_len(self);
                length <= usize::from(u16::MAX)
                    && <Self as $codec>::encode(self, &mut $crate::__alloc::vec::Vec::new()).is_ok()
            }
        }

        #[cfg(feature = "tree")]
        impl $crate::fields::FieldTree for $ty {
            const TAKES: &'static $crate::tree::Takes = &$crate::tree::Takes::Members(None, Self::MEMBERS_TAKE);
            fn to_tree(&self) -> $crate::tree::Value {
                $crate::tree::Tree::to_tree(self)
            }
            fn from_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> core::result::Result<Self, $crate::tree::Refused> {
                <Self as $crate::tree::Tree>::from_tree(value, reading)
            }
        }
    };
    // Whether a field runs to the end of the IE: one of a length that its flags give does not.
    (@ends [] $($kind:tt)+) => { <$($kind)+ as $crate::fields::Field>::TO_THE_END };
    (@ends [$length:expr] $($kind:tt)+) => { false };
    // A field after the flags: always there, or there where its condition holds.
    (@type [] $($kind:tt)+) => { $($kind)+ };
    (@type [$cond:expr] $($kind:tt)+) => { Option<$($kind)+> };
    (@condition []) => { "true" };
    (@condition [$cond:expr]) => { stringify!($cond) };
    (@get [] $field:expr) => { Some(&$field) };
    (@get [$cond:expr] $field:expr) => { $field.as_ref() };
    (@decode [] $read:expr) => { $read? };
    (@decode [$cond:expr] $read:expr) => { match $cond { true => Some($read?), false => None } };
    (@from_tree [] $reading:ident, $read:expr, $name:expr, $kind:ty) => { <$kind as $crate::fields::FieldTree>::read_member($read, $name, $reading) };
    (@from_tree [$cond:expr] $reading:ident, $read:expr, $name:expr, $kind:ty) => { $reading.optional($read, $name, <$kind as $crate::fields::FieldTree>::from_tree) };
    // A field read from the octets left, or from the first `length` of them, the others
    // being the next field's or the unknown octets.
    (@read $rest:ident, [$($kind:tt)+]) => { <$($kind)+ as $crate::fields::Field>::read(&mut $rest) };
    (@read $rest:ident, [$($kind:tt)+], $length:expr) => {{
        let (mut head, tail) = $rest.split_at(usize::min($length, $rest.len()));
        $rest = tail;
        <$($kind)+ as $crate::fields::Field>::read(&mut head)
    }};
    // A field of the octet of flags is one of `bits!`: a flag of one bit, or a number
    // of `width` bits or a value of its table (`as`), at its bit of the octet.
    (@bits $op:ident [] $($rest:tt)*) => { $crate::bits!(@ $op 1 [] $($rest)*) };
    (@bits $op:ident [$width:literal] $($rest:tt)*) => { $crate::bits!(@ $op $width [] $($rest)*) };
    (@bits $op:ident [$width:literal as $kind:ident] $($rest:tt)*) => { $crate::bits!(@ $op $width [$kind] $($rest)*) };
}

/// The values that a table names: a variant each, and `Other` for a value that
/// the table has as spare or reserved, or does not have, kept as it is. A field of `bits`
/// bits, or a whole octet whose other bits are spare. `reserved [..]` lists the values that
/// the table reserves, and `rejected [from ..= to]` gives those that a note of the table
/// has its receiver reject, which are reserved in effect. `read [from ..= to as value]`
/// gives the values that a receiver reads as another, and `others as value` the one that
/// it reads each value that the table does not name as ("all other values are
/// interpreted as"). A value's name in a tree is the table's, or the one written after
/// `tree`. PFCP's Gate (TS 29.244 Tables 8.2.7-1 and 8.2.7-2):
///
/// ```text
/// pub enum Gate "Table 8.2.7-1 and Table 8.2.7-2" bits 2 {
///     0 Open "OPEN",
///     1 Closed "CLOSED",
/// }
/// ```
#[macro_export]
macro_rules! named_values {
    ($(
        $(#[$meta:meta])*
        $vis:vis enum $ty:ident $table:literal bits $bits:literal $(reserved [$($reserved:literal),*])?
            $(rejected [$rejected_from:literal ..= $rejected_to:literal])?
            $(read [$($from:literal ..= $to:literal as $target:literal),+])?
            $(others as $other:literal)? {
            $( $value:literal $variant:ident $name:literal $(tree $tree_name:literal)? ),* $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        ///
        /// Two values are equal when their numbers are: `Other` of a number that the table
        /// names is the variant of that name, which [`from_value`](Self::from_value) gives.
        #[derive(Debug, Clone, Copy)]
        #[non_exhaustive]
        $vis enum $ty {
            $(
                #[doc = concat!(stringify!($value), ": ", $name, ".")]
                $variant,
            )*
            /// A value that the table has as spare or reserved, or does not have.
            Other(u8),
        }

        impl core::cmp::PartialEq for $ty {
            fn eq(&self, other: &Self) -> bool {
                self.value() == other.value()
            }
        }

        impl core::cmp::Eq for $ty {}

        impl core::hash::Hash for $ty {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::hash::Hash::hash(&self.value(), state);
            }
        }

        impl $crate::fields::Narrowable for $ty {
            const BITS: u32 = $bits;
            fn from_value(value: u8) -> Self {
                Self::from_value(value)
            }
            fn value(self) -> u8 {
                self.value()
            }
        }

        #[cfg(feature = "tree")]
        impl $crate::fields::NarrowTree for $ty {
            const NARROW: &'static $crate::tree::Takes = &$crate::tree::Takes::OneOf(&[
                &$crate::tree::Takes::Named(Self::NAMED),
                &$crate::tree::Takes::Number((1 << $bits) - 1),
            ]);
        }

        impl $ty {
            /// The tables of the values.
            pub const TABLE: &'static str = $table;
            /// The values that the tables name, with their names.
            pub const NAMES: &'static [(u8, &'static str)] = &[$(($value, $name)),*];
            /// The names of the values, which a tree writes them by.
            #[cfg(feature = "tree")]
            pub(crate) const NAMED: &'static [&'static str] = &[$($crate::named_values!(@tree_name $name $($tree_name)?)),*];
            /// The width of the field, in bits.
            pub const BITS: u32 = $bits;
            /// The values that the tables reserve, and those that a note of theirs has a
            /// receiver reject: a receiver takes the IE as incorrect, where it ignores a
            /// spare one.
            pub const RESERVED: &'static [u8] = $crate::named_values!(@reserved [$($($reserved),*)?] $([$rejected_from $rejected_to])?);
            /// The values that a sender does not send and a receiver reads as another, as
            /// the tables say: from, to, and the value that they are read as.
            pub const READ_AS: &'static [(u8, u8, u8)] = &[$($(($from, $to, $target)),+)?];
            /// The value that a receiver reads each value that the tables do not name as,
            /// where they say so ("all other values are interpreted as").
            pub const OTHERS_AS: Option<u8> = $crate::named_values!(@others $($other)?);

            /// The variant of a number.
            pub const fn from_value(value: u8) -> Self {
                match value {
                    $( $value => Self::$variant, )*
                    other => Self::Other(other),
                }
            }

            /// The variant that the tables name for a number: `None` for a number that they do
            /// not name, which [`from_value`](Self::from_value) gives as `Other`.
            pub const fn named(value: u8) -> Option<Self> {
                match value {
                    $( $value => Some(Self::$variant), )*
                    _ => None,
                }
            }

            /// The number.
            pub const fn value(self) -> u8 {
                match self {
                    $( Self::$variant => $value, )*
                    Self::Other(value) => value,
                }
            }

            /// The value as a receiver takes it: a value that the table has a sender not
            /// send, and a receiver read as another, is that other.
            pub const fn effective(self) -> Self {
                match self.value() {
                    $($( $from..=$to => Self::from_value($target), )+)?
                    $( _ if self.name().is_none() => Self::from_value($other), )?
                    _ => self,
                }
            }

            /// The name that the table gives the value: `None` for `Other`.
            pub const fn name(self) -> Option<&'static str> {
                match Self::from_value(self.value()) {
                    $( Self::$variant => Some($name), )*
                    Self::Other(_) => None,
                }
            }

            /// The name of the value in a tree: the table's, or the one written after
            /// `tree`. `None` for `Other`.
            pub const fn tree_name(self) -> Option<&'static str> {
                match Self::from_value(self.value()) {
                    $( Self::$variant => Some($crate::named_values!(@tree_name $name $($tree_name)?)), )*
                    Self::Other(_) => None,
                }
            }

            /// The name in a tree, or the number of a value that the table does not name.
            #[cfg(feature = "tree")]
            pub(crate) fn tree(self) -> $crate::tree::Value {
                match self.tree_name() {
                    Some(name) => $crate::tree::text(name),
                    None => $crate::tree::unsigned(self.value().into()),
                }
            }

            /// A name of the table in any spelling, or a number up to `upper`.
            #[cfg(feature = "tree")]
            pub(crate) fn read_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
                upper: u8,
            ) -> core::result::Result<Self, $crate::tree::Refused> {
                const NAMES: &[&str] = &[$($crate::named_values!(@tree_name $name $($tree_name)?)),*];
                const VALUES: &[u8] = &[$($value),*];
                if value.is_number() {
                    return Ok(Self::from_value(reading.unsigned(value, 0, upper.into())? as u8));
                }
                Ok(Self::from_value(VALUES[reading.named(value, NAMES)?]))
            }
        }

        impl core::default::Default for $ty {
            /// The value 0.
            fn default() -> Self {
                Self::from_value(0)
            }
        }

        impl core::convert::From<u8> for $ty {
            fn from(value: u8) -> Self {
                Self::from_value(value)
            }
        }

        impl core::convert::From<$ty> for u8 {
            fn from(value: $ty) -> u8 {
                value.value()
            }
        }

        impl core::fmt::Display for $ty {
            /// The name, or the number of a value that the table does not name.
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match self.name() {
                    Some(name) => f.write_str(name),
                    None => write!(f, "{}", self.value()),
                }
            }
        }

        /// A whole octet; a value of fewer bits, whose octet has spare bits, is a
        /// `Narrow` field.
        impl $crate::fields::Field for $ty {
            const OCTETS: Option<usize> = Some(1);
            fn read(octets: &mut &[u8]) -> Option<Self> {
                let (&first, rest) = octets.split_first()?;
                *octets = rest;
                Some(Self::from_value(first))
            }
            fn write(&self, out: &mut $crate::__alloc::vec::Vec<u8>) {
                out.push(self.value());
            }
            fn size(&self) -> usize {
                1
            }
            fn check<D: $crate::fields::Deviate>(&self, path: &$crate::fields::Path<'_>, clause: &'static str, found: &mut $crate::__alloc::vec::Vec<D>) {
                if self.name().is_none() {
                    let value = self.value().into();
                    let finding = match (Self::RESERVED.contains(&self.value()), self.effective().value()) {
                        (true, _) => $crate::fields::Finding::ReservedValue { value },
                        (false, read) if u32::from(read) != value => $crate::fields::Finding::ReadAs { value, read: read.into() },
                        (false, _) => $crate::fields::Finding::SpareValue { value },
                    };
                    found.push(D::found(path, Some(clause), finding));
                }
            }
        }

        #[cfg(feature = "tree")]
        impl $crate::fields::FieldTree for $ty {
            const TAKES: &'static $crate::tree::Takes = &$crate::tree::Takes::OneOf(&[
                &$crate::tree::Takes::Named(Self::NAMED),
                &$crate::tree::Takes::Number(u8::MAX as u64),
            ]);
            fn to_tree(&self) -> $crate::tree::Value {
                self.tree()
            }
            fn from_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> core::result::Result<Self, $crate::tree::Refused> {
                Self::read_tree(value, reading, u8::MAX)
            }
        }
    )*};
    (@tree_name $name:literal) => { $name };
    (@tree_name $name:literal $tree_name:literal) => { $tree_name };
    (@others) => { None };
    (@others $other:literal) => { Some($other) };
    (@reserved [$($reserved:literal),*]) => { &[$($reserved),*] };
    (@reserved [] [$from:literal $to:literal]) => {
        &$crate::fields::values_from::<{ $to - $from + 1 }>($from)
    };
}
