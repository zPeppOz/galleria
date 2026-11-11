use std::{io, os::fd::AsFd};
pub struct PeerCreds {
    pub uid: u32,
    pub gid: u32,
    pub groups: Vec<u32>,
    pub pid: Option<i32>,
}
pub struct AccessPolicy {
    pub allowed_gid: Option<u32>,
}
pub fn authorize(creds: &PeerCreds, policy: &AccessPolicy) -> bool {
    creds.uid == 0
        || policy
            .allowed_gid
            .is_some_and(|gid| creds.gid == gid || creds.groups.contains(&gid))
}
pub fn peer_creds(stream: &tokio::net::UnixStream) -> io::Result<PeerCreds> {
    let creds = stream.peer_cred()?;
    Ok(PeerCreds {
        uid: creds.uid(),
        gid: creds.gid(),
        pid: creds.pid(),
        groups: crate::peer_groups::peer_groups(stream.as_fd())?,
    })
}
