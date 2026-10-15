//! Protected provisioning input. Loading alone grants no operation authority.
use pokelinux_cellular_boundary::DataProfileInfo;
use pokelinux_cellular_boundary::generated::android::hardware::radio::{
    AccessNetwork::AccessNetwork,
    data::{DataRequestReason::DataRequestReason, LinkAddress::LinkAddress, SliceInfo::SliceInfo},
};
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct Setup {
    pub serial: i32,
    pub access_network: AccessNetwork,
    pub profile: DataProfileInfo,
    pub roaming_allowed: bool,
    pub reason: DataRequestReason,
    pub addresses: Vec<LinkAddress>,
    pub dnses: Vec<String>,
    pub pdu_session_id: i32,
    pub slice: Option<SliceInfo>,
    pub match_all_rule_allowed: bool,
}
pub struct Config {
    pub device: PathBuf,
    pub service: String,
    pub expected_callback_uid: u32,
    pub setup: Setup,
}
impl Config {
    /// Provisioning premise only. Loading does not authorize a modem operation.
    pub fn load(path: &Path) -> io::Result<Self> {
        Self::load_identified(path).map(|(config, _)| config)
    }
    pub fn load_identified(path: &Path) -> io::Result<(Self, [u8; 32])> {
        use sha2::{Digest, Sha256};
        let bytes = protected_bytes(path)?;
        let identity: [u8; 32] = Sha256::digest(&bytes).into();
        Ok((decode(&bytes)?, identity))
    }
    /// Same complete protected contents that Main validated, before worker Binder initialization.
    pub fn load_matching(path: &Path, expected: [u8; 32]) -> io::Result<Self> {
        let (config, actual) = Self::load_identified(path)?;
        if actual != expected {
            return Err(invalid());
        }
        Ok(config)
    }
}

const MAX_BYTES: usize = 32768;
fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
fn protected_bytes(path: &Path) -> io::Result<Vec<u8>> {
    use rustix::fs::{Mode, OFlags, open, openat};
    use std::{fs::File, io::Read, os::unix::fs::MetadataExt};
    if !path.is_absolute() {
        return Err(invalid());
    }
    let parent = path.parent().ok_or_else(invalid)?;
    let name = path.file_name().ok_or_else(invalid)?;
    let directory = File::from(open(
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let owner = rustix::process::geteuid().as_raw();
    let metadata = directory.metadata()?;
    if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o777 != 0o700 {
        return Err(invalid());
    }
    let mut file = File::from(openat(
        &directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let before = file.metadata()?;
    if !before.is_file()
        || before.uid() != owner
        || before.mode() & 0o777 != 0o600
        || before.nlink() != 1
        || before.len() == 0
        || before.len() > MAX_BYTES as u64
    {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() > MAX_BYTES
        || bytes.len() as u64 != before.len()
        || (
            before.dev(),
            before.ino(),
            before.len(),
            before.uid(),
            before.mode(),
            before.nlink(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
        ) != (
            after.dev(),
            after.ino(),
            after.len(),
            after.uid(),
            after.mode(),
            after.nlink(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return Err(invalid());
    }
    Ok(bytes)
}
use serde_json::Value;
struct Object(serde_json::Map<String, Value>);
impl Object {
    fn new(value: Value) -> io::Result<Self> {
        if let Value::Object(value) = value {
            Ok(Self(value))
        } else {
            Err(invalid())
        }
    }
    fn take(&mut self, name: &str) -> io::Result<Value> {
        self.0.remove(name).ok_or_else(invalid)
    }
    fn int(&mut self, name: &str) -> io::Result<i32> {
        i32::try_from(integer(self.take(name)?)?).map_err(|_| invalid())
    }
    fn long(&mut self, name: &str) -> io::Result<i64> {
        integer(self.take(name)?)
    }
    fn boolean(&mut self, name: &str) -> io::Result<bool> {
        self.take(name)?.as_bool().ok_or_else(invalid)
    }
    fn text(&mut self, name: &str, maximum: usize) -> io::Result<String> {
        text(self.take(name)?, maximum)
    }
    fn done(self) -> io::Result<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(invalid())
        }
    }
}
fn integer(value: Value) -> io::Result<i64> {
    value.as_i64().ok_or_else(invalid)
}
fn text(value: Value, maximum: usize) -> io::Result<String> {
    if let Value::String(value) = value
        && value.len() <= maximum
        && !value.as_bytes().contains(&0)
    {
        return Ok(value);
    }
    Err(invalid())
}
fn array(value: Value, maximum: usize) -> io::Result<Vec<Value>> {
    if let Value::Array(value) = value
        && value.len() <= maximum
    {
        Ok(value)
    } else {
        Err(invalid())
    }
}
fn descriptor(value: Value) -> io::Result<pokelinux_cellular_boundary::generated::android::hardware::radio::data::TrafficDescriptor::TrafficDescriptor>{
    use pokelinux_cellular_boundary::generated::android::hardware::radio::data::{
        OsAppId::OsAppId, TrafficDescriptor::TrafficDescriptor,
    };
    let mut object = Object::new(value)?;
    let dnn = match object.take("dnn")? {
        Value::Null => None,
        v => Some(text(v, 128)?),
    };
    let os_app_id = match object.take("osAppId")? {
        Value::Null => None,
        value => Some(OsAppId {
            osAppId: array(value, 256)?
                .into_iter()
                .map(|v| u8::try_from(integer(v)?).map_err(|_| invalid()))
                .collect::<io::Result<_>>()?,
        }),
    };
    object.done()?;
    Ok(TrafficDescriptor {
        dnn,
        osAppId: os_app_id,
    })
}
fn profile(value: Value) -> io::Result<DataProfileInfo> {
    use pokelinux_cellular_boundary::generated::android::hardware::radio::data::{
        ApnAuthType::ApnAuthType, PdpProtocolType::PdpProtocolType,
    };
    let mut o = Object::new(value)?;
    let result = DataProfileInfo {
        profileId: o.int("profileId")?,
        apn: o.text("apn", 128)?,
        protocol: PdpProtocolType(o.int("protocol")?),
        roamingProtocol: PdpProtocolType(o.int("roamingProtocol")?),
        authType: ApnAuthType(o.int("authType")?),
        user: o.text("user", 128)?,
        password: o.text("password", 128)?,
        r#type: o.int("type")?,
        maxConnsTime: o.int("maxConnsTime")?,
        maxConns: o.int("maxConns")?,
        waitTime: o.int("waitTime")?,
        enabled: o.boolean("enabled")?,
        supportedApnTypesBitmap: o.int("supportedApnTypesBitmap")?,
        bearerBitmap: o.int("bearerBitmap")?,
        mtuV4: o.int("mtuV4")?,
        mtuV6: o.int("mtuV6")?,
        preferred: o.boolean("preferred")?,
        persistent: o.boolean("persistent")?,
        alwaysOn: o.boolean("alwaysOn")?,
        trafficDescriptor: descriptor(o.take("trafficDescriptor")?)?,
        infrastructureBitmap: o.int("infrastructureBitmap")?,
    };
    o.done()?;
    Ok(result)
}
fn address(value: Value) -> io::Result<LinkAddress> {
    let mut o = Object::new(value)?;
    let result = LinkAddress {
        address: o.text("address", 128)?,
        addressProperties: o.int("addressProperties")?,
        deprecationTime: o.long("deprecationTime")?,
        expirationTime: o.long("expirationTime")?,
    };
    o.done()?;
    Ok(result)
}
fn slice(value: Value) -> io::Result<Option<SliceInfo>> {
    if value.is_null() {
        return Ok(None);
    }
    let mut o = Object::new(value)?;
    let result = SliceInfo {
        sliceServiceType: i8::try_from(o.int("sliceServiceType")?).map_err(|_| invalid())?,
        sliceDifferentiator: o.int("sliceDifferentiator")?,
        mappedHplmnSst: i8::try_from(o.int("mappedHplmnSst")?).map_err(|_| invalid())?,
        mappedHplmnSd: o.int("mappedHplmnSd")?,
        status: i8::try_from(o.int("status")?).map_err(|_| invalid())?,
    };
    o.done()?;
    Ok(Some(result))
}
fn setup(value: Value) -> io::Result<Setup> {
    let mut o = Object::new(value)?;
    let result = Setup {
        serial: o.int("serial")?,
        access_network: AccessNetwork(o.int("accessNetwork")?),
        profile: profile(o.take("dataProfileInfo")?)?,
        roaming_allowed: o.boolean("roamingAllowed")?,
        reason: DataRequestReason(o.int("reason")?),
        addresses: array(o.take("addresses")?, 4)?
            .into_iter()
            .map(address)
            .collect::<io::Result<_>>()?,
        dnses: array(o.take("dnses")?, 4)?
            .into_iter()
            .map(|v| text(v, 128))
            .collect::<io::Result<_>>()?,
        pdu_session_id: o.int("pduSessionId")?,
        slice: slice(o.take("sliceInfo")?)?,
        match_all_rule_allowed: o.boolean("matchAllRuleAllowed")?,
    };
    o.done()?;
    Ok(result)
}
fn decode(bytes: &[u8]) -> io::Result<Config> {
    let mut o = Object::new(serde_json::from_slice(bytes).map_err(|_| invalid())?)?;
    if o.int("schema")? != 1 {
        return Err(invalid());
    }
    let device = PathBuf::from(o.text("device", 256)?);
    let service = o.text("service", 128)?;
    let instance = service
        .strip_prefix("android.hardware.radio.data.IRadioData/")
        .ok_or_else(invalid)?;
    if !device.is_absolute()
        || instance.is_empty()
        || !instance
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(invalid());
    }
    let expected_callback_uid =
        u32::try_from(o.long("expected_callback_uid")?).map_err(|_| invalid())?;
    let setup = setup(o.take("request")?)?;
    o.done()?;
    Ok(Config {
        device,
        service,
        expected_callback_uid,
        setup,
    })
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
