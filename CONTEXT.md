# CG Arena

CG Arena is a personal, trusted local workbench for bot authors participating in CodinGame challenges. It exists to evaluate bot improvements more reliably than the limited matches available on CodinGame through bounded candidate evaluations and analysis.

## Language

**Bot author**:
An individual who develops and evaluates bots for a CodinGame challenge.
_Avoid_: Arena administrator, tournament organizer, participant

**Challenge**:
A CodinGame bot competition whose rules and referee define valid matches.
_Avoid_: Game, tournament

**Arena**:
A local evaluation workspace controlled by one bot author and dedicated to exactly one challenge, containing its bots, benchmark pool, evaluation plan, match history, ratings, and analytics.
_Avoid_: Installation, tournament

**Bot**:
One immutable program submission with a stable identity. Its display name may change, but changing its source or language creates a new bot.
_Avoid_: Mutable bot, bot revision

**Candidate evaluation**:
The bounded assessment started when a bot is submitted as a candidate and run against the live benchmark pool using the evaluation plan revision assigned at submission. It ends whenever the user promotes or rejects the candidate.
_Avoid_: Experiment object, tournament, indefinite matchmaking

**Evaluation completion**:
The state reached when every configured evaluation stage has collected its required evidence. Completion is not a success verdict and never changes a bot's role automatically.
_Avoid_: Candidate success, promotion

**Evaluation match**:
A match containing exactly one candidate bot, with every other player slot filled by active benchmark bots. Each benchmark co-participant constitutes one candidate–benchmark encounter.
_Avoid_: Candidate-versus-candidate match

**Match**:
A valid result produced by the challenge referee for a completed contest among bots. A Match remains valid evidence when a participating bot errors.
_Avoid_: Match attempt, execution

**Match provenance**:
The minimal context needed to interpret a Match: its seed, ordered immutable bot identities, player count, and evaluation stage revision. Referee identity and version are not part of this record.
_Avoid_: Full execution snapshot

**Bot error**:
A participant failure reported within an otherwise valid Match, such as a crash, timeout, or invalid bot output. It counts toward evaluation coverage and ratings.
_Avoid_: Execution failure

**Execution failure**:
An attempt where CG Arena or the referee cannot produce a valid Match. It does not become match evidence or satisfy evaluation coverage.
_Avoid_: Bot error, failed Match

**Candidate bot**:
A bot submission currently being evaluated against benchmark bots. Candidate is the default role for new submissions, and multiple candidates may represent alternative implementations or parameter values.
_Avoid_: Challenger, experiment

**Benchmark bot**:
A retained bot submission used as a comparison opponent for candidate bots. A bot may be submitted directly as a benchmark or promoted from candidate status.
_Avoid_: Baseline bot, control bot

**Benchmark pool**:
The live set of benchmark bots used to evaluate candidates. Active candidates must adapt to promotions, archival, and deletion that change this set.
_Avoid_: Benchmark group, fixed benchmark snapshot, experiment cohort

**Leaderboard leader**:
The bot with the highest score on a given leaderboard. It is derived from ratings and is not an assigned bot state.
_Avoid_: Local best, best bot

**Promotion**:
The user-controlled transition of a candidate bot into the benchmark pool, allowed at any point in its evaluation.
_Avoid_: Winner declaration, deployment

**Candidate rejection**:
The permanent removal of an unsuccessful candidate bot together with its match evidence.
_Avoid_: Candidate archival

**Benchmark archival**:
The retirement of a weak benchmark bot from future evaluation and active views while retaining its source and historical match evidence. That evidence continues to inform active bots' ratings.
_Avoid_: Benchmark deletion

**Benchmark deletion**:
The permanent removal of a benchmark bot together with its source, builds, Match evidence, analytics, and replay artifacts.
_Avoid_: Benchmark archival, Candidate rejection

**Coverage policy**:
The rule that determines when a candidate has accumulated enough match evidence. Policies may require coverage per benchmark or across the whole benchmark pool.
_Avoid_: Matchmaking algorithm, match limit

**Per-benchmark coverage**:
A coverage policy requiring a candidate to complete a target number of encounters with every active benchmark bot.
_Avoid_: Total matches per bot

**Evaluation plan**:
The currently configured, ordered sequence of evaluation stages used for new candidate evaluations.
_Avoid_: Experiment

**Evaluation plan revision**:
An immutable ordered set of evaluation stage revisions assigned to a candidate at submission. Editing the plan creates a new revision for future candidates without changing active evaluations.
_Avoid_: Live plan

**Evaluation stage**:
One step in an evaluation plan, defining a seed source, player-count range, and match coverage requirement that completes before the next stage begins. Its player-count range defaults to the challenge's full supported range.
_Avoid_: Matchmaking phase

**Evaluation stage revision**:
An immutable historical definition of an evaluation stage. Editing a plan creates new revisions, and each Match references the revision that scheduled it.
_Avoid_: Current stage settings, per-match settings copy

**Player-count schedule**:
The balanced, deterministic distribution of evaluation matches across an evaluation stage's allowed player-count range. Static seed assignments remain stable across candidates.
_Avoid_: Random player-count selection

**Generated seed sequence**:
An arena-level deterministic stream of Match seeds identified by a sequence key and reused across candidate evaluations and benchmark pairings. Changing the key creates a different stream for future evaluations without imposing a fixed sequence length.
_Avoid_: Per-candidate seeds, finite seed suite

**Curated seed**:
A user-selected seed retained because it represents a known scenario or edge case worth evaluating repeatedly.
_Avoid_: Hardcoded seed

**Fresh random seed**:
A newly generated seed used to sample behavior beyond the generated deterministic sequence and curated seeds.
_Avoid_: Static seed

**Referee**:
The challenge-specific authority that executes a Match and reports participant outcomes, attributes, and optional replay data.
_Avoid_: Arena, worker

**Rating**:
An algorithm-derived estimate of a bot's strength based on valid Match outcomes.
_Avoid_: Rank, leaderboard position

**Leaderboard score**:
The scalar derived from a Rating and used to order bots on a Leaderboard.
_Avoid_: Rating, rank

**Leaderboard**:
An ordered analytical view derived from Matches satisfying a Match filter.
_Avoid_: Benchmark pool

**Global leaderboard**:
The protected Leaderboard derived from all retained Match evidence. Archived benchmarks are hidden from its active view while their evidence continues to inform active ratings.
_Avoid_: Benchmark leaderboard

**Custom leaderboard**:
A named Leaderboard derived from Matches satisfying a user-defined Match filter.
_Avoid_: Evaluation stage

**Match attribute**:
A typed observation attached to a Match or one of its participants, optionally at a specific turn.
_Avoid_: Configuration field

**Match filter**:
A predicate over Match attributes used consistently for match browsing, custom Leaderboards, and analytics.
_Avoid_: Bot selector

**Replay**:
An optional viewable record of a completed Match used to inspect what happened.
_Avoid_: Match result
