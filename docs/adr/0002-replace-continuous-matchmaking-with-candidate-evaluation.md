# Replace continuous matchmaking with candidate evaluation

CG Arena uses bounded evaluation of candidate bots against a live benchmark pool. Focused candidate evidence matches the author's improvement workflow better than indefinite all-versus-all ranking.

## Consequences

- Existing bots migrate into the benchmark pool; new submissions default to Candidate but may be submitted directly as Benchmark.
- Each evaluation Match contains exactly one candidate, with all remaining player slots filled by active benchmarks.
- Every candidate pins the evaluation-plan revision active at submission, while later promotions and benchmark archival change the live opponent pool for active candidates.
- Evaluation plans contain ordered stages defining seed source, coverage policy, and player-count range.
- An arena-level key produces an unbounded deterministic seed sequence reused across candidate evaluations; curated and fresh-random seed stages provide edge-case and generalization evidence.
- Player counts are balanced deterministically within each stage's configured range.
- Completion means evidence collection is complete. Promotion and rejection remain entirely user-controlled, with no automatic verdict.
- The primary UI is an evaluation dashboard while global and custom leaderboards remain first-class evidence surfaces.
