use std::net::UdpSocket;

/// Best-effort detection of the local IPv4 address used to reach the
/// outside world. Opens a UDP socket and queries the kernel's chosen
/// source address for a route to a public IP -- no packets are sent.
/// Returns `None` if no route is available (device offline or no
/// network interfaces up).
pub fn local_ipv4() -> Option<String> {
  let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
  socket.connect("1.1.1.1:80").ok()?;
  let addr = socket.local_addr().ok()?;
  let ip = addr.ip();
  if ip.is_unspecified() { None } else { Some(ip.to_string()) }
}
