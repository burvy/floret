# floret

Floret is a hangout game that is cross platform on mobile, desktop, and web.

---

## Running it

Needs the nightly (`rust-toolchain.toml` handles it) and
[`trunk`](https://trunkrs.dev) for wasm builds.

```powershell
.\go.ps1 dev # server in its own window, client in this one
.\go.ps1 release # builds deploy\ for the live server
```

Run `.\go.ps1 dev` a **second time** to get two people in the room — the client
binds port 0, so a second instance gets its own port.

Test the rules without building the client at all:

```powershell
cargo test -p floret-protocol
```

## Layout

```
protocol/src/shared.rs - the rules. pure functions. start here.
protocol/src/protocol.rs - rules, but more technical
server/src/server.rs - listening, TLS, admitting people
src/net.rs - client connecting to the server
src/input.rs - inputs -> Vec2
src/render.rs - the only module that knows about pixels
```


# Me
- Port **5002** (5001 is the shooter's).
- Shares the shooter's Let's Encrypt IP certificate. Renewal command is in the
  comment at the top of `server/src/server.rs`.
- `--features dev-local` = localhost + self-signed cert + no TLS validation +
  `bevy/dynamic_linking`. Never ships.
- Bevy 0.19, lightyear 0.28, nightly-2026-09-03.

---

## Why it is built this way

The game is built using as much functional style as possible as proof of concept 
and to get the benefits functional programming provides.

## Notes

`step()` in `shared.rs` takes dt, which makes it easy to test. The shared 
`DT` constant is also passed into both, making client and server step 
similarly.  

The client and server movement systems are similar, except the server is 
allowed to move everything and the client is obviously only allowed to 
move themselves.  

Velocity is a function that can be calculated from `PlayerInputs` directly.  
This means more predictability, less cost, less pain in rollback.  

Braces in the `bsn!` macro parses it as Rust rather than `bsn`, which helps 
in cases like:
```rust
custom_size: {Some(Vec2::splat(shared::PLAYER_SIZE))},
```

---

# Reminder
Go in and do the `TODO`s later on
