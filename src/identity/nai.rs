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

//! The SUCI in the form of a NAI, and the grammar of a NAI (RFC 7542).
//!
//! A SUPI that is a network specific identifier, a GCI or a GLI is a NAI, and its SUCI is
//! a NAI too (3GPP TS 23.003 clauses 28.7.3, 28.15.5 and 28.16.5). Concealing an identity
//! and reading a concealed one need keys, and are no part of this crate.

#[cfg(feature = "alloc")]
use super::digits::decimal;
use super::{Error, Fault};
#[cfg(feature = "alloc")]
use super::{Faults, Plmn, ProtectionScheme, RoutingIndicator, SupiType, Tolerated};
#[cfg(feature = "alloc")]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "alloc")]
use core::{fmt, str::FromStr};

/// The longest realm, in octets: the longest domain name (RFC 1035 clause 2.3.4).
const LONGEST_REALM: usize = 253;
/// The longest label of a realm, in octets (RFC 1035 clause 2.3.4).
const LONGEST_LABEL: usize = 63;
/// The longest username of a SUPI, in octets, which is also the longest plaintext of a
/// profile of ECIES.
pub(super) const LONGEST_USERNAME: usize = 253;
#[cfg(feature = "alloc")]
/// The longest output of a scheme of the home operator, in octets: 3000 and the size of
/// its input (3GPP TS 33.501 annex C.1).
const LONGEST_OPERATOR_OUTPUT: usize = 3000 + LONGEST_USERNAME;
#[cfg(feature = "alloc")]
/// The longest SUCI in the form of a NAI, in octets: the fields before the scheme output,
/// the longest output in hexadecimal digits, `@` and the longest realm. Nothing longer is
/// looked at.
const LONGEST_NAI: usize = 64 + 2 * LONGEST_OPERATOR_OUTPUT + 1 + LONGEST_REALM;
#[cfg(feature = "alloc")]
/// The octets of the ephemeral public key of the profile A of ECIES (X25519), of the
/// profile B (a compressed point of secp256r1), and of the MAC tag of both (TS 33.501
/// annex C.3.4).
pub(super) const PROFILE_A_KEY: usize = 32;
#[cfg(feature = "alloc")]
pub(super) const PROFILE_B_KEY: usize = 33;
#[cfg(feature = "alloc")]
pub(super) const MAC_TAG: usize = 8;

#[cfg(feature = "alloc")]
/// What a NAI is written as, when it is no NAI.
const A_NAI: &str = "a username, @ and a realm, as type1.rid678.schid0.useriduser17@example.com";

/// Check that `realm` is the realm of a NAI by the grammar of RFC 7542 clause 2.2, which
/// is a domain name of RFC 1035: two labels at least, each of letters, digits,
/// characters above ASCII and hyphens, with no hyphen at either end, of 63 octets at most,
/// and 253 octets in all at most.
///
/// With the feature `idna` the realm is also in Unicode normalization form C (RFC 7542
/// clause 2.4) and a domain name that IDNA takes as it is written (RFC 5891): a label
/// above ASCII is one that IDNA allows and that comes back from its ASCII form, and a
/// label that begins with `xn--` is the ASCII form of one. That feature brings the
/// Unicode tables of `idna` and of `unicode-normalization`.
///
/// ```
/// use oxirush_utils::identity::nai;
///
/// assert!(nai::check_realm("example.com").is_ok());
/// assert!(nai::check_realm("bücher.example").is_ok());
/// assert_eq!(nai::check_realm("example").unwrap_err().member, "realm");
/// assert!(nai::check_realm("-bad.example").is_err());
/// ```
///
/// # Errors
///
/// The realm is empty, longer than 253 octets, or not of that grammar.
pub fn check_realm(realm: &str) -> Result<(), Error> {
    let member = "realm";
    if realm.is_empty() || realm.len() > LONGEST_REALM {
        return Err(Error::length(
            member,
            "octets",
            realm.len(),
            1,
            LONGEST_REALM,
        ));
    }
    let rtext = |character: char| character.is_ascii_alphanumeric() || !character.is_ascii();
    let expected = "a domain name of two labels at least, of letters, digits and hyphens, \
                    with no hyphen at either end and 63 octets at most each, as example.com";
    let form = Error::new(member, Fault::Form { expected });
    let mut labels = 0;
    for label in realm.split('.') {
        labels += 1;
        if let Some(given) = label.chars().find(|&c| !rtext(c) && c != '-') {
            let expected = "letters, digits, characters above ASCII and hyphens";
            return Err(Error::new(member, Fault::Character { given, expected }));
        }
        let ends = label.starts_with(rtext) && label.ends_with(rtext);
        if label.len() > LONGEST_LABEL || !ends {
            return Err(form);
        }
    }
    if labels < 2 {
        return Err(form);
    }
    internationalized(realm)
}

/// Whether a realm of the grammar of one is normalized and is a domain name that IDNA
/// takes as it is written, whatever its case: the case of a domain name is not
/// significant (RFC 7542 clause 2.4, RFC 4343), and IDNA gives it in lower case.
#[cfg(feature = "idna")]
fn internationalized(realm: &str) -> Result<(), Error> {
    normalized("realm", realm)?;
    let expected = "a domain name that IDNA takes as it is written (RFC 5891)";
    let refused = Error::new("realm", Fault::Form { expected });
    let ascii = idna::domain_to_ascii_strict(realm).map_err(|_| refused)?;
    if realm.is_ascii() {
        return Ok(());
    }
    let (unicode, status) = idna::domain_to_unicode(&ascii);
    match status.is_ok() && unicode == realm.to_lowercase() {
        true => Ok(()),
        false => Err(refused),
    }
}

/// Without the feature `idna`, a realm is checked by its grammar alone.
#[cfg(not(feature = "idna"))]
fn internationalized(_: &str) -> Result<(), Error> {
    Ok(())
}

/// Whether a text is in Unicode normalization form C, with the feature `idna`; without
/// it no text is refused for its form.
#[cfg(feature = "idna")]
fn normalized(member: &'static str, text: &str) -> Result<(), Error> {
    use unicode_normalization::UnicodeNormalization;
    match text.nfc().eq(text.chars()) {
        true => Ok(()),
        false => {
            let expected = "a text in Unicode normalization form C (RFC 7542 clause 2.4)";
            Err(Error::new(member, Fault::Form { expected }))
        }
    }
}

#[cfg(not(feature = "idna"))]
fn normalized(_: &'static str, _: &str) -> Result<(), Error> {
    Ok(())
}

/// Check that `username` is the username of a NAI by the grammar of RFC 7542 clause 2.2:
/// atoms with a dot between two of them, none empty, of letters, digits, the characters
/// ``!#$%&'*+-/=?^_`{|}~`` and characters above ASCII that are no control character, 253
/// octets at most. With the feature `idna` it is also in Unicode normalization form C.
///
/// ```
/// use oxirush_utils::identity::nai;
///
/// assert!(nai::check_username("user.name+tag").is_ok());
/// assert!(nai::check_username("élodie").is_ok());
/// assert!(nai::check_username("a b").is_err());
/// assert!(nai::check_username("a..b").is_err());
/// ```
///
/// # Errors
///
/// The username is empty, longer than 253 octets, or not of that grammar.
pub fn check_username(username: &str) -> Result<(), Error> {
    let member = "username";
    if username.is_empty() || username.len() > LONGEST_USERNAME {
        let length = username.len();
        return Err(Error::length(member, "octets", length, 1, LONGEST_USERNAME));
    }
    for atom in username.split('.') {
        if atom.is_empty() {
            let expected = "atoms with a dot between two of them, none of them empty";
            return Err(Error::new(member, Fault::Form { expected }));
        }
        if let Some(given) = atom.chars().find(|&c| !username_character(c)) {
            let expected = "letters, digits, characters above ASCII that are no control \
                            character, and !#$%&'*+-/=?^_`{|}~";
            return Err(Error::new(member, Fault::Character { given, expected }));
        }
    }
    normalized(member, username)
}

fn username_character(character: char) -> bool {
    match character.is_ascii() {
        true => character.is_ascii_alphanumeric() || "!#$%&'*+-/=?^_`{|}~".contains(character),
        false => !character.is_control(),
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
/// The realm of the SUCI of an IMSI in the form of a NAI: the home network domain of its
/// PLMN, `5gc.mnc<MNC>.mcc<MCC>.3gppnetwork.org`, with the MNC of three digits (3GPP
/// TS 23.003 clauses 28.2 and 28.7.3).
///
/// ```
/// use oxirush_utils::identity::{Plmn, nai};
///
/// let plmn = Plmn::new("234", "15")?;
/// assert_eq!(nai::home_network_domain(plmn), "5gc.mnc015.mcc234.3gppnetwork.org");
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
pub fn home_network_domain(plmn: Plmn) -> String {
    format!(
        "5gc.mnc{:0>3}.mcc{}.3gppnetwork.org",
        plmn.mnc(),
        plmn.mcc()
    )
}

#[cfg(feature = "alloc")]
/// Whether `realm` is a home network domain, with the NID of an SNPN or without it
/// (TS 23.003 clauses 28.2 and 28.7.3): `5gc.[nid<NID>.]mnc<MNC>.mcc<MCC>.3gppnetwork.org`.
fn is_home_network_domain(realm: &str) -> bool {
    let mut labels = realm.split('.');
    if labels.next() != Some("5gc") {
        return false;
    }
    let mut label = labels.next();
    let nid = label.and_then(|label| label.strip_prefix("nid"));
    if let Some(nid) = nid {
        // Eleven hexadecimal digits, the first of which is 0 to 2 (TS 23.003 clause 12.7)
        let valid = nid.len() == 11
            && nid.bytes().all(|digit| digit.is_ascii_hexdigit())
            && matches!(nid.as_bytes()[0], b'0'..=b'2');
        if !valid {
            return false;
        }
        label = labels.next();
    }
    let three_digits = |label: Option<&str>, prefix: &str| {
        label
            .and_then(|label| label.strip_prefix(prefix))
            .is_some_and(|digits| digits.len() == 3 && digits.bytes().all(|d| d.is_ascii_digit()))
    };
    three_digits(label, "mnc")
        && three_digits(labels.next(), "mcc")
        && labels.next() == Some("3gppnetwork")
        && labels.next() == Some("org")
        && labels.next().is_none()
}

#[cfg(feature = "alloc")]
/// Whether `text` is a GLI: base64 of RFC 4648 as it is written, with its padding and no
/// other bits, of 150 octets at most, which is 200 characters at most (TS 23.003 clause
/// 28.16.4).
fn is_gli(text: &str) -> bool {
    let characters = text.as_bytes();
    if characters.is_empty() || characters.len() > 200 || !characters.len().is_multiple_of(4) {
        return false;
    }
    let padding = characters.iter().rev().take_while(|&&c| c == b'=').count();
    if padding > 2 {
        return false;
    }
    let value = |character: u8| match character {
        b'A'..=b'Z' => Some(character - b'A'),
        b'a'..=b'z' => Some(character - b'a' + 26),
        b'0'..=b'9' => Some(character - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let data = &characters[..characters.len() - padding];
    let Some(last) = data.iter().map(|&c| value(c)).try_fold(0, |_, v| v) else {
        return false;
    };
    // The bits of the last character that no octet has are zero.
    match padding {
        1 => last & 0x03 == 0,
        2 => last & 0x0f == 0,
        _ => true,
    }
}

#[cfg(feature = "alloc")]
/// Check that `clear` is what a SUPI of type `kind` has before the `@` of its NAI, or the
/// MSIN of an IMSI: the scheme output of the null scheme, and the plaintext of a profile.
fn check_clear(kind: SupiType, clear: &str) -> Result<(), Error> {
    match kind {
        // The realm does not say how many digits the MNC has: an MSIN of 10 digits at
        // most goes with any PLMN.
        SupiType::Imsi => decimal("msin", clear, 1, 10),
        // An anonymous SUCI has no username (TS 23.003 clause 28.7.3).
        SupiType::NetworkSpecific if clear.is_empty() => Ok(()),
        SupiType::NetworkSpecific | SupiType::Gci => check_username(clear),
        SupiType::Gli => match is_gli(clear) {
            true => Ok(()),
            false => {
                let expected = "the GLI in base64 of TS 23.003 clause 28.16.4: 1 to 200 \
                                characters, with their padding, for 150 octets at most";
                Err(Error::new("username", Fault::Form { expected }))
            }
        },
    }
}

#[cfg(feature = "alloc")]
/// The octets of the ephemeral public key of a profile of ECIES.
fn ephemeral_key_octets(scheme: ProtectionScheme) -> Option<usize> {
    match scheme {
        ProtectionScheme::PROFILE_A => Some(PROFILE_A_KEY),
        ProtectionScheme::PROFILE_B => Some(PROFILE_B_KEY),
        _ => None,
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
/// The SUCI of a SUPI in the form of a NAI (3GPP TS 23.003 clauses 28.7.3, 28.15.5 and
/// 28.16.5): its SUPI type, routing indicator, protection scheme, the identifier of the
/// public key of the home network, the scheme output, and the realm, which the scheme
/// leaves in clear. It is what a SUPI that is a network specific identifier, a GCI or a
/// GLI has, and what an IMSI has as a NAI, whose realm is then the home network domain.
///
/// It is the value of the parts, of the text of the NAI and of the 5GS mobile identity of
/// TS 24.501 figure 9.11.3.4.3b, which holds that text. `Display` writes the NAI, and
/// [`parse`](Self::parse) reads it.
///
/// ```
/// use oxirush_utils::identity::{NaiSuci, ProtectionScheme, SupiType};
///
/// // The example of TS 23.003 clause 28.7.3
/// let suci: NaiSuci = "type1.rid678.schid0.useriduser17@example.com".parse()?;
/// assert_eq!(suci.supi_type(), SupiType::NetworkSpecific);
/// assert_eq!(suci.routing_indicator().as_str(), "678");
/// assert_eq!(suci.protection_scheme(), ProtectionScheme::NULL);
/// assert_eq!((suci.clear(), suci.realm()), (Some("user17"), "example.com"));
/// assert_eq!(suci.to_string(), "type1.rid678.schid0.useriduser17@example.com");
///
/// // In a 5GS mobile identity: the SUPI format of a network specific identifier, then the text
/// let octets = suci.to_nas_octets()?;
/// assert_eq!(&octets[..5], [0x11, b't', b'y', b'p', b'e']);
/// assert_eq!(NaiSuci::from_nas_octets(&octets)?, suci);
/// # Ok::<(), oxirush_utils::identity::Error>(())
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiSuci {
    supi_type: SupiType,
    routing_indicator: RoutingIndicator,
    scheme: ProtectionScheme,
    key_id: u8,
    /// The characters of the identity in clear under the null scheme, and the scheme
    /// output otherwise: for a profile of ECIES the ephemeral public key, the ciphertext
    /// and the MAC tag, one after the other.
    output: Vec<u8>,
    realm: String,
}

#[cfg(feature = "alloc")]
impl NaiSuci {
    /// The SUCI of a SUPI that is not concealed: the null scheme, the key identifier 0,
    /// and `clear` as the scheme output, which is the username of the NAI, or the MSIN of
    /// an IMSI.
    ///
    /// # Errors
    ///
    /// The realm is not one of a NAI, or of the home network domain for an IMSI; `clear`
    /// is not what a SUPI of the type has: a username of RFC 7542 (which may be empty for
    /// a network specific identifier, the anonymous SUCI), a GLI in base64, or 1 to 10
    /// decimal digits of an MSIN; or the SUPI is a GCI or a GLI and the routing indicator
    /// is not `0`.
    pub fn null(
        supi_type: SupiType,
        routing_indicator: RoutingIndicator,
        clear: &str,
        realm: &str,
    ) -> Result<Self, Error> {
        let (scheme, output) = (ProtectionScheme::NULL, clear.as_bytes().to_vec());
        Self::checked(supi_type, routing_indicator, scheme, 0, output, realm)
    }

    /// The SUCI of a SUPI that a scheme conceals: the scheme, the identifier of the
    /// public key of the home network that it used, and its output. For a profile of
    /// ECIES the output is the ephemeral public key, the ciphertext and the MAC tag, one
    /// after the other (TS 33.501 annex C.3.4).
    ///
    /// # Errors
    ///
    /// `scheme` is the null scheme, `key_id` is 0 or 255, the SUPI is a GCI or a GLI,
    /// which have the null scheme alone, the realm is not one of a NAI, or the output is
    /// not one of the scheme: for a profile, the key of its profile, 1 to 253 octets of
    /// ciphertext and the tag of 8 octets; for a scheme of the operator, 1 to 3253 octets.
    pub fn concealed(
        supi_type: SupiType,
        routing_indicator: RoutingIndicator,
        scheme: ProtectionScheme,
        key_id: u8,
        output: Vec<u8>,
        realm: &str,
    ) -> Result<Self, Error> {
        if scheme.is_null() {
            let expected = "a scheme that conceals: 1, 2 or 12 to 15";
            let fault = Fault::Value { given: 0, expected };
            return Err(Error::new("protection-scheme", fault));
        }
        Self::checked(supi_type, routing_indicator, scheme, key_id, output, realm)
    }

    /// A SUCI of parts that are checked here, each against the others.
    fn checked(
        supi_type: SupiType,
        routing_indicator: RoutingIndicator,
        scheme: ProtectionScheme,
        key_id: u8,
        output: Vec<u8>,
        realm: &str,
    ) -> Result<Self, Error> {
        // The identifier is 0 if and only if the scheme is the null one (TS 23.003
        // clause 2.2B), and 255 is reserved (TS 24.501 table 9.11.3.4.1).
        let key = "home-network-public-key-identifier";
        match (scheme.is_null(), key_id) {
            (true, 0) | (false, 1..=254) => {}
            (true, given) => {
                let expected = "0 for the null scheme";
                return Err(Error::new(
                    key,
                    Fault::Value {
                        given: given.into(),
                        expected,
                    },
                ));
            }
            (false, given) => {
                let expected = "1 to 254 for a scheme that conceals";
                return Err(Error::new(
                    key,
                    Fault::Value {
                        given: given.into(),
                        expected,
                    },
                ));
            }
        }
        // A GCI and a GLI have the null scheme and the routing indicator 0 (TS 23.003
        // clauses 28.15.5 and 28.16.5).
        if matches!(supi_type, SupiType::Gci | SupiType::Gli) {
            if !scheme.is_null() {
                let expected = "0: a GCI and a GLI have the null scheme alone";
                let given = scheme.identifier().into();
                return Err(Error::new(
                    "protection-scheme",
                    Fault::Value { given, expected },
                ));
            }
            if routing_indicator.as_str() != "0" {
                let expected = "0, which a GCI and a GLI have";
                return Err(Error::new("routing-indicator", Fault::Form { expected }));
            }
        }
        check_realm(realm)?;
        if supi_type == SupiType::Imsi && !is_home_network_domain(realm) {
            let expected = "the home network domain of an IMSI, \
                            5gc.mnc<MNC>.mcc<MCC>.3gppnetwork.org, with the NID of an SNPN \
                            after 5gc or without it";
            return Err(Error::new("realm", Fault::Form { expected }));
        }
        let member = "scheme-output";
        match ephemeral_key_octets(scheme) {
            _ if scheme.is_null() => {
                let expected = "a text in UTF-8";
                let clear = core::str::from_utf8(&output)
                    .map_err(|_| Error::new(member, Fault::Form { expected }))?;
                check_clear(supi_type, clear)?;
            }
            Some(key) => {
                let (shortest, longest) = (key + 1 + MAC_TAG, key + LONGEST_USERNAME + MAC_TAG);
                if !(shortest..=longest).contains(&output.len()) {
                    let length = output.len();
                    return Err(Error::length(member, "octets", length, shortest, longest));
                }
            }
            None => {
                let (length, longest) = (output.len(), LONGEST_OPERATOR_OUTPUT);
                if !(1..=longest).contains(&length) {
                    return Err(Error::length(member, "octets", length, 1, longest));
                }
            }
        }
        Ok(Self {
            supi_type,
            routing_indicator,
            scheme,
            key_id,
            output,
            realm: realm.into(),
        })
    }

    /// The SUPI type.
    pub fn supi_type(&self) -> SupiType {
        self.supi_type
    }

    /// The routing indicator.
    pub fn routing_indicator(&self) -> RoutingIndicator {
        self.routing_indicator
    }

    /// The protection scheme.
    pub fn protection_scheme(&self) -> ProtectionScheme {
        self.scheme
    }

    /// The identifier of the public key of the home network: 0 under the null scheme.
    pub fn home_network_public_key_identifier(&self) -> u8 {
        self.key_id
    }

    /// The realm, which the scheme leaves in clear.
    pub fn realm(&self) -> &str {
        &self.realm
    }

    /// Under the null scheme, the identity in clear: the username of the NAI, empty for
    /// an anonymous SUCI, or the MSIN of an IMSI.
    pub fn clear(&self) -> Option<&str> {
        match self.scheme.is_null() {
            true => core::str::from_utf8(&self.output).ok(),
            false => None,
        }
    }

    /// The output of a scheme that conceals: the octets that the home network reads.
    pub fn concealed_output(&self) -> Option<&[u8]> {
        (!self.scheme.is_null()).then_some(&self.output[..])
    }

    /// The output of a profile of ECIES in its three parts: the ephemeral public key, the
    /// ciphertext and the MAC tag.
    pub fn ecies(&self) -> Option<(&[u8], &[u8], &[u8])> {
        let key = ephemeral_key_octets(self.scheme)?;
        let (public_key, rest) = self.output.split_at(key);
        let (ciphertext, tag) = rest.split_at(rest.len() - MAC_TAG);
        Some((public_key, ciphertext, tag))
    }

    /// The SUCI that `text` writes as a NAI: `type<SUPI type>.rid<routing indicator>.
    /// schid<scheme>.`, then `userid<identity in clear>` under the null scheme,
    /// `hnkey<key identifier>.ecckey<ephemeral public key>.cip<ciphertext>.mac<MAC tag>`
    /// under a profile of ECIES, and `hnkey<key identifier>.out<scheme output>` under a
    /// scheme of the operator, then `@` and the realm (TS 23.003 clause 28.7.3). The
    /// numbers are in decimal, and the octets in hexadecimal digits of either case.
    ///
    /// # Errors
    ///
    /// The text is longer than any SUCI, the realm is checked first and refused, a field
    /// is missing or not written as its name and its value, or the parts are refused as
    /// [`null`](Self::null) and [`concealed`](Self::concealed) refuse them.
    pub fn parse(text: &str) -> Result<Self, Error> {
        // The length first: nothing longer is looked at.
        if text.len() > LONGEST_NAI {
            return Err(Error::length("nai", "octets", text.len(), 1, LONGEST_NAI));
        }
        let (username, realm) = text
            .rsplit_once('@')
            .ok_or(Error::new("nai", Fault::Form { expected: A_NAI }))?;
        check_realm(realm)?;
        // The identity in clear of the null scheme, the last field, may have dots.
        let mut fields = username.splitn(4, '.');
        let mut field = |name: &'static str| fields.next().ok_or(missing(name));
        let supi_type = SupiType::from_type_number(number("type", field("type")?)?)?;
        let routing_indicator = field("rid")?.strip_prefix("rid").ok_or(missing("rid"))?;
        let routing_indicator = RoutingIndicator::new(routing_indicator)?;
        let scheme = ProtectionScheme::new(number("schid", field("schid")?)?)?;
        if scheme.is_null() {
            let rest = field("userid")?;
            let clear = rest.strip_prefix("userid").ok_or(missing("userid"))?;
            return Self::null(supi_type, routing_indicator, clear, realm);
        }
        let rest = field("hnkey")?;
        let (key_id, output) = match ephemeral_key_octets(scheme) {
            Some(key) => {
                let mut parts = rest.split('.');
                let key_id = number("hnkey", parts.next().unwrap_or_default())?;
                let mut output = octets("ecckey", parts.next(), key, key)?;
                output.extend(octets("cip", parts.next(), 1, LONGEST_USERNAME)?);
                output.extend(octets("mac", parts.next(), MAC_TAG, MAC_TAG)?);
                if parts.next().is_some() {
                    let expected = "the last field of a profile: mac and its 8 octets";
                    return Err(Error::new("mac", Fault::Form { expected }));
                }
                (key_id, output)
            }
            None => {
                let (key_id, output) = rest.split_once('.').unwrap_or((rest, ""));
                let key_id = number("hnkey", key_id)?;
                let output = octets("out", Some(output), 1, LONGEST_OPERATOR_OUTPUT)?;
                (key_id, output)
            }
        };
        let (kind, routing) = (supi_type, routing_indicator);
        Self::concealed(kind, routing, scheme, key_id, output, realm)
    }

    /// The octets of the SUCI as a 5GS mobile identity: the SUPI format and the type of
    /// identity of a SUCI, then the NAI in UTF-8 (3GPP TS 24.501 clause 9.11.3.4, figure
    /// 9.11.3.4.3b).
    ///
    /// # Errors
    ///
    /// The SUPI is an IMSI: a mobile identity has its PLMN, which the realm does not tell
    /// apart from another with the MNC of other digits, and has it as an [`ImsiSuci`].
    ///
    /// [`ImsiSuci`]: super::ImsiSuci
    pub fn to_nas_octets(&self) -> Result<Vec<u8>, Error> {
        if self.supi_type == SupiType::Imsi {
            let expected = "a network specific identifier, a GCI or a GLI: the mobile \
                            identity of an IMSI has its PLMN, which its realm does not say";
            return Err(Error::new("supi-type", Fault::Value { given: 0, expected }));
        }
        let text = self.to_string();
        let mut octets = Vec::with_capacity(1 + text.len());
        octets.push(self.supi_type.nas_format() << 4 | 0x01);
        octets.extend_from_slice(text.as_bytes());
        Ok(octets)
    }

    /// The SUCI that `octets` hold as a 5GS mobile identity.
    ///
    /// # Errors
    ///
    /// The first octet is not that of a SUCI with the SUPI format of a NAI (1 to 3), or
    /// has a spare bit set, the text is longer than any SUCI or is not UTF-8, it is not a
    /// SUCI as [`parse`](Self::parse) reads one, or its SUPI type is not that of the SUPI
    /// format.
    pub fn from_nas_octets(octets: &[u8]) -> Result<Self, Error> {
        let (suci, faults) = Self::from_nas_octets_tolerant(octets)?;
        match faults.first() {
            Some(fault) => Err(*fault),
            None => Ok(suci),
        }
    }

    /// [`from_nas_octets`](Self::from_nas_octets), for octets as other implementations
    /// send them: the value, and what a sender by the clause would have written
    /// otherwise. A spare bit of the first octet that is 1 is read, as a fault (3GPP
    /// TS 24.007 clause 11.1.4: a receiver accepts a spare bit as 0 or as 1).
    ///
    /// # Errors
    ///
    /// Octets that are no SUCI in the form of a NAI however they are read.
    pub fn from_nas_octets_tolerant(octets: &[u8]) -> Result<Tolerated<Self>, Error> {
        let member = "suci";
        let Some((&first, text)) = octets.split_first() else {
            return Err(Error::length(member, "octets", 0, 2, usize::MAX));
        };
        let (format, kind) = ((first >> 4) & 0x07, first & 0x07);
        if kind != 1 {
            let fault = Fault::Octet {
                given: first,
                expected: first & 0xf0 | 0x01,
            };
            return Err(Error::new(member, fault));
        }
        if !matches!(format, 1..=3) {
            let expected = "1, 2 or 3: the SUPI format of a NAI";
            let fault = Fault::Value {
                given: format.into(),
                expected,
            };
            return Err(Error::new("supi-format", fault));
        }
        let mut faults = Faults::new();
        if first & 0x88 != 0 {
            faults.push(Error::new(member, Fault::Spare { given: first }));
        }
        if text.len() > LONGEST_NAI {
            return Err(Error::length("nai", "octets", text.len(), 1, LONGEST_NAI));
        }
        let expected = "a text in UTF-8";
        let text =
            core::str::from_utf8(text).map_err(|_| Error::new("nai", Fault::Form { expected }))?;
        let suci = Self::parse(text)?;
        if suci.supi_type.nas_format() != format {
            let expected = "the SUPI type that the SUPI format of the mobile identity says";
            let given = suci.supi_type.type_number().into();
            return Err(Error::new("supi-type", Fault::Value { given, expected }));
        }
        Ok((suci, faults))
    }
}

#[cfg(feature = "alloc")]
/// The error of a field that is missing, or that does not begin with its name.
fn missing(field: &'static str) -> Error {
    let expected = match field {
        "type" => "type and the SUPI type in decimal, as type1",
        "rid" => "rid and the routing indicator, as rid678",
        "schid" => "schid and the protection scheme in decimal, as schid0",
        "userid" => "userid and the identity in clear, as useriduser17",
        "hnkey" => "hnkey and the key identifier in decimal, as hnkey27",
        "ecckey" => "ecckey and the ephemeral public key in hexadecimal digits",
        "cip" => "cip and the ciphertext in hexadecimal digits",
        "mac" => "mac and the 8 octets of the MAC tag in hexadecimal digits",
        _ => "out and the scheme output in hexadecimal digits",
    };
    Error::new(field, Fault::Form { expected })
}

#[cfg(feature = "alloc")]
/// The decimal number of a field after its name: 1 to 3 digits of a number below 256,
/// without a zero before it, so that a number is written in one way.
fn number(field: &'static str, text: &str) -> Result<u8, Error> {
    let digits = text.strip_prefix(field).ok_or(missing(field))?;
    let decimal = (1..=3).contains(&digits.len())
        && digits.bytes().all(|d| d.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'));
    match decimal {
        true => digits.parse().map_err(|_| missing(field)),
        false => Err(missing(field)),
    }
}

#[cfg(feature = "alloc")]
/// The octets that a field writes in hexadecimal digits after its name: `fewest` to
/// `most` of them.
fn octets(
    field: &'static str,
    text: Option<&str>,
    fewest: usize,
    most: usize,
) -> Result<Vec<u8>, Error> {
    let digits = text
        .and_then(|text| text.strip_prefix(field))
        .ok_or(missing(field))?;
    if let Some(given) = digits.chars().find(|digit| !digit.is_ascii_hexdigit()) {
        let expected = "hexadecimal digits";
        return Err(Error::new(field, Fault::Character { given, expected }));
    }
    if !digits.len().is_multiple_of(2) || !(fewest..=most).contains(&(digits.len() / 2)) {
        let expected = match field {
            "ecckey" => "the octets of the ephemeral public key of its profile",
            "cip" => "1 to 253 octets",
            "mac" => "8 octets",
            _ => "1 to 3253 octets",
        };
        return Err(Error::new(field, Fault::Form { expected }));
    }
    let half = |digit: u8| (digit as char).to_digit(16).unwrap_or_default() as u8;
    let (pairs, _) = digits.as_bytes().as_chunks::<2>();
    Ok(pairs
        .iter()
        .map(|[high, low]| half(*high) << 4 | half(*low))
        .collect())
}

#[cfg(feature = "alloc")]
/// Octets in lower-case hexadecimal digits.
fn hexadecimal(f: &mut fmt::Formatter<'_>, octets: &[u8]) -> fmt::Result {
    octets.iter().try_for_each(|octet| write!(f, "{octet:02x}"))
}

#[cfg(feature = "alloc")]
impl fmt::Display for NaiSuci {
    /// The NAI, as [`parse`](NaiSuci::parse) reads it: the octets in lower-case
    /// hexadecimal digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, routing, scheme) = (
            self.supi_type.type_number(),
            self.routing_indicator,
            self.scheme.identifier(),
        );
        write!(f, "type{kind}.rid{routing}.schid{scheme}.")?;
        let key = self.key_id;
        if let Some(clear) = self.clear() {
            write!(f, "userid{clear}")?;
        } else if let Some((public_key, ciphertext, tag)) = self.ecies() {
            write!(f, "hnkey{key}.ecckey")?;
            hexadecimal(f, public_key)?;
            f.write_str(".cip")?;
            hexadecimal(f, ciphertext)?;
            f.write_str(".mac")?;
            hexadecimal(f, tag)?;
        } else {
            write!(f, "hnkey{key}.out")?;
            hexadecimal(f, &self.output)?;
        }
        write!(f, "@{}", self.realm)
    }
}

#[cfg(feature = "alloc")]
impl fmt::Debug for NaiSuci {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NaiSuci({self})")
    }
}

#[cfg(feature = "alloc")]
impl FromStr for NaiSuci {
    type Err = Error;

    /// [`NaiSuci::parse`].
    fn from_str(text: &str) -> Result<Self, Error> {
        Self::parse(text)
    }
}
