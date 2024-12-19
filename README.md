# CG Arena

Local bot arena similar to CodinGame.

![screenshot](/assets/readme_screenshot.png)

## Features

- Web UI
    - Add/Delete bots
    - Check leaderboard
- Matchmaking
- Rating calculation
    - OpenSkill

## Installation

You would need `cargo` installed ([here](https://doc.rust-lang.org/cargo/getting-started/installation.html) is how to
install it).

```shell
cargo install cgarena
```

The same command can be used to update CG Arena to the latest version.

## Usage

1. To create a new arena in the current folder run:
   ```shell
   cgarena init
   ```
2. To run arena previously initialized in the current folder run:
   ```shell
   cgarena run

## Configuration

`cgarena init` command generates `cgarena_config.toml` file including default config.
You can check the contents of the default config [here](/assets/default_config.toml).

The config file is documented, so read through it and don't forget to restart the arena if you make any changes to the
config file.

### Worker configuration

Worker configuration has 3 important properties:

- `cmd_play_match`
- `cmd_build`
- `cmd_run`

Here are some good defaults for them:

```toml
cmd_play_match = "python play_game.py {SEED} {PLAYERS}"
cmd_build = "sh build.sh {DIR} {LANG}"
cmd_run = "sh run.sh {DIR} {LANG}"
```

You would also need to create following files in the arena folder:

CodinGame compatible `play_game.py` (slightly modified version from Psyleague readme)

```python
import sys, subprocess, random, json, tempfile, os
if __name__ == '__main__':
    f, log_file = tempfile.mkstemp(prefix='log_')
    os.close(f)

    n_players = len(sys.argv) - 2
    seed = sys.argv[1]
    # assumes brutaltester-compatible referee.jar is placed in the same folder
    cmd = 'java --add-opens java.base/java.lang=ALL-UNNAMED -jar referee.jar' + ''.join([f' -p{i} "{sys.argv[i + 1]}"' for i in range(1, n_players+1)]) + f' -d seed={seed} -l "{log_file}"'
    task = subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    with open(log_file, 'r') as f:
        json_log = json.load(f)
    os.remove(log_file)
    p_scores = [int(json_log['scores'][str(i)]) for i in range(n_players)]
    rv = {}
    rv['ranks'] = [sum([int(p_score < p2_score) for p2_score in p_scores]) for p_score in p_scores] # assumes higher score is better
    rv['errors'] = [int(p_score < 0) for p_score in p_scores] # assumes negative score means error
    print(json.dumps(rv))
```

Generic `build.sh` which can support multiple programming languages

```shell
if [ "$2" = "c++" ]; then
  g++ -std=c++17 -x c++ "$1"/source.txt -o "$1"/a.exe
elif [ "$2" = "python" ]; then
  cp "$1"/source.txt "$1"/a.py
else
  echo "Unsupported language '$2'" >&2
fi
```

Generic `run.sh` which can support multiple programming languages

```shell
if [ "$2" = "c++" ]; then
  ./"$1"/a.exe
elif [ "$2" = "python" ]; then
  python ./"$1"/a.py
else
  echo "Unsupported language '$2'" >&2
fi
```

For languages which need project folder (e.g. Rust) you build bot the following way:

- somewhere create single project which has all the dependencies installed
- inside `build.sh` copy `source.txt` to that project folder (e.g. `main.rs` for Rust)
- build the project
- copy the executable from the project build to the bot directory

## Building from source

To build CG Arena from source code run the following (make sure `cargo` and `npm` are installed):

```shell
git clone https://github.com/aangairbender/cgarena.git
cd cgarena
cargo build --release
```

You can find executable in `/target/release` folder.