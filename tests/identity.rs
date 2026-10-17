//! The identities against their clauses, against what other implementations sent, and
//! against themselves: every value encodes to the octets that it is read from.
//!
//! The octets marked "captured" are from lab runs of 2026-10-12 (Open5GS v2.8.0, free5GC
//! v4.2.3 and PacketRusher 20250225, with the test subscribers 208-93 0000000001 to
//! 0000000008): each implementation sent them in a NAS message.

use oxirush_utils::identity::{
    Error, Fault, Guti5g, GutiEps, IdentityType, Imei, Imeisv, Imsi, ImsiSuci,
    MobileIdentityType5gs, MobileIdentityTypeEps, NaiSuci, Plmn, ProtectionScheme,
    RoutingIndicator, STmsi5g, STmsiEps, Supi, SupiType, Tai5gs, TaiEps, digits, nai, tbcd,
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

// ── SUPI and SUCI ───────────────────────────────────────────────────────────

/// TS 23.003 clause 2.2B numbers the GLI 2 and the GCI 3; TS 24.501 table 9.11.3.4.1
/// numbers the GCI 2 and the GLI 3.
#[test]
fn a_type_of_supi_has_two_numbers() {
    for (kind, number, format, prefix) in [
        (SupiType::Imsi, 0, 0, "imsi-"),
        (SupiType::NetworkSpecific, 1, 1, "nai-"),
        (SupiType::Gli, 2, 3, "gli-"),
        (SupiType::Gci, 3, 2, "gci-"),
    ] {
        assert_eq!(
            (kind.type_number(), kind.nas_format(), kind.prefix()),
            (number, format, prefix)
        );
        assert_eq!(SupiType::from_type_number(number), Ok(kind));
        assert_eq!(SupiType::from_nas_format(format), Ok(kind));
    }
    for spare in 4..=8 {
        assert!(
            SupiType::from_type_number(spare).is_err() && SupiType::from_nas_format(spare).is_err()
        );
    }
}

#[test]
fn a_supi_is_written_with_its_type() {
    let imsi = Supi::parse("imsi-208930000000001").unwrap();
    assert_eq!(
        (imsi.supi_type(), imsi.identity()),
        (SupiType::Imsi, "208930000000001")
    );
    assert_eq!(imsi.as_imsi(), Some(Imsi::new("208930000000001").unwrap()));
    assert_eq!(
        format!("{imsi} {imsi:?}"),
        "imsi-208930000000001 Supi(imsi-208930000000001)"
    );
    assert_eq!(Supi::imsi("208930000000001"), Ok(imsi.clone()));
    assert_eq!(Supi::from(Imsi::new("208930000000001").unwrap()), imsi);
    let owned: Supi<'static> = "nai-user@example.com".parse().unwrap();
    assert_eq!(
        owned,
        Supi::network_specific("user@example.com")
            .unwrap()
            .into_owned()
    );
    assert_eq!(
        (owned.identity(), owned.as_imsi()),
        ("user@example.com", None)
    );
    // The type is part of the value: the same NAI is three SUPIs.
    let nai = "00-00-5e-00-53-00@5gc.mnc012.mcc345.3gppnetwork.org";
    let (gci, gli) = (Supi::gci(nai).unwrap(), Supi::gli(nai).unwrap());
    assert_ne!(gci, gli);
    assert_eq!(gci.to_string(), format!("gci-{nai}"));
    assert_eq!(Supi::parse(&format!("gli-{nai}")), Ok(gli));
    // A username may be empty, and a realm may not.
    assert!(Supi::network_specific("@example.com").is_ok());
    let error = |result: Result<Supi, Error>| result.unwrap_err().to_string();
    assert_eq!(
        error(Supi::network_specific("user@")),
        "`nai` is written as a username, @ and a realm, as user@example.com"
    );
    assert_eq!(
        error(Supi::gci("no-realm")),
        error(Supi::network_specific("user@"))
    );
    let long = format!("{}@example.com", "u".repeat(500));
    assert_eq!(
        error(Supi::network_specific(&long)),
        "`nai` has 512 octets, and has 3 to 507"
    );
    assert_eq!(
        error(Supi::imsi("20893")),
        "`imsi` has 5 digits, and has 6 to 15"
    );
    assert_eq!(
        error(Supi::parse("208930000000001")),
        "`supi` is written as imsi-, nai-, gli- or gci-, then the identity, as \
         imsi-208930000000001"
    );
    assert!(Supi::parse("imsi-user@example.com").is_err());
}

/// TS 24.501 table 9.11.3.4.1: 1 to 4 digits in BCD, and 1111 for each that is not
/// there. TS 23.003 clause 2.2B: each digit counts.
#[test]
fn a_routing_indicator_is_one_to_four_digits_which_all_count() {
    for (digits, octets) in [
        ("678", [0x76, 0xf8]),
        ("0", [0xf0, 0xff]),
        ("0000", [0x00, 0x00]),
        ("12", [0x21, 0xff]),
        ("9012", [0x09, 0x21]),
    ] {
        let indicator = RoutingIndicator::new(digits).unwrap();
        assert_eq!(indicator.to_nas_octets(), octets, "{digits}");
        assert_eq!(RoutingIndicator::from_nas_octets(octets), Ok(indicator));
        assert_eq!(
            format!("{indicator} {indicator:?}"),
            format!("{digits} RoutingIndicator({digits})")
        );
    }
    assert_eq!(
        RoutingIndicator::default(),
        RoutingIndicator::new("0").unwrap()
    );
    assert_ne!(
        RoutingIndicator::new("012").unwrap(),
        RoutingIndicator::new("12").unwrap()
    );
    assert_ne!(
        RoutingIndicator::new("0").unwrap(),
        RoutingIndicator::new("00").unwrap()
    );
    // Each of the 11,110 indicators is read from what it writes.
    let mut count = 0;
    for width in 1..=4 {
        for number in 0..10_u32.pow(width) {
            let indicator: RoutingIndicator = format!("{number:0width$}", width = width as usize)
                .parse()
                .unwrap();
            assert_eq!(
                RoutingIndicator::from_nas_octets(indicator.to_nas_octets()),
                Ok(indicator)
            );
            count += 1;
        }
    }
    assert_eq!(count, 11_110);
    let error = |result: Result<RoutingIndicator, Error>| result.unwrap_err().to_string();
    assert_eq!(
        error(RoutingIndicator::new("")),
        "`routing-indicator` has 0 digits, and has 1 to 4"
    );
    assert_eq!(
        error(RoutingIndicator::new("12345")),
        "`routing-indicator` has 5 digits, and has 1 to 4"
    );
    assert!(error(RoutingIndicator::new("1a")).contains("the character `a`"));
    // No digit at all, a digit after a filler, and a half octet that is neither.
    for octets in [[0xff, 0xff], [0xf6, 0xf8], [0x76, 0x8f], [0xa6, 0xff]] {
        assert!(
            RoutingIndicator::from_nas_octets(octets).is_err(),
            "{octets:02x?}"
        );
    }
}

/// TS 24.501 table 9.11.3.4.1: 0 the null scheme, 1 and 2 the profiles, 3 to 11
/// reserved, 12 to 15 of the operator. TS 33.501 annex C.1: "The values 0x3 - 0xB are
/// reserved for future standardized protection schemes."
#[test]
fn a_protection_scheme_is_its_identifier_and_no_reserved_one() {
    for identifier in 0..=15 {
        let Ok(scheme) = ProtectionScheme::new(identifier) else {
            assert!((3..=11).contains(&identifier), "{identifier}");
            continue;
        };
        assert!(!(3..=11).contains(&identifier), "{identifier}");
        assert_eq!(scheme.identifier(), identifier);
        let kinds = (scheme.is_null(), scheme.is_of_the_operator());
        assert_eq!(kinds, (identifier == 0, identifier >= 12));
    }
    assert_eq!(ProtectionScheme::new(0), Ok(ProtectionScheme::NULL));
    let shown = format!(
        "{:?} {:?}",
        ProtectionScheme::PROFILE_B,
        ProtectionScheme::new(12).unwrap()
    );
    assert_eq!(
        shown,
        "ProtectionScheme(2: profile B) ProtectionScheme(12: of the operator)"
    );
    let error = |identifier| ProtectionScheme::new(identifier).unwrap_err().to_string();
    assert_eq!(error(16), "`protection-scheme` is 16, and is 15 at most");
    assert_eq!(
        error(3),
        "`protection-scheme` is 3, and is 0, 1, 2 or 12 to 15: 3 to 11 are reserved"
    );
}

/// TS 23.003 clause 2.2B: "assuming the IMSI 234150999999999 ... the Routing Indicator
/// 678 ... the SUCI for the null protection scheme is composed of: 0, 234, 15, 678, 0, 0
/// and 0999999999". TS 33.501 annex C.4.2.1 and C.4.3.1: the scheme outputs of the IMSI
/// 274012001002086 under the null scheme and under profile A.
#[test]
fn the_suci_of_an_imsi_has_the_parts_and_the_octets_of_its_clauses() {
    let suci = ImsiSuci::null(
        plmn("234-15"),
        RoutingIndicator::new("678").unwrap(),
        "0999999999",
    )
    .unwrap();
    assert_eq!(suci.to_string(), "suci-0-234-15-678-0-0-0999999999");
    assert_eq!(
        format!("{suci:?}"),
        "ImsiSuci(suci-0-234-15-678-0-0-0999999999)"
    );
    assert_eq!(suci.imsi().unwrap().as_str(), "234150999999999");
    assert_eq!(
        (suci.msin(), suci.concealed_output()),
        (Some("0999999999"), None)
    );
    assert_eq!(suci.to_string().parse(), Ok(suci.clone()));
    let octets = suci.to_nas_octets();
    assert_eq!(hex::encode(&octets), "0132f45176f800009099999999");
    assert_eq!(ImsiSuci::from_nas_octets(&octets), Ok(suci));

    // Annex C.4.2.1: the scheme output of the null scheme is the MSIN in BCD.
    let imsi = Imsi::new("274012001002086").unwrap();
    let null = ImsiSuci::of(imsi, 3).unwrap();
    assert_eq!(
        (null.plmn(), null.msin()),
        (plmn("274-012"), Some("001002086"))
    );
    assert_eq!(hex::encode(&null.to_nas_octets()[8..]), "00012080f6");
    assert_eq!(null.to_string(), "suci-0-274-012-0-0-0-001002086");
    assert_eq!(null.imsi(), Some(imsi));

    // Annex C.4.3.1: the scheme output of profile A, with the key identifier 27 of the
    // example of TS 23.003.
    let output = "b2e92f836055a255837debf850b528997ce0201cb82adfe4be1f587d07d8457d\
                  cb02352410cddd9e730ef3fa87";
    let concealed = ImsiSuci::concealed(
        plmn("274-012"),
        RoutingIndicator::new("678").unwrap(),
        ProtectionScheme::PROFILE_A,
        27,
        hex::decode(output).unwrap(),
    )
    .unwrap();
    assert_eq!(
        concealed.to_string(),
        format!("suci-0-274-012-678-1-27-{output}")
    );
    assert_eq!(concealed.to_string().parse(), Ok(concealed.clone()));
    assert_eq!((concealed.msin(), concealed.imsi()), (None, None));
    assert_eq!(
        concealed.concealed_output().map(hex::encode).as_deref(),
        Some(output)
    );
    let octets = concealed.to_nas_octets();
    // MCC 274 and MNC 012 in the order of TS 24.008 clause 10.5.1.3: 72 24 10.
    assert_eq!(hex::encode(&octets), format!("0172241076f8011b{output}"));
    assert_eq!(ImsiSuci::from_nas_octets(&octets), Ok(concealed));
}

/// Captured: PacketRusher 20250225 sends this SUCI in its REGISTRATION REQUEST, with a
/// routing indicator of four zeros.
#[test]
fn the_suci_that_another_implementation_sent_is_read_and_written_back() {
    let sent = hex::decode("0102f839000000000000000010").unwrap();
    let suci = ImsiSuci::from_nas_octets(&sent).unwrap();
    assert_eq!(suci.to_string(), "suci-0-208-93-0000-0-0-0000000001");
    assert_eq!(suci.routing_indicator().as_str(), "0000");
    assert_eq!(suci.imsi().unwrap().as_str(), "208930000000001");
    assert_eq!(suci.to_nas_octets(), sent);
    let scheme = (
        suci.protection_scheme(),
        suci.home_network_public_key_identifier(),
    );
    assert_eq!(scheme, (ProtectionScheme::NULL, 0));
}

#[test]
fn a_suci_is_refused_with_its_member_and_the_reason() {
    let routing = RoutingIndicator::default();
    let error = |result: Result<ImsiSuci, Error>| result.unwrap_err().to_string();
    // The MSIN: digits, one at least, and no more than an IMSI of 15 digits has.
    assert_eq!(
        error(ImsiSuci::null(plmn("208-93"), routing, "00000000012")),
        "`msin` has 11 digits, and has 1 to 10"
    );
    assert_eq!(
        error(ImsiSuci::null(plmn("208-093"), routing, "")),
        "`msin` has 0 digits, and has 1 to 9"
    );
    assert!(error(ImsiSuci::null(plmn("208-93"), routing, "12a")).contains("the character `a`"));
    // TS 23.003 clause 2.2B: the key identifier is 0 if and only if the scheme is the
    // null one. TS 24.501 table 9.11.3.4.1: 255 is reserved.
    let concealed =
        |scheme, key_id| ImsiSuci::concealed(plmn("208-93"), routing, scheme, key_id, vec![1]);
    assert_eq!(
        error(concealed(ProtectionScheme::PROFILE_A, 0)),
        "`home-network-public-key-identifier` is 0, and is 1 to 254 for a scheme that conceals"
    );
    assert!(error(concealed(ProtectionScheme::PROFILE_B, 255)).contains("is 255"));
    assert!(error(concealed(ProtectionScheme::NULL, 1)).contains("`protection-scheme` is 0"));
    assert!(concealed(ProtectionScheme::new(12).unwrap(), 254).is_ok());
    let empty = ImsiSuci::concealed(
        plmn("208-93"),
        routing,
        ProtectionScheme::PROFILE_A,
        1,
        vec![],
    );
    assert_eq!(
        error(empty),
        "`scheme-output` has 0 octets, and has 41 to 45"
    );
    for (octets, reason) in [
        (
            "0102f839f0ff0000",
            "`suci` has 8 octets, and has 9 at least",
        ),
        (
            "0102f839f0ff000110",
            "`home-network-public-key-identifier` is 1, and is 0 for the null scheme",
        ),
        (
            "0102f839f0ff01001a",
            "`home-network-public-key-identifier` is 0, and is 1 to 254",
        ),
        // A reserved scheme, and one with its spare bits set too.
        (
            "0102f839f0ff0301aa",
            "`protection-scheme` is 3, and is 0, 1, 2 or 12 to 15",
        ),
        (
            "0102f839f0fffb01aa",
            "`protection-scheme` is 11, and is 0, 1, 2 or 12 to 15",
        ),
        ("010af839f0ff000010", "`mcc` has the half octet 0xa"),
        (
            "0102f839ffff000010",
            "`routing-indicator` has the half octet 0xf where a digit is",
        ),
        ("0102f839f0ff00001a", "`msin` has the character `*`"),
        (
            "0102f839f0ff00001f",
            "`msin` has the half octet 0xf where a digit is",
        ),
        (
            "0202f839f0ff000010",
            "`suci` has the octet 0x02, and has 0x01 there",
        ),
        // The SUPI format of a NAI: no SUCI of an IMSI.
        (
            "1102f839f0ff000010",
            "`suci` has the octet 0x11, and has 0x01 there",
        ),
    ] {
        let refused = error(ImsiSuci::from_nas_octets(&hex::decode(octets).unwrap()));
        assert!(refused.starts_with(reason), "{octets}: {refused}");
        assert!(
            ImsiSuci::from_nas_octets_tolerant(&hex::decode(octets).unwrap()).is_err(),
            "{octets}"
        );
    }
    for text in [
        "suci-1-208-93-0-0-0-1",
        "suci-0-208-93-0-0-1-1",
        "suci-0-208-93-0-1-1-zz",
        "suci-0-208-93-0-1-1-abc",
        "suci-0-208-93-0-0-0",
        // The scheme is one hexadecimal digit, and the key identifier a number in
        // decimal that is written in one way.
        "suci-0-208-93-0-12-1-ab",
        "suci-0-208-93-0-3-1-ab",
        "suci-0-208-93-0-b-1-ab",
        "suci-0-208-93-0-g-1-ab",
        "suci-0-208-93-0-00-0-1",
        "suci-0-208-93-0-1-01-ab",
        "suci-0-208-93-0-1-255-ab",
        "suci-0-208-93-0-1-256-ab",
        "suci-0-208-93-0-0-00-1",
        "SUCI-0-208-93-0-0-0-1",
    ] {
        assert!(text.parse::<ImsiSuci>().is_err(), "{text}");
    }
}

/// The OpenAPI of TS 29.571 (`TS29571_CommonData.yaml` 1.5.0-alpha.6) has this pattern
/// for a SUCI of an IMSI in `SupiOrSuci`:
///
/// ```text
/// suci-(0-[0-9]{3}-[0-9]{2,3}|[1-7]-.+)-[0-9]{1,4}-(0-0-.*|[a-fA-F1-9]-([1-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])-[a-fA-F0-9]+)
/// ```
fn is_a_suci_of_an_imsi_by_ts_29_571(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("suci-0-") else {
        return false;
    };
    let parts: Vec<&str> = rest.splitn(6, '-').collect();
    let [mcc, mnc, routing, scheme, key, output] = parts[..] else {
        return false;
    };
    let digits = |text: &str, least, most| {
        (least..=most).contains(&text.len()) && text.bytes().all(|digit| digit.is_ascii_digit())
    };
    let home = digits(mcc, 3, 3) && digits(mnc, 2, 3) && digits(routing, 1, 4);
    let concealed = scheme.len() == 1
        && scheme != "0"
        && scheme.bytes().all(|digit| digit.is_ascii_hexdigit())
        && digits(key, 1, 3)
        && !key.starts_with('0')
        && key.parse::<u8>().is_ok()
        && !output.is_empty()
        && output.bytes().all(|digit| digit.is_ascii_hexdigit());
    home && ((scheme, key) == ("0", "0") || concealed)
}

#[test]
fn the_text_of_a_suci_is_the_one_of_the_service_based_interfaces() {
    let routing = RoutingIndicator::default();
    // The scheme is one hexadecimal digit: the first scheme of the operator is `c`.
    let scheme = ProtectionScheme::new(12).unwrap();
    let suci = ImsiSuci::concealed(plmn("208-93"), routing, scheme, 254, vec![0xaa, 0xbb]).unwrap();
    assert_eq!(suci.to_string(), "suci-0-208-93-0-c-254-aabb");
    assert_eq!("suci-0-208-93-0-c-254-aabb".parse(), Ok(suci.clone()));
    // The hexadecimal digits are read in either case, and written in lower case.
    assert_eq!("suci-0-208-93-0-C-254-AABB".parse(), Ok(suci));
    // What free5GC printed for the SUCI of PacketRusher in a lab run.
    let sent = hex::decode("0102f839000000000000000010").unwrap();
    let suci = ImsiSuci::from_nas_octets(&sent).unwrap();
    assert_eq!(suci.to_string(), "suci-0-208-93-0000-0-0-0000000001");
    // Every scheme and every key identifier, and the pattern of the specification.
    for scheme in (0..=2).chain(12..=15) {
        for key in 0..=255 {
            let made = match scheme {
                0 => ImsiSuci::null(plmn("208-093"), routing, "123456789"),
                _ => {
                    let scheme = ProtectionScheme::new(scheme).unwrap();
                    // The shortest output of the scheme (TS 33.501 annex C.1).
                    let length = match scheme.identifier() {
                        1 => 41,
                        2 => 42,
                        _ => 1,
                    };
                    ImsiSuci::concealed(
                        plmn("208-093"),
                        routing,
                        scheme,
                        key,
                        vec![scheme.identifier(); length],
                    )
                }
            };
            let Ok(suci) = made else {
                assert!(matches!(key, 0 | 255), "{scheme} {key}");
                continue;
            };
            let text = suci.to_string();
            assert!(is_a_suci_of_an_imsi_by_ts_29_571(&text), "{text}");
            assert_eq!(text.parse(), Ok(suci));
        }
    }
    // What this reads is of the pattern, and the pattern has no more than this reads
    // but a scheme and the key identifier 255 that TS 24.501 table 9.11.3.4.1 reserves,
    // a scheme output of an odd number of hexadecimal digits, which is no octets, and
    // under the null scheme an MSIN that is no digits or that is too long for an IMSI.
    let routings = ["0", "12", "0000", "12345", "a", ""];
    let schemes = ["0", "1", "2", "5", "c", "C", "f", "g", "12", "00", ""];
    let keys = ["0", "1", "27", "254", "255", "256", "01", "00", "a", ""];
    // An output of 42 octets, which the two profiles of ECIES take (TS 33.501 annex C.1).
    let ecies = "5a".repeat(42);
    let outputs = [
        "1",
        "0999999999",
        "09999999990",
        "aabb",
        "AABB",
        "abc",
        "zz",
        "",
        &ecies,
    ];
    let (mut read, mut of_the_pattern_only) = (0, 0);
    for routing in routings {
        for scheme in schemes {
            for key in keys {
                for output in outputs {
                    let text = format!("suci-0-208-93-{routing}-{scheme}-{key}-{output}");
                    let by_the_pattern = is_a_suci_of_an_imsi_by_ts_29_571(&text);
                    match text.parse::<ImsiSuci>() {
                        Ok(suci) => {
                            assert!(by_the_pattern, "{text}");
                            assert_eq!(suci.to_string(), text.to_ascii_lowercase());
                            read += 1;
                        }
                        Err(_) if by_the_pattern => {
                            let msin = output.bytes().all(|digit| digit.is_ascii_digit())
                                && (1..=10).contains(&output.len());
                            let odd = scheme != "0" && output.len() % 2 == 1;
                            let reserved = key == "255" || scheme == "5";
                            // The output of a profile of ECIES has its lengths (TS 33.501
                            // annex C.1), which the pattern does not say.
                            let octets = output.len() / 2;
                            let ecies = (scheme == "1" && !(41..=45).contains(&octets))
                                || (scheme == "2" && !(42..=46).contains(&octets));
                            assert!(
                                reserved || odd || ecies || (scheme == "0" && !msin),
                                "{text}"
                            );
                            of_the_pattern_only += 1;
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
    // 3 routing indicators, by 2 MSINs under the null scheme and by 3 key identifiers
    // under the others: the profiles of ECIES take the output of 42 octets alone, and
    // the 3 schemes of the operator 4 outputs. The pattern has 7 outputs under the null
    // scheme, and 6 schemes, 4 key identifiers and 7 outputs under the others.
    let concealed = 3 * (2 + 3 * 4);
    assert_eq!(
        (read, of_the_pattern_only),
        (3 * (2 + concealed), 3 * (7 + 6 * 4 * 7 - concealed))
    );
}

/// TS 24.007 clause 11.1.4 on spare bits, and TS 24.501 table 9.11.3.4.1: "All other
/// values are interpreted as IMSI by this version of the protocol."
#[test]
fn the_spare_bits_and_a_spare_format_of_a_suci_are_read_with_their_faults() {
    let canonical = hex::decode("0102f839f0ff000010").unwrap();
    let (suci, faults) = ImsiSuci::from_nas_octets_tolerant(&canonical).unwrap();
    assert!(faults.is_empty());
    for (first, scheme, expected) in [
        (0x09, 0x00, vec![("suci", Fault::Spare { given: 0x09 })]),
        (0x81, 0x00, vec![("suci", Fault::Spare { given: 0x81 })]),
        (
            0x01,
            0xf0,
            vec![("protection-scheme", Fault::Spare { given: 0xf0 })],
        ),
        (
            0x41,
            0x00,
            vec![(
                "supi-format",
                Fault::Value {
                    given: 4,
                    expected: "0, the SUPI format of an IMSI",
                },
            )],
        ),
        (
            0xf9,
            0x10,
            vec![
                (
                    "supi-format",
                    Fault::Value {
                        given: 7,
                        expected: "0, the SUPI format of an IMSI",
                    },
                ),
                ("suci", Fault::Spare { given: 0xf9 }),
                ("protection-scheme", Fault::Spare { given: 0x10 }),
            ],
        ),
    ] {
        let mut octets = canonical.clone();
        (octets[0], octets[6]) = (first, scheme);
        let first_fault = ImsiSuci::from_nas_octets(&octets).unwrap_err();
        let (read, faults) = ImsiSuci::from_nas_octets_tolerant(&octets).unwrap();
        let found: Vec<_> = faults
            .iter()
            .map(|fault| (fault.member, fault.fault))
            .collect();
        assert_eq!(found, expected, "{first:#04x} {scheme:#04x}");
        assert_eq!((first_fault.member, first_fault.fault), expected[0]);
        // The value is the one of the canonical octets, which it writes.
        assert_eq!(read, suci);
        assert_eq!(read.to_nas_octets(), canonical);
    }
}

#[test]
fn any_suci_of_an_imsi_is_read_from_what_it_writes() {
    let mut numbers = Numbers(4);
    for _ in 0..20_000 {
        let width = 2 + (numbers.next() % 2) as usize;
        let plmn = Plmn::new(&numbers.digits(3), &numbers.digits(width)).unwrap();
        let count = 1 + (numbers.next() % 4) as usize;
        let routing = RoutingIndicator::new(&numbers.digits(count)).unwrap();
        let suci = match numbers.next() % 2 {
            0 => {
                let count = 1 + numbers.next() as usize % (15 - plmn.digits().len());
                let msin = numbers.digits(count);
                ImsiSuci::null(plmn, routing, &msin).unwrap()
            }
            _ => {
                let scheme = [1, 2, 12, 13, 14, 15][numbers.next() as usize % 6];
                let scheme = ProtectionScheme::new(scheme).unwrap();
                // The output of a profile of ECIES has its key, the ciphertext of the
                // MSIN and its MAC tag (TS 33.501 annex C.1).
                let length = match scheme.identifier() {
                    1 => 41 + numbers.next() % 5,
                    2 => 42 + numbers.next() % 5,
                    _ => 1 + numbers.next() % 60,
                };
                let output: Vec<u8> = (0..length).map(|_| numbers.next() as u8).collect();
                ImsiSuci::concealed(
                    plmn,
                    routing,
                    scheme,
                    1 + (numbers.next() % 254) as u8,
                    output,
                )
                .unwrap()
            }
        };
        assert_eq!(
            ImsiSuci::from_nas_octets(&suci.to_nas_octets()).as_ref(),
            Ok(&suci)
        );
        assert_eq!(suci.to_string().parse().as_ref(), Ok(&suci));
    }
    // And any octets are a SUCI that writes them back, or none.
    let (mut read, mut refused) = (0, 0);
    for _ in 0..50_000 {
        let mut octets: Vec<u8> = (0..9 + numbers.next() % 8)
            .map(|_| numbers.next() as u8)
            .collect();
        // Three in four start as a SUCI of an IMSI does.
        if numbers.next() & 3 != 0 {
            octets[..8].copy_from_slice(&[0x01, 0x02, 0xf8, 0x39, 0xf0, 0xff, 0x00, 0x00]);
            octets[6] = [0, 1, 2, 12][numbers.next() as usize % 4];
            octets[7] = (numbers.next() % 3) as u8;
            // A profile of ECIES has an output of its lengths, or one octet more or less.
            if let 1 | 2 = octets[6] {
                let shortest = 40 + usize::from(octets[6]);
                let length = 8 + shortest - 1 + numbers.next() as usize % 7;
                octets.resize_with(length, || numbers.next() as u8);
            }
        }
        match ImsiSuci::from_nas_octets(&octets) {
            Ok(suci) => {
                assert_eq!(suci.to_nas_octets(), octets);
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

// ── The SUCI in the form of a NAI ───────────────────────────────────────────

fn nai_suci(text: &str) -> NaiSuci {
    text.parse().unwrap()
}

/// What a text that is no SUCI is refused with: the member at fault.
fn refused_member(text: &str) -> &'static str {
    text.parse::<NaiSuci>().unwrap_err().member
}

/// A 5GS mobile identity that holds the NAI `text`, with the SUPI format `format`.
fn nas(format: u8, text: &str) -> Vec<u8> {
    let mut octets = vec![format << 4 | 0x01];
    octets.extend_from_slice(text.as_bytes());
    octets
}

/// The examples of TS 23.003 clauses 28.7.3, 28.15.5 and 28.16.5, as text and as the 5GS
/// mobile identity of TS 24.501 figure 9.11.3.4.3b.
#[test]
fn a_suci_in_the_form_of_a_nai_is_read_and_written_back() {
    let text = "type1.rid678.schid0.useriduser17@example.com";
    let suci = nai_suci(text);
    assert_eq!(suci.supi_type(), SupiType::NetworkSpecific);
    assert_eq!(suci.routing_indicator().as_str(), "678");
    assert_eq!(suci.protection_scheme(), ProtectionScheme::NULL);
    assert_eq!(suci.home_network_public_key_identifier(), 0);
    assert_eq!(
        (suci.clear(), suci.realm()),
        (Some("user17"), "example.com")
    );
    assert_eq!((suci.concealed_output(), suci.ecies()), (None, None));
    assert_eq!(suci.to_string(), text);
    assert_eq!(format!("{suci:?}"), format!("NaiSuci({text})"));
    assert_eq!(suci.to_nas_octets().unwrap(), nas(1, text));
    assert_eq!(NaiSuci::from_nas_octets(&nas(1, text)).unwrap(), suci);

    // The SUPI format of NAS has the GCI as 2 and the GLI as 3, and the SUPI type of the
    // NAI the other way round.
    let gci = "type3.rid0.schid0.userid00-00-5E-00-53-00@operator.com";
    let gli = "type2.rid0.schid0.useridQUJDRA==@operator.com";
    assert_eq!(nai_suci(gci).supi_type(), SupiType::Gci);
    assert_eq!(nai_suci(gli).supi_type(), SupiType::Gli);
    assert_eq!(nai_suci(gci).to_nas_octets().unwrap(), nas(2, gci));
    assert_eq!(nai_suci(gli).to_nas_octets().unwrap(), nas(3, gli));
    for (format, text) in [(1, gci), (2, gli), (3, gci), (2, text)] {
        let error = NaiSuci::from_nas_octets(&nas(format, text)).unwrap_err();
        assert_eq!(error.member, "supi-type", "{format} {text}");
    }
    // An anonymous SUCI has no username.
    let anonymous = nai_suci("type1.rid678.schid0.userid@example.com");
    assert_eq!(anonymous.clear(), Some(""));
    // A GCI and a GLI have the routing indicator 0 and the null scheme.
    assert_eq!(
        refused_member("type3.rid1.schid0.userid00-00-5E-00-53-00@operator.com"),
        "routing-indicator"
    );
    assert_eq!(
        refused_member("type3.rid0.schid12.hnkey1.outaa@operator.com"),
        "protection-scheme"
    );
    // What is no mobile identity of a NAI.
    for octets in [
        &[][..],
        &[0x12, b'a'],
        &[0x01, b'a'],
        &[0x71, b'a'],
        &[0x11, 0xff, b'@'],
    ] {
        assert!(NaiSuci::from_nas_octets(octets).is_err(), "{octets:02x?}");
    }
}

/// TS 33.501 annex C.4: the scheme outputs of the network specific identifier
/// verylongusername1@3gpp.com, with the null scheme (C.4.2.2) and the profiles A
/// (C.4.3.2) and B (C.4.4.2), written as the annex writes them, in upper case.
#[test]
fn the_scheme_outputs_of_annex_c4_are_read_and_written_back() {
    let null = nai_suci("type1.rid0.schid0.useridverylongusername1@3gpp.com");
    assert_eq!(null.clear(), Some("verylongusername1"));
    let concealed = [
        (
            1,
            "ecckey977D8B2FDAA7B64AA700D04227D5B440630EA4EC50F9082273A26BB678C92222.\
             cip8E358A1582ADB15322C10E515141D2039A.mac12E1D7783A97F1AC",
            "977d8b2fdaa7b64aa700d04227d5b440630ea4ec50f9082273a26bb678c92222",
            "8e358a1582adb15322c10e515141d2039a",
            "12e1d7783a97f1ac",
        ),
        (
            2,
            "ecckey03759BB22C563D9F4A6B3C1419E543FC2F39D6823F02A9D71162B39399218B244B.\
             cipBE22D8B9F856A52ED381CD7EAF4CF2D525.mac3CDDC61A0A7882EB",
            "03759bb22c563d9f4a6b3c1419e543fc2f39d6823f02a9d71162b39399218b244b",
            "be22d8b9f856a52ed381cd7eaf4cf2d525",
            "3cddc61a0a7882eb",
        ),
    ];
    for (scheme, output, public_key, ciphertext, tag) in concealed {
        let text = format!("type1.rid678.schid{scheme}.hnkey27.{output}@3gpp.com");
        let suci = nai_suci(&text);
        assert_eq!(suci.protection_scheme().identifier(), scheme);
        assert_eq!(suci.home_network_public_key_identifier(), 27);
        let (key, cipher, mac) = suci.ecies().unwrap();
        assert_eq!(
            (hex::encode(key), hex::encode(cipher), hex::encode(mac)),
            (public_key.into(), ciphertext.into(), tag.into())
        );
        let concealed = [
            hex::decode(public_key).unwrap(),
            hex::decode(ciphertext).unwrap(),
        ];
        let mut whole = concealed.concat();
        whole.extend(hex::decode(tag).unwrap());
        assert_eq!(suci.concealed_output(), Some(&whole[..]));
        // Written in lower case, and read back from it and from its mobile identity
        let written = format!(
            "type1.rid678.schid{scheme}.hnkey27.ecckey{public_key}.cip{ciphertext}.mac{tag}@3gpp.com"
        );
        assert_eq!(suci.to_string(), written);
        assert_eq!(nai_suci(&written), suci);
        assert_eq!(
            NaiSuci::from_nas_octets(&suci.to_nas_octets().unwrap()).unwrap(),
            suci
        );
        let made = NaiSuci::concealed(
            SupiType::NetworkSpecific,
            RoutingIndicator::new("678").unwrap(),
            ProtectionScheme::new(scheme).unwrap(),
            27,
            whole,
            "3gpp.com",
        );
        assert_eq!(made.unwrap(), suci);
    }
}

/// Each mistake names its member.
#[test]
fn a_text_that_is_no_suci_in_the_form_of_a_nai_is_refused_with_its_member() {
    for (text, member) in [
        ("type1.rid678.schid0.useriduser17", "nai"),
        ("type1.rid678.schid0.useriduser17@example", "realm"),
        ("type1.rid678.schid0.useriduser17@", "realm"),
        ("@example.com", "type"),
        ("user17@example.com", "type"),
        ("typex.rid678.schid0.useridu@example.com", "type"),
        ("type1@example.com", "rid"),
        ("type1.schid0.rid1.useridu@example.com", "rid"),
        ("type1.rid678@example.com", "schid"),
        (
            "type1.rid678.schid16.useridu@example.com",
            "protection-scheme",
        ),
        (
            "type1.rid678.schid3.hnkey27.out00@example.com",
            "protection-scheme",
        ),
        ("type1.rid678.schid0@example.com", "userid"),
        ("type1.rid678.schid0.user17@example.com", "userid"),
        ("type1.rid678.schid1@example.com", "hnkey"),
        ("type1.rid678.schid1.hnkey@example.com", "hnkey"),
        ("type1.rid678.schid1.hnkey1000.ecckey@example.com", "hnkey"),
        ("type1.rid678.schid12.hnkey027.out00@example.com", "hnkey"),
        ("type01.rid678.schid0.useridu@example.com", "type"),
        ("type1.rid678.schid00.useridu@example.com", "schid"),
        ("type1.rid678.schid1.hnkey27@example.com", "ecckey"),
        ("type1.rid678.schid12.hnkey27@example.com", "out"),
        ("type1.rid678.schid12.hnkey27.outnot-hex@example.com", "out"),
        ("type1.rid678.schid12.hnkey27.outabc@example.com", "out"),
        ("type7.rid678.schid0.useridu@example.com", "supi-type"),
        (
            "type1.rid12345.schid0.useridu@example.com",
            "routing-indicator",
        ),
        ("type1.rid678.schid0.userida b@example.com", "username"),
    ] {
        assert_eq!(refused_member(text), member, "{text}");
    }
    // The fields of a profile, each missing or of another length
    let key = "42".repeat(32);
    let profile = |ecckey: &str, cip: &str, mac: &str| {
        format!("type1.rid678.schid1.hnkey27.{ecckey}.{cip}.{mac}@example.com")
    };
    let whole = profile(&format!("ecckey{key}"), "cipaabb", "mac0011223344556677");
    assert!(whole.parse::<NaiSuci>().is_ok());
    for (text, member) in [
        (
            profile(
                &format!("ecckey{}", &key[2..]),
                "cipaabb",
                "mac0011223344556677",
            ),
            "ecckey",
        ),
        (
            profile(&format!("key{key}"), "cipaabb", "mac0011223344556677"),
            "ecckey",
        ),
        (
            profile(&format!("ecckey{key}"), "cip", "mac0011223344556677"),
            "cip",
        ),
        (
            profile(&format!("ecckey{key}"), "cipaab", "mac0011223344556677"),
            "cip",
        ),
        (
            profile(&format!("ecckey{key}"), "cipaabb", "mac00112233445566"),
            "mac",
        ),
        (
            profile(
                &format!("ecckey{key}"),
                "cipaabb",
                "mac0011223344556677.more",
            ),
            "mac",
        ),
        (whole.replace(".mac0011223344556677", ""), "mac"),
    ] {
        assert_eq!(refused_member(&text), member, "{text}");
    }
    // The text of an error says what the member is written as.
    let error = "type1.rid678.schid1.hnkey27@example.com".parse::<NaiSuci>();
    assert_eq!(
        error.unwrap_err().to_string(),
        "`ecckey` is written as ecckey and the ephemeral public key in hexadecimal digits"
    );
}

/// The length is looked at first, and the realm before the username.
#[test]
fn a_text_that_is_too_long_is_refused_before_it_is_read() {
    let huge = format!(
        "type1.rid0.schid0.userid{}@example.com",
        "a".repeat(64 * 1024)
    );
    let error = huge.parse::<NaiSuci>().unwrap_err();
    assert_eq!(error.member, "nai");
    assert!(
        matches!(error.fault, Fault::Length { max: 6824, .. }),
        "{error:?}"
    );
    assert_eq!(
        NaiSuci::from_nas_octets(&nas(1, &huge)).unwrap_err().member,
        "nai"
    );
    let long_realm = format!("type1.rid0.schid0.useridu@{}.com", "a".repeat(250));
    assert_eq!(refused_member(&long_realm), "realm");
    // The longest realm, of labels of 63 octets at most
    let realm = format!("{0}.{0}.{0}.{1}.com", "a".repeat(63), "a".repeat(57));
    assert_eq!(realm.len(), 253);
    assert!(
        format!("type1.rid0.schid0.useridu@{realm}")
            .parse::<NaiSuci>()
            .is_ok()
    );
    let long_label = format!("type1.rid0.schid0.useridu@{}.com", "a".repeat(64));
    assert_eq!(refused_member(&long_label), "realm");
    // A username of more than 253 octets, in clear and as ciphertext
    let long_username = format!("type1.rid0.schid0.userid{}@example.com", "a".repeat(254));
    assert_eq!(refused_member(&long_username), "username");
    let long_ciphertext = format!(
        "type1.rid0.schid1.hnkey1.ecckey{}.cip{}.mac0011223344556677@example.com",
        "42".repeat(32),
        "aa".repeat(254)
    );
    assert_eq!(refused_member(&long_ciphertext), "cip");
}

/// The home network public key identifier is 1 to 254 with a scheme that conceals, and 0
/// with the null scheme, in this form as in a mobile identity.
#[test]
fn a_key_identifier_is_one_of_its_scheme() {
    for valid in [1, 27, 254] {
        let text = format!("type1.rid678.schid12.hnkey{valid}.out00abff@example.com");
        let suci = nai_suci(&text);
        assert_eq!(suci.home_network_public_key_identifier(), valid);
        assert_eq!(suci.concealed_output(), Some(&[0x00, 0xab, 0xff][..]));
        assert_eq!(suci.to_string(), text);
    }
    for invalid in [0u8, 255] {
        let text = format!("type1.rid678.schid12.hnkey{invalid}.out00abff@example.com");
        let error = text.parse::<NaiSuci>().unwrap_err();
        assert_eq!(error.member, "home-network-public-key-identifier");
        assert!(matches!(error.fault, Fault::Value { given, .. } if given == u64::from(invalid)));
    }
    // Hexadecimal digits of either case are read, and written in lower case
    let upper = nai_suci("type1.rid678.schid12.hnkey27.out00ABFF@example.com");
    assert_eq!(
        upper.to_string(),
        "type1.rid678.schid12.hnkey27.out00abff@example.com"
    );
    // The null scheme has no key
    let routing = RoutingIndicator::default();
    let kind = SupiType::NetworkSpecific;
    let null = NaiSuci::concealed(kind, routing, ProtectionScheme::NULL, 0, vec![1], "a.b");
    assert_eq!(null.unwrap_err().member, "protection-scheme");
}

#[test]
fn the_grammar_of_rfc_7542_is_strict() {
    for realm in [
        ".",
        "a..b",
        "-bad.example",
        "bad-.example",
        "example",
        "a b.example",
    ] {
        assert_eq!(
            nai::check_realm(realm).unwrap_err().member,
            "realm",
            "realm {realm:?}"
        );
    }
    for username in ["a b", "a@b", "a..b", ".a", "a.", ""] {
        assert_eq!(
            nai::check_username(username).unwrap_err().member,
            "username",
            "username {username:?}"
        );
    }
    for username in [
        "user\u{a0}name",
        "user\u{2003}name",
        "élodie",
        "a!#$%&'*+-/=?^_`{|}~z",
    ] {
        assert_eq!(nai::check_username(username), Ok(()), "{username:?}");
    }
    for username in ["user\u{85}name", "user\u{7f}name"] {
        assert!(nai::check_username(username).is_err(), "{username:?}");
    }
    // A realm is a domain name, whose case is not significant (RFC 7542 clause 2.4,
    // RFC 4343), in Unicode as in ASCII.
    for realm in [
        "bücher.example",
        "xn--bcher-kva.example",
        "a-b.c-d.example",
        "BÜCHER.example",
        "Bücher.EXAMPLE",
        "XN--BCHER-KVA.example",
    ] {
        assert_eq!(nai::check_realm(realm), Ok(()), "{realm}");
    }
    // A realm and a username that are not normalized, and realms that IDNA maps to another
    // name: refused with the feature `idna`, and taken by the grammar alone without it.
    let idna = cfg!(feature = "idna");
    for realm in [
        "ｅxample.com",
        "a．b.example",
        "bu\u{0308}cher.example",
        "xn--zz.example",
    ] {
        assert_eq!(nai::check_realm(realm).is_err(), idna, "{realm}");
    }
    assert_eq!(nai::check_username("e\u{301}").is_err(), idna);
}

/// TS 23.003 clause 28.7.3: the SUCI of an IMSI in the form of a NAI, whose realm is the
/// home network domain. Its MSIN is not known to belong to one PLMN, as the realm writes
/// the MNC with three digits: it has no mobile identity of its own.
#[test]
fn an_imsi_in_the_form_of_a_nai_has_the_home_network_domain() {
    for text in [
        "type0.rid678.schid0.userid0999999999@5gc.mnc015.mcc234.3gppnetwork.org",
        "type0.rid678.schid0.userid0999999999@5gc.nid000007ed9d5.mnc015.mcc234.3gppnetwork.org",
    ] {
        let suci = nai_suci(text);
        assert_eq!(suci.supi_type(), SupiType::Imsi);
        assert_eq!(suci.to_string(), text);
        assert_eq!(suci.to_nas_octets().unwrap_err().member, "supi-type");
    }
    for (text, member) in [
        (
            "type0.rid678.schid0.userid@5gc.mnc015.mcc234.3gppnetwork.org",
            "msin",
        ),
        (
            "type0.rid678.schid0.useridabc@5gc.mnc015.mcc234.3gppnetwork.org",
            "msin",
        ),
        (
            "type0.rid678.schid0.userid01234567890@5gc.mnc015.mcc234.3gppnetwork.org",
            "msin",
        ),
        ("type0.rid678.schid0.userid0123@example.com", "realm"),
        (
            "type0.rid678.schid0.userid0123@5gc.mnc15.mcc234.3gppnetwork.org",
            "realm",
        ),
        (
            "type0.rid678.schid0.userid0123@5gc.nid3.mnc015.mcc234.3gppnetwork.org",
            "realm",
        ),
        (
            "type0.rid678.schid0.userid0123@5gc.mnc015.mcc234.3gppnetwork.org.x",
            "realm",
        ),
        ("type3.rid0.schid0.userid@operator.com", "username"),
        ("type2.rid0.schid0.userid***@operator.com", "username"),
    ] {
        assert_eq!(refused_member(text), member, "{text}");
    }
    assert_eq!(
        nai::home_network_domain(plmn("234-15")),
        "5gc.mnc015.mcc234.3gppnetwork.org"
    );
    assert_eq!(
        nai::home_network_domain(plmn("310-410")),
        "5gc.mnc410.mcc310.3gppnetwork.org"
    );
}

/// TS 23.003 clause 28.16.4: a GLI is base64 of RFC 4648 as it is written, of 150 octets at
/// most.
#[test]
fn a_gli_is_base64_as_it_is_written() {
    let gli = |username: &str| format!("type2.rid0.schid0.userid{username}@operator.com");
    for valid in ["QUJDRA==", "QUJD", "QUI=", "+/+/"] {
        assert!(gli(valid).parse::<NaiSuci>().is_ok(), "{valid}");
    }
    let longest = "QUJD".repeat(50);
    assert!(gli(&longest).parse::<NaiSuci>().is_ok());
    for invalid in [
        "QUJDRA=", "QUJDRB==", "QUJ=", "Q===", "QU=D", "QUJD====", "QU JD", "",
    ] {
        assert_eq!(refused_member(&gli(invalid)), "username", "{invalid}");
    }
    assert_eq!(refused_member(&gli(&format!("{longest}QUJD"))), "username");
}

/// The spare bits of the first octet are read by the tolerant reader, which says so.
#[test]
fn a_spare_bit_of_a_suci_in_the_form_of_a_nai_is_read_with_its_fault() {
    let text = "type1.rid678.schid0.useriduser17@example.com";
    for first in [0x91, 0x19] {
        let octets = nas(1, text);
        let mut spare = octets.clone();
        spare[0] = first;
        let (suci, faults) = NaiSuci::from_nas_octets_tolerant(&spare).unwrap();
        assert_eq!(suci.to_nas_octets().unwrap(), octets);
        assert_eq!(faults.len(), 1);
        assert_eq!(
            (faults[0].member, faults[0].fault),
            ("suci", Fault::Spare { given: first })
        );
        let error: Error = NaiSuci::from_nas_octets(&spare).unwrap_err();
        assert_eq!(error.fault, Fault::Spare { given: first });
    }
}

/// Every SUCI that is made is read from the text and from the octets that it writes, and
/// any text near one is read as a SUCI that writes it again, or refused.
#[test]
fn any_suci_in_the_form_of_a_nai_is_read_from_what_it_writes() {
    let mut numbers = Numbers(0x5ec1);
    let hex = |numbers: &mut Numbers, count: usize| -> Vec<u8> {
        (0..count).map(|_| numbers.next() as u8).collect()
    };
    let realms = ["example.com", "operator.com", "a-b.c.example", "3gpp.com"];
    let mut made = Vec::new();
    for _ in 0..2_000 {
        let realm = realms[(numbers.next() % 4) as usize];
        let length = 1 + (numbers.next() % 4) as usize;
        let routing = RoutingIndicator::new(&numbers.digits(length));
        let routing = routing.unwrap();
        let suci = match numbers.next() % 4 {
            0 => {
                let length = (numbers.next() % 20) as usize;
                let username = format!("u{}", numbers.digits(length));
                NaiSuci::null(SupiType::NetworkSpecific, routing, &username, realm)
            }
            1 => NaiSuci::null(SupiType::Gci, RoutingIndicator::default(), "a.b-c", realm),
            2 => {
                let scheme = ProtectionScheme::new(1 + (numbers.next() % 2) as u8).unwrap();
                let key = if scheme == ProtectionScheme::PROFILE_A {
                    32
                } else {
                    33
                };
                let length = key + 1 + (numbers.next() % 40) as usize + 8;
                let output = hex(&mut numbers, length);
                let key_id = 1 + (numbers.next() % 254) as u8;
                NaiSuci::concealed(
                    SupiType::NetworkSpecific,
                    routing,
                    scheme,
                    key_id,
                    output,
                    realm,
                )
            }
            _ => {
                let scheme = ProtectionScheme::new(12 + (numbers.next() % 4) as u8).unwrap();
                let length = 1 + (numbers.next() % 60) as usize;
                let output = hex(&mut numbers, length);
                let key_id = 1 + (numbers.next() % 254) as u8;
                NaiSuci::concealed(
                    SupiType::NetworkSpecific,
                    routing,
                    scheme,
                    key_id,
                    output,
                    realm,
                )
            }
        };
        let suci = suci.unwrap();
        assert_eq!(nai_suci(&suci.to_string()), suci);
        assert_eq!(
            NaiSuci::from_nas_octets(&suci.to_nas_octets().unwrap()).unwrap(),
            suci
        );
        made.push(suci.to_string());
    }
    // A character of a text changed: read as a SUCI that writes it again, or refused
    let (mut read, mut refused) = (0, 0);
    let alphabet = b"0123456789abcdef.@-ABxyz";
    for text in made.iter().cycle().take(20_000) {
        let mut changed = text.clone().into_bytes();
        let at = (numbers.next() as usize) % changed.len();
        changed[at] = alphabet[(numbers.next() as usize) % alphabet.len()];
        let changed = String::from_utf8(changed).unwrap();
        match changed.parse::<NaiSuci>() {
            Ok(suci) => {
                // Hexadecimal digits are written in lower case, and the rest as it came.
                assert_eq!(nai_suci(&suci.to_string()), suci);
                if !changed.bytes().any(|c| c.is_ascii_uppercase()) {
                    assert_eq!(suci.to_string(), changed);
                }
                read += 1;
            }
            Err(_) => refused += 1,
        }
    }
    assert!(
        read > 1_000 && refused > 1_000,
        "{read} read, {refused} refused"
    );
}

/// TS 33.501 annex C.1: the output of profile A is a public key of 256 bits, the
/// ciphertext of the scheme input and a MAC of 64 bits; profile B has a key of 264 bits;
/// a scheme of the operator 3000 octets at most beside its input. The input of an IMSI
/// is its MSIN in packed BCD (annex C.3.1): 5 octets at most for an MSIN of 10 digits.
#[test]
fn the_output_of_a_profile_of_ecies_has_the_lengths_of_its_parts() {
    let routing = RoutingIndicator::default();
    let concealed = |scheme: u8, length: usize| {
        let scheme = ProtectionScheme::new(scheme).unwrap();
        ImsiSuci::concealed(plmn("208-93"), routing, scheme, 1, vec![0x5a; length])
    };
    // Profile A: 32 + 1..=5 + 8 octets.
    assert_eq!(
        concealed(1, 1).unwrap_err().to_string(),
        "`scheme-output` has 1 octet, and has 41 to 45"
    );
    assert!(concealed(1, 41).is_ok() && concealed(1, 45).is_ok());
    assert!(concealed(1, 40).is_err() && concealed(1, 46).is_err());
    // Profile B: 33 + 1..=5 + 8 octets.
    assert!(concealed(2, 42).is_ok() && concealed(2, 46).is_ok());
    assert!(concealed(2, 41).is_err() && concealed(2, 47).is_err());
    // A scheme of the operator: its output, 3000 octets at most beside the input.
    assert!(concealed(12, 1).is_ok() && concealed(12, 3005).is_ok());
    assert_eq!(
        concealed(15, 3006).unwrap_err().to_string(),
        "`scheme-output` has 3006 octets, and has 1 to 3005"
    );
    // The octets of a NAS message and the text are read with the same lengths.
    let suci = concealed(1, 41).unwrap();
    let mut octets = suci.to_nas_octets();
    octets.pop();
    assert!(ImsiSuci::from_nas_octets(&octets).is_err());

    // What a peer sends is read with another length, as a fault: the scheme output is
    // opaque to every node but the home network, whose de-concealment decides, as it
    // refuses the uncompressed ephemeral key of profile B (TS 33.514 clause 4.2.1.3).
    // The value writes the octets that were received, so that a node passes them on.
    let mut uncompressed = concealed(2, 46).unwrap().to_nas_octets();
    uncompressed.extend([0x5a; 32]);
    for (octets, said) in [
        (&octets, "`scheme-output` has 40 octets, and has 41 to 45"),
        (
            &uncompressed,
            "`scheme-output` has 78 octets, and has 42 to 46",
        ),
    ] {
        let (read, faults) = ImsiSuci::from_nas_octets_tolerant(octets).unwrap();
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].to_string(), said);
        assert_eq!(read.concealed_output(), Some(&octets[8..]));
        assert_eq!(&read.to_nas_octets(), octets);
        assert_eq!(ImsiSuci::from_nas_octets(octets), Err(faults[0]));
    }
    // With the spare bits, four faults: as many as one reading has.
    let mut spare = uncompressed.clone();
    (spare[0], spare[6]) = (0xc9, 0x12);
    let (_, faults) = ImsiSuci::from_nas_octets_tolerant(&spare).unwrap();
    let members: Vec<_> = faults.iter().map(|fault| fault.member).collect();
    assert_eq!(
        members,
        ["supi-format", "suci", "protection-scheme", "scheme-output"]
    );
}
