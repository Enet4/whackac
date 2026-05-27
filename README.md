# Whack-a-Creature

[![ci](https://github.com/Enet4/whackac/actions/workflows/ci.yml/badge.svg)](https://github.com/Enet4/whackac/actions/workflows/ci.yml)

Created for the [VGA & Adlib Game Jam](https://itch.io/jam/dos-vgaadlib-jam-2026-take-2).
Also available on [itch.io](https://e-net4.itch.io/whackac).

## Playing

Use the arrow keys on your keyboard to select an option and aim.
Press Z to whack, X to grab.

## Building

First you need:

- Rust, preferably installed via Rustup.
  It will install the right toolchain via [rust-toolchain.toml](./rust-toolchain.toml).
- `elf2djgpp`, available on [this repository](https://github.com/cknave/elf2djgpp)
- The [DJGPP GCC toolchain](https://www.delorie.com/djgpp)
  (version 14.1.0 is known to work, but it should also work with v12).

Then:

- If your DJGPP toolchain is not named `i686-pc-msdosdjgpp-gcc`,
  set the environment variable `CC` to the right path.
- Set `ARCH` depending on the target architecture intended
  (default is `i486`)
- Run `./build.sh`

You will find the .exe file in `build/release/`.
Adding `debug` to `./build.sh` builds it in debug mode.

## Running

Add the resulting `WHACKAC.EXE` alongside `CWSDPMI.EXE`
to your DOS machine or emulator.
The absolute minimum requirements are
an i486 with a VGA display.

```bat
WHACKAC
```

To run the game without sound or music,
append `nosound` to the command line arguments:

```bat
WHACKAC nosound
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
