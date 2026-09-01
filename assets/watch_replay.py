import json
import shutil
import signal
import subprocess
import sys
from pathlib import Path

REFEREE_PATH = Path("referee/target/referee.jar")
child_process = None


def fail(message):
    raise RuntimeError(message)


def cleanup_child(signum=None, frame=None):
    global child_process
    if child_process is not None and child_process.poll() is None:
        child_process.terminate()
        try:
            child_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child_process.kill()
            child_process.wait()
    if signum is not None:
        raise SystemExit(128 + signum)


def copy_replay_bundle(source_directory, destination_directory):
    shutil.copytree(source_directory, destination_directory, dirs_exist_ok=True)

    # Some Codingame games emit image URLs relative to assets/assets.
    source_assets = destination_directory / "assets"
    nested_assets = source_assets / "assets"
    nested_assets.mkdir(parents=True, exist_ok=True)
    for png_file in source_assets.glob("*.png"):
        shutil.copy2(png_file, nested_assets / png_file.name)

    app_path = destination_directory / "app.js"
    if app_path.is_file():
        app = app_path.read_text(encoding="utf-8")
        app = app.replace("from '../config.js'", "from './config.js'")
        app = app.replace("from '../demo.js'", "from './demo.js'")
        app = app.replace("viewerUrl: '/core/Drawer.js'", "viewerUrl: './core/Drawer.js'")
        app_path.write_text(app, encoding="utf-8")


def main():
    if len(sys.argv) != 5:
        fail(
            "usage: watch_replay.py REPLAY_PATH REPLAY_DIR PORT PLAYER_COUNT"
        )

    replay_path = Path(sys.argv[1])
    replay_directory = Path(sys.argv[2])
    port = sys.argv[3]
    try:
        player_count = int(sys.argv[4])
    except ValueError as error:
        fail(f"invalid player count: {error}")

    java = shutil.which("java")
    if java is None:
        fail("Java was not found on PATH; install a supported JDK")
    if not REFEREE_PATH.is_file():
        fail(f"referee JAR not found: {REFEREE_PATH}")
    if not replay_path.is_file():
        fail(f"replay artifact not found: {replay_path}")

    try:
        with replay_path.open("r", encoding="utf-8") as replay_file:
            replay = json.load(replay_file)
        artifact_player_count = len(replay["agents"])
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        fail(f"invalid replay artifact {replay_path}: {error}")
    if artifact_player_count != player_count:
        fail(
            f"replay participant count mismatch: artifact has "
            f"{artifact_player_count}, match has {player_count}"
        )

    command = [
        java,
        "--add-opens",
        "java.base/java.lang=ALL-UNNAMED",
        "-jar",
        str(REFEREE_PATH),
        "-r",
        str(replay_path),
        "-port",
        port,
    ]

    global child_process
    child_process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        text=True,
    )

    exposed_directory = None
    assert child_process.stdout is not None
    for line in child_process.stdout:
        if line.startswith("Exposed web server dir: "):
            exposed_directory = Path(
                line.removeprefix("Exposed web server dir: ").strip()
            )
            break

    if exposed_directory is None:
        return_code = child_process.wait()
        fail(
            f"renderer exited with status {return_code} without producing a replay bundle"
        )
    if not exposed_directory.is_dir():
        fail(f"renderer exposed directory does not exist: {exposed_directory}")

    replay_directory.mkdir(parents=True, exist_ok=True)
    copy_replay_bundle(exposed_directory, replay_directory)
    if not (replay_directory / "test.html").is_file():
        fail("renderer replay bundle does not contain test.html")


if __name__ == "__main__":
    signal.signal(signal.SIGINT, cleanup_child)
    signal.signal(signal.SIGTERM, cleanup_child)
    try:
        main()
    except Exception as error:
        print(f"replay launcher failed: {error}", file=sys.stderr)
        raise SystemExit(1)
    finally:
        cleanup_child()