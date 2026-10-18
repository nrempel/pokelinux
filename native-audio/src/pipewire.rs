//! A bounded backend for a private PipeWire manager socket and preallocated mono nodes.
//! WirePlumber's competing link policy must be disabled for this ownership mode.
use crate::Role;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct Route {
    id: String,
    label: String,
    target: String,
    roles: Vec<Role>,
}

#[derive(Deserialize)]
struct ProfileConfig {
    sources: BTreeMap<String, String>,
    routes: Vec<Route>,
}

/// Trusted backend configuration. Logical selectors are independent of node names.
#[derive(Deserialize)]
#[serde(try_from = "ProfileConfig")]
pub struct Profile {
    pub sources: BTreeMap<String, String>,
    routes: Vec<Route>,
}

/// Configured Media choices only; neither a graph observation nor an availability grant.
#[derive(Serialize)]
pub struct MediaRoute<'a> {
    pub id: &'a str,
    pub label: &'a str,
}

impl TryFrom<ProfileConfig> for Profile {
    type Error = &'static str;

    fn try_from(config: ProfileConfig) -> Result<Self, Self::Error> {
        if config.routes.is_empty() || config.routes.len() > 32 {
            return Err("invalid_route_count");
        }
        let mut ids = BTreeSet::new();
        for route in &config.routes {
            if route.id.is_empty()
                || route.id.len() > 64
                || !route
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                || !ids.insert(route.id.as_str())
            {
                return Err("invalid_or_duplicate_route_id");
            }
            if route.label.trim().is_empty()
                || route.label.len() > 96
                || route.label.chars().any(char::is_control)
            {
                return Err("invalid_route_label");
            }
            if route.target.is_empty()
                || route.target.len() > 256
                || route.target.chars().any(char::is_control)
            {
                return Err("invalid_route_target");
            }
            if route.roles.is_empty()
                || route.roles.len() > 3
                || route
                    .roles
                    .iter()
                    .enumerate()
                    .any(|(i, role)| route.roles[..i].contains(role))
            {
                return Err("invalid_route_roles");
            }
        }
        Ok(Self {
            sources: config.sources,
            routes: config.routes,
        })
    }
}

impl Profile {
    pub fn source(&self, role: Role) -> Result<&str, &'static str> {
        self.sources
            .get(match role {
                Role::Media => "media",
                Role::Call => "call",
                Role::Capture => "capture",
            })
            .map(String::as_str)
            .ok_or("unsupported_role")
    }
    pub fn media_routes(&self) -> Vec<MediaRoute<'_>> {
        let mut routes: Vec<_> = self
            .routes
            .iter()
            .filter(|route| route.roles.contains(&Role::Media))
            .map(|route| MediaRoute {
                id: &route.id,
                label: &route.label,
            })
            .collect();
        routes.sort_by(|a, b| a.id.cmp(b.id));
        routes
    }

    pub fn target(&self, role: Role, id: &str) -> Result<&str, &'static str> {
        self.routes
            .iter()
            .find(|route| route.id == id && route.roles.contains(&role))
            .map(|route| route.target.as_str())
            .ok_or("unsupported_route")
    }

    pub fn supports(&self, role: Role, route: &str) -> bool {
        self.routes
            .iter()
            .any(|candidate| candidate.id == route && candidate.roles.contains(&role))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedLink {
    pub source: String,
    pub target: String,
    pub state: String,
}

struct Link {
    source: String,
    target: String,
    state: String,
}

pub struct Graph {
    cookie: u64,
    nodes: BTreeMap<String, u64>,
    ports: Vec<(u64, u64, String, String)>,
    links: Vec<Link>,
    source_identities: BTreeMap<String, Result<SourceIdentity, &'static str>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
    pub cookie: u64,
    pub node_serial: u64,
    pub client_serial: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaSource {
    Automatic,
    Owned {
        pid: u32,
        uid: u32,
        gid: u32,
        cookie: u64,
        identity: Option<SourceIdentity>,
    },
    Retiring,
}

pub struct RenderPlan {
    pub desired: Vec<(u64, Role, String)>,
    pub wanted: BTreeSet<(String, String)>,
    pub idle_media: bool,
    pub media_present: bool,
    pub preview_identity: Option<SourceIdentity>,
}

fn number(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

/// Fail the service closed if an owned, killed worker cannot be reaped on time.
/// Returning and dropping a live Child would silently detach it.
fn reap_owned(child: &mut Child, deadline: Instant) {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                eprintln!("owned audio worker cleanup deadline or wait failure");
                std::process::exit(70);
            }
        }
    }
}

/// Every child is bounded, read concurrently, and explicitly waited or killed/reaped.
fn command(program: &str, args: &[String]) -> Result<Vec<u8>, &'static str> {
    let mut child = Command::new(std::env::current_exe().map_err(|_| "missing_worker")?)
        .args(["--tool-worker", &std::process::id().to_string(), program])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "tool_start_failed")?;
    let stdout = child.stdout.take().ok_or("missing_tool_stdout")?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                reap_owned(&mut child, Instant::now() + Duration::from_secs(2));
                break Err("tool_deadline_or_wait_failure");
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "tool_reader_failed")?
        .map_err(|_| "tool_read_failed")?;
    if !status?.success() {
        return Err("tool_failed");
    }
    if bytes.len() == 1024 * 1024 {
        return Err("graph_size_limit");
    }
    Ok(bytes)
}

pub fn read_graph() -> Result<Graph, &'static str> {
    let bytes = command("/usr/bin/pw-dump", &[])?;
    Graph::from_value(serde_json::from_slice(&bytes).map_err(|_| "invalid_graph")?)
}

impl Graph {
    pub fn from_value(value: Value) -> Result<Self, &'static str> {
        let objects: Vec<Value> = serde_json::from_value(value).map_err(|_| "invalid_graph")?;
        let cookie = objects
            .iter()
            .find(|object| {
                object["type"]
                    .as_str()
                    .is_some_and(|t| t.ends_with(":Core"))
            })
            .and_then(|object| number(&object["info"]["cookie"]))
            .ok_or("missing_core_identity")?;
        let mut by_id = BTreeMap::new();
        let mut nodes = BTreeMap::new();
        let mut ports = Vec::new();
        for object in &objects {
            let Some(kind) = object["type"].as_str() else {
                continue;
            };
            if kind.ends_with(":Node") {
                if let (Some(id), Some(name)) = (
                    number(&object["id"]),
                    object["info"]["props"]["node.name"].as_str(),
                ) {
                    if nodes.insert(name.to_owned(), id).is_some() {
                        return Err("ambiguous_node_identity");
                    }
                    by_id.insert(id, name.to_owned());
                }
            } else if kind.ends_with(":Port")
                && let (Some(id), Some(node), Some(direction)) = (
                    number(&object["id"]),
                    number(&object["info"]["props"]["node.id"]),
                    object["info"]["props"]["port.direction"].as_str(),
                )
            {
                ports.push((
                    id,
                    node,
                    direction.to_owned(),
                    object["info"]["props"]["port.name"]
                        .as_str()
                        .ok_or("missing_port_name")?
                        .to_owned(),
                ));
            }
        }
        let mut links = Vec::new();
        for object in &objects {
            if !object["type"]
                .as_str()
                .is_some_and(|t| t.ends_with(":Link"))
            {
                continue;
            }
            if let (Some(id), Some(output), Some(input), Some(state)) = (
                number(&object["id"]),
                number(&object["info"]["output-node-id"]),
                number(&object["info"]["input-node-id"]),
                object["info"]["state"].as_str(),
            ) && let (Some(source), Some(target)) = (by_id.get(&output), by_id.get(&input))
            {
                let _ = id;
                links.push(Link {
                    source: source.clone(),
                    target: target.clone(),
                    state: state.to_owned(),
                });
            }
        }
        let mut source_identities = BTreeMap::new();
        for object in &objects {
            if !object["type"]
                .as_str()
                .is_some_and(|v| v.ends_with(":Node"))
            {
                continue;
            }
            let Some(name) = object["info"]["props"]["node.name"].as_str() else {
                continue;
            };
            let identity = (|| {
                let props = &object["info"]["props"];
                let client_id = number(&props["client.id"]).ok_or("missing_source_client")?;
                let clients: Vec<_> = objects
                    .iter()
                    .filter(|v| {
                        v["type"].as_str().is_some_and(|t| t.ends_with(":Client"))
                            && number(&v["id"]) == Some(client_id)
                    })
                    .collect();
                if clients.len() != 1 {
                    return Err("missing_source_client");
                }
                let security = &clients[0]["info"]["props"];
                let integer = |key: &str| {
                    security[key]
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or("missing_source_security")
                };
                let formats = object["info"]["params"]["EnumFormat"]
                    .as_array()
                    .ok_or("missing_source_format")?;
                if !formats.iter().any(|v| {
                    v["mediaType"] == "audio"
                        && v["mediaSubtype"] == "raw"
                        && v["format"] == "F32LE"
                        && v["rate"] == 48000
                        && v["channels"] == 1
                        && v["position"] == serde_json::json!(["MONO"])
                }) {
                    return Err("unsupported_preview_format");
                }
                Ok(SourceIdentity {
                    pid: integer("pipewire.sec.pid")?,
                    uid: integer("pipewire.sec.uid")?,
                    gid: integer("pipewire.sec.gid")?,
                    cookie,
                    node_serial: props["object.serial"]
                        .as_u64()
                        .ok_or("missing_node_serial")?,
                    client_serial: security["object.serial"]
                        .as_u64()
                        .ok_or("missing_client_serial")?,
                })
            })();
            source_identities.insert(name.to_owned(), identity);
        }
        Ok(Graph {
            cookie,
            nodes,
            ports,
            links,
            source_identities,
        })
    }
}

impl Graph {
    pub fn cookie(&self) -> u64 {
        self.cookie
    }
    pub fn render_plan(
        &self,
        profile: &Profile,
        desired: &[(u64, Role, String)],
        media: &MediaSource,
    ) -> Result<RenderPlan, &'static str> {
        let source = profile.sources.get("media");
        let media_present = source.is_some_and(|s| self.nodes.contains_key(s));
        let mut render = Vec::new();
        let mut idle_media = false;
        let mut preview_identity = None;
        for (lease, role, route) in desired {
            // Selected policy still validates the actual target capability when idle.
            self.mono_port(profile.target(*role, route)?, "in")?;
            if *role == Role::Media {
                match media {
                    MediaSource::Retiring => {
                        idle_media = true;
                        continue;
                    }
                    MediaSource::Automatic if !media_present => {
                        idle_media = true;
                        continue;
                    }
                    MediaSource::Owned {
                        pid,
                        uid,
                        gid,
                        cookie,
                        identity,
                    } => {
                        if !media_present {
                            return Err("preview_source_pending");
                        }
                        let found = self
                            .source_identities
                            .get(profile.source(Role::Media)?)
                            .ok_or("missing_source_identity")?
                            .clone()?;
                        if found.pid != *pid
                            || found.uid != *uid
                            || found.gid != *gid
                            || found.cookie != *cookie
                            || identity.as_ref().is_some_and(|old| old != &found)
                        {
                            return Err("wrong_preview_source_identity");
                        }
                        preview_identity = Some(found);
                    }
                    MediaSource::Automatic => {}
                }
            }
            self.mono_port(profile.source(*role)?, "out")?;
            render.push((*lease, *role, route.clone()));
        }
        let wanted = wanted(profile, &render)?;
        Ok(RenderPlan {
            desired: render,
            wanted,
            idle_media,
            media_present,
            preview_identity,
        })
    }
    pub fn media_present(&self, profile: &Profile) -> bool {
        profile
            .sources
            .get("media")
            .is_some_and(|source| self.nodes.contains_key(source))
    }

    pub fn observed(&self, profile: &Profile) -> Vec<ObservedLink> {
        let mut links: Vec<_> = self
            .links
            .iter()
            .filter(|link| {
                profile
                    .sources
                    .values()
                    .any(|source| *source == link.source)
            })
            .map(|link| ObservedLink {
                source: link.source.clone(),
                target: link.target.clone(),
                state: link.state.clone(),
            })
            .collect();
        links.sort_by(|a, b| (&a.source, &a.target).cmp(&(&b.source, &b.target)));
        links
    }
    fn mono_port(&self, name: &str, direction: &str) -> Result<String, &'static str> {
        let node = self.nodes.get(name).ok_or("route_or_source_unavailable")?;
        let ports: Vec<_> = self
            .ports
            .iter()
            .filter(|(_, id, dir, _)| id == node && dir == direction)
            .collect();
        if ports.len() != 1 {
            return Err("unsupported_channel_layout");
        }
        Ok(format!("{name}:{}", ports[0].3))
    }
}

pub fn wanted(
    profile: &Profile,
    desired: &[(u64, Role, String)],
) -> Result<BTreeSet<(String, String)>, &'static str> {
    desired
        .iter()
        .map(|(_, role, route)| {
            Ok((
                profile.source(*role)?.to_owned(),
                profile.target(*role, route)?.to_owned(),
            ))
        })
        .collect()
}

/// Live non-lingering links belong to kernel-supervised children of one stable actor thread.
#[derive(Default)]
pub struct Backend {
    workers: BTreeMap<(String, String), Child>,
}

impl Backend {
    pub fn stop_all(&mut self) {
        for child in self.workers.values_mut() {
            let _ = child.kill();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        for child in self.workers.values_mut() {
            reap_owned(child, deadline);
        }
        self.workers.clear();
    }

    pub fn apply(
        &mut self,
        profile: &Profile,
        desired: &[(u64, Role, String)],
        graph: &Graph,
    ) -> Result<(), &'static str> {
        let wanted = wanted(profile, desired)?;
        let old: Vec<_> = self
            .workers
            .keys()
            .filter(|key| !wanted.contains(*key))
            .cloned()
            .collect();
        for key in &old {
            if let Some(child) = self.workers.get_mut(key) {
                let _ = child.kill();
            }
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        for key in old {
            if let Some(mut child) = self.workers.remove(&key) {
                reap_owned(&mut child, deadline);
            }
        }
        for (source, target) in wanted {
            let output = graph.mono_port(&source, "out")?;
            let input = graph.mono_port(&target, "in")?;
            let key = (source, target);
            if let Some(child) = self.workers.get_mut(&key) {
                if child.try_wait().map_err(|_| "link_wait_failed")?.is_none() {
                    continue;
                }
                self.workers.remove(&key);
            }
            let child = Command::new(std::env::current_exe().map_err(|_| "missing_worker")?)
                .args([
                    "--tool-worker",
                    &std::process::id().to_string(),
                    "/usr/bin/pw-link",
                    "-m",
                    &output,
                    &input,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|_| "link_start_failed")?;
            self.workers.insert(key, child);
        }
        Ok(())
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.stop_all();
    }
}
