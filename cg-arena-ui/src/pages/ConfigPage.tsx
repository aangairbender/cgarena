import NumbericInput from "@/components/form/NumericInput";
import { Config, configSchema } from "@/models";
import { useForm } from "@tanstack/react-form";
import { Button, Card, Form, FormGroup } from "react-bootstrap";

const defaultConfig: Config = {
  game: {
    min_players: 2,
    max_players: 2,
    symmetric: true,
  },
  matchmaking: {
    enabled_on_start: true,
    algorithm: "v2",
    min_matches_per_pair: 200,
  },
  ranking: {
    algorithm: "BradleyTerry",
  },
  server: {
    port: 0,
    expose: false,
  },
  log: {
    level: "INFO",
    file: "cgarena.log",
  },
  leaderboards: {
    uncertainty_coefficient: undefined,
  },
  workers: [
    {
      type: "embedded",
      threads: 1,
      cmd_play_match: "",
      cmd_build: "",
      cmd_run: "",
    },
  ],
};

export default function ConfigPage() {
  const form = useForm({
    defaultValues: defaultConfig,
    validators: {
      onChange: configSchema,
    },
    onSubmit: async ({ value }) => {
      console.log(value);
    },
  });

  return (
    <Card>
      <Card.Header>Edit config</Card.Header>
      <Card.Body>
        <Form noValidate>
          <h2>Game</h2>

          <div className="d-flex flex-column gap-3">
            <form.Field
              name="game.min_players"
              children={(field) => (
                <NumbericInput
                  label="Min players"
                  value={field.state.value}
                  onChange={(v) => field.handleChange(v)}
                  onBlur={field.handleBlur}
                  errors={field.state.meta.errors.map((e) => e?.message ?? "")}
                />
              )}
            />

            <form.Field
              name="game.max_players"
              children={(field) => (
                <NumbericInput
                  label="Max players"
                  value={field.state.value}
                  onChange={(v) => field.handleChange(v)}
                  onBlur={field.handleBlur}
                  errors={field.state.meta.errors.map((e) => e?.message ?? "")}
                />
              )}
            />

            <form.Field
              name="game.symmetric"
              children={(field) => (
                <FormGroup>
                  <Form.Check
                    type="checkbox"
                    label="Symmetric"
                    checked={field.state.value}
                    onChange={(e) => field.handleChange(e.target.checked)}
                    onBlur={field.handleBlur}
                  />
                  <Form.Text className="text-muted">
                    Whether the map is symmetric for all the players.
                  </Form.Text>
                </FormGroup>
              )}
            />
          </div>

          <h2 className="mt-4">Matchmaking</h2>

          <div className="d-flex flex-column gap-3">
            <form.Field
              name="matchmaking.enabled_on_start"
              children={(field) => (
                <FormGroup>
                  <Form.Check
                    type="checkbox"
                    label="Enabled on start"
                    checked={field.state.value}
                    onChange={(e) => field.handleChange(e.target.checked)}
                    onBlur={field.handleBlur}
                  />
                </FormGroup>
              )}
            />

            <form.Field
              name="matchmaking.algorithm"
              children={(field) => (
                <FormGroup>
                  <Form.Label>Algorithm</Form.Label>
                  <Form.Select
                    value={field.state.value}
                    onChange={(e) =>
                      field.setValue(e.target.value as "v1" | "v2")
                    }
                  >
                    <option value="v1">Version 1</option>
                    <option value="v2">Version 2</option>
                  </Form.Select>
                </FormGroup>
              )}
            />

            <form.Subscribe
              selector={(state) => state.values.matchmaking.algorithm}
            >
              {(algorithm) => {
                if (algorithm === "v1") {
                  return (
                    <>
                      <form.Field
                        name="matchmaking.min_matches"
                        children={(field) => (
                          <NumbericInput
                            key={field.name}
                            label="Min matches"
                            value={field.state.value}
                            onChange={(v) => field.handleChange(v)}
                            onBlur={field.handleBlur}
                            errors={field.state.meta.errors.map(
                              (e) => e?.message ?? "",
                            )}
                          />
                        )}
                      />
                      <form.Field
                        name="matchmaking.min_matches_preference"
                        children={(field) => (
                          <NumbericInput
                            key={field.name}
                            label="Min matches preference"
                            value={field.state.value}
                            onChange={(v) => field.handleChange(v)}
                            onBlur={field.handleBlur}
                            errors={field.state.meta.errors.map(
                              (e) => e?.message ?? "",
                            )}
                          />
                        )}
                      />
                    </>
                  );
                }
                if (algorithm === "v2") {
                  return (
                    <>
                      <form.Field
                        name="matchmaking.min_matches_per_pair"
                        children={(field) => (
                          <NumbericInput
                            key={field.name}
                            label="Min matches per pair"
                            value={field.state.value}
                            onChange={(v) => field.handleChange(v)}
                            onBlur={field.handleBlur}
                            errors={field.state.meta.errors.map(
                              (e) => e?.message ?? "",
                            )}
                          />
                        )}
                      />
                      <form.Field
                        name="matchmaking.min_matches_against_best"
                        children={(field) => (
                          <NumbericInput
                            key={field.name}
                            label="Min matches against best"
                            value={field.state.value}
                            onChange={(v) => field.handleChange(v)}
                            onBlur={field.handleBlur}
                            errors={field.state.meta.errors.map(
                              (e) => e?.message ?? "",
                            )}
                          />
                        )}
                      />
                      <form.Field
                        name="matchmaking.max_matches"
                        children={(field) => (
                          <NumbericInput
                            key={field.name}
                            label="Max matches"
                            value={field.state.value}
                            onChange={(v) => field.handleChange(v)}
                            onBlur={field.handleBlur}
                            errors={field.state.meta.errors.map(
                              (e) => e?.message ?? "",
                            )}
                          />
                        )}
                      />
                    </>
                  );
                }
                return null;
              }}
            </form.Subscribe>
          </div>

          <h2 className="mt-4">Ranking</h2>

          <div className="d-flex flex-column gap-3">
            <form.Field
              name="ranking.algorithm"
              children={(field) => (
                <FormGroup>
                  <Form.Label>Algorithm</Form.Label>
                  <Form.Select
                    value={field.state.value}
                    onChange={(e) => field.setValue(e.target.value)}
                  >
                    <option value="OpenSkill">OpenSkill</option>
                    <option value="TrueSkill">TrueSkill</option>
                    <option value="Elo">Elo</option>
                    <option value="BradleyTerry">BradleyTerry</option>
                  </Form.Select>
                </FormGroup>
              )}
            />
          </div>

          <h2 className="mt-4">Server</h2>

          <div className="d-flex flex-column gap-3">
            <form.Field
              name="server.port"
              children={(field) => (
                <NumbericInput
                  key={field.name}
                  label="Port"
                  value={field.state.value}
                  onChange={(v) => field.handleChange(v)}
                  onBlur={field.handleBlur}
                  errors={field.state.meta.errors.map((e) => e?.message ?? "")}
                />
              )}
            />

            <form.Field
              name="server.expose"
              children={(field) => (
                <FormGroup>
                  <Form.Check
                    type="checkbox"
                    label="Expose"
                    checked={field.state.value}
                    onChange={(e) => field.handleChange(e.target.checked)}
                    onBlur={field.handleBlur}
                  />
                </FormGroup>
              )}
            />
          </div>

          <h2 className="mt-4">Logs</h2>

          <div className="d-flex flex-column gap-3">
            <form.Field
              name="log.level"
              children={(field) => (
                <FormGroup>
                  <Form.Label>Level</Form.Label>
                  <Form.Select
                    value={field.state.value}
                    onChange={(e) => field.setValue(e.target.value)}
                  >
                    <option value="INFO">INFO</option>
                    <option value="DEBUG">DEBUG</option>
                  </Form.Select>
                </FormGroup>
              )}
            />

            <form.Field
              name="log.file"
              children={(field) => (
                <FormGroup>
                  <Form.Label>File</Form.Label>
                  <Form.Control
                    type="text"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                  />
                </FormGroup>
              )}
            />
          </div>

          <h2 className="mt-4">Leaderboards</h2>

          <div className="d-flex flex-column gap-3">
            <form.Field
              name="leaderboards.uncertainty_coefficient"
              children={(field) => (
                <NumbericInput
                  key={field.name}
                  label="Uncertainty coefficient"
                  value={field.state.value}
                  onChange={(v) => field.handleChange(v)}
                  onBlur={field.handleBlur}
                  errors={field.state.meta.errors.map((e) => e?.message ?? "")}
                />
              )}
            />
          </div>

          <h2 className="mt-4">Workers</h2>

          <div className="d-flex flex-column gap-3"></div>
        </Form>
      </Card.Body>
      <Card.Footer>
        <div className="d-flex justify-content-between">
          <Button variant="secondary" onClick={() => {}}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => {}}>
            Save
          </Button>
        </div>
      </Card.Footer>
    </Card>
  );
}
