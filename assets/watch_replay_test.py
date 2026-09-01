import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import watch_replay


class FakeProcess:
    def __init__(self, lines):
        self.stdout = io.StringIO("".join(lines))
        self.returncode = None
        self.terminated = False

    def poll(self):
        return self.returncode

    def terminate(self):
        self.terminated = True
        self.returncode = 0

    def kill(self):
        self.returncode = -9

    def wait(self, timeout=None):
        return 0 if self.returncode is None else self.returncode


class WatchReplayTests(unittest.TestCase):
    def tearDown(self):
        watch_replay.child_process = None

    def test_passes_argument_list_and_copies_bundle_for_paths_with_spaces(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            referee = root / "referee jar.jar"
            referee.write_bytes(b"jar")
            artifact = root / "replay artifact.json"
            artifact.write_text(json.dumps({"agents": [{}, {}]}), encoding="utf-8")
            generated = root / "generated bundle"
            (generated / "assets").mkdir(parents=True)
            (generated / "test.html").write_text("fixture", encoding="utf-8")
            (generated / "assets" / "image.png").write_bytes(b"png")
            destination = root / "session bundle"
            process = FakeProcess(
                [
                    "http://localhost:32123/test.html\n",
                    f"Exposed web server dir: {generated}\n",
                ]
            )

            with (
                mock.patch.object(watch_replay, "REFEREE_PATH", referee),
                mock.patch.object(watch_replay.shutil, "which", return_value="/java path/java"),
                mock.patch.object(watch_replay.subprocess, "Popen", return_value=process) as popen,
                mock.patch.object(
                    watch_replay.sys,
                    "argv",
                    [
                        "watch_replay.py",
                        str(artifact),
                        str(destination),
                        "32123",
                        "2",
                    ],
                ),
            ):
                watch_replay.main()
                watch_replay.cleanup_child()

            popen.assert_called_once_with(
                [
                    "/java path/java",
                    "--add-opens",
                    "java.base/java.lang=ALL-UNNAMED",
                    "-jar",
                    str(referee),
                    "-r",
                    str(artifact),
                    "-port",
                    "32123",
                ],
                stdout=watch_replay.subprocess.PIPE,
                text=True,
            )
            self.assertTrue(process.terminated)
            self.assertEqual((destination / "test.html").read_text(), "fixture")
            self.assertTrue((destination / "assets" / "assets" / "image.png").is_file())

    def test_rejects_participant_count_mismatch_before_starting_java(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            referee = root / "referee.jar"
            referee.write_bytes(b"jar")
            artifact = root / "replay.json"
            artifact.write_text(json.dumps({"agents": [{}, {}]}), encoding="utf-8")

            with (
                mock.patch.object(watch_replay, "REFEREE_PATH", referee),
                mock.patch.object(watch_replay.shutil, "which", return_value="java"),
                mock.patch.object(watch_replay.subprocess, "Popen") as popen,
                mock.patch.object(
                    watch_replay.sys,
                    "argv",
                    ["watch_replay.py", str(artifact), str(root / "bundle"), "1", "3"],
                ),
            ):
                with self.assertRaisesRegex(RuntimeError, "participant count mismatch"):
                    watch_replay.main()
                popen.assert_not_called()


if __name__ == "__main__":
    unittest.main()
