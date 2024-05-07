use async_std::task;
use byteorder::BigEndian;
use byteorder::ByteOrder;
use dotenv::dotenv;
use std::io;
use std::net::UdpSocket;
use std::time::Duration;
use std::time::SystemTime;

const HOSTNAME_KEY: &str = "MUMBLE_HOST";
const PORT_KEY: &str = "MUMBLE_PORT";
const TIMEOUT: &u64 = &1;

struct MumbleHostInfo {
    hostname: String,
    port: u16,
}

// See https://wiki.mumble.info/wiki/Protocol for details on the packet
#[derive(Debug)]
pub struct PingResponse {
    pub version: u32,
    pub identity: u64,
    pub users: u32,
    pub max_users: u32,
    pub bandwidth: u32,
}

fn unpack_ping_response(packet: &[u8]) -> PingResponse {
    PingResponse {
        version: BigEndian::read_u32(&packet[0..4]),
        identity: BigEndian::read_u64(&packet[4..12]),
        users: BigEndian::read_u32(&packet[12..16]),
        max_users: BigEndian::read_u32(&packet[16..20]),
        bandwidth: BigEndian::read_u32(&packet[20..24]),
    }
}

fn get_mumble_info_from_env() -> MumbleHostInfo {
    dotenv().ok().expect("Create a .env file");
    return MumbleHostInfo {
        hostname: dotenv::var(HOSTNAME_KEY).expect("Please add the hostname to .env"),
        port: dotenv::var(PORT_KEY)
            .expect("Please add the port to .env")
            .parse::<u16>()
            .expect("Please add a valid port number"),
    };
}

pub async fn ping_mumble() -> Option<PingResponse> {
    // Set up socket
    let socket = UdpSocket::bind("0.0.0.0:12345")
        .ok()
        .expect("Failed to bind socket");
    socket.set_nonblocking(true).ok();
    socket
        .set_read_timeout(Some(Duration::new(*TIMEOUT, 0)))
        .ok();
    let hostinfo = get_mumble_info_from_env();
    let server_address = format!("{}:{}", hostinfo.hostname, hostinfo.port);

    // Ping mumble server
    let mut ping_packet = [0; 12];
    // timestamp for identity
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Unable to get the current time");

    ping_packet[..4].copy_from_slice(&0i32.to_be_bytes());
    ping_packet[4..].copy_from_slice(&now.as_secs().to_be_bytes());
    socket.send_to(&ping_packet, server_address).ok();

    // Wait for a response from the server
    let mut buffer = [0; 24];

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((len, server)) => {
                let response = unpack_ping_response(&buffer[..len]);
                println!(
                    "Received response: {:?} from server: {:?}",
                    response, server
                );
                return Some(response);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                eprintln!("Would block error: {:?}", e);
                task::sleep(Duration::from_secs(*TIMEOUT)).await;
            }
            Err(e) => {
                eprintln!("Failed to get a response from server: {:?}", e);
                return None::<PingResponse>;
            }
        }
    }
}
