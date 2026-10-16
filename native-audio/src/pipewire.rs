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
    let objects: Vec<Value> = serde_json::from_slice(&bytes).map_err(|_| "invalid_graph")?;
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
    Ok(Graph {
        cookie,
        nodes,
        ports,
        links,
    })
}

impl Graph {
    pub fn cookie(&self) -> u64 {
        self.cookie
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
