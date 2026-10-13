//! Windows-first LAN networking prototype.
//! Protocol v1 is deliberately independent from the WoW realm protocol.
//! Run: cargo run --release -p survivors-coop -- host 0.0.0.0:45454
//!      cargo run --release -p survivors-coop -- join 127.0.0.1:45454
//! This is a transport test harness, not yet hooked into the game simulation.
use std::{
    collections::BTreeMap,
    env,
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const MAX_PLAYERS: usize = 4;
const MAX_LINE: usize = 256;
const TICK: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug)]
struct Position { x: f32, z: f32 }

#[derive(Default)]
struct Lobby {
    players: BTreeMap<u8, Position>,
    peers: BTreeMap<u8, TcpStream>,
}

fn parse_move(line: &str) -> Option<Position> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "MOVE" { return None; }
    let x: f32 = parts.next()?.parse().ok()?;
    let z: f32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !x.is_finite() || !z.is_finite() { return None; }
    // Reject implausible coordinates before they enter the shared state.
    if x.abs() > 100_000.0 || z.abs() > 100_000.0 { return None; }
    Some(Position { x, z })
}

fn snapshot(players: &BTreeMap<u8, Position>) -> String {
    let mut out = String::from("STATE");
    for (id, p) in players {
        out.push_str(&format!(" {id}:{:.3}:{:.3}", p.x, p.z));
    }
    out.push('\n');
    out
}

fn run_host(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let shared = Arc::new(Mutex::new(Lobby::default()));
    println!("Azeroth Survivors LAN lobby listening on {addr}; max {MAX_PLAYERS} players");
    let broadcast = Arc::clone(&shared);
    thread::spawn(move || loop {
        thread::sleep(TICK);
        let mut lobby = broadcast.lock().expect("lobby lock");
        let msg = snapshot(&lobby.players);
        let mut disconnected = Vec::new();
        for (&id, peer) in &mut lobby.peers {
            if peer.write_all(msg.as_bytes()).is_err() {
                disconnected.push(id);
            }
        }
        for id in disconnected {
            lobby.peers.remove(&id);
            lobby.players.remove(&id);
        }
    });
    loop {
        match listener.accept() {
            Ok((mut socket, remote)) => {
                socket.set_nodelay(true)?;
                socket.set_write_timeout(Some(Duration::from_millis(250)))?;
                let id = {
                    let mut lobby = shared.lock().expect("lobby lock");
                    let vacant = (1..=MAX_PLAYERS as u8)
                        .find(|id| !lobby.players.contains_key(id));
                    if let Some(id) = vacant {
                        // Send the handshake before exposing the connection to the snapshot loop.
                        socket.write_all(format!("WELCOME {id}\n").as_bytes())?;
                        lobby.players.insert(id, Position { x: 0.0, z: 0.0 });
                        lobby.peers.insert(id, socket.try_clone()?);
                    }
                    vacant
                };
                if let Some(id) = id {
                    println!("Player {id} connected from {remote}");
                    let lobby = Arc::clone(&shared);
                    thread::spawn(move || {
                        let _ = handle_peer(socket, id, &lobby);
                        let mut state = lobby.lock().expect("lobby lock");
                        state.players.remove(&id);
                        state.peers.remove(&id);
                        println!("Player {id} disconnected");
                    });
                } else {
                    let _ = socket.write_all(b"FULL\n");
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock =>
                thread::sleep(Duration::from_millis(20)),
            Err(err) => return Err(err),
        }
    }
}

fn handle_peer(socket: TcpStream, id: u8, shared: &Arc<Mutex<Lobby>>) -> io::Result<()> {
    let reader = BufReader::new(socket);
    for line in reader.lines() {
        let line = line?;
        if line.len() > MAX_LINE { break; }
        if let Some(p) = parse_move(&line) {
            if let Some(current) = shared.lock().expect("lobby lock").players.get_mut(&id) {
                *current = p;
            }
        }
    }
    Ok(())
}

fn run_join(addr: &str) -> io::Result<()> {
    let mut socket = TcpStream::connect(addr)?;
    socket.set_nodelay(true)?;
    let mut reader = BufReader::new(socket.try_clone()?);
    let mut welcome = String::new();
    reader.read_line(&mut welcome)?;
    print!("{welcome}");
    if !welcome.starts_with("WELCOME ") {
        return Err(io::Error::other("lobby rejected connection"));
    }
    thread::spawn(move || {
        for line in reader.lines() {
            match line {
                Ok(line) if line.starts_with("STATE") => println!("{line}"),
                _ => break,
            }
        }
    });
    println!("Type coordinates as: MOVE 10 20 (Ctrl+C to quit)");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if parse_move(&line).is_some() {
            socket.write_all(line.as_bytes())?;
            socket.write_all(b"\n")?;
        } else {
            eprintln!("Expected MOVE <x> <z>, finite coordinates");
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, mode, addr] if mode == "host" => run_host(addr),
        [_, mode, addr] if mode == "join" => run_join(addr),
        _ => {
            eprintln!("Usage: survivors-coop host 0.0.0.0:45454 | join HOST_IP:45454");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn move_input_is_validated() {
        assert!(parse_move("MOVE 1.2 -5").is_some());
        for bad in ["MOVE NaN 0", "MOVE 1 inf", "MOVE 1", "MOVE 1 2 3", "MOVE 100001 0"] {
            assert!(parse_move(bad).is_none(), "{bad}");
        }
    }
    #[test]
    fn snapshots_are_stably_ordered() {
        let mut p = BTreeMap::new();
        p.insert(2, Position { x: 3.0, z: 4.0 });
        p.insert(1, Position { x: 1.0, z: 2.0 });
        assert_eq!(snapshot(&p), "STATE 1:1.000:2.000 2:3.000:4.000\n");
    }
}
