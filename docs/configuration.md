# Configuration reference

New arenas store game, evaluation-plan, ranking, leaderboard, worker, and referee settings in the
database. Edit them together on the web UI's **Config** page; **Apply configuration** validates
and saves the complete candidate atomically. Invalid candidates do not replace the active
configuration.

`cgarena_config.toml` is the bootstrap file. It contains only `[server]` and `[log]`, because
those settings are required before the HTTP application can start. The remaining settings are
database-backed UI fields.
Any other top-level section is rejected. A legacy full-arena file is accepted only for the
automatic one-time migration: CG Arena archives it and replaces it with the bootstrap file.


## Game

### `min_players`

Minimum amount of players the game supports, e.g. 2 for chess.

### `max_players`

Maximum amount of players the game supports, e.g. 2 for chess, 4 for tron

### `symmetric`

Whether the map is symmetric for all the players.

- if `symmetric` is **true** CG Arena will play 1 match per seed.
- if `symmetric` is **false** CG Arena will play n! matches per seed (all permutations), where n is the amount of players.

## Candidate evaluation

### `enabled_on_start`

Whether matchmaking for Candidate evaluations starts with the arena. Defaults to `true`.

### `seed_sequence_key`

A 32-bit key that deterministically generates a different referee seed for every match sequence
position. The sequence does not repeat after 100 matches. It remains stable across restarts and
plan edits so Candidates receive comparable scenarios. Editing the key or using **Randomize**
creates a different sequence for future Candidates; active Candidates retain their pinned plan.

### Ordered stages

Every evaluation plan contains one or more ordered stages. A Candidate pins the complete plan
revision active when it is submitted. Later edits affect only later Candidates.

Each stage configures:

- A name.
- A seed source: the generated deterministic sequence, an explicit curated seed list, or fresh random seeds.
- A coverage policy: a target number of matches per Candidate–Benchmark pair or a total match
  target across the active Benchmark pool.
- Optional minimum and maximum player counts. Omitted values use the challenge's full configured
  range.

Stages run in order. For asymmetric challenges, every player-position permutation runs for a
scheduled seed. Completing all stages records that the requested evidence exists; it never
promotes or rejects a Candidate automatically.

## Ranking

### `algorithm`

The skill rating algorithm to use for ranking the bots.

Supported algorithms are:

- `"OpenSkill"`
- `"TrueSkill"`
- `"Elo"`
- `"BradleyTerry"`

**Bradley–Terry** → best for global, high-accuracy ranking

**OpenSkill / TrueSkill** → best for uncertainty-aware ratings

**Elo** → best for simple, fast, lightweight ranking

Each algorithm has their own configuration parameters which you can also set if desired.

#### OpenSkill:

**Best for**: Flexible, open Bayesian rating system.

Similar to TrueSkill (μ + σ per bot), but open and configurable.

Pros

- Uncertainty modeling
- Multiplayer support
- Open implementation
- Flexible update behavior

Cons

- Approximate inference
- More tuning required

👉 Use OpenSkill if you want Bayesian ratings with flexibility and transparency.

Config:

- `beta` - The skill-class width, aka the number of difference in rating points needed to have a ~67% win probability against another player.
By default set to 25 / 6 ≈ 4.167.
If your game is more reliant on pure skill, decrease this value, if there are more random factors, increase it.
- `uncertainty_tolerance` - The lower ceiling of the sigma value, in the uncertainty calculations. The lower this value, the lower the possible uncertainty values.
By default set to 0.000_001.
Do not set this to a negative value.

#### TrueSkill:

**Best for**: Online rating with uncertainty tracking.

Each bot has:

- μ (skill estimate)
- σ (uncertainty)

Ratings update incrementally after each match using Bayesian inference.

Pros

- Explicit uncertainty modeling
- Handles teams/multiplayer
- Good for uncertainty-aware ratings

Cons

- More complex
- Approximate inference

👉 Use TrueSkill if you need live updates and uncertainty-aware ratings.

Config:

- `draw_probability` - The probability of draws occurring in match. The higher the probability, the bigger the updates to the ratings in a non-drawn outcome.
By default set to 0.1, meaning 10% chance of a draw.
Increase or decrease the value to match the values occurring in your game.
- `beta` - The skill-class width, aka the number of difference in rating points needed to have an 80% win probability against another player.
By default set to (25 / 3) * 0.5 ≈ 4.167.
If your game is more reliant on pure skill, decrease this value, if there are more random factors, increase it.
- `default_dynamics` - The additive dynamics factor. It determines how easy it will be for a player to move up and down a leaderboard. A larger value will tend to cause more volatility of player positions. By default set to 25 / 300 ≈ 0.0833.

#### Elo:

**Best for**: Simple, fast, online updates.

Each bot has a single rating number. After every match, ratings are adjusted based on expected vs actual outcome.

Pros

- Very fast
- Easy to understand
- Good for continuous online updates

Cons

- No uncertainty modeling
- Fixed learning rate (K-factor tuning required)
- Less statistically efficient with large datasets

👉 Use Elo if you want simplicity and lightweight real-time updates.

Config:

- `k` - The k-value is the maximum amount of rating change from a single match. In chess, k-values from 40 to 10 are used, with the most common being 32, 24, 16 or 10. The higher the number, the more volatile the ranking.
Here the default is 32.

#### BradleyTerry:

**Best for**: Accurate ranking from large batches of matches.

Each bot has a real-valued skill parameter. Ratings are estimated by maximizing likelihood over all match results.

Pros

- Statistically principled
- Very stable rankings with enough data
- Can compute uncertainty (via covariance matrix)
- Works well for batch recomputation

Cons

- Requires iterative optimization
- More computationally expensive

👉 Use Bradley–Terry if you run many matches and want the most statistically accurate global ranking.

Config:

- `max_iter` - The maximum number of optimization iterations allowed when fitting the model.

## Leaderboards

### `uncertainty_coefficient`

Controls how rating is calculated on the leaderboard:

```
bot.rating = bot.mu + bot.sigma * uncertainty_coefficient
```

Default value is **3**.

## Server (`[server]` in `cgarena_config.toml`)

### `port`

Controls the web server port. If `port` is omitted then OS assigns some available port.

### `expose`

Controls whether to expose web server to the local network.

## Logging (`[log]` in `cgarena_config.toml`)

### `level`

CG Arena log level.

### `file`

CG Arena log file

## Embedded worker

The database-backed configuration currently requires exactly one embedded worker.

### `type`

Type of worker. Currently only `"embedded"` is supported.

### `threads`

Controls the number of concurrent matches being run. Don't set this higher than the number of cpu cores you have.

### `referee`

Select exactly one referee adapter.

#### `managed_codingame`

Selects a public CodinGame referee repository. Saving configuration does not contact the
repository. Use **Install referee** in the configuration UI to clone its default branch (or the
configured `branch`), add CG Arena's maintained command-line/Maven adaptation on the reserved
local `cgarena` branch, build and probe a candidate, and activate it. The platform Maven Wrapper
(`mvnw.cmd` on Windows, `mvnw` elsewhere) is used when present; otherwise the platform Maven
executable (`mvn.cmd` on Windows, `mvn` elsewhere) is used. Every match uses league 19.

Configure the managed adapter on the **Config** page with:

- `type`: `managed_codingame`
- `repository_url`: for example, `https://github.com/CodinGame/SpringChallenge2023.git`
- Optional `branch`, `java`, and `maven` values. Explicit `java` and `maven` executable values
  override the platform defaults and are used verbatim.

The visible checkout is `<arena>/referee`; the active JAR is an internal stable artifact. Install,
Check for updates, Rebuild, Update, and Replace are explicit asynchronous UI actions. Startup,
configuration saves, and status reads never fetch or build. Local Git changes remain visible and
are preserved by rebuilds and updates; a dirty checkout blocks replacement.

#### `command`

Use this adapter for custom or non-Java referees. `play_match` must produce CG Arena's established JSON match result and write the owned `{REPLAY_PATH}` artifact. `watch_replay` must create `{REPLAY_DIR}/test.html`.

Configure the command adapter on the **Config** page with:

- `type`: `command`
- `play_match`: for example, `my-referee {SEED} {REPLAY_PATH} {PLAYERS}`
- `watch_replay`: for example,
  `my-renderer {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}`

Legacy full-arena files with both `cmd_play_match` and `cmd_watch_replay` directly under the
worker continue to migrate as a `command` referee with their previous validation rules. New
database-backed configuration must select an adapter and include every shown placeholder. CG
Arena rejects ambiguous configuration containing both forms.

`cmd_run` is expanded once per participant. The managed adapter passes each resulting command as one player argument; it is never shell-split.

### `cmd_build`

Whenever a new bot is submitted CG Arena will run `cmd_build` command for the new bot.

CG Arena would also make the following substitutions in `cmd_build`:
- `{DIR}` would be replaced with target bot directory
- `{LANG}` would be replaced with target bot language

Example:
```
cmd_build = "sh build.sh {DIR} {LANG}"
```

### `cmd_run`

This command is passed to the configured referee after applying substitutions for each bot.

The substitutions for `cmd_run` are same as for `cmd_build`:
- `{DIR}` would be replaced with target bot directory
- `{LANG}` would be replaced with target bot language

Example:
```
cmd_run = "sh run.sh {DIR} {LANG}"
```
