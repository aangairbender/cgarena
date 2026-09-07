import * as api from "@/api";
import { ConfigurationAdapter, configurationQueryKey } from "@/configuration";
import {
  ArenaConfiguration,
  ConfigurationState,
  EmbeddedWorkerConfiguration,
  EvaluationStageConfiguration,
  RankingConfiguration,
  RefereeConfiguration,
  RefereeAction,
} from "@/models";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FormEvent, useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Col,
  Form,
  Row,
  Spinner,
  Stack,
} from "react-bootstrap";

const httpAdapter: ConfigurationAdapter = {
  fetch: api.fetchConfiguration,
  apply: api.applyConfiguration,
};

function generatedSeedSuite(): number[] {
  const seeds = new Set<number>();
  while (seeds.size < 100) {
    seeds.add(Math.floor(Math.random() * 0x1_0000_0000) - 0x8000_0000);
  }
  return [...seeds];
}

function defaultEvaluationStage(): EvaluationStageConfiguration {
  return {
    name: "Generated suite",
    seed_source: { type: "generated_static" },
    coverage: { type: "per_benchmark", target: 100 },
    min_players: null,
    max_players: null,
  };
}

function defaultConfiguration(): ArenaConfiguration {
  return {
    game: {
      min_players: 2,
      max_players: 2,
      symmetric: true,
    },
    evaluation: {
      enabled_on_start: true,
      generated_seeds: generatedSeedSuite(),
      stages: [defaultEvaluationStage()],
    },
    ranking: { algorithm: "BradleyTerry", max_iter: null },
    leaderboards: { uncertainty_coefficient: null },
    workers: [
      {
        type: "embedded",
        threads: 1,
        cmd_build: "g++ -std=c++20 -x c++ {DIR}/source.txt -o {DIR}/a",
        cmd_run: "./{DIR}/a",
        referee: {
          type: "managed_codingame",
          repository_url:
            "https://github.com/CodinGame/SpringChallenge2023.git",
          branch: null,
          java: null,
          maven: null,
        },
      },
    ],
  };
}

function optionalNumber(value: string): number | null {
  return value === "" ? null : Number(value);
}

function parseNumbers(value: string): number[] {
  return value
    .split(/[,\s]+/)
    .map(Number)
    .filter(Number.isFinite);
}

function parseWeights(value: string): Record<string, number> {
  return Object.fromEntries(
    value
      .split(/[,\s]+/)
      .map((entry) => entry.split(":"))
      .filter(
        (entry) =>
          entry.length === 2 &&
          entry[0] !== "" &&
          Number.isFinite(Number(entry[1])),
      )
      .map(([id, weight]) => [id, Number(weight)]),
  );
}

interface ConfigPageProps {
  adapter?: ConfigurationAdapter;
}

export default function ConfigPage({ adapter = httpAdapter }: ConfigPageProps) {
  const configuration = useQuery({
    queryKey: configurationQueryKey,
    queryFn: () => adapter.fetch(),
  });

  if (configuration.isPending) {
    return <Spinner animation="border" aria-label="Loading configuration" />;
  }
  if (configuration.error) {
    return <Alert variant="danger">{configuration.error.message}</Alert>;
  }

  return (
    <ConfigurationForm adapter={adapter} initialState={configuration.data} />
  );
}

function ConfigurationForm({
  adapter,
  initialState,
}: {
  adapter: ConfigurationAdapter;
  initialState: ConfigurationState;
}) {
  const queryClient = useQueryClient();
  const [state, setState] = useState(initialState);
  const [draft, setDraft] = useState<ArenaConfiguration>(
    initialState.active ?? defaultConfiguration(),
  );
  const [saved, setSaved] = useState(false);
  const apply = useMutation({
    mutationFn: (candidate: ArenaConfiguration) => adapter.apply(candidate),
    onSuccess: (nextState) => {
      queryClient.setQueryData(configurationQueryKey, nextState);
      setState(nextState);
      setDraft(nextState.active ?? draft);
      setSaved(true);
    },
  });

  const firstRun = state.active === null;
  const worker = draft.workers[0];
  const refereeStatus = useQuery({
    queryKey: ["managed-referee"],
    queryFn: api.fetchRefereeStatus,
    enabled: worker.referee.type === "managed_codingame",
    refetchInterval: (query) =>
      query.state.data?.operation.action ? 500 : false,
  });
  const refereeAction = useMutation({
    mutationFn: async (action: RefereeAction) => {
      if (action === "install") {
        const nextState = await adapter.apply(draft);
        queryClient.setQueryData(configurationQueryKey, nextState);
        setState(nextState);
      }
      await api.startRefereeAction(action);
    },
    onSuccess: () => refereeStatus.refetch(),
  });
  useEffect(() => {
    if (
      refereeStatus.data?.operation.action === null &&
      refereeStatus.data.operation.diagnostic
    ) {
      void adapter.fetch().then((nextState) => {
        queryClient.setQueryData(configurationQueryKey, nextState);
        setState(nextState);
      });
    }
  }, [
    queryClient,
    refereeStatus.data?.operation.action,
    adapter,
    refereeStatus.data?.operation.diagnostic,
  ]);

  const setWorker = (next: Partial<EmbeddedWorkerConfiguration>) => {
    setDraft((current) => ({
      ...current,
      workers: [{ ...current.workers[0], ...next }],
    }));
    setSaved(false);
  };

  const updateStage = (
    index: number,
    update: (
      stage: EvaluationStageConfiguration,
    ) => EvaluationStageConfiguration,
  ) => {
    setDraft((current) => ({
      ...current,
      evaluation: {
        ...current.evaluation,
        stages: current.evaluation.stages.map((stage, stageIndex) =>
          stageIndex === index ? update(stage) : stage,
        ),
      },
    }));
    setSaved(false);
  };

  const moveStage = (index: number, offset: number) => {
    setDraft((current) => {
      const stages = [...current.evaluation.stages];
      const [stage] = stages.splice(index, 1);
      stages.splice(index + offset, 0, stage);
      return {
        ...current,
        evaluation: { ...current.evaluation, stages },
      };
    });
    setSaved(false);
  };

  const updateCommandReferee = (
    next: Partial<{ play_match: string; watch_replay: string }>,
  ) => {
    if (worker.referee.type === "command") {
      setWorker({ referee: { ...worker.referee, ...next } });
    }
  };

  const updateManagedReferee = (
    next: Partial<{
      repository_url: string;
      branch: string | null;
      java: string | null;
      maven: string | null;
    }>,
  ) => {
    if (worker.referee.type === "managed_codingame") {
      setWorker({ referee: { ...worker.referee, ...next } });
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setSaved(false);
    apply.mutate(draft);
  };

  return (
    <Stack gap={3}>
      <div>
        <h1>{firstRun ? "Set up your arena" : "Arena configuration"}</h1>
        <p className="text-body-secondary mb-0">
          Apply one complete configuration. Server and logging settings remain
          in cgarena_config.toml and require a restart.
        </p>
      </div>

      {firstRun && (
        <Alert variant="info">
          The server is ready, but matches and replays stay unavailable until a
          valid arena configuration and its runtime prerequisites are active.
        </Alert>
      )}
      {state.runtime_error && (
        <Alert variant="warning">{state.runtime_error}</Alert>
      )}
      {saved && (
        <Alert variant="success">
          Configuration saved as the active configuration.
          {!state.runtime_available &&
            " Runtime features are still unavailable in this process."}
        </Alert>
      )}
      {apply.error && <Alert variant="danger">{apply.error.message}</Alert>}

      <Form onSubmit={submit}>
        <Stack gap={3}>
          <Card>
            <Card.Body>
              <Card.Title>Game</Card.Title>
              <Row className="g-3">
                <Col md={4}>
                  <Form.Group controlId="min-players">
                    <Form.Label>Minimum players</Form.Label>
                    <Form.Control
                      type="number"
                      min={1}
                      max={8}
                      required
                      value={draft.game.min_players}
                      onChange={(event) =>
                        setDraft((current) => ({
                          ...current,
                          game: {
                            ...current.game,
                            min_players: Number(event.target.value),
                          },
                        }))
                      }
                    />
                  </Form.Group>
                </Col>
                <Col md={4}>
                  <Form.Group controlId="max-players">
                    <Form.Label>Maximum players</Form.Label>
                    <Form.Control
                      type="number"
                      min={1}
                      max={8}
                      required
                      value={draft.game.max_players}
                      onChange={(event) =>
                        setDraft((current) => ({
                          ...current,
                          game: {
                            ...current.game,
                            max_players: Number(event.target.value),
                          },
                        }))
                      }
                    />
                  </Form.Group>
                </Col>
                <Col md={4} className="d-flex align-items-end">
                  <Form.Check
                    id="symmetric-game"
                    type="switch"
                    label="Symmetric game"
                    checked={draft.game.symmetric}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        game: {
                          ...current.game,
                          symmetric: event.target.checked,
                        },
                      }))
                    }
                  />
                </Col>
              </Row>
            </Card.Body>
          </Card>

          <Card>
            <Card.Body>
              <div className="d-flex justify-content-between align-items-center mb-3">
                <div>
                  <Card.Title className="mb-1">
                    Candidate evaluation plan
                  </Card.Title>
                  <Card.Text className="text-body-secondary mb-0">
                    Candidates pin this ordered plan when submitted. Existing
                    candidates keep their pinned revision after edits.
                  </Card.Text>
                </div>
                <Button
                  variant="outline-secondary"
                  onClick={() => {
                    setDraft((current) => ({
                      ...current,
                      evaluation: {
                        ...current.evaluation,
                        generated_seeds: generatedSeedSuite(),
                      },
                    }));
                    setSaved(false);
                  }}
                >
                  Regenerate 100-seed suite
                </Button>
              </div>

              <Form.Check
                id="start-evaluation-scheduling"
                className="mb-3"
                type="switch"
                label="Start evaluation scheduling automatically"
                checked={draft.evaluation.enabled_on_start ?? true}
                onChange={(event) => {
                  setDraft((current) => ({
                    ...current,
                    evaluation: {
                      ...current.evaluation,
                      enabled_on_start: event.target.checked,
                    },
                  }));
                  setSaved(false);
                }}
              />

              <Stack gap={3}>
                {draft.evaluation.stages.map((stage, index) => (
                  <Card key={index} className="border-secondary">
                    <Card.Body>
                      <div className="d-flex justify-content-between align-items-center mb-3">
                        <strong>Stage {index + 1}</strong>
                        <Stack direction="horizontal" gap={2}>
                          <Button
                            size="sm"
                            variant="outline-secondary"
                            disabled={index === 0}
                            onClick={() => moveStage(index, -1)}
                          >
                            Up
                          </Button>
                          <Button
                            size="sm"
                            variant="outline-secondary"
                            disabled={
                              index === draft.evaluation.stages.length - 1
                            }
                            onClick={() => moveStage(index, 1)}
                          >
                            Down
                          </Button>
                          <Button
                            size="sm"
                            variant="outline-danger"
                            disabled={draft.evaluation.stages.length === 1}
                            onClick={() => {
                              setDraft((current) => ({
                                ...current,
                                evaluation: {
                                  ...current.evaluation,
                                  stages: current.evaluation.stages.filter(
                                    (_, stageIndex) => stageIndex !== index,
                                  ),
                                },
                              }));
                              setSaved(false);
                            }}
                          >
                            Remove
                          </Button>
                        </Stack>
                      </div>
                      <Row className="g-3">
                        <Col md={4}>
                          <Form.Group controlId={`stage-${index}-name`}>
                            <Form.Label>Name</Form.Label>
                            <Form.Control
                              required
                              value={stage.name}
                              onChange={(event) =>
                                updateStage(index, (current) => ({
                                  ...current,
                                  name: event.target.value,
                                }))
                              }
                            />
                          </Form.Group>
                        </Col>
                        <Col md={4}>
                          <Form.Group controlId={`stage-${index}-seed-source`}>
                            <Form.Label>Seed source</Form.Label>
                            <Form.Select
                              value={stage.seed_source.type}
                              onChange={(event) =>
                                updateStage(index, (current) => ({
                                  ...current,
                                  seed_source:
                                    event.target.value === "curated"
                                      ? { type: "curated", seeds: [1] }
                                      : event.target.value === "fresh_random"
                                        ? { type: "fresh_random" }
                                        : { type: "generated_static" },
                                }))
                              }
                            >
                              <option value="generated_static">
                                Generated static suite
                              </option>
                              <option value="curated">Curated seeds</option>
                              <option value="fresh_random">Fresh random</option>
                            </Form.Select>
                          </Form.Group>
                        </Col>
                        <Col md={4}>
                          <Form.Group controlId={`stage-${index}-coverage`}>
                            <Form.Label>Coverage</Form.Label>
                            <Form.Select
                              value={stage.coverage.type}
                              onChange={(event) =>
                                updateStage(index, (current) => ({
                                  ...current,
                                  coverage:
                                    event.target.value === "total"
                                      ? { type: "total", target: 100 }
                                      : event.target.value === "weighted_total"
                                        ? {
                                            type: "weighted_total",
                                            target: 100,
                                            weights: {},
                                          }
                                        : {
                                            type: "per_benchmark",
                                            target: 100,
                                          },
                                }))
                              }
                            >
                              <option value="per_benchmark">
                                Per benchmark
                              </option>
                              <option value="total">Total matches</option>
                              <option value="weighted_total">
                                Weighted total
                              </option>
                            </Form.Select>
                          </Form.Group>
                        </Col>
                        <Col md={4}>
                          <Form.Group controlId={`stage-${index}-target`}>
                            <Form.Label>Target</Form.Label>
                            <Form.Control
                              type="number"
                              min={1}
                              required
                              value={stage.coverage.target}
                              onChange={(event) =>
                                updateStage(index, (current) => ({
                                  ...current,
                                  coverage: {
                                    ...current.coverage,
                                    target: Number(event.target.value),
                                  },
                                }))
                              }
                            />
                          </Form.Group>
                        </Col>
                        <Col md={4}>
                          <Form.Group controlId={`stage-${index}-min-players`}>
                            <Form.Label>Stage minimum players</Form.Label>
                            <Form.Control
                              type="number"
                              min={draft.game.min_players}
                              max={draft.game.max_players}
                              placeholder={`${draft.game.min_players} (game default)`}
                              value={stage.min_players ?? ""}
                              onChange={(event) =>
                                updateStage(index, (current) => ({
                                  ...current,
                                  min_players: optionalNumber(
                                    event.target.value,
                                  ),
                                }))
                              }
                            />
                          </Form.Group>
                        </Col>
                        <Col md={4}>
                          <Form.Group controlId={`stage-${index}-max-players`}>
                            <Form.Label>Stage maximum players</Form.Label>
                            <Form.Control
                              type="number"
                              min={draft.game.min_players}
                              max={draft.game.max_players}
                              placeholder={`${draft.game.max_players} (game default)`}
                              value={stage.max_players ?? ""}
                              onChange={(event) =>
                                updateStage(index, (current) => ({
                                  ...current,
                                  max_players: optionalNumber(
                                    event.target.value,
                                  ),
                                }))
                              }
                            />
                          </Form.Group>
                        </Col>
                        {stage.seed_source.type === "curated" && (
                          <Col xs={12}>
                            <Form.Group
                              controlId={`stage-${index}-curated-seeds`}
                            >
                              <Form.Label>Curated seeds</Form.Label>
                              <Form.Control
                                as="textarea"
                                value={stage.seed_source.seeds.join(", ")}
                                onChange={(event) =>
                                  updateStage(index, (current) => ({
                                    ...current,
                                    seed_source: {
                                      type: "curated",
                                      seeds: parseNumbers(event.target.value),
                                    },
                                  }))
                                }
                              />
                            </Form.Group>
                          </Col>
                        )}
                        {stage.coverage.type === "weighted_total" && (
                          <Col xs={12}>
                            <Form.Group controlId={`stage-${index}-weights`}>
                              <Form.Label>Benchmark weights</Form.Label>
                              <Form.Control
                                placeholder="bot-id:weight, for example 3:2, 7:0.5"
                                value={Object.entries(stage.coverage.weights)
                                  .map(([id, weight]) => `${id}:${weight}`)
                                  .join(", ")}
                                onChange={(event) =>
                                  updateStage(index, (current) => ({
                                    ...current,
                                    coverage: {
                                      type: "weighted_total",
                                      target: current.coverage.target,
                                      weights: parseWeights(event.target.value),
                                    },
                                  }))
                                }
                              />
                            </Form.Group>
                          </Col>
                        )}
                      </Row>
                    </Card.Body>
                  </Card>
                ))}
                <Button
                  variant="outline-primary"
                  onClick={() => {
                    setDraft((current) => ({
                      ...current,
                      evaluation: {
                        ...current.evaluation,
                        stages: [
                          ...current.evaluation.stages,
                          defaultEvaluationStage(),
                        ],
                      },
                    }));
                    setSaved(false);
                  }}
                >
                  Add stage
                </Button>
              </Stack>
            </Card.Body>
          </Card>

          <Card>
            <Card.Body>
              <Card.Title>Ranking and leaderboard</Card.Title>
              <Row className="g-3">
                <Col md={6}>
                  <Form.Group controlId="ranking-algorithm">
                    <Form.Label>Ranking algorithm</Form.Label>
                    <Form.Select
                      value={draft.ranking.algorithm}
                      onChange={(event) => {
                        const algorithms: Record<string, RankingConfiguration> =
                          {
                            OpenSkill: {
                              algorithm: "OpenSkill",
                              beta: null,
                              uncertainty_tolerance: null,
                            },
                            TrueSkill: {
                              algorithm: "TrueSkill",
                              draw_probability: null,
                              beta: null,
                              default_dynamics: null,
                            },
                            Elo: { algorithm: "Elo", k: null },
                            BradleyTerry: {
                              algorithm: "BradleyTerry",
                              max_iter: null,
                            },
                          };
                        setDraft((current) => ({
                          ...current,
                          ranking: algorithms[event.target.value],
                        }));
                      }}
                    >
                      <option value="OpenSkill">OpenSkill</option>
                      <option value="TrueSkill">TrueSkill</option>
                      <option value="Elo">Elo</option>
                      <option value="BradleyTerry">Bradley–Terry</option>
                    </Form.Select>
                  </Form.Group>
                </Col>
                <Col md={6}>
                  <Form.Group controlId="uncertainty-coefficient">
                    <Form.Label>Leaderboard uncertainty coefficient</Form.Label>
                    <Form.Control
                      type="number"
                      step="any"
                      placeholder="Default: 3"
                      value={draft.leaderboards.uncertainty_coefficient ?? ""}
                      onChange={(event) =>
                        setDraft((current) => ({
                          ...current,
                          leaderboards: {
                            uncertainty_coefficient: optionalNumber(
                              event.target.value,
                            ),
                          },
                        }))
                      }
                    />
                  </Form.Group>
                </Col>
                {draft.ranking.algorithm === "OpenSkill" && (
                  <>
                    <OptionalNumberField
                      id="openskill-beta"
                      label="OpenSkill beta"
                      value={draft.ranking.beta}
                      onChange={(beta) =>
                        setDraft((current) => ({
                          ...current,
                          ranking: {
                            ...current.ranking,
                            beta,
                          } as RankingConfiguration,
                        }))
                      }
                    />
                    <OptionalNumberField
                      id="uncertainty-tolerance"
                      label="Uncertainty tolerance"
                      value={draft.ranking.uncertainty_tolerance}
                      onChange={(uncertainty_tolerance) =>
                        setDraft((current) => ({
                          ...current,
                          ranking: {
                            ...current.ranking,
                            uncertainty_tolerance,
                          } as RankingConfiguration,
                        }))
                      }
                    />
                  </>
                )}
                {draft.ranking.algorithm === "TrueSkill" && (
                  <>
                    <OptionalNumberField
                      id="draw-probability"
                      label="Draw probability"
                      value={draft.ranking.draw_probability}
                      onChange={(draw_probability) =>
                        setDraft((current) => ({
                          ...current,
                          ranking: {
                            ...current.ranking,
                            draw_probability,
                          } as RankingConfiguration,
                        }))
                      }
                    />
                    <OptionalNumberField
                      id="trueskill-beta"
                      label="TrueSkill beta"
                      value={draft.ranking.beta}
                      onChange={(beta) =>
                        setDraft((current) => ({
                          ...current,
                          ranking: {
                            ...current.ranking,
                            beta,
                          } as RankingConfiguration,
                        }))
                      }
                    />
                    <OptionalNumberField
                      id="default-dynamics"
                      label="Default dynamics"
                      value={draft.ranking.default_dynamics}
                      onChange={(default_dynamics) =>
                        setDraft((current) => ({
                          ...current,
                          ranking: {
                            ...current.ranking,
                            default_dynamics,
                          } as RankingConfiguration,
                        }))
                      }
                    />
                  </>
                )}
                {draft.ranking.algorithm === "Elo" && (
                  <OptionalNumberField
                    id="elo-k"
                    label="Elo K-factor"
                    value={draft.ranking.k}
                    onChange={(k) =>
                      setDraft((current) => ({
                        ...current,
                        ranking: {
                          ...current.ranking,
                          k,
                        } as RankingConfiguration,
                      }))
                    }
                  />
                )}
                {draft.ranking.algorithm === "BradleyTerry" && (
                  <OptionalNumberField
                    id="maximum-iterations"
                    label="Maximum iterations"
                    value={draft.ranking.max_iter}
                    onChange={(max_iter) =>
                      setDraft((current) => ({
                        ...current,
                        ranking: {
                          ...current.ranking,
                          max_iter,
                        } as RankingConfiguration,
                      }))
                    }
                  />
                )}
              </Row>
            </Card.Body>
          </Card>

          <Card>
            <Card.Body>
              <Card.Title>Embedded worker</Card.Title>
              <Row className="g-3">
                <Col md={3}>
                  <Form.Group controlId="worker-threads">
                    <Form.Label>Worker threads</Form.Label>
                    <Form.Control
                      type="number"
                      min={1}
                      max={255}
                      required
                      value={worker.threads}
                      onChange={(event) =>
                        setWorker({ threads: Number(event.target.value) })
                      }
                    />
                  </Form.Group>
                </Col>
                <Col md={9}>
                  <Form.Group controlId="build-command">
                    <Form.Label>Bot build command</Form.Label>
                    <Form.Control
                      required
                      value={worker.cmd_build}
                      onChange={(event) =>
                        setWorker({ cmd_build: event.target.value })
                      }
                    />
                  </Form.Group>
                </Col>
                <Col xs={12}>
                  <Form.Group controlId="run-command">
                    <Form.Label>Bot run command</Form.Label>
                    <Form.Control
                      required
                      value={worker.cmd_run}
                      onChange={(event) =>
                        setWorker({ cmd_run: event.target.value })
                      }
                    />
                  </Form.Group>
                </Col>
              </Row>
            </Card.Body>
          </Card>

          <Card>
            <Card.Body>
              <Card.Title>Referee</Card.Title>
              <Row className="g-3">
                <Col md={4}>
                  <Form.Group controlId="referee-type">
                    <Form.Label>Adapter</Form.Label>
                    <Form.Select
                      value={worker.referee.type}
                      onChange={(event) => {
                        const referee: RefereeConfiguration =
                          event.target.value === "managed_codingame"
                            ? {
                                type: "managed_codingame",
                                repository_url:
                                  "https://github.com/CodinGame/SpringChallenge2023.git",
                                branch: null,
                                java: null,
                                maven: null,
                              }
                            : {
                                type: "command",
                                play_match:
                                  "my-referee {SEED} {REPLAY_PATH} {PLAYERS}",
                                watch_replay:
                                  "my-renderer {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}",
                              };
                        setWorker({ referee });
                      }}
                    >
                      <option value="command">Custom commands</option>
                      <option value="managed_codingame">
                        Managed CodinGame repository
                      </option>
                    </Form.Select>
                  </Form.Group>
                </Col>
                {worker.referee.type === "command" ? (
                  <>
                    <Col xs={12}>
                      <Form.Group controlId="play-match-command">
                        <Form.Label>Play-match command</Form.Label>
                        <Form.Control
                          required
                          value={worker.referee.play_match}
                          onChange={(event) =>
                            updateCommandReferee({
                              play_match: event.target.value,
                            })
                          }
                        />
                      </Form.Group>
                    </Col>
                    <Col xs={12}>
                      <Form.Group controlId="watch-replay-command">
                        <Form.Label>Watch-replay command</Form.Label>
                        <Form.Control
                          required
                          value={worker.referee.watch_replay}
                          onChange={(event) =>
                            updateCommandReferee({
                              watch_replay: event.target.value,
                            })
                          }
                        />
                      </Form.Group>
                    </Col>
                  </>
                ) : (
                  <>
                    <Col md={8}>
                      <Form.Group controlId="referee-repository-url">
                        <Form.Label>Repository URL</Form.Label>
                        <Form.Control
                          required
                          value={worker.referee.repository_url}
                          onChange={(event) =>
                            updateManagedReferee({
                              repository_url: event.target.value,
                            })
                          }
                        />
                      </Form.Group>
                    </Col>
                    <OptionalTextField
                      id="referee-branch"
                      label="Branch"
                      value={worker.referee.branch}
                      onChange={(branch) => updateManagedReferee({ branch })}
                    />
                    <OptionalTextField
                      id="java-command"
                      label="Java executable"
                      value={worker.referee.java}
                      placeholder="java"
                      onChange={(java) => updateManagedReferee({ java })}
                    />
                    <OptionalTextField
                      id="maven-command"
                      label="Maven executable"
                      value={worker.referee.maven}
                      placeholder="mvn"
                      onChange={(maven) => updateManagedReferee({ maven })}
                    />
                  </>
                )}
              </Row>
            </Card.Body>
          </Card>

          {worker.referee.type === "managed_codingame" && (
            <Card>
              <Card.Body>
                <Card.Title>Managed referee lifecycle</Card.Title>
                {refereeStatus.isPending && (
                  <Spinner
                    animation="border"
                    size="sm"
                    aria-label="Loading referee status"
                  />
                )}
                {refereeStatus.error && (
                  <Alert variant="danger">{refereeStatus.error.message}</Alert>
                )}
                {refereeStatus.data && (
                  <Stack gap={2}>
                    <div>
                      Checkout: <code>{refereeStatus.data.checkout_path}</code>
                      <br />
                      Artifact: <code>{refereeStatus.data.artifact_path}</code>
                    </div>
                    <div>
                      {refereeStatus.data.installed
                        ? `Installed branch ${refereeStatus.data.branch} (${refereeStatus.data.update_status.replace(/_/g, " ")})`
                        : "No managed referee is installed."}
                    </div>
                    {(refereeStatus.data.staged ||
                      refereeStatus.data.unstaged ||
                      refereeStatus.data.untracked) && (
                      <Alert variant="warning" className="mb-0">
                        Local checkout changes:
                        {refereeStatus.data.staged && " staged"}
                        {refereeStatus.data.unstaged && " unstaged"}
                        {refereeStatus.data.untracked && " untracked"}
                      </Alert>
                    )}
                    {refereeStatus.data.operation.phase && (
                      <Alert variant="info" className="mb-0">
                        {refereeStatus.data.operation.action}:{" "}
                        {refereeStatus.data.operation.phase}
                      </Alert>
                    )}
                    {refereeStatus.data.operation.diagnostic && (
                      <Alert variant="secondary" className="mb-0">
                        {refereeStatus.data.operation.diagnostic}
                      </Alert>
                    )}
                    {refereeStatus.data.last_successful_check && (
                      <small className="text-body-secondary">
                        Last successful check:{" "}
                        {new Date(
                          refereeStatus.data.last_successful_check,
                        ).toLocaleString()}
                      </small>
                    )}
                    <div className="d-flex flex-wrap gap-2">
                      {(!refereeStatus.data.installed ||
                        refereeStatus.data.installed_repository_url !==
                          worker.referee.repository_url ||
                        (worker.referee.branch !== null &&
                          refereeStatus.data.branch !==
                            worker.referee.branch)) && (
                        <Button
                          type="button"
                          disabled={
                            refereeAction.isPending ||
                            refereeStatus.data.operation.action !== null
                          }
                          onClick={() => refereeAction.mutate("install")}
                        >
                          {refereeStatus.data.installed
                            ? "Replace referee"
                            : "Install referee"}
                        </Button>
                      )}
                      <Button
                        type="button"
                        variant="outline-primary"
                        disabled={
                          !refereeStatus.data.installed ||
                          refereeAction.isPending ||
                          refereeStatus.data.operation.action !== null
                        }
                        onClick={() => refereeAction.mutate("check")}
                      >
                        Check for updates
                      </Button>
                      <Button
                        type="button"
                        variant="outline-primary"
                        disabled={
                          !refereeStatus.data.installed ||
                          refereeAction.isPending ||
                          refereeStatus.data.operation.action !== null
                        }
                        onClick={() => refereeAction.mutate("rebuild")}
                      >
                        Rebuild referee
                      </Button>
                      {refereeStatus.data.update_status ===
                        "update_available" && (
                        <Button
                          type="button"
                          variant="outline-primary"
                          disabled={
                            refereeAction.isPending ||
                            refereeStatus.data.operation.action !== null
                          }
                          onClick={() => refereeAction.mutate("update")}
                        >
                          Update referee
                        </Button>
                      )}
                    </div>
                  </Stack>
                )}
                {refereeAction.error && (
                  <Alert variant="danger" className="mt-3 mb-0">
                    {refereeAction.error.message}
                  </Alert>
                )}
              </Card.Body>
            </Card>
          )}

          <div>
            <Button type="submit" disabled={apply.isPending}>
              {apply.isPending ? "Applying…" : "Apply configuration"}
            </Button>
          </div>
        </Stack>
      </Form>
    </Stack>
  );
}

interface OptionalNumberFieldProps {
  id: string;
  label: string;
  value: number | null;
  onChange(value: number | null): void;
}

function OptionalNumberField({
  id,
  label,
  value,
  onChange,
}: OptionalNumberFieldProps) {
  return (
    <Col md={4}>
      <Form.Group controlId={id}>
        <Form.Label>{label}</Form.Label>
        <Form.Control
          type="number"
          step="any"
          value={value ?? ""}
          onChange={(event) => onChange(optionalNumber(event.target.value))}
        />
      </Form.Group>
    </Col>
  );
}

interface OptionalTextFieldProps {
  id: string;
  label: string;
  value: string | null;
  placeholder?: string;
  onChange(value: string | null): void;
}

function OptionalTextField({
  id,
  label,
  value,
  placeholder,
  onChange,
}: OptionalTextFieldProps) {
  return (
    <Col md={4}>
      <Form.Group controlId={id}>
        <Form.Label>{label}</Form.Label>
        <Form.Control
          value={value ?? ""}
          placeholder={placeholder}
          onChange={(event) => onChange(event.target.value || null)}
        />
      </Form.Group>
    </Col>
  );
}
