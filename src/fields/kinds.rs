//! The kinds of fields.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::net::{Ipv4Addr, Ipv6Addr};

#[cfg(feature = "tree")]
use super::text_from_tree;
use super::{Deviate, FieldError, Finding, Octets, Path, together};

/// A field of a layout: its octets, read in network order.
pub trait Field: Sized {
    /// How many octets it has, when they are fixed.
    const OCTETS: Option<usize> = None;
    /// The cells narrower than an octet that it lays out, which the test `spec` compares
    /// with the figure: none for a field of whole octets.
    const CELLS: &'static [NarrowCell] = &[];
    /// Read the field, and step over it.
    fn read(octets: &mut &[u8]) -> Option<Self>;
    /// Write the field, which fits its bits: [`put`] refuses one that does not.
    fn write(&self, out: &mut Vec<u8>);
    /// Its length.
    fn size(&self) -> usize;
    /// What it breaks of the clause of its layout: a value that its table does not name.
    #[inline]
    fn check<D: Deviate>(&self, _path: &Path<'_>, _clause: &'static str, _found: &mut Vec<D>) {}
    /// Whether its length, its number or its type fit the bits that say them: [`put`],
    /// which writes each field of a value, refuses one that does not, and `check()`
    /// reports it.
    #[inline]
    fn fits(&self) -> bool {
        true
    }
    /// Whether it reads the octets up to the end of the IE, which then has no unknown
    /// octets after it.
    const TO_THE_END: bool = false;
}

/// The tree of a field: what a value writes in its tree for it, and reads from it.
#[cfg(feature = "tree")]
pub trait FieldTree: Field {
    /// Its tree.
    fn to_tree(&self) -> crate::tree::Value;
    /// Put its tree in the tree of its value, as the member `name`: a spare field only
    /// where it is set.
    #[inline]
    fn put_member(&self, members: &mut crate::tree::Members, name: &'static str) {
        members.put(name, self.to_tree());
    }
    /// Read it from the member `name` of the tree of its value: a spare field is 0 where
    /// the tree leaves it out.
    #[inline]
    fn read_member(
        read: &crate::tree::Read<'_>,
        name: &'static str,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading.member(read, name, Self::from_tree)
    }
    /// Read it from its tree.
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused>;
    /// What its tree takes, which a path below it is judged by where a value lacks it, and
    /// in either form of a tree of two.
    const TAKES: &'static crate::tree::Takes;
}

/// The names of the fields of a value read by hand, `FIELDS`, and the members of its tree,
/// `MEMBERS`: its fields and its unknown octets.
#[macro_export]
macro_rules! field_names {
    ($($name:literal),* $(,)?) => {
        /// The names of its fields.
        pub(crate) const FIELDS: &'static [&'static str] = &[$($name),*];
        /// The members that its tree may have: its fields and its unknown octets.
        pub(crate) const MEMBERS: &'static [&'static str] = &[$($name,)* $crate::fields::UNKNOWN_OCTETS];
    };
}

/// The rule of the fields at the end of a value that are read where octets remain after
/// those before it, as the fields that a later release adds: a field is there
/// where a later one, or unknown octets (`extra`), are, which would read as it. Of
/// `fields`, each by its name and whether the value has it, the first that is missing so;
/// `None` for none.
#[inline]
pub fn trailing(fields: &[(&'static str, bool)], extra: &[u8]) -> Option<&'static str> {
    let last = fields.iter().rposition(|(_, there)| *there);
    let read = match extra.is_empty() {
        true => last.unwrap_or_default(),
        false => fields.len(),
    };
    let missing = fields[..read].iter().find(|(_, there)| !there);
    missing.map(|(name, _)| *name)
}

/// Write `field`, named `name`, of a value of the IE `ie`: refused where its length, its
/// number or its type do not fit the bits that say them ([`Field::fits`]). Each value
/// writes each of its fields so.
#[inline]
pub fn put<F: Field>(
    field: &F,
    ie: &'static str,
    name: &'static str,
    out: &mut Vec<u8>,
) -> Result<(), FieldError> {
    if !field.fits() {
        return Err(FieldError::Length { ie, field: name });
    }
    field.write(out);
    Ok(())
}

/// How a value is laid out, as the crate reads it: what the test `spec` compares with the
/// figure of its clause.
#[doc(hidden)]
#[derive(Debug)]
pub struct Layout {
    /// The fields before its flags, in order.
    pub head: &'static [Member],
    /// Its flags, and the fields of its octets of bits.
    pub bits: &'static [Bits],
    /// The fields after them, in order.
    pub fields: &'static [Member],
    /// The names of the fields that it reads from octets, or `None` where they do not
    /// decode: the conditions of its fields as its reader takes them.
    pub read: fn(&[u8]) -> Option<Vec<&'static str>>,
}

/// A cell of bits of a figure as a test reads it from the document: its octet, from 0 for
/// the first octet of the value, its name, its lowest bit, from 1, and how many bits it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FigureCell {
    /// Its octet, from 0 for the first octet of the value.
    pub octet: usize,
    /// Its name in the figure.
    pub name: String,
    /// Its lowest bit, from 1.
    pub bit: u8,
    /// How many bits it has.
    pub width: u8,
}

impl Layout {
    /// How its bits differ from the cells of bits of its figure, each in words: a cell of
    /// the figure that it lays out elsewhere or not at all, and one of its fields of bits
    /// that the figure does not have there. None for a layout that has each cell where its
    /// figure has it. A name compares in any case and with any punctuation.
    pub fn differences(&self, figure: &[FigureCell]) -> Vec<String> {
        let same = |a: &str, b: &str| {
            let letters = |name: &str| {
                name.chars()
                    .filter(char::is_ascii_alphanumeric)
                    .map(|c| c.to_ascii_lowercase())
                    .collect::<String>()
            };
            letters(a) == letters(b)
        };
        let at = |bits: &Bits, cell: &FigureCell| {
            bits.octet == cell.octet && bits.bit == cell.bit && bits.width == cell.width
        };
        let mut out = Vec::new();
        for cell in figure {
            let named: Vec<&Bits> = self
                .bits
                .iter()
                .filter(|bits| same(bits.name, &cell.name))
                .collect();
            if named.is_empty() {
                out.push(alloc::format!(
                    "{} (octet {}, bit {}, {} bits) is not laid out",
                    cell.name,
                    cell.octet,
                    cell.bit,
                    cell.width
                ));
            } else if !named.iter().any(|bits| at(bits, cell)) {
                let bits = named[0];
                out.push(alloc::format!(
                    "{} is at octet {}, bit {}, {} bits, where the figure has octet {}, bit {}, {} bits",
                    cell.name, bits.octet, bits.bit, bits.width, cell.octet, cell.bit, cell.width
                ));
            }
        }
        for bits in self.bits {
            if figure.iter().all(|cell| !same(bits.name, &cell.name)) {
                out.push(alloc::format!(
                    "{} (octet {}, bit {}, {} bits) is not in the figure",
                    bits.name,
                    bits.octet,
                    bits.bit,
                    bits.width
                ));
            }
        }
        out
    }
}

/// A field of bits: where it is.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bits {
    /// Its name in the figure.
    pub name: &'static str,
    /// Its name in the crate, which the conditions of the fields after it say.
    pub ident: &'static str,
    /// Its octet, from 0 for the first octet of the value.
    pub octet: usize,
    /// Its lowest bit, from 1.
    pub bit: u8,
    /// How many bits it has.
    pub width: u8,
}

/// A field of octets: its octets when they are fixed, its cells narrower than an octet,
/// and when it is there.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    /// Its name in the figure.
    pub name: &'static str,
    /// How many octets it has, when they are fixed.
    pub octets: Option<usize>,
    /// The cells narrower than an octet that it lays out.
    pub cells: &'static [NarrowCell],
    /// When it is there, as the crate writes it of the flags: `true` for always.
    pub condition: &'static str,
}

/// A cell of a figure narrower than an octet that a field lays out: a value of the low
/// bits of its octet, whose others are spare, or a part of a field of its own layout.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NarrowCell {
    /// Its name in the figure: `None` for the field itself.
    pub name: Option<&'static str>,
    /// Its octet, from 0 for the first of the field.
    pub octet: usize,
    /// Its lowest bit, from 1.
    pub bit: u8,
    /// How many bits it has.
    pub width: u8,
}

/// A rule of a clause on which flags of a value go together, which `broken` breaks:
/// `rule` says how, in words.
#[inline]
pub fn flag_rule<D: Deviate>(
    broken: bool,
    path: &Path<'_>,
    clause: &'static str,
    rule: &'static str,
    found: &mut Vec<D>,
) {
    if broken {
        found.push(D::found(path, Some(clause), Finding::Rule(rule)));
    }
}

/// The octets of a field whose octets are fixed, or 0.
#[inline]
pub const fn octets_of(octets: Option<usize>) -> usize {
    match octets {
        Some(octets) => octets,
        None => 0,
    }
}

#[inline]
fn take<const N: usize>(octets: &mut &[u8]) -> Option<[u8; N]> {
    let (taken, rest) = octets.split_first_chunk::<N>()?;
    *octets = rest;
    Some(*taken)
}

macro_rules! number_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            const OCTETS: Option<usize> = Some(core::mem::size_of::<$t>());

            #[inline]
            fn read(octets: &mut &[u8]) -> Option<Self> {
                take(octets).map(<$t>::from_be_bytes)
            }
            #[inline]
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
            #[inline]
            fn size(&self) -> usize {
                core::mem::size_of::<$t>()
            }
        }

        #[cfg(feature = "tree")]
        impl FieldTree for $t {
            const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Number(<$t>::MAX as u64);
            #[inline]
            fn to_tree(&self) -> crate::tree::Value {
                crate::tree::unsigned((*self).into())
            }
            #[inline]
            fn from_tree(
                value: &crate::tree::Value,
                reading: &mut crate::tree::Reading,
            ) -> Result<Self, crate::tree::Refused> {
                Ok(reading.unsigned(value, 0, <$t>::MAX.into())? as $t)
            }
        }
    )*};
}
number_field!(u8, u16, u32, u64);

/// The signed numbers of a layout, in two's complement.
macro_rules! signed_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            const OCTETS: Option<usize> = Some(core::mem::size_of::<$t>());

            #[inline]
            fn read(octets: &mut &[u8]) -> Option<Self> {
                take(octets).map(<$t>::from_be_bytes)
            }
            #[inline]
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
            #[inline]
            fn size(&self) -> usize {
                core::mem::size_of::<$t>()
            }
        }

        #[cfg(feature = "tree")]
        impl FieldTree for $t {
            const TAKES: &'static crate::tree::Takes =
                &crate::tree::Takes::Signed(<$t>::MIN as i64, <$t>::MAX as i64);
            #[inline]
            fn to_tree(&self) -> crate::tree::Value {
                crate::tree::signed((*self).into())
            }
            #[inline]
            fn from_tree(
                value: &crate::tree::Value,
                reading: &mut crate::tree::Reading,
            ) -> Result<Self, crate::tree::Refused> {
                Ok(reading.signed(value, <$t>::MIN.into(), <$t>::MAX.into())? as $t)
            }
        }
    )*};
}
signed_field!(i32, i64);

/// An enumeration of the tables of values, as a [`Narrow`] field reads it.
pub trait Narrowable: Copy + Field {
    /// The width of its field, in bits.
    const BITS: u32;
    /// The value of a number.
    fn from_value(value: u8) -> Self;
    /// The number.
    fn value(self) -> u8;
}

/// The tree of a value of a few bits in a [`Narrow`] field.
#[cfg(feature = "tree")]
pub trait NarrowTree: Narrowable + FieldTree {
    /// What its tree takes in a [`Narrow`] field: a value of its bits.
    const NARROW: &'static crate::tree::Takes;
}

/// A value of the low bits of its octet, whose other bits are spare, as the Time Unit of
/// a Packet Rate and the Node Type of an FQ-CSID of PFCP (TS 29.244 §8.2.63, §8.2.43). Its
/// tree is the value, or `{"value": ..., "Spare": ...}` for spare bits that are set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Narrow<E> {
    /// The value.
    pub value: E,
    /// The other bits of the octet, which are spare: zero from a sender.
    pub spare: u8,
}

impl<E> Narrow<E> {
    /// The value, without spare bits.
    #[inline]
    pub fn new(value: E) -> Self {
        Self { value, spare: 0 }
    }
}

/// The bits of the octet of a [`Narrow`] value that are its value.
#[inline]
const fn mask<E: Narrowable>() -> u8 {
    ((1u16 << E::BITS) - 1) as u8
}

impl<E: Narrowable> Field for Narrow<E> {
    const OCTETS: Option<usize> = Some(1);

    const CELLS: &'static [NarrowCell] = &[NarrowCell {
        name: None,
        octet: 0,
        bit: 1,
        width: E::BITS as u8,
    }];
    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let octet = u8::read(octets)?;
        Some(Self {
            value: E::from_value(octet & mask::<E>()),
            spare: octet & !mask::<E>(),
        })
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        let (spare, value) = (self.spare & !mask::<E>(), self.value.value() & mask::<E>());
        out.push(together(spare, value));
    }
    #[inline]
    fn size(&self) -> usize {
        1
    }
    #[inline]
    fn fits(&self) -> bool {
        self.value.value() <= mask::<E>() && self.spare & mask::<E>() == 0
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if !self.fits() {
            found.push(D::found(
                path,
                Some(clause),
                Finding::Rule("a value wider than its bits"),
            ));
        }
        if self.spare != 0 {
            found.push(D::found(path, None, Finding::SpareBits));
        }
        Field::check(&self.value, path, clause, found);
    }
}

#[cfg(feature = "tree")]
impl<E: NarrowTree> FieldTree for Narrow<E> {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Members(
        Some("value"),
        &[
            ("value", E::NARROW),
            ("Spare", &crate::tree::Takes::Number(u8::MAX as u64)),
        ],
    );
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        if self.spare == 0 {
            return self.value.to_tree();
        }
        let mut members = crate::tree::Members::new();
        members.put("value", self.value.to_tree());
        members.put("Spare", crate::tree::unsigned(self.spare.into()));
        members.finish()
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        let narrow = match value.is_object() {
            false => Self::new(E::from_tree(value, reading)?),
            true => {
                let read = reading.members(value, &["value", "Spare"])?;
                let spare = reading
                    .optional(&read, "Spare", |value, reading| {
                        reading.unsigned(value, 0, 255)
                    })?
                    .unwrap_or(0) as u8;
                Self {
                    value: reading.member(&read, "value", E::from_tree)?,
                    spare,
                }
            }
        };
        if !narrow.fits() {
            return Err(
                reading.mismatch("a value of its bits, and spare bits of the others", value)
            );
        }
        Ok(narrow)
    }
}

/// A number of the low `BITS` bits of its octet, as a [`Narrow`] field reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Number<const BITS: u32>(pub u8);

/// A number of the six low bits of its octet, whose two others are spare: as a QFI, or the
/// Paging Policy Indication value and the DSCP value of PFCP (TS 29.244 §8.2.27, §8.2.214).
pub type SixBits = Narrow<Number<6>>;

#[cfg(feature = "tree")]
impl<const BITS: u32> NarrowTree for Number<BITS> {
    const NARROW: &'static crate::tree::Takes = &crate::tree::Takes::Number((1 << BITS) - 1);
}

impl<const BITS: u32> Narrowable for Number<BITS> {
    const BITS: u32 = BITS;
    #[inline]
    fn from_value(value: u8) -> Self {
        Self(value)
    }
    #[inline]
    fn value(self) -> u8 {
        self.0
    }
}

/// A whole octet; [`Narrow`] keeps its spare bits apart.
impl<const BITS: u32> Field for Number<BITS> {
    const OCTETS: Option<usize> = Some(1);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        u8::read(octets).map(Self)
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        1
    }
}

#[cfg(feature = "tree")]
impl<const BITS: u32> FieldTree for Number<BITS> {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Number(u8::MAX as u64);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::unsigned(self.0.into())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        u8::from_tree(value, reading).map(Self)
    }
}

/// A number of five octets: a bitrate of MBR and GBR, in kbps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct U40(pub u64);

impl Field for U40 {
    const OCTETS: Option<usize> = Some(5);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let [a, b, c, d, e] = take::<5>(octets)?;
        Some(Self(u64::from_be_bytes([0, 0, 0, a, b, c, d, e])))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.to_be_bytes()[3..]);
    }
    #[inline]
    fn size(&self) -> usize {
        5
    }
    #[inline]
    fn fits(&self) -> bool {
        self.0 < 1 << 40
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if !self.fits() {
            found.push(D::found(
                path,
                Some(clause),
                Finding::Rule("a value wider than its bits"),
            ));
        }
    }
}

#[cfg(feature = "tree")]
impl FieldTree for U40 {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Number((1 << 40) - 1);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::unsigned(self.0)
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        Ok(Self(reading.unsigned(value, 0, (1 << 40) - 1)?))
    }
}

/// A VLAN tag of three octets, as the C-TAG and S-TAG of PFCP have it (TS 29.244 §8.2.56).
pub type VlanTag = [u8; 3];

/// Sixteen octets that their clause does not split: an NF Instance ID (a UUID).
pub type OctetArray16 = [u8; 16];

/// Octets of a fixed number, as they are.
impl<const N: usize> Field for [u8; N] {
    const OCTETS: Option<usize> = Some(N);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        take::<N>(octets)
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
    #[inline]
    fn size(&self) -> usize {
        N
    }
}

#[cfg(feature = "tree")]
impl<const N: usize> FieldTree for [u8; N] {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Octets(Some(N));
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::octets(self)
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading.octet_array::<N>(value)
    }
}

/// A MAC address.
pub type Mac = [u8; 6];

/// Octets after a length of one octet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lv8(pub Octets);

/// Octets after a length of two octets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lv16(pub Octets);

/// The [`Field`] of a type of octets after a length of `$length`: `octets`, whose tree is the
/// octets in hexadecimal, or `name(FIELDS, TAKES)`, whose tree is the name of its labels
/// ([`name_tree`]) with `FIELDS` the member of its octets that are no name.
#[doc(hidden)]
#[macro_export]
macro_rules! __length_value {
    ($ty:ident $length:ty, $tree:ident $(($fields:expr, $takes:expr))?) => {
        impl $crate::fields::Field for $ty {
            #[inline]
            fn read(octets: &mut &[u8]) -> Option<Self> {
                let length = usize::from(<$length as $crate::fields::Field>::read(octets)?);
                let (value, rest) = octets.split_at_checked(length)?;
                *octets = rest;
                Some(Self($crate::fields::Octets::copied(value)))
            }
            #[inline]
            fn write(&self, out: &mut alloc::vec::Vec<u8>) {
                // encode() refuses a value longer than its length can say.
                let length = <$length>::try_from(self.0.len()).unwrap_or(<$length>::MAX);
                out.extend_from_slice(&length.to_be_bytes());
                out.extend_from_slice(&self.0);
            }
            #[inline]
            fn size(&self) -> usize {
                core::mem::size_of::<$length>() + self.0.len()
            }
            #[inline]
            fn fits(&self) -> bool {
                self.0.len() <= usize::from(<$length>::MAX)
            }
            #[inline]
            fn check<D: $crate::fields::Deviate>(&self, path: &$crate::fields::Path<'_>, clause: &'static str, found: &mut alloc::vec::Vec<D>) {
                if !self.fits() {
                    found.push(D::found(path, Some(clause), $crate::fields::Finding::Rule("more octets than their length can say")));
                }
            }
        }

        #[cfg(feature = "tree")]
        impl $crate::fields::FieldTree for $ty {
            const TAKES: &'static $crate::tree::Takes = $crate::__length_value!(@takes $tree $(($fields, $takes))?);
            #[inline]
            fn to_tree(&self) -> $crate::tree::Value {
                $crate::__length_value!(@to_tree $tree $(($fields, $takes))? &self.0)
            }
            #[inline]
            fn from_tree(
                value: &$crate::tree::Value,
                reading: &mut $crate::tree::Reading,
            ) -> core::result::Result<Self, $crate::tree::Refused> {
                let octets = $crate::__length_value!(@from_tree $tree $(($fields, $takes))? value, reading)?;
                if octets.len() > usize::from(<$length>::MAX) {
                    return Err(reading.mismatch("octets that their length can say", value));
                }
                Ok(Self(octets.into()))
            }
        }
    };
    (@to_tree octets $octets:expr) => { $crate::tree::octets($octets) };
    (@to_tree name($fields:expr, $takes:expr) $octets:expr) => { $crate::fields::name_tree($octets, $fields) };
    (@from_tree octets $value:ident, $reading:ident) => { $reading.octets($value) };
    (@from_tree name($fields:expr, $takes:expr) $value:ident, $reading:ident) => { $crate::fields::name_from_tree($value, $reading, $fields) };
    (@takes octets) => { &$crate::tree::Takes::Octets(None) };
    (@takes name($fields:expr, $takes:expr)) => { $takes };
}
crate::__length_value!(Lv8 u8, octets);
crate::__length_value!(Lv16 u16, octets);

/// The octets up to the end of the IE: the last field of a layout whose length its clause
/// does not give.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rest(pub Octets);

impl Field for Rest {
    const TO_THE_END: bool = true;

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let all = core::mem::take(octets);
        Some(Self(Octets::copied(all)))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        self.0.len()
    }
}

#[cfg(feature = "tree")]
impl FieldTree for Rest {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Octets(None);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::octets(&self.0)
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading.octets(value).map(|octets| Self(octets.into()))
    }
}

/// A percentage, from 0 to 100: a receiver takes another value as 0, as PFCP has it (TS
/// 29.244 §8.2.34, §8.2.126, §8.2.196).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Percent(pub u8);

impl Percent {
    /// The percentage as a receiver takes it: 0 for a value above 100.
    #[inline]
    pub const fn effective(self) -> u8 {
        match self.0 {
            0..=100 => self.0,
            _ => 0,
        }
    }
}

impl Field for Percent {
    const OCTETS: Option<usize> = Some(1);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        u8::read(octets).map(Self)
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        1
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if self.0 > 100 {
            found.push(D::found(
                path,
                Some(clause),
                Finding::ReadAs {
                    value: self.0.into(),
                    read: 0,
                },
            ));
        }
    }
}

#[cfg(feature = "tree")]
impl FieldTree for Percent {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Number(u8::MAX as u64);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::unsigned(self.0.into())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        u8::from_tree(value, reading).map(Self)
    }
}

/// An octet that is spare: zero from a sender, kept as it is. A tree has it where it is
/// set, as `"Spare": 8`, and is 0 without it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Spare(pub u8);

impl Field for Spare {
    const OCTETS: Option<usize> = Some(1);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        u8::read(octets).map(Self)
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        1
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, _clause: &'static str, found: &mut Vec<D>) {
        if self.0 != 0 {
            found.push(D::found(path, None, Finding::SpareBits));
        }
    }
}

#[cfg(feature = "tree")]
impl FieldTree for Spare {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Number(u8::MAX as u64);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::unsigned(self.0.into())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        u8::from_tree(value, reading).map(Self)
    }
    #[inline]
    fn put_member(&self, members: &mut crate::tree::Members, name: &'static str) {
        if self.0 != 0 {
            members.put(name, self.to_tree());
        }
    }
    #[inline]
    fn read_member(
        read: &crate::tree::Read<'_>,
        name: &'static str,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        let spare = reading.optional(read, name, Self::from_tree)?;
        Ok(spare.unwrap_or_default())
    }
}

/// A number of one octet, then that many values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counted<T>(pub Vec<T>);

impl<T: Field> Field for Counted<T> {
    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let count = u8::read(octets)?;
        let mut values = Vec::with_capacity(usize::from(count).min(octets.len()));
        for _ in 0..count {
            values.push(T::read(octets)?);
        }
        Some(Self(values))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        // More values than the number can say are written with the number cut: check()
        // reports it.
        out.push(u8::try_from(self.0.len()).unwrap_or(u8::MAX));
        for value in &self.0 {
            value.write(out);
        }
    }
    #[inline]
    fn size(&self) -> usize {
        1 + self.0.iter().map(Field::size).sum::<usize>()
    }
    #[inline]
    fn fits(&self) -> bool {
        self.0.len() <= usize::from(u8::MAX) && self.0.iter().all(Field::fits)
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if self.0.len() > usize::from(u8::MAX) {
            found.push(D::found(
                path,
                Some(clause),
                Finding::Rule("more values than their number can say"),
            ));
        }
        for value in &self.0 {
            value.check(path, clause, found);
        }
    }
}

#[cfg(feature = "tree")]
impl<T: FieldTree> FieldTree for Counted<T> {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::List(T::TAKES);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::Value::Array(self.0.iter().map(FieldTree::to_tree).collect())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        let values = reading.list(value, T::from_tree)?;
        if values.len() > usize::from(u8::MAX) {
            return Err(reading.mismatch("at most 255 values", value));
        }
        Ok(Self(values))
    }
}

/// Values up to the end of the IE, each after the other, without their number.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Each<T>(pub Vec<T>);

impl<T: Field> Field for Each<T> {
    const TO_THE_END: bool = true;
    const CELLS: &'static [NarrowCell] = T::CELLS;

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let mut values = Vec::new();
        while !octets.is_empty() {
            values.push(T::read(octets)?);
        }
        Some(Self(values))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        for value in &self.0 {
            value.write(out);
        }
    }
    #[inline]
    fn size(&self) -> usize {
        self.0.iter().map(Field::size).sum()
    }
    #[inline]
    fn fits(&self) -> bool {
        self.0.iter().all(Field::fits)
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        for value in &self.0 {
            value.check(path, clause, found);
        }
    }
}

#[cfg(feature = "tree")]
impl<T: FieldTree> FieldTree for Each<T> {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::List(T::TAKES);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::Value::Array(self.0.iter().map(FieldTree::to_tree).collect())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading.list(value, T::from_tree).map(Self)
    }
}

/// A list of entries behind a two-octet length that counts the octets of all the entries.
/// The PFD Contents of PFCP have it for their additional flow descriptions, URLs and
/// domain names (TS 29.244 Figures 8.2.39-2 to 8.2.39-4). In a tree, it is a list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entries16<T>(pub Vec<T>);

impl<T: Field> Field for Entries16<T> {
    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let length = usize::from(u16::read(octets)?);
        let (mut within, rest) = octets.split_at_checked(length)?;
        *octets = rest;
        let mut entries = Vec::new();
        while !within.is_empty() {
            entries.push(T::read(&mut within)?);
        }
        Some(Self(entries))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        // put() refuses entries longer than their length can say.
        let length = self.size() - 2;
        out.extend_from_slice(&u16::try_from(length).unwrap_or(u16::MAX).to_be_bytes());
        for entry in &self.0 {
            entry.write(out);
        }
    }
    #[inline]
    fn size(&self) -> usize {
        2 + self.0.iter().map(Field::size).sum::<usize>()
    }
    #[inline]
    fn fits(&self) -> bool {
        self.size() - 2 <= usize::from(u16::MAX) && self.0.iter().all(Field::fits)
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if !self.fits() {
            let rule = Finding::Rule("more octets than their length can say");
            found.push(D::found(path, Some(clause), rule));
        }
        for entry in &self.0 {
            entry.check(path, clause, found);
        }
    }
}

#[cfg(feature = "tree")]
impl<T: FieldTree> FieldTree for Entries16<T> {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::List(T::TAKES);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::Value::Array(self.0.iter().map(FieldTree::to_tree).collect())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading.list(value, T::from_tree).map(Self)
    }
}

/// A PLMN of three octets, the MCC and the MNC in the order of TS 24.008 clause 10.5.1.3:
/// `"208-93"` in a tree, or the octets of digits that are not decimal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlmnId(pub [u8; 3]);

impl PlmnId {
    /// The PLMN, when its digits are decimal.
    #[inline]
    pub fn plmn(self) -> Option<crate::identity::Plmn> {
        crate::identity::Plmn::from_nas_octets(self.0).ok()
    }
}

impl From<crate::identity::Plmn> for PlmnId {
    #[inline]
    fn from(plmn: crate::identity::Plmn) -> Self {
        Self(plmn.to_nas_octets())
    }
}

impl Field for PlmnId {
    const OCTETS: Option<usize> = Some(3);

    /// Its digits, by two in an octet (TS 24.008 clause 10.5.1.3).
    const CELLS: &'static [NarrowCell] = &[
        NarrowCell {
            name: Some("MCC digit 2"),
            octet: 0,
            bit: 5,
            width: 4,
        },
        NarrowCell {
            name: Some("MCC digit 1"),
            octet: 0,
            bit: 1,
            width: 4,
        },
        NarrowCell {
            name: Some("MNC digit 3"),
            octet: 1,
            bit: 5,
            width: 4,
        },
        NarrowCell {
            name: Some("MCC digit 3"),
            octet: 1,
            bit: 1,
            width: 4,
        },
        NarrowCell {
            name: Some("MNC digit 2"),
            octet: 2,
            bit: 5,
            width: 4,
        },
        NarrowCell {
            name: Some("MNC digit 1"),
            octet: 2,
            bit: 1,
            width: 4,
        },
    ];
    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        take(octets).map(Self)
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        3
    }
}

#[cfg(feature = "tree")]
impl FieldTree for PlmnId {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Plmn;
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        match self.plmn() {
            Some(plmn) if plmn.to_nas_octets() == self.0 => {
                crate::tree::text(alloc::format!("{plmn}"))
            }
            _ => crate::tree::octets(&self.0),
        }
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        if let Some(plmn) = value
            .as_str()
            .and_then(|text| text.parse::<crate::identity::Plmn>().ok())
        {
            return Ok(plmn.into());
        }
        reading.octet_array::<3>(value).map(Self)
    }
}

/// A number of three octets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct U24(pub u32);

impl Field for U24 {
    const OCTETS: Option<usize> = Some(3);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        let [a, b, c] = take::<3>(octets)?;
        Some(Self(u32::from_be_bytes([0, a, b, c])))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.to_be_bytes()[1..]);
    }
    #[inline]
    fn size(&self) -> usize {
        3
    }
    #[inline]
    fn fits(&self) -> bool {
        self.0 < 1 << 24
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if !self.fits() {
            found.push(D::found(
                path,
                Some(clause),
                Finding::Rule("a value wider than its bits"),
            ));
        }
    }
}

#[cfg(feature = "tree")]
impl FieldTree for U24 {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Number(0xff_ffff);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::unsigned(self.0.into())
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        Ok(Self(reading.unsigned(value, 0, 0xff_ffff)? as u32))
    }
}

macro_rules! address_field {
    ($($t:ty, $n:literal, $takes:ident);*) => {$(
        impl Field for $t {
            const OCTETS: Option<usize> = Some($n);

            #[inline]
            fn read(octets: &mut &[u8]) -> Option<Self> {
                take::<$n>(octets).map(<$t>::from)
            }
            #[inline]
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.octets());
            }
            #[inline]
            fn size(&self) -> usize {
                $n
            }
        }

        #[cfg(feature = "tree")]
        impl FieldTree for $t {
            const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::$takes;
            #[inline]
            fn to_tree(&self) -> crate::tree::Value {
                crate::tree::text(alloc::format!("{self}"))
            }
            #[inline]
            fn from_tree(
                value: &crate::tree::Value,
                reading: &mut crate::tree::Reading,
            ) -> Result<Self, crate::tree::Refused> {
                let text = reading.text(value)?;
                text.parse().map_err(|_| reading.mismatch(stringify!($t), value))
            }
        }
    )*};
}
address_field!(Ipv4Addr, 4, Ipv4; Ipv6Addr, 16, Ipv6);

/// An IPv4 address that a value always has, as the DNS Server Address and the NBNS Server
/// Address of PFCP (TS 29.244 §8.2.193, §8.2.194), `0.0.0.0` by default. Its tree is the
/// address, as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Address(pub Ipv4Addr);

impl Default for Ipv4Address {
    #[inline]
    fn default() -> Self {
        Self(Ipv4Addr::UNSPECIFIED)
    }
}

impl From<Ipv4Addr> for Ipv4Address {
    #[inline]
    fn from(address: Ipv4Addr) -> Self {
        Self(address)
    }
}

impl Field for Ipv4Address {
    const OCTETS: Option<usize> = Some(4);

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        Ipv4Addr::read(octets).map(Self)
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        self.0.write(out);
    }
    #[inline]
    fn size(&self) -> usize {
        4
    }
}

#[cfg(feature = "tree")]
impl FieldTree for Ipv4Address {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Ipv4;
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        self.0.to_tree()
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        Ipv4Addr::from_tree(value, reading).map(Self)
    }
}

/// What the labels of an FQDN break of RFC 1035, written as a DNS message writes them: `None` for labels of 1 to 63 octets, each after its length.
#[inline]
pub fn fqdn_broken(labels: &[u8]) -> Option<&'static str> {
    match labels {
        [] => Some("an FQDN without a label"),
        _ if !fqdn_valid(labels) => Some(
            "labels that are no FQDN of RFC 1035: each of 1 to 63 octets, after its length, and 255 octets at most with the trailing zero byte",
        ),
        _ => None,
    }
}

/// Whether octets are labels of an FQDN as RFC 1035 writes them, without the last zero
/// octet: each of 1 to 63 octets, after its length, and [`FQDN_OCTETS`] at most.
#[inline]
fn fqdn_valid(mut octets: &[u8]) -> bool {
    if octets.len() > FQDN_OCTETS {
        return false;
    }
    while let [length @ 1..=63, rest @ ..] = octets {
        let Some(tail) = rest.get(usize::from(*length)..) else {
            return false;
        };
        octets = tail;
    }
    octets.is_empty()
}

/// The most octets of the labels of an FQDN, without the trailing zero byte: 255 with it
/// (RFC 1035 §2.3.4).
pub const FQDN_OCTETS: usize = 254;

/// The labels of an FQDN as RFC 1035 writes them, without the last zero octet: `None`
/// for octets that are not labels of 1 to 63 octets, each after its length, or that are
/// more than [`FQDN_OCTETS`].
#[inline]
fn fqdn_labels(mut octets: &[u8]) -> Option<Vec<&[u8]>> {
    if octets.len() > FQDN_OCTETS {
        return None;
    }
    let mut labels = Vec::new();
    while let [length @ 1..=63, rest @ ..] = octets {
        let (label, tail) = rest.split_at_checked(usize::from(*length))?;
        labels.push(label);
        octets = tail;
    }
    (octets.is_empty() && !labels.is_empty()).then_some(labels)
}

/// Whether octets are base64-encoded characters, as the `byte` format of OpenAPI that TS
/// 29.571 clause 5.5.2 names has them (IETF RFC 4648 clause 4): groups of four characters
/// of the alphabet, the last with one or two `=` of padding where the octets end short of
/// three.
#[inline]
pub fn base64(octets: &[u8]) -> bool {
    let padding = octets
        .iter()
        .rev()
        .take(2)
        .take_while(|octet| **octet == b'=');
    let characters = &octets[..octets.len() - padding.count()];
    octets.len().is_multiple_of(4)
        && characters
            .iter()
            .all(|octet| octet.is_ascii_alphanumeric() || matches!(octet, b'+' | b'/'))
}

/// Whether a text is a URL by its generic syntax (IETF RFC 3986 clause 3): a scheme, of a
/// letter and of letters, digits, `+`, `-` and `.`, a colon, and visible characters of
/// ASCII after it.
#[inline]
pub fn url(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once(':') else {
        return false;
    };
    let of_scheme = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.');
    scheme
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        && scheme.bytes().all(of_scheme)
        && !rest.is_empty()
        && rest.bytes().all(|c| c.is_ascii_graphic())
}

/// Whether a text is a SIP URI by its scheme (IETF RFC 3261 clause 19.1.1): `sip:` or
/// `sips:`, in letters of any case, and what a URL has after it.
#[inline]
pub fn sip_uri(text: &str) -> bool {
    let scheme = text.split(':').next().unwrap_or_default();
    (scheme.eq_ignore_ascii_case("sip") || scheme.eq_ignore_ascii_case("sips")) && url(text)
}

/// Whether octets are a field name of HTTP, a token (IETF RFC 9110 clauses 5.1 and 5.6.2):
/// one or more letters, digits and ``!#$%&'*+-.^_`|~``.
#[inline]
pub fn http_field_name(octets: &[u8]) -> bool {
    let token = |octet: &u8| octet.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(octet);
    !octets.is_empty() && octets.iter().all(token)
}

/// Whether octets are a field value of HTTP (IETF RFC 9110 clause 5.5): visible characters
/// and the octets above them, with spaces and tabs between them, and none before or after.
#[inline]
pub fn http_field_value(octets: &[u8]) -> bool {
    let blank = |octet: &u8| matches!(octet, b' ' | b'\t');
    let content = |octet: &u8| blank(octet) || matches!(octet, 0x21..=0x7e | 0x80..);
    octets.iter().all(content)
        && !octets.first().is_some_and(blank)
        && !octets.last().is_some_and(blank)
}

/// Whether labels are a full APN or DNN, of a Network Identifier and an Operator
/// Identifier (TS 23.003 clauses 9.1.1 and 9.1.2): the Operator Identifier "is composed of
/// three labels. The last label (or domain) shall be "gprs"", in letters of any case,
/// after the "at least one label" of the Network Identifier.
#[inline]
pub fn full_apn(octets: &[u8]) -> bool {
    fqdn_labels(octets).is_some_and(|labels| {
        labels.len() >= 4
            && labels
                .last()
                .is_some_and(|last| last.eq_ignore_ascii_case(b"gprs"))
    })
}

/// The labels of `name`, each after its length: `None` for an empty label, a label of
/// more than 63 octets, or more than [`FQDN_OCTETS`] of labels.
#[inline]
pub fn fqdn_of(name: &str) -> Option<Vec<u8>> {
    if name.len() + 1 > FQDN_OCTETS {
        return None;
    }
    let mut labels = Vec::with_capacity(name.len() + 1);
    for label in name.split('.') {
        let length = u8::try_from(label.len())
            .ok()
            .filter(|length| (1..=63).contains(length))?;
        labels.push(length);
        labels.extend_from_slice(label.as_bytes());
    }
    Some(labels)
}

/// The name of labels, with dots: `None` for labels that are not each of 1 to 63 octets
/// of UTF-8 without a dot, or more than [`FQDN_OCTETS`], which the name would not give
/// back.
#[inline]
pub fn fqdn_name(octets: &[u8]) -> Option<String> {
    let mut text = String::new();
    for label in fqdn_labels(octets)? {
        let label = core::str::from_utf8(label)
            .ok()
            .filter(|label| !label.contains('.'))?;
        if !text.is_empty() {
            text.push('.');
        }
        text.push_str(label);
    }
    Some(text)
}

/// An FQDN as a DNS message writes it (RFC 1035 §3.1), without the trailing zero byte:
/// its labels, each after its length, the rest of the IE, as the SMF Set ID of PFCP has
/// it (TS 29.244 §8.2.131). Its tree is the name, `"smf.example"`, or `{"octets": ...}`
/// for octets that are no such labels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fqdn(pub Octets);

impl Fqdn {
    /// The labels of a name, `smf.example`: `None` for a name with an empty label, a
    /// label of more than 63 octets, or of more than 253 octets, whose labels are more
    /// than 255 with the trailing zero byte (RFC 1035 §2.3.4).
    #[inline]
    pub fn new(name: &str) -> Option<Self> {
        fqdn_of(name).map(|labels| Self(labels.into()))
    }

    /// The name, its labels with dots: `None` for octets that are no labels of 1 to 63
    /// octets of UTF-8 without a dot, which the name would not give back.
    #[inline]
    pub fn name(&self) -> Option<String> {
        fqdn_name(&self.0)
    }
}

impl Field for Fqdn {
    const TO_THE_END: bool = true;

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        Some(Self(Octets::copied(core::mem::take(octets))))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        self.0.len()
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if let Some(broken) = fqdn_broken(&self.0) {
            found.push(D::found(path, Some(clause), Finding::Rule(broken)));
        }
    }
}

#[cfg(feature = "tree")]
impl FieldTree for Fqdn {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::OneOf(&[
        &crate::tree::Takes::Name,
        &crate::tree::Takes::Members(None, &[("octets", &crate::tree::Takes::Octets(None))]),
    ]);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        use crate::tree;
        match self.name() {
            Some(name) => tree::text(name),
            None => tree::alternative("octets", tree::octets(&self.0)),
        }
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        if let Some(name) = value.as_str() {
            return Self::new(name).ok_or_else(|| {
                reading.mismatch(
                    "a name of 253 octets at most, of labels of 1 to 63 octets with dots between them",
                    value,
                )
            });
        }
        let (_, content) = reading.alternative(value, &["octets"])?;
        let octets = reading.within("octets", content, |content, reading| {
            reading.octets(content)
        })?;
        Ok(Self(octets.into()))
    }
}

// ── Secrets ────────────────────────────────────────────────────────────────

/// Octets of a key or a password that are no text, which `Debug` does not print: their
/// length alone.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SecretLv8(pub Octets);

impl fmt::Debug for SecretLv8 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SecretLv8({} octets)", self.0.len())
    }
}

impl Field for SecretLv8 {
    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        Lv8::read(octets).map(|value| Self(value.0))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        Lv8(self.0.clone()).write(out);
    }
    #[inline]
    fn size(&self) -> usize {
        1 + self.0.len()
    }
    #[inline]
    fn fits(&self) -> bool {
        self.0.len() <= usize::from(u8::MAX)
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        if !self.fits() {
            found.push(D::found(
                path,
                Some(clause),
                Finding::Rule("more octets than their length can say"),
            ));
        }
    }
}

#[cfg(feature = "tree")]
impl FieldTree for SecretLv8 {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Octets(None);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::octets(&self.0)
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading
            .secret(|reading| Lv8::from_tree(value, reading))
            .map(|value| Self(value.0))
    }
}

/// The octets of a key up to the end of the IE, which `Debug` does not print: their
/// length alone. The last field of a layout, as [`Rest`] is.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SecretRest(pub Octets);

impl fmt::Debug for SecretRest {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SecretRest({} octets)", self.0.len())
    }
}

impl Field for SecretRest {
    const TO_THE_END: bool = true;

    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        Rest::read(octets).map(|value| Self(value.0))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    #[inline]
    fn size(&self) -> usize {
        self.0.len()
    }
}

#[cfg(feature = "tree")]
impl FieldTree for SecretRest {
    const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::Octets(None);
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        crate::tree::octets(&self.0)
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading
            .secret(|reading| reading.octets(value))
            .map(|octets| Self(octets.into()))
    }
}

/// A passphrase after a length of one octet, a UTF8String, which `Debug` does not print,
/// nor an error of its tree, as the Shared Secret of PFCP (TS 29.244 §8.2.276).
/// Its tree is the text, or `{"octets": ...}` for octets that are no UTF-8.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct SecretTextLv8(pub Octets);

impl fmt::Debug for SecretTextLv8 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SecretTextLv8({} octets)", self.0.len())
    }
}

impl Field for SecretTextLv8 {
    #[inline]
    fn read(octets: &mut &[u8]) -> Option<Self> {
        Lv8::read(octets).map(|value| Self(value.0))
    }
    #[inline]
    fn write(&self, out: &mut Vec<u8>) {
        Lv8(self.0.clone()).write(out);
    }
    #[inline]
    fn size(&self) -> usize {
        1 + self.0.len()
    }
    #[inline]
    fn fits(&self) -> bool {
        self.0.len() <= usize::from(u8::MAX)
    }
    #[inline]
    fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
        Lv8(self.0.clone()).check(path, clause, found);
    }
}

#[cfg(feature = "tree")]
impl FieldTree for SecretTextLv8 {
    const TAKES: &'static crate::tree::Takes = TextLv8::TAKES;
    #[inline]
    fn to_tree(&self) -> crate::tree::Value {
        TextLv8(self.0.clone()).to_tree()
    }
    #[inline]
    fn from_tree(
        value: &crate::tree::Value,
        reading: &mut crate::tree::Reading,
    ) -> Result<Self, crate::tree::Refused> {
        reading.secret(|reading| {
            let octets = text_from_tree(value, reading, "octets")?;
            if octets.len() > usize::from(u8::MAX) {
                return Err(reading.mismatch("octets that their length can say", value));
            }
            Ok(Self(octets.into()))
        })
    }
}

/// Digits after a length of one octet, as TBCD: an IMSI, an IMEI or an MSISDN, as TS
/// 29.274 §8.3, §8.10 and §8.11 write them. Its tree is the digits, or `{"octets": ...}`
/// for octets that are no TBCD string.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct DigitsLv8(pub Octets);

impl DigitsLv8 {
    /// The digits, as `tbcd::encode` gives back their octets.
    #[inline]
    pub fn digits(&self) -> Option<alloc::string::String> {
        let text = crate::identity::tbcd::decode(&self.0).ok()?;
        (crate::identity::tbcd::encode(&text).ok()? == *self.0).then_some(text)
    }

    /// How many digits it has, when they are TBCD digits of 0 to 9 alone.
    #[inline]
    pub fn decimal(&self) -> Option<usize> {
        let digits = self.digits()?;
        digits
            .bytes()
            .all(|digit| digit.is_ascii_digit())
            .then_some(digits.len())
    }

    /// The octets of `digits`.
    ///
    /// # Errors
    ///
    /// A character that TBCD does not have.
    #[inline]
    pub fn new(digits: &str) -> Result<Self, crate::identity::Error> {
        crate::identity::tbcd::encode(digits).map(|octets| Self(octets.into()))
    }
}

impl fmt::Debug for DigitsLv8 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.digits() {
            Some(digits) => write!(f, "DigitsLv8({digits:?})"),
            None => write!(f, "DigitsLv8({:?})", Octets::copied(&self.0)),
        }
    }
}

/// Text after a length of one octet, a UTF8String: as a NAI, a SUPI, a GPSI or a PEI, and
/// the Key ID of a Key ID and Shared Secret of PFCP (TS 29.244 §8.2.276). Its tree is the text, or
/// `{"octets": ...}` for octets that are no UTF-8.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextLv8(pub Octets);

impl TextLv8 {
    /// The text.
    #[inline]
    pub fn text(&self) -> Option<&str> {
        core::str::from_utf8(&self.0).ok()
    }
}

impl From<&str> for TextLv8 {
    #[inline]
    fn from(text: &str) -> Self {
        Self(Octets::copied(text.as_bytes()))
    }
}

macro_rules! identity_lv8 {
    ($($ty:ident($length:ty) $as_text:ident $takes:ident $from_text:expr),*) => {$(
        impl Field for $ty {
            #[inline]
            fn read(octets: &mut &[u8]) -> Option<Self> {
                let length = usize::from(<$length as Field>::read(octets)?);
                let (value, rest) = octets.split_at_checked(length)?;
                *octets = rest;
                Some(Self(Octets::copied(value)))
            }
            #[inline]
            fn write(&self, out: &mut Vec<u8>) {
                let length = <$length>::try_from(self.0.len()).unwrap_or(<$length>::MAX);
                out.extend_from_slice(&length.to_be_bytes());
                out.extend_from_slice(&self.0);
            }
            #[inline]
            fn size(&self) -> usize {
                core::mem::size_of::<$length>() + self.0.len()
            }
            #[inline]
            fn fits(&self) -> bool {
                self.0.len() <= usize::from(<$length>::MAX)
            }
            #[inline]
            fn check<D: Deviate>(&self, path: &Path<'_>, clause: &'static str, found: &mut Vec<D>) {
                if !self.fits() {
                    found.push(D::found(path, Some(clause), Finding::Rule("more octets than their length can say")));
                }
            }
        }

        #[cfg(feature = "tree")]
        impl FieldTree for $ty {
            const TAKES: &'static crate::tree::Takes = &crate::tree::Takes::OneOf(&[
                &crate::tree::Takes::$takes,
                &crate::tree::Takes::Members(None, &[("octets", &crate::tree::Takes::Octets(None))]),
            ]);
            #[inline]
            fn to_tree(&self) -> crate::tree::Value {
                match self.$as_text() {
                    Some(text) => crate::tree::text(text),
                    None => crate::tree::alternative("octets", crate::tree::octets(&self.0)),
                }
            }
            #[inline]
            fn from_tree(
                value: &crate::tree::Value,
                reading: &mut crate::tree::Reading,
            ) -> Result<Self, crate::tree::Refused> {
                let octets = match value.as_str() {
                    Some(text) => {
                        let from_text: fn(&str) -> Option<Vec<u8>> = $from_text;
                        from_text(text).ok_or_else(|| reading.mismatch(stringify!($ty), value))?
                    }
                    None => {
                        let (_, content) = reading.alternative(value, &["octets"])?;
                        reading.within("octets", content, |content, reading| reading.octets(content))?
                    }
                };
                if octets.len() > usize::from(<$length>::MAX) {
                    return Err(reading.mismatch("octets that their length can say", value));
                }
                Ok(Self(octets.into()))
            }
        }
    )*};
}
identity_lv8!(
    DigitsLv8(u8) digits Digits |text| crate::identity::tbcd::encode(text).ok(),
    TextLv8(u8) text_owned Text |text| Some(text.as_bytes().to_vec()),
    TextLv16(u16) text_owned Text |text| Some(text.as_bytes().to_vec())
);

impl TextLv8 {
    #[cfg(feature = "tree")]
    #[inline]
    fn text_owned(&self) -> Option<alloc::string::String> {
        self.text().map(alloc::string::String::from)
    }
}

/// Text after a length of two octets, a UTF8String: as a Flow Description, the URL and the
/// Domain Name of PFD Contents, and the Redirect Server Addresses of a Redirect Information
/// of PFCP (TS 29.244 §8.2.39, §8.2.20). Its tree is the text, or `{"octets": ...}` for octets that are
/// no UTF-8.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextLv16(pub Octets);

impl TextLv16 {
    /// The text.
    #[inline]
    pub fn text(&self) -> Option<&str> {
        core::str::from_utf8(&self.0).ok()
    }

    #[cfg(feature = "tree")]
    #[inline]
    fn text_owned(&self) -> Option<alloc::string::String> {
        self.text().map(alloc::string::String::from)
    }
}

impl From<&str> for TextLv16 {
    #[inline]
    fn from(text: &str) -> Self {
        Self(Octets::copied(text.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Number of six bits is, as a field of its own, its whole octet, which a Narrow parts
    /// into the number and the spare bits.
    #[test]
    #[inline]
    fn a_number_of_six_bits_as_a_field() {
        let mut octets: &[u8] = &[0x2e, 0xc1];
        assert_eq!(Number::<6>::read(&mut octets), Some(Number(0x2e)));
        assert_eq!(octets, [0xc1]);
        assert_eq!(Number::<6>::read(&mut &[][..]), None);
        let mut out = Vec::new();
        Number::<6>(0xc1).write(&mut out);
        assert_eq!(out, [0xc1]);
        assert_eq!(Number::<6>(0x2e).size(), 1);
        #[cfg(feature = "tree")]
        assert_eq!(
            FieldTree::to_tree(&Number::<6>(46)),
            crate::tree::unsigned(46)
        );
    }
}
