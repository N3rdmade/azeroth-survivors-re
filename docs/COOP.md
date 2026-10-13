# Native LAN co-op — phase 1

This branch preserves the original game and starts with a separate **native Windows-compatible networking harness**. It is **not yet playable multiplayer**, and does not patch the uploaded original EXE.

## Test the transport

On the host PC:

```sh
cargo run --release -p survivors-coop -- host 0.0.0.0:45454
```

On each client PC (up to four connections, including the host if the host also joins):

```sh
cargo run --release -p survivors-coop -- join 192.168.1.100:45454
```

Replace the sample address with the host PC's LAN address. Type `MOVE 12 8` on the client terminal to broadcast coordinates. The server periodically sends `STATE` snapshots containing all connected player positions. The host process is currently a dedicated headless process; a future phase will put Host/Join into the game menu and run the server alongside the host's game.

## Next integration steps

1. Add stable player IDs and a `LocalHero` marker while keeping rendering focused on a single local character.
2. Split `Run` into match-wide and per-player state; do not make all heroes share HP, class, abilities, or XP.
3. Attach the transport to Bevy with channels, apply client input on the authoritative host, and replicate player positions.
4. Replace `Hero::single()` assumptions in enemy AI, pickup and cast logic; synchronize combat and leveling.
5. Add the in-game lobby, connect/disconnect recovery, and four-player Windows smoke testing.

## Limits

Protocol v1 is plain-text TCP, designed only for trusted LAN prototype use. It has no authentication, rate limits or internet-safe threat model. It carries test coordinates, **not actual player movement**. Internet deployment should use a validated binary protocol with input rate limits and appropriate authentication.
