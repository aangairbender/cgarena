# CG Arena

[![Crates.io](https://img.shields.io/crates/v/cgarena.svg)](https://crates.io/crates/cgarena)
[![Crates.io](https://img.shields.io/crates/d/cgarena.svg)](https://crates.io/crates/cgarena)

CG Arena is a personal, trusted local workbench for CodinGame bot authors. Each arena belongs to
one challenge and helps an author evaluate bot improvements with substantially more local match
evidence than CodinGame provides.

See [CONTEXT.md](CONTEXT.md) for the canonical domain language.

![screenshot](/docs/img/readme_screenshot.png)

## Features

- Local web UI with guided arena configuration
- Managed CodinGame referee installation and updates
- Custom-command referee integration
- Submit immutable bot code as a Candidate or Benchmark, then promote or reject Candidates and archive or delete Benchmarks
- Bounded, staged Candidate evaluation against a live Benchmark pool
- Rating calculation using OpenSkill, TrueSkill, Elo, or Bradley–Terry
- Realtime global and filtered custom leaderboards
- Match browsing with attribute filters
- Per-turn bot analytics with average, minimum, and maximum aggregation
- Match replays and seed inspection
- Local SQLite storage and optional trusted-LAN access

## Evaluation workflow

New submissions default to Candidate and pin the active evaluation-plan revision. The scheduler
collects the configured evidence against active Benchmarks; promotion and rejection always remain
manual. See [the candidate-evaluation decision](docs/adr/0002-replace-continuous-matchmaking-with-candidate-evaluation.md).

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
make build
```

You can find the executable in `/target/release` folder.

## Thanks

Thanks to
- Psyho's [psyleague](https://github.com/FakePsyho/psyleague) for the idea inspiration!
- Magus's [CG stats](https://cgstats.magusgeek.com/) for the UI inspiration!
- [CodinGame](https://www.codingame.com/) for such an amazing platform and bot competitions!
- You for your interest in CG Arena!