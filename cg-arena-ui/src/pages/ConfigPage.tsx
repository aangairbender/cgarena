import * as api from "@/api";
import { ConfigurationAdapter, configurationQueryKey } from "@/configuration";
import {
  ArenaConfiguration,
  ConfigurationState,
  EmbeddedWorkerConfiguration,
  MatchmakingConfiguration,
  RankingConfiguration,
  RefereeConfiguration,
} from "@/models";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FormEvent, useState } from "react";
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

function defaultConfiguration(): ArenaConfiguration {
  return {
    game: {
      min_players: 2,
      max_players: 2,
      symmetric: true,
      referee_git_url: null,
    },
    matchmaking: {
      algorithm: "v2",
      min_matches_against_best: null,
      min_matches_per_pair: 100,
      max_matches: 1000,
      enabled_on_start: true,
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
          type: "command",
          play_match: "",
          watch_replay: "",
        },
      },
    ],
  };
}

function optionalNumber(value: string): number | null {
  return value === "" ? null : Number(value);
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

  const setWorker = (next: Partial<EmbeddedWorkerConfiguration>) => {
    setDraft((current) => ({
      ...current,
      workers: [{ ...current.workers[0], ...next }],
    }));
    setSaved(false);
  };

  const updateCommandReferee = (
    next: Partial<{ play_match: string; watch_replay: string }>,
  ) => {
    if (worker.referee.type === "command") {
      setWorker({ referee: { ...worker.referee, ...next } });
    }
  };

  const updateJarReferee = (
    next: Partial<{ path: string; java: string | null; league: number | null }>,
  ) => {
    if (worker.referee.type === "codingame_jar") {
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
              <Card.Title>Matchmaking</Card.Title>
              <Row className="g-3">
                <Col md={4}>
                  <Form.Group controlId="matchmaking-algorithm">
                    <Form.Label>Algorithm</Form.Label>
                    <Form.Select
                      value={draft.matchmaking.algorithm}
                      onChange={(event) => {
                        const next: MatchmakingConfiguration =
                          event.target.value === "v1"
                            ? {
                                algorithm: "v1",
                                min_matches: 100,
                                min_matches_preference: 1,
                                enabled_on_start:
                                  draft.matchmaking.enabled_on_start,
                              }
                            : {
                                algorithm: "v2",
                                min_matches_against_best: null,
                                min_matches_per_pair: 100,
                                max_matches: 1000,
                                enabled_on_start:
                                  draft.matchmaking.enabled_on_start,
                              };
                        setDraft((current) => ({
                          ...current,
                          matchmaking: next,
                        }));
                      }}
                    >
                      <option value="v1">V1</option>
                      <option value="v2">V2</option>
                    </Form.Select>
                  </Form.Group>
                </Col>
                {draft.matchmaking.algorithm === "v1" ? (
                  <>
                    <Col md={4}>
                      <Form.Group controlId="minimum-matches">
                        <Form.Label>Minimum matches</Form.Label>
                        <Form.Control
                          type="number"
                          min={0}
                          required
                          value={draft.matchmaking.min_matches}
                          onChange={(event) =>
                            setDraft((current) => ({
                              ...current,
                              matchmaking: {
                                ...current.matchmaking,
                                min_matches: Number(event.target.value),
                              } as MatchmakingConfiguration,
                            }))
                          }
                        />
                      </Form.Group>
                    </Col>
                    <Col md={4}>
                      <Form.Group controlId="minimum-matches-preference">
                        <Form.Label>Minimum matches preference</Form.Label>
                        <Form.Control
                          type="number"
                          min={0}
                          max={1}
                          step="any"
                          required
                          value={draft.matchmaking.min_matches_preference}
                          onChange={(event) =>
                            setDraft((current) => ({
                              ...current,
                              matchmaking: {
                                ...current.matchmaking,
                                min_matches_preference: Number(
                                  event.target.value,
                                ),
                              } as MatchmakingConfiguration,
                            }))
                          }
                        />
                      </Form.Group>
                    </Col>
                  </>
                ) : (
                  <>
                    <Col md={4}>
                      <Form.Group controlId="minimum-matches-per-pair">
                        <Form.Label>Minimum matches per pair</Form.Label>
                        <Form.Control
                          type="number"
                          min={0}
                          required
                          value={draft.matchmaking.min_matches_per_pair}
                          onChange={(event) =>
                            setDraft((current) => ({
                              ...current,
                              matchmaking: {
                                ...current.matchmaking,
                                min_matches_per_pair: Number(
                                  event.target.value,
                                ),
                              } as MatchmakingConfiguration,
                            }))
                          }
                        />
                      </Form.Group>
                    </Col>
                    <Col md={4}>
                      <Form.Group controlId="maximum-matches">
                        <Form.Label>Maximum matches</Form.Label>
                        <Form.Control
                          type="number"
                          min={0}
                          value={draft.matchmaking.max_matches ?? ""}
                          onChange={(event) =>
                            setDraft((current) => ({
                              ...current,
                              matchmaking: {
                                ...current.matchmaking,
                                max_matches: optionalNumber(event.target.value),
                              } as MatchmakingConfiguration,
                            }))
                          }
                        />
                      </Form.Group>
                    </Col>
                    <Col md={4}>
                      <Form.Group controlId="matches-against-best">
                        <Form.Label>Matches against best</Form.Label>
                        <Form.Control
                          type="number"
                          min={0}
                          value={
                            draft.matchmaking.min_matches_against_best ?? ""
                          }
                          onChange={(event) =>
                            setDraft((current) => ({
                              ...current,
                              matchmaking: {
                                ...current.matchmaking,
                                min_matches_against_best: optionalNumber(
                                  event.target.value,
                                ),
                              } as MatchmakingConfiguration,
                            }))
                          }
                        />
                      </Form.Group>
                    </Col>
                  </>
                )}
                <Col xs={12}>
                  <Form.Check
                    id="start-matchmaking"
                    type="switch"
                    label="Start matchmaking automatically"
                    checked={draft.matchmaking.enabled_on_start ?? true}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        matchmaking: {
                          ...current.matchmaking,
                          enabled_on_start: event.target.checked,
                        } as MatchmakingConfiguration,
                      }))
                    }
                  />
                </Col>
              </Row>
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
                          event.target.value === "codingame_jar"
                            ? {
                                type: "codingame_jar",
                                path: "referee/target/referee.jar",
                                java: null,
                                league: null,
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
                      <option value="codingame_jar">
                        Compatible CodinGame JAR
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
                      <Form.Group controlId="referee-jar-path">
                        <Form.Label>Compatible JAR path</Form.Label>
                        <Form.Control
                          required
                          value={worker.referee.path}
                          onChange={(event) =>
                            updateJarReferee({ path: event.target.value })
                          }
                        />
                      </Form.Group>
                    </Col>
                    <OptionalTextField
                      id="java-command"
                      label="Java executable"
                      value={worker.referee.java}
                      placeholder="java"
                      onChange={(java) => updateJarReferee({ java })}
                    />
                    <OptionalNumberField
                      id="league"
                      label="League"
                      value={worker.referee.league}
                      onChange={(league) => updateJarReferee({ league })}
                    />
                  </>
                )}
              </Row>
            </Card.Body>
          </Card>

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
