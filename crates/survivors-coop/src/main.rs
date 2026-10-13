//! Windows-first LAN co-op lobby prototype. This is not yet connected to the game simulation.
//! Launch host: survivors-coop host 0.0.0.0:45454
//! Join:        survivors-coop join 192.168.1.10:45454
use std::{
    collections::BTreeMap,
    env,
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

const MAX_PLAYERS: usize = 4;
const PORT: u16 = 45454;

#[derive(Clone, Copy)]
struct Player { ready: bool }

#[derive(Default)]
struct Lobby {
    players: BTreeMap<u8, Player>,
    peers: BTreeMap<u8, TcpStream>,
    started: bool,
}

fn lobby_snapshot(lobby: &Lobby) -> String {
    let mut s = String::from("LOBBY");
    for (id,p) in &lobby.players {
        s.push_str(&format!(" {id}:{}", if p.ready { "READY" } else { "WAITING" }));
    }
    s.push('\n');
    s
}

fn host(addr: &str) -> io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let shared = Arc::new(Mutex::new(Lobby::default()));
    println!("Co-op lobby hosted at {addr}. Up to {MAX_PLAYERS} clients (host joins as player 1).");
    let broadcast = shared.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));
        let mut lobby = broadcast.lock().expect("lobby lock");
        let snapshot = lobby_snapshot(&lobby);
        let mut disconnected = Vec::new();
        for (&id, peer) in &mut lobby.peers {
            if peer.write_all(snapshot.as_bytes()).is_err() {
                disconnected.push(id);
            }
        }
        for id in disconnected {
            lobby.players.remove(&id);
            lobby.peers.remove(&id);
        }
    });
    loop {
        match listener.accept() {
            Ok((mut socket, address)) => {
                socket.set_nodelay(true)?;
                socket.set_write_timeout(Some(Duration::from_secs(1)))?;
                let id = {
                    let mut lobby = shared.lock().expect("lobby lock");
                    let vacant = if lobby.started { None } else {
                        (1..=MAX_PLAYERS as u8).find(|id| !lobby.players.contains_key(id))
                    };
                    if let Some(id) = vacant {
                        // Welcome must precede snapshots.
                        socket.write_all(format!("WELCOME {id}\n").as_bytes())?;
                        lobby.players.insert(id, Player { ready: false });
                        lobby.peers.insert(id, socket.try_clone()?);
                    }
                    vacant
                };
                if let Some(id) = id {
                    println!("Player {id} connected from {address}");
                    let lobby = shared.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_player(socket, id, &lobby) {
                            eprintln!("Player {id}: {e}");
                        }
                        let mut lobby = lobby.lock().expect("lobby lock");
                        lobby.players.remove(&id);
                        lobby.peers.remove(&id);
                    });
                } else {
                    let _ = socket.write_all(b"FULL_OR_STARTED\n");
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock =>
                thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e),
        }
    }
}

fn handle_player(socket: TcpStream, id: u8, shared: &Arc<Mutex<Lobby>>) -> io::Result<()> {
    let mut reader = BufReader::new(socket);
    loop {
        let mut line = String::new();
        let count = reader.read_line(&mut line)?;
        if count == 0 || count > 128 { return Ok(()); }
        let cmd = line.trim().to_ascii_uppercase();
        let mut lobby = shared.lock().expect("lobby lock");
        match cmd.as_str() {
            "READY" | "UNREADY" if !lobby.started => {
                if let Some(p) = lobby.players.get_mut(&id) { p.ready = cmd == "READY"; }
            }
            "START" if id == 1 && !lobby.started => {
                if !lobby.players.is_empty() && lobby.players.values().all(|p| p.ready) {
                    lobby.started = true;
                    println!("All players ready: lobby started");
                    for peer in lobby.peers.values_mut() { let _ = peer.write_all(b"STARTED\n"); }
                } else if let Some(peer) = lobby.peers.get_mut(&id) {
                    let _ = peer.write_all(b"NOT_READY\n");
                }
            }
            "START" if id != 1 => {
                if let Some(peer) = lobby.peers.get_mut(&id) {
                    let _ = peer.write_all(b"HOST_ONLY\n");
                }
            }
            _ => {}
        }
    }
}

fn join(addr: &str) -> io::Result<()> {
    let mut socket = TcpStream::connect(addr)?;
    socket.set_nodelay(true)?;
    let mut reader = BufReader::new(socket.try_clone()?);
    let mut hello = String::new();
    reader.read_line(&mut hello)?;
    if !hello.starts_with("WELCOME ") {
        println!("{}", hello.trim());
        return Err(io::Error::other("Lobby unavailable"));
    }
    let host = hello.trim() == "WELCOME 1";
    println!("Connected as {}.", if host { "HOST (Player 1)" } else { "guest" });
    println!("Commands: ready, unready{} , quit", if host { ", start" } else { "" });
    thread::spawn(move || {
        let mut last = String::new();
        for line in reader.lines() {
            match line {
                Ok(line) if line.starts_with("LOBBY") => {
                    if line != last { println!("{line}"); last = line; }
                }
                Ok(line) if line == "STARTED" => println!("All ready! STARTED (game integration comes next)."),
                Ok(line) => println!("{line}"),
                Err(_) => break,
            }
        }
        println!("Connection closed.");
    });
    for line in io::stdin().lock().lines() {
        let input = line?;
        let cmd = input.trim().to_ascii_uppercase();
        if cmd == "QUIT" { break; }
        if matches!(cmd.as_str(), "READY" | "UNREADY" | "START") {
            socket.write_all(format!("{cmd}\n").as_bytes())?;
        } else {
            println!("Use ready, unready{} or quit.", if host { ", start" } else { "" });
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, action, address] if action == "host" => host(address),
        [_, action, address] if action == "join" => join(address),
        _ => {
            println!("Host: survivors-coop host 0.0.0.0:{PORT}");
            println!("Then on host and guests: survivors-coop join HOST_IP:{PORT}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lobby_status() {
        let mut lobby = Lobby::default();
        lobby.players.insert(1, Player { ready: false });
        lobby.players.insert(2, Player { ready: true });
        assert_eq!(lobby_snapshot(&lobby), "LOBBY 1:WAITING 2:READY\n");
    }
}
