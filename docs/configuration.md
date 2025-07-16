# Configuration reference

CG Arena uses [toml](https://toml.io/) format for its' config.

## `[game]`

### `min_players`

Minimum amount of players the game supports, e.g. 2 for chess.

### `max_players`

Maximum amount of players the game supports, e.g. 2 for chess, 4 for tron

### `symmetric`

Whether the map is symmetric for all the players.

- if `symmetric` is **true** CG Arena will play 1 match per seed.
- if `symmetric` is **false** CG Arena will play n! matches per seed (all permutations), where n is the amount of players.

## `[matchmaking]`

### `min_matches`

Matchmaking prioritizes bots which played less than `min_matches` matches with probability `min_matches_preference`. Otherwise matchmaking picks bots randomly.

### `min_matches_preference`

Check the explanation for `min_matches` above.

## `[ranking]`

### `algorithm`

The skill rating algorithm to use for ranking the bots.

Supported algorithms are:

- `"OpenSkill"`
- `"TrueSkill"`
- `"Elo"`

Each algorithm has their own configuration parameters which you can also set if desired:


#### OpenSkill:

- `beta` - The skill-class width, aka the number of difference in rating points needed to have a ~67% win probability against another player.
By default set to 25 / 6 ≈ 4.167.
If your game is more reliant on pure skill, decrease this value, if there are more random factors, increase it.
- `uncertainty_tolerance` - The lower ceiling of the sigma value, in the uncertainty calculations. The lower this value, the lower the possible uncertainty values.
By default set to 0.000_001.
Do not set this to a negative value.

#### TrueSkill:

- `draw_probability` - The probability of draws occurring in match. The higher the probability, the bigger the updates to the ratings in a non-drawn outcome.
By default set to 0.1, meaning 10% chance of a draw.
Increase or decrease the value to match the values occurring in your game.
- `beta` - The skill-class width, aka the number of difference in rating points needed to have an 80% win probability against another player.
By default set to (25 / 3) * 0.5 ≈ 4.167.
If your game is more reliant on pure skill, decrease this value, if there are more random factors, increase it.
- `default_dynamics` - The additive dynamics factor. It determines how easy it will be for a player to move up and down a leaderboard. A larger value will tend to cause more volatility of player positions. By default set to 25 / 300 ≈ 0.0833.

#### Elo:

- `k` - The k-value is the maximum amount of rating change from a single match. In chess, k-values from 40 to 10 are used, with the most common being 32, 24, 16 or 10. The higher the number, the more volatile the ranking.
Here the default is 32.

## `[server]`

### `port`

Controls the web server port. If `port` is omitted then OS assigns some available port.

### `expose`

Controls whether to expose web server to the local network.

## `[log]`

### `level`

CG Arena log level.

### `file`

CG Arena log file

## `[[workers]]`

This is where you can specify a list of configurations for workers that would run your matches. Currently CG Arena support only the list of 1 embedded worker.

### `type`

Type of worker. Currently only `"embedded"` is supported.

### `threads`

Controls the number of concurrent matches being run. Don't set this higher than the number of cpu cores you have.

### `cmd_play_match`

Whenever CG Arena wants to run a match, it would execute `cmd_play_match` command.

CG Arena would also make the following substitutions in `cmd_play_match`:

- `{SEED}` - would be replaced with the seed of the match
- `{P1}`, `{P2}`, ... would be replaced with `cmd_run` command configured for match participant 1, 2, ...
- `{PLAYERS}` would be replaced with concatenated version of the above. Please use this when the game can have varying player counts. 

Example:

```
cmd_play_match = "python play_game.py {SEED} {PLAYERS}"
```

You can find the example of `play_game.py` [here](example_codingame_setup.md#player_gamepy).

The `cmd_play_match` command should print JSON to stdout in the following format:

```js
{ "ranks" [..], "errors": [..], "attributes": [..] }
```

Where:

- `ranks` - the list of numbers where i-th number is i-th match participant final placement (e.g. 0 for winner). Duplicates are allowed in the case of draw.
- `errors` - the list of numbers where i-th number is 1 if i-th match participant failed during match or 0 otherwise
- `attributes` - the list of match attributes emitted by the bot. Each attribute is a json object with the following fields:

    - `name` - name of attribute
    - `player` - index of a player who the attribute belongs to (or `null` if it's match attribute, not specific to a particular bot)
    - `turn` - turn of the attribute (or `null` if the attribute is not specific to any particular turn)
    - `value` - the attribute value (integer, float or string)

Example:

```js
{
    "ranks": [0, 1],
    "errors": [0, 0],
    "attributes": [
        {
            "name": "map_size",
            "value": 16
        },
        {
            "name": "final_score",
            "player": 0,
            "value": 86
        },
        {
            "name": "final_score",
            "player": 1,
            "value": 53
        },
    ],
}
```

### `cmd_build`

Whenever a new bot is submitted CG Arena will run `cmd_build` command for the new bot.

CG Arena would also make the following substitutions in `cmd_build`:
- `{DIR}` would be replaced with target bot directory
- `{LANG}` would be replaced with target bot language

Example:
```
cmd_build = "sh build.sh {DIR} {LANG}"
```

You can find the example of `build.sh` [here](example_codingame_setup.md#buildsh).

### `cmd_run`

This command is passed to `cmd_play_match` after applying substitutions for each bot.

The substitutions for `cmd_run` are same as for `cmd_build`:
- `{DIR}` would be replaced with target bot directory
- `{LANG}` would be replaced with target bot language

Example:
```
cmd_run = "sh run.sh {DIR} {LANG}"
```

You can find the example of `run.sh` [here](example_codingame_setup.md#runsh).