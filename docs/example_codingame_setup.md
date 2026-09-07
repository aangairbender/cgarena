# Example setup for CodinGame

We are going to setup CG Arena for generic CodinGame challenge. We would use [Cellularena](https://www.codingame.com/multiplayer/bot-programming/winter-challenge-2024) as an example.

## Creating the new arena

First, let's create a new arena:

```
cgarena init winter-challenge-2024
cd winter-challenge-2024
```

## Arena configuration

Start CG Arena, open its **Local** URL, and use the first-run **Config** page. Runtime settings are
stored in `cgarena.db`; `cgarena_config.toml` remains limited to server and logging settings that
require a restart.

The form starts with these game defaults:

- `min_players`: `2`
- `max_players`: `2`
- `symmetric`: enabled

## Worker configuration

### Concurrency

Set **Worker threads** for your CPU. This example uses `4`.

### Managed referee

Keep the default embedded worker and managed referee adapter, then set:

- **Worker threads**: `4`
- **Build command**: `g++ -std=c++20 -x c++ {DIR}/source.txt -o {DIR}/a`
- **Run command**: `./{DIR}/a`
- **Repository URL**:
  `https://github.com/CodinGame/WinterChallenge2024-Cellularena.git`

Install Java 17 or newer, apply the arena configuration, and choose **Install referee**. This
explicit action clones the repository into `referee`, creates the reserved local `cgarena` branch,
installs the maintained command-line and Maven adaptation, prefers the repository Maven Wrapper,
builds and validates the candidate, and publishes the active JAR to an arena-owned location.
Saving configuration or restarting does not contact Git or rebuild.

Use **Check for updates**, **Update referee**, or **Rebuild referee** explicitly. Local commits and
working-tree changes are reported in the same panel. A repository or branch change is a
separately validated **Replace referee** operation.

Use the [`command` referee adapter](configuration.md#command) for a custom or non-Java referee.

### Languages that need workspace

For languages which need project folder (e.g. Rust) you build bot the following way:

- create a single project somewhere and configure it to be the same as CG (e.g. has all the dependencies)
- use `build.sh` file for `cmd_build` command
- inside the `build.sh` copy `source.txt` to that project folder (e.g. `main.rs` for Rust)
- build the project
- copy the executable from the project build to the bot directory

The `cargo.toml` inside `rust-workdir` looks like this (this is same as CG, might be outdated):

```toml
[package]
name = "rust-workdir"
version = "0.1.0"
edition = "2018"

[dependencies]
chrono = "0.4.26"
itertools = "0.11.0"
libc = "0.2.147"
# rand is a bit of a mess, see https://www.codingame.com/forum/t/languages-update/1574/120
rand_core = "0.6.3"
rand = { version = "0.8.5", features = ["small_rng"] }
regex = "1.8.4"
time = "0.3.22"
```