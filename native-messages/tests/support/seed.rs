//! Authored, test-only AOSP-qualified GSM DELIVER fixture. No modem producer.
fn main() -> std::io::Result<()> {
    use pokelinux_messaging::receipt::{BackendScope, IncomingKind, ReceiptOwner, Slot};
    if std::env::var("NATIVE_MESSAGES_ISOLATED").as_deref() != Ok("1")
        || rustix::process::getuid().as_raw() != 0
    {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(args.len() == 1 || args.len() == 2 && args[1] == "--renderer-text") {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let mut raw = ReceiptOwner::open(std::path::Path::new(&args[0]))
        .map_err(|_| std::io::ErrorKind::Other)?;
    let s = raw
        .attach(Slot(0), BackendScope::Unqualified)
        .map_err(|_| std::io::ErrorKind::Other)?;
    let h = "000008915155100000006201213141510005e8329bfd06";
    let bytes: Vec<_> = (0..h.len())
        .step_by(2)
        .map(|n| u8::from_str_radix(&h[n..n + 2], 16).unwrap())
        .collect();
    let bytes = if args.len() == 2 {
        let text = "<b>synthetic plain text</b>";
        let mut pdu = vec![
            0, 0, 8, 0x91, 0x51, 0x55, 0x10, 0, 0, 8, 0x62, 1, 0x21, 0x31, 0x41, 0x51, 0,
        ];
        let body: Vec<_> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
        pdu.push(body.len() as u8);
        pdu.extend(body);
        pdu
    } else {
        bytes
    };
    for _ in 0..2 {
        raw.admit(s, IncomingKind::Deliver, &bytes)
            .map_err(|_| std::io::ErrorKind::Other)?;
    }
    Ok(())
}
