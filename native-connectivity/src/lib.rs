//! Read-only NetworkManager boundary; device-specific radio controls stay outside shared policy.
use pokelinux_power::clock::{BootClock, BootTime, Clock};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

type BoundaryResult<T> = Result<T, Box<dyn std::error::Error>>;
const NAME: &str = "org.freedesktop.NetworkManager";
const PATH: &str = "/org/freedesktop/NetworkManager";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";

#[derive(Debug, Serialize)]
pub struct DeviceObservation {
    pub path: String,
    pub device_type: u32,
    pub state: u32,
}

#[derive(Debug, Serialize)]
pub struct Observation {
    pub bus_id: String,
    pub owner: String,
    pub version: String,
    pub state: u32,
    pub connectivity: u32,
    pub wireless_enabled: bool,
    pub wireless_hardware_enabled: bool,
    pub devices: Vec<DeviceObservation>,
}

struct Deadline {
    last: BootTime,
    end: BootTime,
}
impl Deadline {
    fn new(clock: &impl Clock) -> BoundaryResult<Self> {
        let last = clock.now();
        let end = last
            .checked_after(Duration::from_secs(1))
            .ok_or("deadline_overflow")?;
        Ok(Self { last, end })
    }
    fn check(&mut self, clock: &impl Clock) -> BoundaryResult<()> {
        let now = clock.now();
        if now < self.last || now >= self.end {
            return Err("observation_expired_or_clock_regressed".into());
        }
        self.last = now;
        Ok(())
    }
}

pub struct Endpoint {
    connection: Connection,
    bus_id: String,
    owner: String,
}

impl Endpoint {
    pub fn discover(address: &str) -> BoundaryResult<Self> {
        // Never silently fall back to Harbour's system bus.
        if !address.starts_with("unix:") {
            return Err("explicit Unix D-Bus address required".into());
        }
        let connection = zbus::blocking::connection::Builder::address(address)?
            .method_timeout(Duration::from_millis(250))
            .max_queued(32)
            .build()?;
        let bus = Self::bus(&connection)?;
        let owner: String = bus.call("GetNameOwner", &(NAME,))?;
        let bus_id: String = bus.call("GetId", &())?;
        drop(bus);
        Ok(Self {
            connection,
            bus_id,
            owner,
        })
    }

    fn bus(connection: &Connection) -> zbus::Result<Proxy<'_>> {
        Proxy::new(
            connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
    }

    fn verify_owner(&self) -> BoundaryResult<()> {
        let owner: String = Self::bus(&self.connection)?.call("GetNameOwner", &(NAME,))?;
        if owner != self.owner {
            return Err("owner_replaced".into());
        }
        Ok(())
    }

    fn properties(
        &self,
        path: &str,
        interface: &str,
    ) -> BoundaryResult<HashMap<String, OwnedValue>> {
        let proxy = Proxy::new(
            &self.connection,
            self.owner.as_str(),
            path,
            "org.freedesktop.DBus.Properties",
        )?;
        let response = proxy.call_method("GetAll", &(interface,))?;
        // This bounds our decoding. zbus's upstream wire allocation limit is separate.
        if response.body().len() > 65_536 {
            return Err("properties_size_limit".into());
        }
        let values: HashMap<String, OwnedValue> = response.body().deserialize()?;
        if values.len() > 128 {
            return Err("properties_count_limit".into());
        }
        Ok(values)
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn snapshot(&self) -> BoundaryResult<Observation> {
        self.snapshot_with_clock(&BootClock)
    }

    pub fn snapshot_with_clock(&self, clock: &impl Clock) -> BoundaryResult<Observation> {
        let mut deadline = Deadline::new(clock)?;
        deadline.check(clock)?;
        self.verify_owner()?;
        deadline.check(clock)?;
        let mut values = self.properties(PATH, NAME)?;
        deadline.check(clock)?;
        let version = String::try_from(values.remove("Version").ok_or("missing_version")?)?;
        if version.len() > 128 {
            return Err("version_size_limit".into());
        }
        let state = u32::try_from(values.remove("State").ok_or("missing_state")?)?;
        let connectivity = u32::try_from(
            values
                .remove("Connectivity")
                .ok_or("missing_connectivity")?,
        )?;
        let wireless_enabled = bool::try_from(
            values
                .remove("WirelessEnabled")
                .ok_or("missing_wireless_enabled")?,
        )?;
        let wireless_hardware_enabled = bool::try_from(
            values
                .remove("WirelessHardwareEnabled")
                .ok_or("missing_wireless_hardware")?,
        )?;
        deadline.check(clock)?;
        let nm = Proxy::new(&self.connection, self.owner.as_str(), PATH, NAME)?;
        let response = nm.call_method("GetDevices", &())?;
        deadline.check(clock)?;
        if response.body().len() > 8_192 {
            return Err("devices_size_limit".into());
        }
        let paths: Vec<OwnedObjectPath> = response.body().deserialize()?;
        if paths.len() > 16 {
            return Err("device_count_limit".into());
        }
        let mut devices = Vec::new();
        for path in paths {
            deadline.check(clock)?;
            let mut values = self.properties(path.as_str(), DEVICE)?;
            deadline.check(clock)?;
            devices.push(DeviceObservation {
                path: path.to_string(),
                device_type: u32::try_from(
                    values.remove("DeviceType").ok_or("missing_device_type")?,
                )?,
                state: u32::try_from(values.remove("State").ok_or("missing_device_state")?)?,
            });
        }
        deadline.check(clock)?;
        self.verify_owner()?;
        deadline.check(clock)?;
        Ok(Observation {
            bus_id: self.bus_id.clone(),
            owner: self.owner.clone(),
            version,
            state,
            connectivity,
            wireless_enabled,
            wireless_hardware_enabled,
            devices,
        })
    }
}
