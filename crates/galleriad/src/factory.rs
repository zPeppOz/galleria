use crate::manager::BackendFactory;
use galleria_backend_wireguard::{KernelDevice, WireGuardBackend, validate};
use galleria_ipc::*;
use std::{path::PathBuf, sync::Arc};
pub fn real_factory(_daemon_exe: PathBuf) -> BackendFactory {
    Arc::new(|params, options| match params {
        BackendParams::Wireguard(p) => {
            let cfg = validate(p)?;
            let device = KernelDevice::new()
                .map_err(|_| ErrorCode::BackendUnavailable.to_rpc("WireGuard is unavailable"))?;
            Ok(Box::new(WireGuardBackend::new(
                cfg,
                options,
                Box::new(device),
            )))
        }
        BackendParams::Openvpn(_) => {
            Err(ErrorCode::BackendUnavailable.to_rpc("OpenVPN backend is not installed"))
        }
    })
}
pub fn backend_infos() -> Vec<BackendInfo> {
    let available =
        cfg!(target_os = "linux") && std::path::Path::new("/sys/module/wireguard").exists();
    vec![BackendInfo {
        protocol: Protocol::Wireguard,
        available,
        version: None,
        unavailable_reason: (!available).then(|| "WireGuard kernel module is not loaded".into()),
        capabilities: galleria_backend_wireguard::session::capabilities(),
    }]
}
