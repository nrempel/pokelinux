use pokelinux_inbox::{Inbox, ProjectionState};
use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
use pokelinux_messaging::receive::ReferenceWidth;
fn unhex(s: &str) -> Vec<u8> {
    if s == "-" {
        return Vec::new();
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn durable_projected_text_metadata_matches_33_recorded_original_aosp_decoder_outputs() {
    let root = std::env::temp_dir().join(format!("inbox-original-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let mut receipts = ReceiptOwner::open(&root.join("raw")).unwrap();
    let mut inbox = Inbox::open(&root.join("inbox")).unwrap();
    let c = inbox.control();
    c.set_import_allowed(true).unwrap();
    c.set_read_allowed(true).unwrap();
    let import = c.import_lease().unwrap();
    let read = c.read_lease().unwrap();
    let s = receipts.attach(Slot(0), BackendScope::Unqualified).unwrap();
    let mut count = 0;
    for line in include_str!("../../native-messaging/tests/fixtures/aosp-deliver.tsv").lines() {
        let f: Vec<_> = line.split('\t').collect();
        assert_eq!(f.len(), 9);
        assert_eq!(f[8], "PASS");
        let id = receipts
            .admit(s, IncomingKind::Deliver, &unhex(f[2]))
            .unwrap();
        let r = inbox.import(&import, &receipts, id).unwrap();
        assert_eq!(r.state, ProjectionState::Text);
        let m = inbox.message(&read, r.receipt).unwrap().unwrap();
        assert_eq!(m.sender.as_bytes(), unhex(f[3]), "{}", f[1]);
        assert_eq!(m.text.as_bytes(), unhex(f[4]), "{}", f[1]);
        assert_eq!(m.timestamp_millis, f[5].parse::<i64>().unwrap());
        assert_eq!(m.reply_path, f[7] == "1");
        if f[6] == "-" {
            assert!(m.concatenation.is_none())
        } else {
            let v: Vec<_> = f[6].split(':').map(|s| s.parse::<u16>().unwrap()).collect();
            let x = m.concatenation.unwrap();
            assert_eq!(
                [
                    if x.width == ReferenceWidth::Eight {
                        8
                    } else {
                        16
                    },
                    x.reference,
                    u16::from(x.total),
                    u16::from(x.sequence)
                ],
                v.as_slice()
            );
        }
        count += 1;
    }
    assert_eq!(count, 33);
    assert_eq!(inbox.stats(&read).unwrap().text_messages, 33);
    drop(inbox);
    let inbox = Inbox::open(&root.join("inbox")).unwrap();
    let c = inbox.control();
    c.set_read_allowed(true).unwrap();
    assert_eq!(
        inbox.stats(&c.read_lease().unwrap()).unwrap().text_messages,
        33
    );
    drop(inbox);
    drop(receipts);
    std::fs::remove_dir_all(root).unwrap();
}
