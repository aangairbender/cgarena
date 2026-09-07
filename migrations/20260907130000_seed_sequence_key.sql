UPDATE arena_configuration
SET config_json = json_remove(
    json_set(
        config_json,
        '$.evaluation.seed_sequence_key',
        abs(random() % 4294967296)
    ),
    '$.evaluation.generated_seeds'
)
WHERE json_type(config_json, '$.evaluation') = 'object'
  AND json_type(config_json, '$.evaluation.seed_sequence_key') IS NULL;
