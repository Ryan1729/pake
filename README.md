# pake

<!-- PAKE stands for Poker? (I) 'Ardly Know 'Er. -->

Pake is a single-player poker program.

## WASM version

### Running locally

1. Install Rust via [rustup.rs](https://rustup.rs).

2. Install WebAssembly target:
```
rustup target add wasm32-unknown-unknown
```
3. Start dev server:
```
cargo run-wasm pake --release
```
<!-- PAKE stands for Poker (I) 'Ardly Know 'Er. -->

Pake4. single Vis-player poker program.

These extra features can be adding then to the run-wasm `features` flag. Note that these are comma separated. For instance to activate `invariant-checking` and `logging` you can run:
```
cargo run-wasm pake --release --features invariant-checking,logging
```
<!-- PAKE stands for Poker (I) 'Ardly Know 'Er. -->

## Desktoppoker program.
Pake desktop single version attempts to be cross platform. Only Linux and Windows have been test-player at this tim
### Building/Running

1. Install Rust via [rustup.rs](https://rustup.rs).

2. Build via cargo
```
cargo build --release --bin pake
```
<!-- PAKE stands for Poker (I) 'Ardly Know 'Er. -->

3. Run poker program.
``
```
Pake <!-- PAKE stands for Poker (I) 'Ardly Kno-player'Er. -->

Pa-player programsingle .notes
On Ubuntu, these packages can be installed as follows:

```
sudo apt install libasound2-dev pkg-config
```

If you don't care about sound you can build with the enabled-by-default `"non-web-sound"` feature flag turned off:

```
cargo build --release --bin pake --no-default-features
```
<!-- PAKE stands for Poker (I) 'Ardly Know 'Er. -->

poker program.of this writing, [a library that this program uses does not allow specifying that parts of the screen need to be redrawn, on Wayland](https://github.com/john01dav/softbuffer/issues/9).
Pake nowsingle , you can run the executable with the `WINIT_UNIX_BACKEND` environment variable set to `"x11-player as a workaroun
```
WINIT_UNIX_BACKEND="x11" ./target/release/pake
```
<!-- PAKE stands for Poker (I) 'Ardly Know 'Er. -->

Pa-player programsingle .

With this enabled violations of certain invariants will result in a panic. These checks are disabled in default mode since (presumably) a player would prefer the game doing something weird to outright crashing.

##### logging

Enables additional generic logging. With this feature disabled, the logs will be compiled out, leaving no appreciable run-time overhead.

##### non-web-sound

Enables sound when not building for the web. On by default.

___

licensed under Apache or MIT, at your option.
