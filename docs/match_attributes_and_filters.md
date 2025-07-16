# Match attributes and filters

## Table of contents

- [Match attributes overview](#match-attributes-overview)
- [Emitting match attributes](#emitting-match-attributes)
    - [Match data](#match-data)
    - [Turn-specific match data](#turn-specific-match-data)
    - [Player data](#player-data)
    - [Turn-specific player data](#turn-specific-player-data)

## Match attributes overview

CG Arena [uses](configuration.md#cmd_play_match) `cmd_play_match` command to run matches. That command should write the JSON of specific format to the stdout. One of the properties is `attributes`. CG Arena records those attributes for each match, so that they can be used for match filtering later.

The attributes can describe 2 kinds of data:

- match data - not specific to any player, e.g. "seed", "map_size", etc.
- player data - specific to each player, e.g. "final_score"

Also, the attributes can be turn-specific, e.g. "player score on turn 10".

## Emitting match attributes

If you are using arena configured using [example CodinGame setup](example_codingame_setup.md), bots can emit match attributes by printing specific output to the **stderr**:

### Match data

The format is `[TDATA] name = value`, for example:

```rust
// rust
eprintln!("[TDATA] map_width = {}", map_width);

// stderr
[TDATA] map_width = 12
```

### Turn-specific match data

The format is `[TDATA][turn] name = value`, for example:

```rust
// rust
eprintln!("[TDATA][{}] empty_cells = {}", turn, empty_cells);

// stderr
[TDATA][0] empty_cells = 57
[TDATA][1] empty_cells = 55
...
[TDATA][99] empty_cells = 3
```

### Player data

The format is `[PDATA] name = value`, for example:

```rust
// rust
eprintln!("[PDATA] final_score = {}", final_score);

// stderr
[PDATA] final_score = 86
```

### Turn-specific player data

The format is `[PDATA][turn] name = value`, for example:

```rust
// rust
eprintln!("[PDATA][{}] money = {}", turn, money);

// stderr
[PDATA][0] money = 10
[PDATA][1] money = 6
...
[PDATA][99] money = 45
```

## Builtin match attributes

CG Arena injects several match arguments by default for every match. It will overwrite any bot-emitted data with same conflicting name.

- Match data:
    - `seed` - match seed
    - `player_count` - amount of players in match. Only recorded if `min_players != max_players` inthe config
- Player data:
    - `index` - index of a bot in match (e.g. if bot was the 1st player then index is 0)
    - `error` - set to `1` if bot crashed in the matched, otherwise is not set

## Match filters

When creating custom leaderboards or charts you are prompted to input match filter.

Match filter is a boolean expression which can use match attributes.

The expression include the following elements:

- `OR`
- `AND`
- `(..)` - parens
- `<condition>`

The condition consists of 2 arguments and operator between them.

Condition argument can be:

- match attribute
    - match data, e.g. `match.map_size`
    - turn-specific match data, e.g. `match[10].empty_cells`
    - player data, e.g. `bot(5).final_score` where `5` is the bot ID.
    - turn-specific player data, e.g. `bot(5)[50].money`
- number, e.g `5` or `0.3`
- string, e.g. `"small"`

Condition operator can be:

- `==`
- `!=`
- `>` (not applicable to strings)
- `>=` (not applicable to strings)
- `<` (not applicable to strings)
- `<=` (not applicable to strings)

Examples of match filters (each line has separate filter):

```
match.player_count == 2 OR match.player_count == 3
match[5].some_data != -2
bot(23).final_score > 5
bot(1)[50].protein_a > 20 AND bot(1)[50].protein_c < 10
match.map_kind == "small"
match.initial_stones > 20 AND (match.x > 1 OR match.y < 1)
bot(1).error == 1
```

Keywords in the match filters are case-insensitive, **but match attributes names are case-sensitive**.