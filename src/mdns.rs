use std::collections::HashMap;

const SERVICE_TYPE: &str = "_agentchat._tcp.local.";

/// Holds the mDNS daemon handle for graceful shutdown
pub struct MdnsHandle {
    daemon: mdns_sd::ServiceDaemon,
    fullname: String,
}

impl MdnsHandle {
    pub fn fullname(&self) -> &str {
        &self.fullname
    }
}

impl Drop for MdnsHandle {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// Start mDNS service advertisement.
/// Returns a handle that keeps the service registered until dropped.
pub fn start_mdns(port: u16, instance_name: &str) -> Result<MdnsHandle, String> {
    let mdns = mdns_sd::ServiceDaemon::new().map_err(|e| format!("mDNS daemon: {e}"))?;

    // Detect local hostname and IP
    let host = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "localhost".to_string());

    let host_fqdn = if host.ends_with(".local.") {
        host.clone()
    } else if host.ends_with(".local") {
        format!("{host}.")
    } else {
        format!("{host}.local.")
    };

    // Try to get a local IP address
    let ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let mut properties = HashMap::new();
    properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    properties.insert("path".to_string(), "/api/v1".to_string());
    properties.insert("protocol".to_string(), "http".to_string());

    let service_info = mdns_sd::ServiceInfo::new(
        SERVICE_TYPE,
        instance_name,
        &host_fqdn,
        &ip,
        port,
        Some(properties),
    )
    .map_err(|e| format!("mDNS service info: {e}"))?;

    let fullname = service_info.get_fullname().to_string();

    mdns.register(service_info)
        .map_err(|e| format!("mDNS register: {e}"))?;

    Ok(MdnsHandle { daemon: mdns, fullname })
}

/// Service type constant for use in discover endpoint
pub fn service_type() -> &'static str {
    SERVICE_TYPE
}
