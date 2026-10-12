//! The identities against their clauses, against what other implementations sent, and
//! against themselves: every value encodes to the octets that it is read from.
//!
//! The octets marked "captured" are from lab runs of 2026-10-12 (Open5GS v2.8.0, free5GC
//! v4.2.3 and PacketRusher 20250225, with the test subscribers 208-93 0000000001 to
//! 0000000008): each implementation sent them in a NAS message.

use oxirush_utils::identity::{
    Error, Fault, Guti5g, GutiEps, IdentityType, Imei, Imeisv, Imsi, MobileIdentityType5gs,
    MobileIdentityTypeEps, Plmn, STmsi5g, STmsiEps, Tai5gs, TaiEps, digits, tbcd,
};
use std::collections::{BTreeSet, HashMap};

/// Numbers that look random, the same in every run.
struct Numbers(u64);

impl Numbers {
    fn next(&mut self) -> u64 {
        // SplitMix64.
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn digits(&mut self, count: usize) -> String {
        (0..count)
            .map(|_| char::from(b'0' + (self.next() % 10) as u8))
            .collect()
    }
}

fn plmn(text: &str) -> Plmn {
    text.parse().unwrap()
}

// ── PLMN ────────────────────────────────────────────────────────────────────

/// TS 24.008 clause 10.5.1.3: octet 2 has the third digit of the MNC beside the third of
/// the MCC, and `1111` there for an MNC of two digits. TS 38.413 clause 9.3.3.5 and
/// TS 36.413 clause 9.2.3.8: the digits in their order, a filler before an MNC of two.
#[test]
fn a_plmn_has_the_octets_of_each_clause() {
    for (text, nas, ap) in [
        ("208-93", [0x02, 0xf8, 0x39], [0x02, 0xf8, 0x39]),
        ("001-01", [0x00, 0xf1, 0x10], [0x00, 0xf1, 0x10]),
        ("310-410", [0x13, 0x00, 0x14], [0x13, 0x40, 0x01]),
        ("999-123", [0x99, 0x39, 0x21], [0x99, 0x19, 0x32]),
        ("208-093", [0x02, 0x38, 0x90], [0x02, 0x08, 0x39]),
    ] {
        let plmn = plmn(text);
        assert_eq!(plmn.to_nas_octets(), nas, "{text} in NAS");
        assert_eq!(plmn.to_ap_octets(), ap, "{text} in NGAP and S1AP");
        assert_eq!(Plmn::from_nas_octets(nas), Ok(plmn), "{text}");
        assert_eq!(Plmn::from_ap_octets(ap), Ok(plmn), "{text}");
        assert_eq!(plmn.to_string(), text);
        assert_eq!(format!("{plmn:?}"), format!("Plmn({text})"));
    }
}

/// The two orders are not each other's for an MNC of three digits: what one writes, the
/// other reads as another PLMN. tshark 4.6.4 reads `13 40 01` in an NGAP and in an S1AP
/// PLMN identity as 310-410, and `13 00 14` as 310-041.
#[test]
fn an_mnc_of_three_digits_is_read_in_the_order_it_was_written_in() {
    let written = plmn("310-410");
    let in_the_other_order = Plmn::from_ap_octets(written.to_nas_octets()).unwrap();
    assert_eq!(in_the_other_order, plmn("310-041"));
    let in_the_other_order = Plmn::from_nas_octets(written.to_ap_octets()).unwrap();
    assert_eq!(in_the_other_order, plmn("310-104"));
    // An MNC of two digits has the same octets in both.
    let two = plmn("208-93");
    assert_eq!(two.to_nas_octets(), two.to_ap_octets());
}

#[test]
fn every_plmn_is_read_from_the_octets_that_it_writes_in_both_orders() {
    let mut count = 0;
    for mcc in 0..1000 {
        for (mnc, width) in (0..100)
            .map(|mnc| (mnc, 2))
            .chain((0..1000).map(|mnc| (mnc, 3)))
        {
            let (mcc, mnc) = (format!("{mcc:03}"), format!("{mnc:0width$}"));
            let plmn = Plmn::new(&mcc, &mnc).unwrap();
            assert_eq!((plmn.mcc(), plmn.mnc()), (mcc.as_str(), mnc.as_str()));
            assert_eq!(Plmn::from_nas_octets(plmn.to_nas_octets()), Ok(plmn));
            assert_eq!(Plmn::from_ap_octets(plmn.to_ap_octets()), Ok(plmn));
            count += 1;
        }
    }
    assert_eq!(count, 1_100_000);
}

/// Each three octets are a PLMN that writes them back, or no PLMN: there is no third
/// kind, in either order.
#[test]
fn any_three_octets_are_a_plmn_that_writes_them_back_or_none() {
    let mut numbers = Numbers(1);
    let (mut read, mut refused) = (0, 0);
    for _ in 0..200_000 {
        let [a, b, c, ..] = numbers.next().to_be_bytes();
        // Half of the octets are of digits, so that many are PLMNs.
        let octets = match numbers.next() % 2 {
            0 => [a, b, c],
            _ => [
                (a % 10) << 4 | (b % 10),
                0xf0 | (c % 10),
                (a % 7) << 4 | (b % 9),
            ],
        };
        for (from, to) in [
            (
                Plmn::from_nas_octets as fn(_) -> _,
                Plmn::to_nas_octets as fn(_) -> _,
            ),
            (Plmn::from_ap_octets, Plmn::to_ap_octets),
        ] {
            match from(octets) {
                Ok(plmn) => {
                    assert_eq!(to(plmn), octets);
                    read += 1;
                }
                Err(_) => refused += 1,
            }
        }
    }
    assert!(
        read > 50_000 && refused > 50_000,
        "{read} read, {refused} refused"
    );
}

#[test]
fn a_plmn_is_refused_with_its_member_and_the_reason() {
    let error = |result: Result<Plmn, Error>| result.unwrap_err().to_string();
    assert_eq!(
        error(Plmn::new("20", "93")),
        "`mcc` has 2 digits, and has 3"
    );
    assert_eq!(
        error(Plmn::new("208", "9")),
        "`mnc` has 1 digit, and has 2 or 3"
    );
    assert_eq!(
        error(Plmn::new("208", "9345")),
        "`mnc` has 4 digits, and has 2 or 3"
    );
    assert_eq!(
        error(Plmn::new("2o8", "93")),
        "`mcc` has the character `o`, and has decimal digits"
    );
    assert_eq!(
        error("20893".parse()),
        "`plmn` is written as an MCC of three digits, a hyphen and an MNC of two or three, as \
         208-93"
    );
    assert_eq!(
        error(Plmn::from_digits([2, 0, 8], &[9, 10])),
        "`mnc` has the half octet 0xa where a digit is, and a digit is 0 to 9"
    );
    // A digit above 9, and a filler where the clause has none.
    for octets in [
        [0x0a, 0xf8, 0x39],
        [0x02, 0xfb, 0x39],
        [0x02, 0xf8, 0xf9],
        [0x02, 0xf8, 0x3f],
    ] {
        assert!(Plmn::from_nas_octets(octets).is_err(), "{octets:02x?}");
        assert!(Plmn::from_ap_octets(octets).is_err(), "{octets:02x?}");
    }
    // The filler of one order is no filler of the other, with three digits beside it.
    assert!(Plmn::from_nas_octets([0x13, 0x40, 0xf1]).is_err());
    assert_eq!(
        Plmn::from_nas_octets([0xff, 0xff, 0xff]).unwrap_err(),
        Plmn::from_digits([15, 15, 15], &[15, 15]).unwrap_err()
    );
}

/// TS 23.003 clause 2.2: the MNC has two or three digits, and which it has is part of
/// what the PLMN is.
#[test]
fn the_length_of_the_mnc_is_part_of_the_value() {
    let (two, three) = (plmn("208-93"), plmn("208-093"));
    assert_ne!(two, three);
    assert_ne!(two.to_nas_octets(), three.to_nas_octets());
    assert_eq!((two.digits(), three.digits()), ("20893", "208093"));
    // A value that keys a map and that sorts.
    let served: HashMap<Plmn, &str> = [(two, "home"), (three, "other")].into();
    assert_eq!(served[&plmn("208-93")], "home");
    let sorted: BTreeSet<Plmn> = [plmn("310-410"), three, two, plmn("001-01")].into();
    let sorted: Vec<String> = sorted.iter().map(Plmn::to_string).collect();
    assert_eq!(sorted, ["001-01", "208-093", "208-93", "310-410"]);
}

// ── Digits ──────────────────────────────────────────────────────────────────

#[test]
fn a_tbcd_string_has_two_characters_in_an_octet_and_a_filler_at_its_end_alone() {
    for (text, octets) in [
        ("", &[][..]),
        ("1", &[0xf1]),
        ("12", &[0x21]),
        ("12345", &[0x21, 0x43, 0xf5]),
        ("0000000001", &[0x00, 0x00, 0x00, 0x00, 0x10]),
        ("*#abc", &[0xba, 0xdc, 0xfe]),
    ] {
        assert_eq!(tbcd::encode(text).unwrap(), octets, "{text}");
        assert_eq!(tbcd::decode(octets).unwrap(), text, "{octets:02x?}");
    }
    for octets in [&[0xf1, 0x32][..], &[0x1f], &[0xff], &[0x21, 0xff]] {
        assert!(tbcd::decode(octets).is_err(), "{octets:02x?}");
    }
    let error = tbcd::encode("12d").unwrap_err().to_string();
    assert_eq!(
        error,
        "`tbcd` has the character `d`, and has digits, *, #, a, b and c"
    );
}

/// TS 24.008 clause 10.5.1.4: the first digit beside the odd/even indication and the type
/// of identity, then two digits an octet, and the end mark `1111` after an even number.
#[test]
fn identity_digits_are_coded_as_the_mobile_identity_of_ts_24_008_has_them() {
    for (kind, text, octets) in [
        (
            1,
            "208930000000001",
            &[0x29, 0x80, 0x39, 0x00, 0x00, 0x00, 0x00, 0x10][..],
        ),
        (
            3,
            "490154203237510",
            &[0x4b, 0x09, 0x51, 0x24, 0x30, 0x32, 0x57, 0x01],
        ),
        (
            5,
            "4901542032375101",
            &[0x45, 0x09, 0x51, 0x24, 0x30, 0x32, 0x57, 0x01, 0xf1],
        ),
        (1, "1", &[0x19]),
        (2, "12", &[0x12, 0xf2]),
    ] {
        assert_eq!(digits::encode(kind, text).unwrap(), octets, "{text}");
        assert_eq!(digits::decode(octets).unwrap(), (kind, text.to_string()));
    }
    let error = |octets: &[u8]| digits::decode(octets).unwrap_err().to_string();
    assert_eq!(
        error(&[]),
        "`identity-digits` has 0 octets, and has 1 at least"
    );
    assert!(error(&[0xa9]).contains("half octet 0xa where a digit is"));
    // An even number of digits has its end mark, and an odd number has a digit there.
    assert!(error(&[0x12, 0x32]).contains("the half octet 0x3 where its end mark is"));
    assert!(error(&[0x1a, 0xf2]).contains("the half octet 0xf where a digit is"));
    // One octet has one digit, which is no even number.
    assert!(error(&[0x11]).contains("has 1 octet"));
    assert!(digits::encode(8, "12").is_err());
    assert!(digits::encode(1, "").is_err());
    assert!(digits::encode(1, "12a").is_err());
}

#[test]
fn any_digits_are_read_from_the_octets_that_they_write() {
    let mut numbers = Numbers(2);
    for _ in 0..20_000 {
        let count = 1 + (numbers.next() % 20) as usize;
        let (kind, text) = ((numbers.next() % 8) as u8, numbers.digits(count));
        let octets = digits::encode(kind, &text).unwrap();
        assert_eq!(octets.len(), 1 + count / 2);
        assert_eq!(digits::decode(&octets).unwrap(), (kind, text.clone()));
        assert_eq!(tbcd::decode(&tbcd::encode(&text).unwrap()).unwrap(), text);
    }
    // And any octets are digits that write them back, or none.
    let (mut read, mut refused) = (0, 0);
    for _ in 0..50_000 {
        let count = 1 + (numbers.next() % 6) as usize;
        let octets: Vec<u8> = (0..count)
            .map(|_| match numbers.next() % 4 {
                0 => numbers.next() as u8,
                _ => ((numbers.next() % 10) << 4 | (numbers.next() % 10)) as u8,
            })
            .collect();
        match digits::decode(&octets) {
            Ok((kind, text)) => {
                assert_eq!(digits::encode(kind, &text).unwrap(), octets);
                read += 1;
            }
            Err(_) => refused += 1,
        }
    }
    assert!(
        read > 5_000 && refused > 5_000,
        "{read} read, {refused} refused"
    );
}

#[test]
fn an_imsi_has_its_plmn_and_its_msin_by_the_length_of_the_mnc() {
    let imsi = Imsi::new("208930000000001").unwrap();
    assert_eq!(imsi.plmn(2).unwrap(), plmn("208-93"));
    assert_eq!(imsi.msin(2).unwrap(), "0000000001");
    assert_eq!(imsi.plmn(3).unwrap(), plmn("208-930"));
    assert_eq!(imsi.msin(3).unwrap(), "000000001");
    assert!(imsi.plmn(4).is_err() && imsi.msin(1).is_err());
    assert_eq!(
        (imsi.to_string(), format!("{imsi:?}")),
        ("208930000000001".into(), "Imsi(208930000000001)".into())
    );
    // TS 23.003 clause 2.3: 15 digits at most, and an MCC, an MNC and one digit at least.
    assert_eq!(Imsi::new("208931").unwrap().msin(2).unwrap(), "1");
    let error = |digits: &str| Imsi::new(digits).unwrap_err().to_string();
    assert_eq!(error("20893"), "`imsi` has 5 digits, and has 6 to 15");
    assert_eq!(
        error("2089300000000012"),
        "`imsi` has 16 digits, and has 6 to 15"
    );
    assert!(error("20893000000000x").contains("the character `x`"));
    // The type of identity of an IMSI is 1 in TS 24.008 and in TS 24.301.
    let octets = imsi
        .to_nas_octets(MobileIdentityTypeEps::Imsi.bits())
        .unwrap();
    assert_eq!(octets, [0x29, 0x80, 0x39, 0x00, 0x00, 0x00, 0x00, 0x10]);
    assert_eq!(Imsi::from_nas_octets(1, &octets), Ok(imsi));
    // Another type of identity is not this one.
    let other = Imsi::from_nas_octets(3, &octets).unwrap_err();
    assert_eq!(
        other.to_string(),
        "`imsi` has the octet 0x29, and has 0x2b there"
    );
}

/// TS 23.003 clause 6.2.1 and annex B. 490154203237518 is the example of a check digit
/// that the descriptions of the Luhn formula for an IMEI give.
#[test]
fn an_imei_has_a_spare_digit_or_its_check_digit() {
    let sent = Imei::with_spare_digit("49015420323751").unwrap();
    let checked = Imei::with_check_digit("49015420323751").unwrap();
    assert_eq!(
        (sent.as_str(), checked.as_str()),
        ("490154203237510", "490154203237518")
    );
    assert!(checked.has_check_digit() && !sent.has_check_digit());
    assert_eq!((sent.tac(), sent.snr()), ("49015420", "323751"));
    assert_eq!(Imei::new("490154203237518"), Ok(checked));
    assert!(
        Imei::new("49015420323751").is_err() && Imei::with_check_digit("4901542032375").is_err()
    );
    // Each digit that the check digit protects changes it.
    for position in 0..14 {
        let mut other = *b"49015420323751";
        other[position] = b'0' + (other[position] - b'0' + 1) % 10;
        let other = Imei::with_check_digit(std::str::from_utf8(&other).unwrap()).unwrap();
        assert_ne!(other.as_str().as_bytes()[14], b'8', "{other}");
    }
    let imeisv = Imeisv::new("4901542032375101").unwrap();
    assert_eq!(
        (imeisv.tac(), imeisv.snr(), imeisv.svn()),
        ("49015420", "323751", "01")
    );
    assert_eq!(
        format!("{imeisv} {imeisv:?}"),
        "4901542032375101 Imeisv(4901542032375101)"
    );
    assert!(Imeisv::new("490154203237510").is_err());
}

/// Captured: PacketRusher 20250225 sends this IMEISV in its SECURITY MODE COMPLETE, with
/// `0000` where TS 24.008 clause 10.5.1.4 has the end mark `1111`. Open5GS, free5GC and
/// pycrate read it.
#[test]
fn an_imeisv_with_another_end_mark_is_read_with_its_fault_and_is_not_canonical() {
    let sent = [0x15, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let kind = MobileIdentityType5gs::Imeisv.bits();
    let fault = Imeisv::from_nas_octets(kind, &sent).unwrap_err();
    assert_eq!(
        (fault.member, fault.fault),
        ("imeisv", Fault::EndMark { given: 0 })
    );
    let (imeisv, faults) = Imeisv::from_nas_octets_tolerant(kind, &sent).unwrap();
    assert_eq!(imeisv.as_str(), "1110000000000000");
    assert_eq!(faults, [fault]);
    assert_eq!(
        fault.to_string(),
        "`imeisv` has the half octet 0x0 where its end mark is, and the end mark is 0xf"
    );
    // What the value writes is the canonical form, which has no fault.
    let canonical = imeisv.to_nas_octets(kind).unwrap();
    assert_eq!(
        canonical,
        [0x15, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0]
    );
    let read = Imeisv::from_nas_octets_tolerant(kind, &canonical).unwrap();
    assert_eq!((read.0, read.1.len()), (imeisv, 0));
    // Tolerance takes that half octet alone: no digit is made up.
    let mut no_digit = sent;
    no_digit[3] = 0x0a;
    assert!(Imeisv::from_nas_octets_tolerant(kind, &no_digit).is_err());
    assert!(Imeisv::from_nas_octets_tolerant(kind, &sent[..8]).is_err());
    assert!(Imeisv::from_nas_octets_tolerant(3, &sent).is_err());
}

// ── GUTI and S-TMSI ─────────────────────────────────────────────────────────

/// Captured: each implementation sent these as a 5GS mobile identity (TS 24.501 clause
/// 9.11.3.4) or as an EPS mobile identity (TS 24.301 clause 9.9.3.12).
#[test]
fn the_gutis_that_other_implementations_sent_are_read_and_written_back() {
    for (source, octets, region, set, pointer, tmsi) in [
        (
            "free5GC AMF, REGISTRATION ACCEPT",
            "f202f839cafe0000000001",
            0xca,
            0x3f8,
            0,
            1,
        ),
        (
            "free5GC AMF, REGISTRATION ACCEPT",
            "f202f839cafe000000000e",
            0xca,
            0x3f8,
            0,
            14,
        ),
        (
            "Open5GS AMF, REGISTRATION ACCEPT",
            "f202f839020040c00006cf",
            2,
            1,
            0,
            0xc000_06cf,
        ),
        (
            "Open5GS AMF, CONFIGURATION UPDATE COMMAND",
            "f202f839020040c0000363",
            2,
            1,
            0,
            0xc000_0363,
        ),
        (
            "PacketRusher, DEREGISTRATION REQUEST",
            "f202f839010040fc982b93",
            1,
            1,
            0,
            0xfc98_2b93,
        ),
    ] {
        let octets = hex::decode(octets).unwrap();
        let guti = Guti5g::from_nas_octets(&octets).unwrap();
        assert_eq!(guti.plmn(), plmn("208-93"), "{source}");
        let parts = (
            guti.amf_region_id(),
            guti.amf_set_id(),
            guti.amf_pointer(),
            guti.tmsi(),
        );
        assert_eq!(parts, (region, set, pointer, tmsi), "{source}");
        assert_eq!(guti.to_nas_octets()[..], octets[..], "{source}");
        assert_eq!(guti.to_string().parse(), Ok(guti), "{source}");
        let (tolerant, faults) = Guti5g::from_nas_octets_tolerant(&octets).unwrap();
        assert!(tolerant == guti && faults.is_empty(), "{source}");
    }
    for (octets, tmsi) in [
        ("f602f839000201c00004f4", 0xc000_04f4),
        ("f602f839000201c0000536", 0xc000_0536),
    ] {
        // Open5GS MME, ATTACH ACCEPT.
        let octets = hex::decode(octets).unwrap();
        let guti = GutiEps::from_nas_octets(&octets).unwrap();
        let parts = (
            guti.plmn(),
            guti.mme_group_id(),
            guti.mme_code(),
            guti.m_tmsi(),
        );
        assert_eq!(parts, (plmn("208-93"), 2, 1, tmsi));
        assert_eq!(guti.to_nas_octets()[..], octets[..]);
        assert_eq!(guti.to_string().parse(), Ok(guti));
        assert_eq!(guti.s_tmsi(), STmsiEps::new(1, tmsi));
    }
}

#[test]
fn a_guti_and_an_s_tmsi_have_the_octets_and_the_text_of_their_parts() {
    let guti = Guti5g::new(plmn("310-410"), 0xca, 0x3fe, 0x01, 0xdead_beef).unwrap();
    let octets = [
        0xf2, 0x13, 0x00, 0x14, 0xca, 0xff, 0x81, 0xde, 0xad, 0xbe, 0xef,
    ];
    assert_eq!(guti.to_nas_octets(), octets);
    assert_eq!(Guti5g::from_nas_octets(&octets), Ok(guti));
    assert_eq!(guti.to_string(), "5g-guti-310410caff81deadbeef");
    assert_eq!(format!("{guti:?}"), "Guti5g(5g-guti-310410caff81deadbeef)");
    let s_tmsi = guti.s_tmsi();
    assert_eq!(s_tmsi, STmsi5g::new(0x3fe, 0x01, 0xdead_beef).unwrap());
    assert_eq!(
        s_tmsi.to_nas_octets(),
        [0xf4, 0xff, 0x81, 0xde, 0xad, 0xbe, 0xef]
    );
    assert_eq!(
        STmsi5g::from_nas_octets(&s_tmsi.to_nas_octets()),
        Ok(s_tmsi)
    );
    assert_eq!(s_tmsi.to_u64(), 0xff81_dead_beef);
    assert_eq!(STmsi5g::from_u64(0xff81_dead_beef), Ok(s_tmsi));
    assert_eq!(
        format!("{s_tmsi} {s_tmsi:?}"),
        "5g-s-tmsi-ff81deadbeef STmsi5g(5g-s-tmsi-ff81deadbeef)"
    );
    assert_eq!(Guti5g::of(plmn("310-410"), 0xca, s_tmsi), guti);

    let error = |result: Result<Guti5g, Error>| result.unwrap_err().to_string();
    let new = |set, pointer| Guti5g::new(plmn("208-93"), 0, set, pointer, 0);
    assert_eq!(
        error(new(1024, 0)),
        "`amf-set-id` is 1024, and is 1023 at most"
    );
    assert_eq!(error(new(0, 64)), "`amf-pointer` is 64, and is 63 at most");
    assert_eq!(
        error(Guti5g::from_nas_octets(&octets[..10])),
        "`5g-guti` has 10 octets, and has 11"
    );
    assert!(STmsi5g::from_u64(1 << 48).is_err());
    for text in [
        "5g-guti-20893",
        "4g-guti-20893caff81deadbeef",
        "5g-guti-20893caff81deadbeeg",
        "5g-guti-2089Xcaff81deadbeef",
    ] {
        assert!(text.parse::<Guti5g>().is_err(), "{text}");
    }
    let eps = GutiEps::new(plmn("310-410"), 0x8001, 0x02, 7);
    assert_eq!(
        eps.to_nas_octets(),
        [0xf6, 0x13, 0x00, 0x14, 0x80, 0x01, 0x02, 0, 0, 0, 7]
    );
    assert_eq!(
        format!("{eps} {eps:?}"),
        "4g-guti-31041080010200000007 GutiEps(4g-guti-31041080010200000007)"
    );
    assert_eq!(format!("{:?}", eps.s_tmsi()), "STmsiEps(s-tmsi-0200000007)");
}

/// TS 24.501 figures 9.11.3.4.1 and 9.11.3.4.5: bits 8 to 5 of the first octet are 1111
/// and bit 4 is spare. TS 24.007 clause 11.1.4: a spare bit is sent as 0 and "can be
/// accepted as a 0 or a 1 by the receiver".
#[test]
fn a_spare_bit_is_read_with_its_fault_and_is_not_canonical() {
    let spare = hex::decode("fa02f839cafe0000000001").unwrap();
    let fault = Guti5g::from_nas_octets(&spare).unwrap_err();
    assert_eq!(
        (fault.member, fault.fault),
        ("5g-guti", Fault::Spare { given: 0xfa })
    );
    let (guti, faults) = Guti5g::from_nas_octets_tolerant(&spare).unwrap();
    assert_eq!(faults, [fault]);
    assert_eq!(guti.to_nas_octets()[0], 0xf2);
    assert_eq!(guti.to_nas_octets()[1..], spare[1..]);
    assert_eq!(
        faults[0].to_string(),
        "`5g-guti` has a spare bit set in the octet 0xfa, and a spare bit is 0"
    );
    let (s_tmsi, faults) = STmsi5g::from_nas_octets_tolerant(&[0xfc, 0, 0x40, 0, 0, 0, 1]).unwrap();
    assert_eq!(
        (s_tmsi.amf_set_id(), s_tmsi.tmsi(), faults.len()),
        (1, 1, 1)
    );
    assert!(STmsi5g::from_nas_octets(&[0xfc, 0, 0x40, 0, 0, 0, 1]).is_err());
    // The other bits of the octet are the identity: no tolerance reads another one.
    for first in [0xe2, 0x02, 0xf4, 0xf6, 0xf3] {
        let mut other = spare.clone();
        other[0] = first;
        let error = Guti5g::from_nas_octets_tolerant(&other).unwrap_err();
        assert_eq!(
            error.fault,
            Fault::Octet {
                given: first,
                expected: 0xf2
            }
        );
    }
    // Nor a PLMN that is none, nor another length.
    let mut no_plmn = spare.clone();
    no_plmn[1] = 0xaa;
    assert!(Guti5g::from_nas_octets_tolerant(&no_plmn).is_err());
    assert!(Guti5g::from_nas_octets_tolerant(&spare[..10]).is_err());
    // The GUTI of EPS has the odd/even indication there, which is 0 for a GUTI.
    let eps = hex::decode("fe02f839000201c00004f4").unwrap();
    let error = GutiEps::from_nas_octets(&eps).unwrap_err();
    assert_eq!(
        error.to_string(),
        "`guti` has the octet 0xfe, and has 0xf6 there"
    );
}

#[test]
fn any_guti_is_read_from_what_it_writes() {
    let mut numbers = Numbers(3);
    for _ in 0..20_000 {
        let width = 2 + (numbers.next() % 2) as usize;
        let plmn = Plmn::new(&numbers.digits(3), &numbers.digits(width)).unwrap();
        let next = numbers.next();
        let (region, set, pointer) = (
            next as u8,
            (next >> 8) as u16 & 0x3ff,
            (next >> 24) as u8 & 0x3f,
        );
        let guti = Guti5g::new(plmn, region, set, pointer, (next >> 32) as u32).unwrap();
        assert_eq!(Guti5g::from_nas_octets(&guti.to_nas_octets()), Ok(guti));
        assert_eq!(guti.to_string().parse(), Ok(guti));
        assert_eq!(STmsi5g::from_u64(guti.s_tmsi().to_u64()), Ok(guti.s_tmsi()));
        let eps = GutiEps::new(plmn, next as u16, (next >> 16) as u8, (next >> 32) as u32);
        assert_eq!(GutiEps::from_nas_octets(&eps.to_nas_octets()), Ok(eps));
        assert_eq!(eps.to_string().parse(), Ok(eps));
        let tai = Tai5gs::new(plmn, next as u32 & 0xff_ffff).unwrap();
        assert_eq!(Tai5gs::from_nas_octets(tai.to_nas_octets()), Ok(tai));
        let tai = TaiEps::new(plmn, next as u16);
        assert_eq!(TaiEps::from_nas_octets(tai.to_nas_octets()), Ok(tai));
    }
}

// ── TAI, and the types of identity ──────────────────────────────────────────

#[test]
fn a_tracking_area_identity_is_a_plmn_and_a_code() {
    let tai = Tai5gs::new(plmn("310-410"), 0x00_1234).unwrap();
    assert_eq!(tai.to_nas_octets(), [0x13, 0x00, 0x14, 0x00, 0x12, 0x34]);
    assert_eq!(
        format!("{tai} {tai:?}"),
        "310-410-001234 Tai5gs(310-410-001234)"
    );
    assert_eq!((tai.plmn(), tai.tac()), (plmn("310-410"), 0x1234));
    let error = Tai5gs::new(plmn("208-93"), 1 << 24)
        .unwrap_err()
        .to_string();
    assert_eq!(error, "`tac` is 16777216, and is 16777215 at most");
    // TS 24.501 clause 9.11.3.8: an MCC or an MNC that is no digits is a deleted
    // identity, which is no value here.
    assert!(Tai5gs::from_nas_octets([0xff, 0xff, 0xff, 0, 0, 1]).is_err());
    let eps = TaiEps::new(plmn("208-93"), 1);
    assert_eq!(eps.to_nas_octets(), [0x02, 0xf8, 0x39, 0x00, 0x01]);
    assert_eq!(format!("{eps} {eps:?}"), "208-93-0001 TaiEps(208-93-0001)");
    assert!(TaiEps::from_nas_octets([0x02, 0xf8, 0xf9, 0, 1]).is_err());
}

/// TS 24.501 table 9.11.3.4.1, TS 24.301 table 9.9.3.12.1, TS 24.008 table 10.5.4.
#[test]
fn a_type_of_identity_has_the_number_of_its_clause() {
    use MobileIdentityType5gs as Fgs;
    let fgs = [
        Fgs::NoIdentity,
        Fgs::Suci,
        Fgs::Guti,
        Fgs::Imei,
        Fgs::STmsi,
        Fgs::Imeisv,
        Fgs::MacAddress,
        Fgs::Eui64,
    ];
    for (bits, kind) in fgs.into_iter().enumerate() {
        assert_eq!(
            (kind.bits(), Fgs::try_from(bits as u8)),
            (bits as u8, Ok(kind))
        );
        assert_eq!(Fgs::of(0xf8 | bits as u8), Ok(kind));
    }
    use MobileIdentityTypeEps as Eps;
    for (bits, kind) in [(1, Eps::Imsi), (3, Eps::Imei), (6, Eps::Guti)] {
        assert_eq!((u8::from(kind), Eps::try_from(bits)), (bits, Ok(kind)));
    }
    for (bits, kind) in [
        (0, IdentityType::NoIdentity),
        (1, IdentityType::Imsi),
        (2, IdentityType::Imei),
        (3, IdentityType::Imeisv),
        (4, IdentityType::Tmsi),
        (5, IdentityType::Tmgi),
    ] {
        assert_eq!(
            (kind.bits(), IdentityType::try_from(bits)),
            (bits, Ok(kind))
        );
    }
    let error = Eps::try_from(2).unwrap_err().to_string();
    assert_eq!(
        error,
        "`type-of-identity` is 2, and is a type of identity of the EPS mobile identity"
    );
    assert!(IdentityType::try_from(6).is_err() && Fgs::try_from(8).is_err());
    // The IMEI is 3 in a mobile identity of 5GS and of EPS, and 2 in TS 24.008.
    assert_eq!(
        (
            Fgs::Imei.bits(),
            Eps::Imei.bits(),
            IdentityType::Imei.bits()
        ),
        (3, 3, 2)
    );
}
