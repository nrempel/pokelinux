//! Fixed installed permission-controller identity; requests cannot select a scope.
pub const APP: &str = "org.omarchy.Permissions";
pub const UID: u32 = 11045;

/// Exact installation-owned scopes; callers cannot choose either field over IPC.
pub fn supported_scope(target: &str, permission: crate::Permission) -> bool {
    (target == crate::audio::APP && permission == crate::Permission::MediaRoute)
        || (target == crate::connectivity::APP && permission == crate::Permission::NetworkControl)
}
