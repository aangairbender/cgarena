// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PropsWithChildren } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ConfigurationAdapter } from "@/configuration";
import { ArenaConfiguration, ConfigurationState } from "@/models";
import ConfigPage from "./ConfigPage";

class InMemoryConfigurationAdapter implements ConfigurationAdapter {
  readonly applied: ArenaConfiguration[] = [];

  constructor(
    private state: ConfigurationState,
    private readonly applyError?: Error,
  ) {}

  async fetch(): Promise<ConfigurationState> {
    return this.state;
  }

  async apply(candidate: ArenaConfiguration): Promise<ConfigurationState> {
    this.applied.push(candidate);
    if (this.applyError) {
      throw this.applyError;
    }
    this.state = {
      active: candidate,
      runtime_available: false,
      runtime_error: null,
    };
    return this.state;
  }
}

function renderPage(adapter: ConfigurationAdapter) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      Response.json({
        selected: null,
        installed: false,
        replacement_required: false,
        checkout_path: "/arena/referee",
        artifact_path: "/arena/.cgarena/referee/referee.jar",
        installed_repository_url: null,
        branch: null,
        upstream_commit: null,
        adaptation_commit: null,
        committed_ahead: null,
        committed_behind: null,
        staged: false,
        unstaged: false,
        untracked: false,
        update_status: "unavailable",
        last_successful_check: null,
        observed_remote_commit: null,
        operation: { action: null, phase: null, diagnostic: null },
      }),
    ),
  );
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  const wrapper = ({ children }: PropsWithChildren) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return render(<ConfigPage adapter={adapter} />, { wrapper });
}

afterEach(() => {
  document.body.innerHTML = "";
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("configuration page", () => {
  it("edits and atomically applies one complete arena configuration", async () => {
    const adapter = new InMemoryConfigurationAdapter({
      active: null,
      runtime_available: false,
      runtime_error: null,
    });
    vi.spyOn(Math, "random").mockReturnValueOnce(0.25).mockReturnValueOnce(0.5);
    renderPage(adapter);

    expect(
      await screen.findByRole("heading", { name: "Set up your arena" }),
    ).toBeTruthy();
    for (const section of [
      "Game",
      "Candidate evaluation plan",
      "Ranking and leaderboard",
      "Embedded worker",
      "Referee",
    ]) {
      expect(screen.getByText(section)).toBeTruthy();
    }
    expect(
      screen
        .getByText("Referee")
        .compareDocumentPosition(screen.getByText("Game")) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.getByLabelText("Enable matchmaking on startup")).toBeTruthy();
    expect(screen.getByText("Unsaved changes")).toBeTruthy();
    const applyButton = screen.getByRole("button", {
      name: "Apply configuration",
    }) as HTMLButtonElement;
    expect(applyButton.disabled).toBe(false);
    const seedSequenceKey = screen.getByLabelText(
      "Seed sequence key",
    ) as HTMLInputElement;
    expect(seedSequenceKey.value).toBe("1073741824");
    fireEvent.click(screen.getByRole("button", { name: "Randomize" }));
    expect(seedSequenceKey.value).toBe("2147483648");
    expect(
      Array.from(
        (screen.getByLabelText("Coverage") as HTMLSelectElement).options,
        (option) => option.textContent,
      ),
    ).toEqual(["Matches per pair", "Total matches"]);
    const coverage = screen.getByLabelText("Coverage");
    expect(screen.getByLabelText("Matches per pair")).toBeTruthy();
    fireEvent.change(coverage, { target: { value: "total" } });
    expect(screen.getByLabelText("Total matches")).toBeTruthy();
    fireEvent.change(coverage, { target: { value: "per_benchmark" } });
    const advancedSettings = screen.getByText("Advanced settings");
    const advancedSettingsDisclosure =
      advancedSettings.parentElement as HTMLDetailsElement;
    expect(advancedSettingsDisclosure.open).toBe(false);
    fireEvent.click(advancedSettings);
    expect(advancedSettingsDisclosure.open).toBe(true);
    const maximumIterationsHelp = screen.getByRole("button", {
      name: "More information about Maximum iterations",
    });
    fireEvent.focus(maximumIterationsHelp);
    expect((await screen.findByRole("tooltip")).textContent).toContain(
      "maximum optimization steps",
    );
    fireEvent.blur(maximumIterationsHelp);
    expect(
      (screen.getByLabelText("Repository URL") as HTMLInputElement).value,
    ).toBe("");
    expect(
      await screen.findByRole("button", { name: "Install referee" }),
    ).toBeTruthy();
    expect(
      (
        (await screen.findByRole("button", {
          name: "Check for updates",
        })) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    fireEvent.change(screen.getByLabelText("Adapter"), {
      target: { value: "command" },
    });
    fireEvent.change(screen.getByLabelText("Adapter"), {
      target: { value: "managed_codingame" },
    });
    expect(
      (screen.getByLabelText("Repository URL") as HTMLInputElement).value,
    ).toBe("");
    fireEvent.change(screen.getByLabelText("Adapter"), {
      target: { value: "command" },
    });

    fireEvent.change(screen.getByLabelText("Minimum players"), {
      target: { value: "3" },
    });
    fireEvent.change(screen.getByLabelText("Maximum players"), {
      target: { value: "4" },
    });
    fireEvent.change(screen.getByLabelText("Worker threads"), {
      target: { value: "2" },
    });
    fireEvent.change(screen.getByLabelText("Ranking algorithm"), {
      target: { value: "Elo" },
    });
    fireEvent.change(screen.getByLabelText("Elo K-factor"), {
      target: { value: "24" },
    });
    fireEvent.change(screen.getByLabelText("Play-match command"), {
      target: {
        value: "my-referee {SEED} {REPLAY_PATH} {PLAYERS}",
      },
    });
    fireEvent.change(screen.getByLabelText("Watch-replay command"), {
      target: {
        value: "my-renderer {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}",
      },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Apply configuration" }),
    );

    await waitFor(() => expect(adapter.applied).toHaveLength(1));
    expect(adapter.applied[0]).toMatchObject({
      game: { min_players: 3, max_players: 4, symmetric: true },
      evaluation: { enabled_on_start: true },
      ranking: { algorithm: "Elo", k: 24 },
      workers: [
        {
          type: "embedded",
          threads: 2,
          referee: { type: "command" },
        },
      ],
    });
    expect(await screen.findByText("Configuration up to date")).toBeTruthy();
    expect(applyButton.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Minimum players"), {
      target: { value: "2" },
    });
    expect(screen.getByText("Unsaved changes")).toBeTruthy();
    expect(applyButton.disabled).toBe(false);
  });

  it("converts legacy weighted coverage to total matches", async () => {
    const adapter = new InMemoryConfigurationAdapter({
      active: {
        game: { min_players: 2, max_players: 2, symmetric: true },
        evaluation: {
          enabled_on_start: true,
          seed_sequence_key: 42,
          stages: [
            {
              name: "Legacy coverage",
              seed_source: { type: "generated" },
              coverage: {
                type: "weighted_total",
                target: 17,
                weights: { "1": 2 },
              },
              min_players: null,
              max_players: null,
            },
          ],
        },
        ranking: { algorithm: "BradleyTerry", max_iter: null },
        leaderboards: { uncertainty_coefficient: null },
        workers: [
          {
            type: "embedded",
            threads: 1,
            cmd_build: "build {DIR}",
            cmd_run: "run {DIR}",
            referee: {
              type: "command",
              play_match: "play {SEED} {REPLAY_PATH} {PLAYERS}",
              watch_replay:
                "watch {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}",
            },
          },
        ],
      },
      runtime_available: true,
      runtime_error: null,
    });
    renderPage(adapter);

    expect(
      ((await screen.findByLabelText("Coverage")) as HTMLSelectElement).value,
    ).toBe("total");
    fireEvent.click(
      screen.getByRole("button", { name: "Apply configuration" }),
    );

    await waitFor(() => expect(adapter.applied).toHaveLength(1));
    expect(adapter.applied[0].evaluation.stages[0].coverage).toEqual({
      type: "total",
      target: 17,
    });
  });

  it("shows server validation errors without replacing the draft", async () => {
    const adapter = new InMemoryConfigurationAdapter(
      { active: null, runtime_available: false, runtime_error: null },
      new Error(
        "Validation failed: command referee play_match must contain {SEED}",
      ),
    );
    renderPage(adapter);
    fireEvent.change(await screen.findByLabelText("Adapter"), {
      target: { value: "command" },
    });

    const playMatch = await screen.findByLabelText("Play-match command");
    fireEvent.change(playMatch, { target: { value: "broken command" } });
    fireEvent.change(screen.getByLabelText("Watch-replay command"), {
      target: {
        value: "my-renderer {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}",
      },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Apply configuration" }),
    );

    expect(
      await screen.findByText(
        "Validation failed: command referee play_match must contain {SEED}",
      ),
    ).toBeTruthy();
    expect(adapter.applied).toHaveLength(1);
    expect(
      (screen.getByLabelText("Play-match command") as HTMLInputElement).value,
    ).toBe("broken command");
    expect(screen.getByText("Configuration was not saved")).toBeTruthy();
    expect(
      (
        screen.getByRole("button", {
          name: "Apply configuration",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false);
  });
});
