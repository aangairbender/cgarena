import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

REFEREE_PATH = Path("referee/target/referee.jar")


def fail(message):
    print(message, file=sys.stderr)
    raise SystemExit(1)


def main():
    if len(sys.argv) < 5:
        fail("usage: play_game.py SEED REPLAY_PATH PLAYER_COMMAND PLAYER_COMMAND [...]")

    seed = sys.argv[1]
    replay_path = Path(sys.argv[2])
    player_commands = sys.argv[3:]
    java = shutil.which("java")
    if java is None:
        fail("Java was not found on PATH; install a supported JDK")
    if not REFEREE_PATH.is_file():
        fail(f"referee JAR not found: {REFEREE_PATH}")

    replay_path.parent.mkdir(parents=True, exist_ok=True)
    command = [
        java,
        "--add-opens",
        "java.base/java.lang=ALL-UNNAMED",
        "-jar",
        str(REFEREE_PATH),
    ]
    for index, player_command in enumerate(player_commands, start=1):
        command.extend([f"-p{index}", player_command])
    command.extend(["-seed", seed, "-l", str(replay_path)])

    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode != 0:
        fail(
            f"referee exited with status {completed.returncode}: "
            f"{completed.stderr.strip() or completed.stdout.strip()}"
        )
    if not replay_path.is_file():
        fail(f"referee produced no replay artifact: {replay_path}")

    try:
        with replay_path.open("r", encoding="utf-8") as replay_file:
            json_log = json.load(replay_file)
        scores = [int(json_log["scores"][str(i)]) for i in range(len(player_commands))]
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        fail(f"invalid replay artifact {replay_path}: {error}")

    attributes = []
    pattern = re.compile(
        r"\[(T|P)DATA\](?:\[(\d+)\])?\s+(\w+)\s*=\s*(.+)",
        re.IGNORECASE,
    )
    for player, key in enumerate(str(i) for i in range(len(player_commands))):
        for data in json_log.get("errors", {}).get(key, []):
            if not data:
                continue
            for line in (line.strip() for line in data.splitlines()):
                match = pattern.match(line)
                if not match:
                    continue
                type_tag, turn, name, value = match.groups()
                attributes.append(
                    {
                        "name": name,
                        "player": player if type_tag.upper() == "P" else None,
                        "turn": int(turn) if turn else None,
                        "value": value,
                    }
                )

    result = {
        "scores": scores,
        "ranks": [
            sum(int(player_score < other_score) for other_score in scores)
            for player_score in scores
        ],
        "errors": [int(player_score < 0) for player_score in scores],
        "attributes": attributes,
    }
    print(json.dumps(result))


if __name__ == "__main__":
    main()