use std::{
    io,
    os::fd::{AsRawFd, BorrowedFd},
};
/// Linux snapshots supplementary groups on connection; never consult /proc/<pid> (PID reuse).
pub fn peer_groups(fd: BorrowedFd<'_>) -> io::Result<Vec<u32>> {
    let mut count = 16usize;
    loop {
        let mut groups = vec![0 as libc::gid_t; count];
        let mut bytes = (groups.len() * std::mem::size_of::<libc::gid_t>()) as libc::socklen_t;
        // SAFETY: fd remains borrowed for this call, groups is initialized and writable for bytes,
        // and bytes points to a live socklen_t. The kernel returns at most the supplied buffer length.
        let result = unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERGROUPS,
                groups.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if result == 0 {
            let used = bytes as usize;
            if used > groups.len() * std::mem::size_of::<libc::gid_t>()
                || !used.is_multiple_of(std::mem::size_of::<libc::gid_t>())
            {
                return Err(io::Error::other("Invalid peer group length"));
            }
            groups.truncate(used / std::mem::size_of::<libc::gid_t>());
            return Ok(groups);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ERANGE) {
            return Err(error);
        }
        let required = (bytes as usize).div_ceil(std::mem::size_of::<libc::gid_t>());
        count = (count * 2).max(required);
        if count > 65536 {
            return Err(io::Error::other("Too many peer groups"));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::os::fd::AsFd;
    #[test]
    fn peer_groups_match_the_process_groups() {
        let (socket, _) = std::os::unix::net::UnixStream::pair().unwrap();
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let mut expected = status
            .lines()
            .find_map(|l| l.strip_prefix("Groups:"))
            .unwrap()
            .split_whitespace()
            .map(|g| g.parse::<u32>().unwrap())
            .collect::<Vec<_>>();
        let mut actual = super::peer_groups(socket.as_fd()).unwrap();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected);
    }
}
