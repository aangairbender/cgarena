# Version evaluation plans and match stages

Evaluation plans and stages are stored as immutable revisions: each candidate references the plan revision assigned at submission, and each Match references the stage revision that scheduled it. This preserves interpretable history with only one small reference per Match instead of copying configuration into every row; existing Match data already records the seed, player count and order, and immutable bot identities, while referee identity and version are intentionally excluded.
