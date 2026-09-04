# Match attributes and filters

## Table of contents

- [Match attributes overview](#match-attributes-overview)
- [Emitting match attributes](#emitting-match-attributes)
  - [Match data](#match-data)
  - [Turn-specific match data](#turn-specific-match-data)
  - [Player data](#player-data)
  - [Turn-specific player data](#turn-specific-player-data)
- [Builtin match attributes](#builtin-match-attributes)
- [Match filters](#match-filters)
- [Browsing matching matches](#browsing-matching-matches)

## Match attributes overview

CG Arena records attributes returned by a custom `command` referee or extracted from tagged
CodinGame replay errors by the `codingame_jar` referee. These attributes can be used for match
filtering later.

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
  - `rank` - rank of a bot in match (e.g. if bot was the winner the rank is 0. In case of a draw in a 2-player match, both bots will have rank 0).
  - `error` - set to `1` if bot crashed in the matched, otherwise is not set
  - `score` - set to bot's score computed by the game referee

## Match filters

Match filters are used by custom leaderboards and charts, and can be searched directly from the **Matches** page.

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
bot(12).rank > bot(34).rank
```

Keywords in the match filters are case-insensitive, **but match attributes names are case-sensitive**.

Ranks are zero-based, and a lower rank is a better result. For example, `bot(12).rank > bot(34).rank` matches games where bot 12 placed worse than bot 34.

## Browsing matching matches

Open the **Matches** page to search recorded matches directly. A search can combine:

- a match filter expression; and
- required bots, which restrict results to matches containing every listed bot.

Required bots are added automatically when you follow match links from a bot overview or leaderboard. They are shown above the filter as removable labels. Remove a label and select **Search** to broaden the results. The applied filter, required bot IDs, page, and page size are stored in the URL, so a copied or reloaded URL opens the same result page.

Leaderboard match links use the bot selected on the Home page as the point of view. A row's **Total** link requires both the selected bot and the row's opponent and preserves the leaderboard filter. Its **W / L / D** links add one of these result conditions:

- win: `bot(selected).rank < bot(opponent).rank`
- loss: `bot(selected).rank > bot(opponent).rank`
- draw: `bot(selected).rank == bot(opponent).rank`

These links are only available when a bot is selected. If the server rejects a direct search, the Matches page keeps the entered criteria and any previous results visible so the filter can be corrected and submitted again.
