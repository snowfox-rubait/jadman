use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use anyhow::{Result, anyhow};

pub struct Socks5Server {
    port: u16,
    token: String,
}

impl Socks5Server {
    pub fn new(port: u16, token: String) -> Self {
        Self { port, token }
    }

    pub async fn run(&self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;
        println!("SOCKS5 TCP-Tuner Proxy listening on {}", addr);

        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(val) => val,
                Err(_) => continue,
            };

            let token = self.token.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_socks_connection(&mut socket, &token).await {
                    eprintln!("SOCKS5 connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_socks_connection(socket: &mut TcpStream, expected_token: &str) -> Result<()> {
    // 1. Handshake
    let mut header = [0u8; 2];
    socket.read_exact(&mut header).await?;
    if header[0] != 5 {
        return Err(anyhow!("Unsupported SOCKS version"));
    }
    let nmethods = header[1] as usize;
    let mut methods = vec![0u8; nmethods];
    socket.read_exact(&mut methods).await?;

    // Enforce Username/Password Authentication (0x02)
    socket.write_all(&[5, 2]).await?;

    // Read Auth Request
    let mut auth_ver = [0u8; 1];
    socket.read_exact(&mut auth_ver).await?;
    if auth_ver[0] != 1 {
        return Err(anyhow!("Unsupported auth version"));
    }
    let mut ulen = [0u8; 1];
    socket.read_exact(&mut ulen).await?;
    let mut uname = vec![0u8; ulen[0] as usize];
    socket.read_exact(&mut uname).await?;
    
    let mut plen = [0u8; 1];
    socket.read_exact(&mut plen).await?;
    let mut pass = vec![0u8; plen[0] as usize];
    socket.read_exact(&mut pass).await?;

    if String::from_utf8_lossy(&uname) != "jadm" || String::from_utf8_lossy(&pass) != expected_token {
        let _ = socket.write_all(&[1, 1]).await; // Auth failed
        return Err(anyhow!("SOCKS5 auth failed"));
    }
    socket.write_all(&[1, 0]).await?; // Auth success

    // 2. Request
    let mut req_header = [0u8; 4];
    socket.read_exact(&mut req_header).await?;
    if req_header[0] != 5 {
        return Err(anyhow!("Unsupported SOCKS version in request"));
    }
    if req_header[1] != 1 {
        return Err(anyhow!("Only SOCKS5 CONNECT is supported"));
    }

    let atyp = req_header[3];
    let host = match atyp {
        1 => { // IPv4
            let mut ip = [0u8; 4];
            socket.read_exact(&mut ip).await?;
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3])).to_string()
        }
        3 => { // Domain Name
            let len = socket.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            socket.read_exact(&mut domain).await?;
            String::from_utf8(domain)?
        }
        4 => { // IPv6
            let mut ip = [0u8; 16];
            socket.read_exact(&mut ip).await?;
            let mut ip6 = [0u16; 8];
            for i in 0..8 {
                ip6[i] = ((ip[i * 2] as u16) << 8) | (ip[i * 2 + 1] as u16);
            }
            std::net::IpAddr::V6(std::net::Ipv6Addr::new(ip6[0], ip6[1], ip6[2], ip6[3], ip6[4], ip6[5], ip6[6], ip6[7])).to_string()
        }
        _ => return Err(anyhow!("Unsupported address type")),
    };

    let port = socket.read_u16().await?;

    // Connect to destination
    let dest_addr = format!("{}:{}", host, port);
    
    // Resolve and filter
    let mut resolved = tokio::net::lookup_host(&dest_addr).await?;
    let ip_addr = resolved.next().ok_or_else(|| anyhow!("Failed to resolve destination"))?;
    
    if is_private_or_local_ip(ip_addr.ip()) {
        // Send connection refused / forbidden
        let _ = socket.write_all(&[5, 2, 0, 1, 0, 0, 0, 0, 0, 0]).await;
        return Err(anyhow!("Destination IP is private or local. SSRF blocked."));
    }

    let remote_stream = TcpStream::connect(ip_addr).await?;

    // Tune socket options on the outgoing remote socket!
    #[cfg(unix)]
    tune_socket_for_windows_spoofing(&remote_stream);

    // Respond success
    socket.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await?;

    // Bidirectional copy
    let mut remote_stream = remote_stream;
    tokio::io::copy_bidirectional(socket, &mut remote_stream).await?;

    Ok(())
}

fn is_private_or_local_ip(addr: std::net::IpAddr) -> bool {
    match addr {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            let is_private = octets[0] == 10 || 
                             (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) || 
                             (octets[0] == 192 && octets[1] == 168);
            let is_loopback = octets[0] == 127;
            let is_link_local = octets[0] == 169 && octets[1] == 254;
            is_private || is_loopback || is_link_local || octets[0] == 0
        }
        std::net::IpAddr::V6(v6) => {
            let segments = v6.segments();
            let is_loopback = v6 == std::net::Ipv6Addr::LOCALHOST;
            let is_unique_local = (segments[0] & 0xfe00) == 0xfc00;
            let is_link_local = (segments[0] & 0xffc0) == 0xfe80;
            is_loopback || is_unique_local || is_link_local || v6 == std::net::Ipv6Addr::UNSPECIFIED
        }
    }
}

#[cfg(unix)]
fn tune_socket_for_windows_spoofing(stream: &TcpStream) {
    let fd = stream.as_raw_fd();
    unsafe {
        // Basic TCP/IP option tuning. 
        // Note: Changing TTL and MSS alone is insufficient to spoof an OS against modern DPI (which uses TLS fingerprints, etc.),
        // but it provides a baseline consistency with typical Windows client behavior at the network layer.
        let ttl: libc::c_int = 128;
        let _ = libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_TTL,
            &ttl as *const _ as *const _,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t
        );

        let mss: libc::c_int = 1440;
        let _ = libc::setsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_MAXSEG,
            &mss as *const _ as *const _,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t
        );

        let rcvbuf: libc::c_int = 65536;
        let _ = libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVBUF,
            &rcvbuf as *const _ as *const _,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t
        );
    }
}
