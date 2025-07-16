# CG Arena

Local bot arena similar to CodinGame.

![screenshot](/docs/img/readme_screenshot.png)

## Features

- Web UI
- Add/Delete/Rename bots
- Matchmaking
- Rating calculation
    - OpenSkill
    - TrueSkill
    - Elo
- Realtime leaderboard
- Analytics
  - Custom leaderboards based on some match criteria (e.g. small maps)
  - Visualize bot data, x-axis for turn, y-axis for your param (e.g. average/min/max money on each turn)
- Fully local, but you can expose web server to check leaderboard from your phone

## Installation

You would need `cargo` installed. ([Here](https://doc.rust-lang.org/cargo/getting-started/installation.html) is how to
install it).

```shell
cargo install cgarena
```

The same command can be used to update CG Arena to the latest version.

## Usage

Please check the full usage documentation [here](docs/index.md).

You can also check [the example setup guide for CodinGame](docs/example_codingame_setup.md).

## Building from source

To build CG Arena from source code run the following (make sure `cargo` and `npm` are installed):

```shell
git clone https://github.com/aangairbender/cgarena.git
cd cgarena
cargo build --release
```

You can find the executable in `/target/release` folder.

## Thanks

Thanks to
- Psyho's [psyleague](https://github.com/FakePsyho/psyleague) for the idea inspiration!
- Magus's [CG stats](https://cgstats.magusgeek.com/) for the UI inspiration!
- [CodinGame](https://www.codingame.com/) for such an amazing platform and bot competitions!
- You for your interest in CG Arena!