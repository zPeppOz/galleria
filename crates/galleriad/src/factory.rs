use crate::manager::BackendFactory;
use galleria_backend_wireguard::{KernelDevice, WireGuardBackend, validate};
use galleria_ipc::*;
use std::{path::PathBuf, sync::Arc};
pub fn real_factory(daemon_exe: PathBuf) -> BackendFactory {
    factory_with_probe(daemon_exe, Arc::new(wireguard_available))
}
fn factory_with_probe(
    daemon_exe: PathBuf,
    probe: Arc<dyn Fn() -> bool + Send + Sync>,
) -> BackendFactory {
    Arc::new(move |params, options| match params {
        BackendParams::Wireguard(p) => {
            let cfg = validate(p)?;
            if !probe() {
                return Err(
                    ErrorCode::BackendUnavailable.to_rpc("WireGuard kernel module is unavailable")
                );
            }
            let device = KernelDevice::new()
                .map_err(|_| ErrorCode::BackendUnavailable.to_rpc("WireGuard is unavailable"))?;
            Ok(Box::new(WireGuardBackend::new(
                cfg,
                options,
                Box::new(device),
            )))
        }
        BackendParams::Openvpn(p) => {
            let cfg = galleria_backend_openvpn::config::validate(p)?;
            let bin = galleria_backend_openvpn::probe().map_err(|_| {
                ErrorCode::BackendUnavailable.to_rpc("OpenVPN 2.6 or newer is unavailable")
            })?;
            Ok(Box::new(galleria_backend_openvpn::OpenVpnBackend::new(
                cfg,
                bin,
                options.clone(),
                daemon_exe.clone(),
            )))
        }
    })
}
pub fn wireguard_available() -> bool {
    if std::path::Path::new("/sys/module/wireguard").exists() {
        return true;
    }
    ["/usr/sbin/modprobe", "/sbin/modprobe"].iter().any(|path| {
        std::process::Command::new(path)
            .args(["--dry-run", "--quiet", "wireguard"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}
fn wireguard_info(available: bool) -> BackendInfo {
    BackendInfo {
        protocol: Protocol::Wireguard,
        available,
        version: None,
        unavailable_reason: (!available).then(|| "wireguard_module_unavailable".into()),
        capabilities: galleria_backend_wireguard::session::capabilities(),
    }
}
pub fn backend_infos() -> Vec<BackendInfo> {
    let (available, version, reason) = match galleria_backend_openvpn::probe() {
        Ok(bin) => (
            true,
            Some(format!("{}.{}", bin.version.major, bin.version.minor)),
            None,
        ),
        Err(galleria_backend_openvpn::Unavailable::NotFound) => {
            (false, None, Some("openvpn_not_found".into()))
        }
        Err(galleria_backend_openvpn::Unavailable::TooOld(v)) => (
            false,
            Some(format!("{}.{}", v.major, v.minor)),
            Some("openvpn_too_old".into()),
        ),
    };
    vec![
        wireguard_info(wireguard_available()),
        BackendInfo {
            protocol: Protocol::Openvpn,
            available,
            version,
            unavailable_reason: reason,
            capabilities: galleria_backend_openvpn::session::capabilities(),
        },
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn discovery_checks_loadability_each_time() {
        let loaded = std::sync::atomic::AtomicBool::new(false);
        let probe = || loaded.load(std::sync::atomic::Ordering::SeqCst);
        assert!(!wireguard_info(probe()).available);
        loaded.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(wireguard_info(probe()).available);
        assert_eq!(
            wireguard_info(false).unavailable_reason.as_deref(),
            Some("wireguard_module_unavailable")
        );
    }
    #[test]
    fn missing_module_is_backend_unavailable_at_connect() {
        let f = factory_with_probe(PathBuf::new(), Arc::new(|| false));
        let p = BackendParams::Wireguard(serde_json::from_value(serde_json::json!({"private_key":"AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=","addresses":["10.0.0.1/24"],"dns":[],"search_domains":[],"mtu":null,"peers":[{"public_key":"AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=","allowed_ips":["10.0.0.0/24"],"endpoint":null,"preshared_key":null,"persistent_keepalive":null}]})).unwrap());
        let opts = serde_json::from_value(serde_json::json!({"ipv6_block":true,"ignore_pushed_routes":false,"reconnect":{"enabled":false}})).unwrap();
        assert_eq!(
            f(&p, &opts).err().unwrap().data.unwrap()["code"],
            "backend_unavailable"
        );
    }
}
