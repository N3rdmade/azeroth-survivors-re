# Windows co-op development

## Phase 1: Downloadable lobby preview

The separate `survivors-coop.exe` is a Windows LAN lobby test harness. **It does not run
the actual game yet.** This first step tests connectivity, four player slots, per-player
ready state, and a host-only START message.

Every push to the `multiplayer` branch that changes co-op code triggers
[Windows Co-op Preview](../.github/workflows/coop-windows.yml). Download its
`AzerothSurvivors-Coop-Lobby-Windows` artifact from a successful workflow run, extract
it, and double-click `Launch-Lobby.bat`.

Host selects 1. Friends select 2 and enter the host's LAN IP address.
Everyone types `ready` and hits Enter. Host types `start` after all players are
ready. Starting currently displays a confirmation, not the game.

The host occupies slot 1; three guests can join. TCP 45454; LAN/private networks only.

## Next stages

- Add actual Windows game UI for Host/Join, lobby slots, ready indicators and Start.
- Connect the native host transport to the Bevy game's entities and local player.
- Separate match and per-player run state; synchronize enemy AI, combat, XP and level-up.
- Ship a versioned Windows release with optional update-check prompt (never silently
  overwriting, and use verifiable GitHub releases).

## Build locally

```sh
cargo test -p survivors-coop
cargo build --release -p survivors-coop
```

The source is cross-platform Rust, while packaging and testing prioritize Windows.
