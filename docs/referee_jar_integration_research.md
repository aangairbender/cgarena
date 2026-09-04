# Native CodinGame referee-JAR integration

## Decision

**Implement this feature, but do not define it as “accept any `referee.jar`.”**

A path-only configuration can remove `play_game.py` and `watch_replay.py` for the normal CodinGame workflow **when the file is an executable, self-contained, CG-Arena-compatible referee JAR**. A generic official referee source checkout or arbitrary JAR does not supply a stable command-line contract, and cannot safely be inferred.

The public interface should therefore be a tagged referee configuration with two adapters:

- `codingame_jar`: a supported built-in adapter that owns Java invocation, match-result extraction, replay capture, static-bundle generation, validation, and cleanup.
- `command`: the existing `cmd_play_match` / `cmd_watch_replay` contract for custom referees, alternative runtimes, or unsupported JARs.

That makes the common path one field while retaining the actual variation point. It avoids forcing a Java-specific abstraction onto the 1% of users with a native or custom referee.

## Pre-migration CG Arena contract

Before the native adapter, CG Arena delegated two operations to configured commands:

1. **Match execution.** The embedded worker expands `cmd_run` into one command per bot; expands `{SEED}`, `{REPLAY_PATH}`, and player placeholders in `cmd_play_match`; reserves an owned replay path when requested; and requires a JSON result on stdout. It validates that `ranks`, `errors`, and optional `scores` have exactly one value per participant. [worker.rs:684-841](../src/worker.rs#L684-L841)
2. **Replay viewing.** `ReplayViewer` allocates an isolated session directory, calls `cmd_watch_replay`, requires a successful exit and `test.html`, then serves the copied static bundle from the arena origin. It has a 30-second timeout and reaps its direct child on timeout. [replay_viewer.rs:133-170](../src/replay_viewer.rs#L133-L170), [replay_viewer.rs:237-293](../src/replay_viewer.rs#L237-L293)

The bundled legacy scripts implemented the CodinGame-specific part of these generic contracts:

- `play_game.py` calls Java with one complete player command per `-pN`, a seed, and replay-output path; parses `scores` and `errors` from the resulting replay JSON; turns tagged lines from player stderr into attributes; and prints CG Arena's JSON result. [play_game.py:16-89](../assets/play_game.py#L16-L89)
- `watch_replay.py` validates the replay JSON and participant count, starts Java replay mode, reads the engine's exposed-directory marker, copies that directory to the session, applies compatibility rewrites, and terminates/reaps Java. [watch_replay.py:48-133](../assets/watch_replay.py#L48-L133)

The scripts are not merely launchers. Moving to Rust must preserve their artifact/result conversion and cleanup behavior; otherwise match ranking and replay UI regress.

## What an upstream CodinGame referee provides

### The source repository is not an executable JAR contract

The supplied official Summer Challenge 2025 source at commit [`82bbb1b`](https://github.com/CodinGame/SummerChallenge2025-SoakOverflow/tree/82bbb1b383f7cd9ec87e5d2c3f95daf31dde5cd2) declares game-engine libraries but no build plugins, main class, or shaded artifact in its committed POM. [upstream `pom.xml`](https://github.com/CodinGame/SummerChallenge2025-SoakOverflow/blob/82bbb1b383f7cd9ec87e5d2c3f95daf31dde5cd2/pom.xml)

The local working copy contains uncommitted changes: a build section, a `commons-cli` dependency, and an untracked `CommandLineInterface.java`. Those are precisely the additions that create the runnable JAR and its CLI. This is strong evidence that a download/build of the official source **as-is** is not sufficient for CG Arena.

The built local JAR's manifest names `com.codingame.gameengine.runner.CommandLineInterface` as `Main-Class`, but this verifies only that the locally patched/shaded output is executable—not that all official referees are. [`META-INF/MANIFEST.MF`](file:///Users/yevhenkazmin/Projects/cg/referees/SummerChallenge2025-SoakOverflow/target/summer-challenge-2025-super-soaker-1.0-SNAPSHOT.jar:META-INF/MANIFEST.MF)

**Observed experiment (2026-09-04).** Running that locally patched JAR with `java --add-opens java.base/java.lang=ALL-UNNAMED -jar … -p1 "python3 config/Boss.py" -p2 "python3 config/Boss.py" -seed 1 -l /tmp/cgarena-referee-jar-research-1.json` exited successfully in 4.67 seconds, printed one score per player, and wrote the expected replay JSON. The supplied bot fixture failed with `NameError`, so both scores were `-1`; the artifact still contained `errors`, `outputs`, `summaries`, `views`, and `agents`. This proves the *patched* CLI/JAR path is viable and that replay JSON carries the fields the native adapter needs. It does not validate an unmodified upstream JAR.

### CG Arena's maintained CLI is the needed compatibility layer

CG Arena currently supplies a replacement `CommandLineInterface` for referee sources. It accepts:

- `-p1` through `-p8`: each complete bot command;
- `-seed`: signed Java `long` seed;
- `-league`: engine league level, default `19`;
- `-l`: replay JSON output;
- `-r`: replay JSON input;
- `-port`: replay-renderer port.

[CommandLineInterface.java:20-49](../assets/CommandLineInterface.java#L20-L49)

In match mode it constructs `MultiplayerGameRunner`, sets the seed and league, adds agents, invokes engine-private methods reflectively, writes the engine JSON result to `-l`, and prints scores plus referee input. [CommandLineInterface.java:75-123](../assets/CommandLineInterface.java#L75-L123)

In replay mode it derives player count from `agents` in the persisted JSON and starts the engine renderer on `-port`. [CommandLineInterface.java:61-73](../assets/CommandLineInterface.java#L61-L73)

This is the functional contract CG Arena can support. It is version-sensitive because it relies on engine private fields and methods (`initialize`, `runAgents`, `getJSONResult`, `players`, and `process`). The same symbols are private in the current engine source at [`9e32d14`](https://github.com/CodinGame/codingame-game-engine/tree/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c). [GameRunner.java](https://github.com/CodinGame/codingame-game-engine/blob/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c/runner/src/main/java/com/codingame/gameengine/runner/GameRunner.java#L55-L68) and [the private match methods](https://github.com/CodinGame/codingame-game-engine/blob/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c/runner/src/main/java/com/codingame/gameengine/runner/GameRunner.java#L114-L219).

### Java and process behavior

The scripts invoke `java --add-opens java.base/java.lang=ALL-UNNAMED -jar …`. [play_game.py:23-40](../assets/play_game.py#L23-L40), [watch_replay.py:62-92](../assets/watch_replay.py#L62-L92) The flag exists because the runner's reflection and process cleanup are sensitive to modern Java access restrictions; it must remain a default of the built-in adapter, with an advanced override only if evidence shows a game needs a different JVM invocation.

The example referee requests Java 17. [example `pom.xml`:9-14](file:///Users/yevhenkazmin/Projects/cg/referees/SummerChallenge2025-SoakOverflow/pom.xml#L9-L14) CG Arena should require a Java runtime compatible with the JAR's bytecode rather than hard-code a Java version.

The runner executes each player command through `Runtime.exec`; player command tokenization is therefore governed by Java rather than a shell. [MultiplayerGameRunner.java:88-97](https://github.com/CodinGame/codingame-game-engine/blob/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c/runner/src/main/java/com/codingame/gameengine/runner/MultiplayerGameRunner.java#L88-L97) CG Arena must pass every expanded `cmd_run` as one `-pN` argument; it must not split or shell-wrap the player command.

## Replay risks that native integration must explicitly solve

The engine renderer is unsuitable as a long-running shared CG Arena replay server:

- It writes to the fixed directory `${java.io.tmpdir}/codingame` and deletes it before every render. Concurrent renderers race and can destroy each other's source bundle. [Renderer.java:335-365](https://github.com/CodinGame/codingame-game-engine/blob/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c/runner/src/main/java/com/codingame/gameengine/runner/Renderer.java#L335-L365)
- It logs `Exposed web server dir: …`, then starts an Undertow server bound to `0.0.0.0`, not loopback. [Renderer.java:646-656](https://github.com/CodinGame/codingame-game-engine/blob/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c/runner/src/main/java/com/codingame/gameengine/runner/Renderer.java#L646-L656)
- A bind failure is downgraded to a warning, so observing a process start or a log line does not prove that the requested port is serving the intended replay. [Renderer.java:875-893](https://github.com/CodinGame/codingame-game-engine/blob/9e32d14b8845d1a42b6fef61b41f0084d1eaf81c/runner/src/main/java/com/codingame/gameengine/runner/Renderer.java#L875-L893)
- The legacy wrapper copied the exposed files into the CG Arena-owned session, verified `test.html`, applied known relative-asset fixes, then terminated Java. [watch_replay.py:29-45](../assets/watch_replay.py#L29-L45), [watch_replay.py:101-121](../assets/watch_replay.py#L101-L121)

The native adapter must preserve this disposable-renderer model. Run every renderer with a unique, adapter-owned `java.io.tmpdir`; capture stdout until the exposed directory is announced; copy it to the already-isolated replay session; apply only compatibility rewrites with regression fixtures; then terminate and reap the full renderer process tree. Do not expose the renderer port to the browser or accept the renderer's directory as an artifact path without containment validation.

The pre-migration `ReplayViewer` only killed its direct configured child. [replay_viewer.rs:251-271](../src/replay_viewer.rs#L251-L271) The native implementation must improve on the Python wrapper by placing the Java process in its own process group/session where supported, so shutdown cannot leave an Undertow JVM behind.

## Recommended interface

Use an exclusive `referee` value in each embedded worker; do not put `referee_jar` beside the existing command fields. Two adjacent sources of truth would make precedence, replay ownership, and error semantics ambiguous.

```toml
[[workers]]
type = "embedded"
threads = 4
cmd_build = "g++ -std=c++20 -x c++ {DIR}/source.txt -o {DIR}/a"
cmd_run = "./{DIR}/a"

[workers.referee]
type = "codingame_jar"
path = "referee/target/referee.jar"
# Optional. Defaults to `java` and `19`.
# java = "java"
# league = 19
```

For the compatible custom path:

```toml
[workers.referee]
type = "command"
play_match = "my-referee {SEED} {REPLAY_PATH} {PLAYERS}"
watch_replay = "my-renderer {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}"
```

Clean cutover: migrate the default configuration and generated setup to `codingame_jar`; remove the generated Python scripts from the default path; retain the documented `command` adapter as the sole migration target for non-Java/custom referees. Existing configurations can be migrated in the same release or supported temporarily through an explicit config-format migration. Do not silently prioritize one set of fields over another.

The external interface remains small:

- one selected referee adapter;
- `cmd_run` yields an opaque player command per participant;
- match result is CG Arena's existing `ranks`, `errors`, optional `scores`, and attributes;
- replay result is the existing owned static bundle with `test.html`.

The adapter hides all Java arguments, replay JSON parsing, renderer output parsing, copy/normalization, cancellation, and JAR validation. That is a deep module: callers learn one configuration form, while the engine-specific behavior stays local and testable.

## Implementation plan

1. **Lock the supported-JAR definition.** Call it `CG-Arena-compatible CodinGame referee JAR`: self-contained `java -jar` entrypoint implementing the current `-pN/-seed/-l/-r/-port` contract. Update the referee patch/build guide to produce that exact artifact. Reject missing, non-regular, or incompatible JARs at arena startup with a diagnostic that names the required contract. Do not claim support for an unmodified official JAR.
2. **Introduce a `RefereeAdapter` seam.** Configuration selects exactly one adapter. Implement `CommandRefereeAdapter` by moving the present command fields intact, then implement `CodingameJarRefereeAdapter`. Both produce the existing match output and static replay bundle contracts; worker, arena, database, and API need no domain-model change.
3. **Port match behavior with contract tests.** Invoke Java directly with argument vectors; pass each complete bot command as one `-pN`; create the replay through `ReplayArtifacts`; parse replay `scores`, player errors, and `[TDATA]/[PDATA]` attributes identically to `play_game.py`; calculate ranks using the existing wrapper rule; report actionable stderr without corrupting the result channel.
4. **Port replay behavior safely.** Give each invocation a unique JVM temp directory and ephemeral loopback port; validate the JSON `agents` count against the persisted participant count; wait for and validate the exposed path; copy it into the session; apply only tested asset rewrites; terminate and reap Java in success, error, timeout, and cancellation paths.
5. **Migrate defaults and verify real artifacts.** Replace default generated Python commands with the JAR adapter and update setup/configuration docs. Keep the command adapter example for custom implementations. Add tests for config exclusivity, Java argument construction with spaces, replay score/error/attribute conversion, invalid artifacts, concurrent renderer isolation, timeout cleanup, and a fixture JAR covering a real match plus replay bundle.

## Bottom line

The feature is high-leverage: it removes Python and two editable scripts from the dominant setup, centralizes a fragile CodinGame integration, and gives CG Arena ownership of artifacts and process cleanup. The safe product promise is **“supply a supported CG-Arena-compatible referee JAR”**, not **“supply any official JAR.”** The latter would be misleading because upstream referee repositories and JARs do not consistently provide the required executable CLI, artifact behavior, or renderer semantics.
