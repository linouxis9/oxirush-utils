// The Rust blocks of the README and of the documentation, each as a function.
// `tests/readme.rs` writes this file and says how: do not edit it.
#![allow(unused)]

use std::cell::RefCell;

thread_local!(static PRINTED: RefCell<String> = const { RefCell::new(String::new()) });

// What a block prints is kept, to be compared with what its page shows.
macro_rules! print {
    ($($argument:tt)*) => {{
        let text = format!($($argument)*);
        PRINTED.with(|printed| printed.borrow_mut().push_str(&text))
    }};
}

macro_rules! println {
    () => {
        print!("\n")
    };
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        print!("{line}\n")
    }};
}

/// What `block` prints.
pub fn printed(block: fn()) -> String {
    PRINTED.with(|printed| printed.borrow_mut().clear());
    block();
    PRINTED.with(|printed| printed.take())
}

pub const BLOCKS: &[fn()] = &[
    block_1,
    block_2,
    block_3,
    block_4,
    block_5,
    block_6,
    block_7,
    block_8,
    block_9,
];

/// The Rust block 1 of README.md.
fn block_1() {
    use oxirush_utils::identity::{Guti5g, Plmn};

    fn main() -> Result<(), oxirush_utils::identity::Error> {
        let plmn: Plmn = "310-410".parse()?;
        let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;

        // The three octets of a PLMN in a NAS message, and in an NGAP or S1AP message
        println!("{plmn}: {:02x?} in NAS, {:02x?} in NGAP", plmn.to_nas_octets(), plmn.to_ap_octets());

        // A 5G-GUTI as text, and as a 5GS mobile identity
        println!("{guti}: {:02x?}", guti.to_nas_octets());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 2 of README.md.
fn block_2() {
    use oxirush_utils::identity::{Guti5g, Imsi, ImsiSuci, Plmn, RoutingIndicator, Supi};

    fn main() -> Result<(), oxirush_utils::identity::Error> {
        // An IMSI whose MNC has three digits: its PLMN and its MSIN
        let imsi = Imsi::new("310410123456789")?;
        let (plmn, msin) = (imsi.plmn(3)?, imsi.msin(3)?);
        println!("IMSI {imsi}: PLMN {plmn}, MSIN {msin}");

        // The PLMN in a NAS message (TS 24.008 clause 10.5.1.3), and in an NGAP
        // or S1AP message (TS 38.413 clause 9.3.3.5)
        println!("{:02x?} in NAS, {:02x?} in NGAP", plmn.to_nas_octets(), plmn.to_ap_octets());

        // Its SUCI under the null scheme, as text and as a 5GS mobile identity,
        // and the SUPI that the SUCI gives
        let suci = ImsiSuci::null(plmn, RoutingIndicator::default(), msin)?;
        println!("{suci}: {:02x?}", suci.to_nas_octets());
        println!("{}", Supi::from(suci.imsi().unwrap()));

        // The 5G-GUTI that an AMF assigns: AMF region, set and pointer, and the
        // 5G-TMSI. Its octets in a 5GS mobile identity, read back, and its 5G-S-TMSI
        let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;
        let octets = guti.to_nas_octets();
        println!("{guti}: {octets:02x?}");
        println!("{}", Guti5g::from_nas_octets(&octets)?.s_tmsi());

        // What is no PLMN says which member is at fault, and why
        println!("{}", Plmn::new("208", "9").unwrap_err());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 3 of README.md.
fn block_3() {
    use oxirush_utils::identity::{GutiEps, Imei, Imeisv, Imsi, Plmn, Tai5gs, TaiEps};

    fn main() -> Result<(), oxirush_utils::identity::Error> {
        // The identity digits of an IMSI in a mobile identity, after its type of identity
        let imsi = Imsi::new("208930000000001")?;
        println!("{:02x?}", imsi.to_nas_octets(1)?);

        // An IMEI as a mobile sends it, and with its check digit; the SVN of an IMEISV
        println!("{}", Imei::with_spare_digit("49015420323751")?);
        println!("{}", Imei::with_check_digit("49015420323751")?);
        println!("{}", Imeisv::new("4901542032375101")?.svn());

        // The GUTI of EPS: its MME group, MME code and M-TMSI; a tracking area of each system
        let plmn = Plmn::new("208", "93")?;
        println!("{}", GutiEps::new(plmn, 0x0002, 0x01, 0xc000_04f4));
        println!("{} {}", Tai5gs::new(plmn, 1)?, TaiEps::new(plmn, 1));
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 4 of README.md.
fn block_4() {
    use oxirush_utils::identity::{NaiSuci, nai};

    fn main() -> Result<(), oxirush_utils::identity::Error> {
        // The example of TS 23.003 clause 28.7.3: a network specific identifier in clear
        let suci: NaiSuci = "type1.rid678.schid0.useriduser17@example.com".parse()?;
        println!("{:?} {:?} {}", suci.supi_type(), suci.clear(), suci.realm());

        // In a 5GS mobile identity, after the SUPI format of a network specific identifier
        println!("{:#04x}", suci.to_nas_octets()?[0]);

        // A realm has two labels at least
        println!("{}", nai::check_realm("example").unwrap_err());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 5 of README.md.
fn block_5() {
    use oxirush_utils::identity::Imeisv;

    fn main() -> Result<(), oxirush_utils::identity::Error> {
        // PacketRusher sends an IMEISV with 0000 where its end mark 1111 belongs
        let sent = [0x15, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        println!("{}", Imeisv::from_nas_octets(5, &sent).unwrap_err());

        let (imeisv, faults) = Imeisv::from_nas_octets_tolerant(5, &sent)?;
        println!("{imeisv}, read with: {}", faults[0]);
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 6 of README.md.
fn block_6() {
    use oxirush_utils::{tree, tree::{Members, Reading, Refused, Tree, Value}};

    #[derive(Debug)]
    struct Slice {
        sst: u8,
        sd: Option<[u8; 3]>,
    }

    impl Tree for Slice {
        const NAME: &'static str = "S-NSSAI";

        fn to_tree(&self) -> Value {
            let mut members = Members::new();
            members.put("sst", tree::unsigned(self.sst.into()));
            members.put_optional("sd", self.sd.map(|sd| tree::octets(&sd)));
            members.finish()
        }

        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, &["sst", "sd"])?;
            Ok(Self {
                sst: reading.member(&members, 0, |value, reading| reading.unsigned(value, 0, 255))? as u8,
                sd: reading.optional(&members, 1, |value, reading| reading.octet_array(value))?,
            })
        }
    }

    fn main() -> Result<(), oxirush_utils::tree::Error> {
        println!("{}", tree::to_value(&Slice { sst: 1, sd: Some([0, 0, 0xff]) }));

        // A name is read however it is written, and octets in either case
        let read: Slice = tree::from_value(&tree!({"SST": 2, "sd": "0000FF"}))?;
        println!("{read:?}");

        // A refusal says where and why
        println!("{}", tree::from_value::<Slice>(&tree!({"sst": 256})).unwrap_err());
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 7 of README.md.
fn block_7() {
    use oxirush_utils::tree;

    fn main() -> Result<(), oxirush_utils::tree::Error> {
        let mut message = tree!({
            "message-type": "registration-request",
            "requested-nssai": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}],
            "t3512-value": null,
        });

        // Select, by names however they are written; what is not there selects nothing
        println!("{:?}", tree::select(&message, "/Requested NSSAI/*/sst")?);
        println!("{:?}", tree::select(&message, "/t3512-value/unit")?);

        // A name that the value does not have is an error
        println!("{}", tree::select(&message, "/requested-nssai/0/sts").unwrap_err());

        // Each value with the path that selects it
        for (path, value) in tree::paths(&message) {
            println!("{path} = {value}");
        }

        // An edit is done whole, or not at all
        tree::set(&mut message, "/requested-nssai/0/sd", tree!("000001"))?;
        tree::insert(&mut message, "/requested-nssai/-", tree!({"sst": 3}))?;
        tree::remove(&mut message, "/requested-nssai/1")?;
        println!("{}", message["requested-nssai"]);
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 8 of README.md.
fn block_8() {
    use oxirush_utils::tree::{self, Members, Reading, Refused, Tree, Value};

    #[derive(Debug, PartialEq)]
    struct Timer {
        seconds: u32,
    }

    impl Tree for Timer {
        const NAME: &'static str = "GPRS timer";

        fn to_tree(&self) -> Value {
            let mut members = Members::new();
            members.put("seconds", tree::unsigned(self.seconds.into()));
            members.finish()
        }

        fn from_tree(value: &Value, reading: &mut Reading) -> Result<Self, Refused> {
            let members = reading.members(value, &["seconds"])?;
            let seconds = reading.member(&members, 0, |value, reading| reading.unsigned(value, 0, 11160))?;
            Ok(Self { seconds: seconds as u32 })
        }
    }

    // `Serialize` and `Deserialize`, through the tree
    oxirush_utils::impl_serde!(Timer);

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let timer = Timer { seconds: 3600 };
        println!("{}", serde_json::to_string(&timer)?);
        print!("{}", serde_norway::to_string(&timer)?);
        println!("{:?}", serde_norway::from_str::<Timer>("Seconds: 3600")?);
        Ok(())
    }

    main().unwrap();
}

/// The Rust block 1 of src/lib.rs.
fn block_9() {
    use oxirush_utils::identity::{Guti5g, Plmn};

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let plmn: Plmn = "310-410".parse()?;
        let guti = Guti5g::new(plmn, 0xca, 0x3fe, 0x01, 0xdead_beef)?;

        // The three octets of a PLMN in a NAS message, and in an NGAP or S1AP message
        println!("{plmn}: {:02x?} in NAS, {:02x?} in NGAP", plmn.to_nas_octets(), plmn.to_ap_octets());

        // A 5G-GUTI as text, and as a 5GS mobile identity
        println!("{guti}: {:02x?}", guti.to_nas_octets());
        Ok(())
    }

    main().unwrap();
}
