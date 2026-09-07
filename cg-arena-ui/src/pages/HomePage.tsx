import BotOverview from "@/components/BotOverview";
import Leaderboard from "@/components/Leaderboard";
import { useAppStore } from "@/hooks/useAppStore";
import { useDialogs } from "@/hooks/useDialogs";
import useEnsureValidSelectedBot from "@/hooks/useEnsureValidSelectedBot";
import { getRouteApi } from "@tanstack/react-router";
import { useEffect } from "react";
import {
  Badge,
  Button,
  Card,
  Container,
  ProgressBar,
  Stack,
} from "react-bootstrap";
import { FaPlus } from "react-icons/fa6";
import { Link } from "@tanstack/react-router";

const routeApi = getRouteApi("/");

function stageProgressPercent(
  target: number,
  matches: number,
  coverage: string,
  encounters: Record<string, number>,
): number {
  const completed =
    coverage === "per_benchmark"
      ? Object.values(encounters).reduce(
          (sum, count) => sum + Math.min(count, target),
          0,
        )
      : Math.min(matches, target);
  const required =
    coverage === "per_benchmark"
      ? target * Math.max(Object.keys(encounters).length, 1)
      : target;
  return Math.min(100, Math.round((completed / required) * 100));
}

export default function HomePage() {
  useEnsureValidSelectedBot();
  const {
    viewCodeDialog,
    confirmDialog,
    renameBotDialog,
    createLeaderboardDialog,
  } = useDialogs();

  const { selectedBotId } = routeApi.useSearch();

  const bots = useAppStore((state) => state.bots);
  const leaderboards = useAppStore((state) => state.leaderboards);
  const rejectCandidate = useAppStore((state) => state.rejectCandidate);
  const promoteCandidate = useAppStore((state) => state.promoteCandidate);
  const archiveBenchmark = useAppStore((state) => state.archiveBenchmark);
  const renameBot = useAppStore((state) => state.renameBot);
  const patchLeaderboard = useAppStore((state) => state.patchLeaderboard);
  const deleteLeaderboard = useAppStore((state) => state.deleteLeaderboard);
  const createLeaderboard = useAppStore((state) => state.createLeaderboard);

  const selectedBot = bots.find((b) => b.id == selectedBotId);
  const candidates = bots.filter((bot) => bot.role === "candidate");
  const activeBots = bots.filter((bot) => bot.role !== "archived_benchmark");

  useEffect(() => {
    const fetch = useAppStore.getState().fetchStatus;

    fetch(); // initial load

    const interval = setInterval(() => {
      useAppStore.getState().refreshLeaderboard();
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="d-flex flex-column gap-4">
      <Card>
        <Card.Header className="d-flex justify-content-between align-items-center">
          <span>Candidate evaluations</span>
          <Badge
            bg={
              candidates.every((bot) => bot.evaluation?.complete)
                ? "success"
                : "primary"
            }
          >
            {candidates.length} active
          </Badge>
        </Card.Header>
        <Card.Body className={candidates.length === 0 ? "py-3" : "py-2"}>
          {candidates.length === 0 ? (
            <p className="text-body-secondary mb-0">
              Submit a Candidate to start an evaluation.
            </p>
          ) : (
            <Stack gap={2}>
              {candidates.map((candidate, index) => (
                <div
                  key={candidate.id}
                  className={
                    index < candidates.length - 1 ? "border-bottom pb-2" : ""
                  }
                >
                  <div className="d-flex flex-wrap justify-content-between align-items-center gap-2 mb-2">
                    <div className="d-flex flex-wrap align-items-center gap-2">
                      <strong>{candidate.name}</strong>
                      <Badge
                        bg={
                          candidate.evaluation?.complete ? "success" : "primary"
                        }
                      >
                        {candidate.evaluation?.complete
                          ? "Complete"
                          : "Evaluating"}
                      </Badge>
                      <small className="text-body-secondary">
                        Plan {candidate.evaluation_plan_revision_id}
                      </small>
                    </div>
                    <div className="d-flex flex-wrap gap-1">
                      <Link
                        to="/matches"
                        search={{ withBots: [candidate.id] }}
                        className="btn btn-outline-secondary btn-sm"
                      >
                        Matches
                      </Link>
                      <Button
                        size="sm"
                        variant="success"
                        onClick={() => promoteCandidate(candidate.id)}
                      >
                        Promote
                      </Button>
                      <Button
                        size="sm"
                        variant="outline-danger"
                        onClick={() =>
                          confirmDialog.show({
                            prompt: `Reject '${candidate.name}' and permanently delete its evaluation evidence?`,
                            action: () => rejectCandidate(candidate.id),
                          })
                        }
                      >
                        Reject
                      </Button>
                    </div>
                  </div>
                  <Stack gap={2}>
                    {candidate.evaluation?.stages.map((stage) => {
                      const percent = stageProgressPercent(
                        stage.target,
                        stage.matches,
                        stage.coverage,
                        stage.benchmark_encounters,
                      );
                      const encounters = Object.entries(
                        stage.benchmark_encounters,
                      )
                        .map(([id, count]) => {
                          const benchmark = bots.find(
                            (bot) => bot.id === Number(id),
                          );
                          return `${benchmark?.name ?? `Bot ${id}`}: ${count}`;
                        })
                        .join(" · ");
                      return (
                        <div key={stage.id}>
                          <div className="d-flex justify-content-between align-items-center gap-3 small">
                            <span className="text-truncate">
                              <span className="fw-medium">{stage.name}</span>
                              {encounters && (
                                <span className="text-body-secondary">
                                  {" "}
                                  · {encounters}
                                </span>
                              )}
                            </span>
                            <span
                              className={`text-nowrap ${
                                stage.candidate_errors > 0 ? "text-danger" : ""
                              }`}
                            >
                              {percent}% · {stage.matches} matches ·{" "}
                              {stage.candidate_errors} errors
                            </span>
                          </div>
                          <ProgressBar
                            now={percent}
                            style={{ height: "0.35rem" }}
                            aria-label={`${stage.name} ${percent}% complete`}
                          />
                        </div>
                      );
                    })}
                  </Stack>
                </div>
              ))}
            </Stack>
          )}
        </Card.Body>
      </Card>

      <Card>
        <Card.Header>Selected bot</Card.Header>
        <Card.Body>
          {selectedBot && (
            <BotOverview
              bot={selectedBot}
              showCodeDialog={viewCodeDialog.show}
              lifecycleAction={
                selectedBot.role === "candidate"
                  ? {
                      label: "Reject",
                      variant: "outline-danger",
                      onClick: () =>
                        confirmDialog.show({
                          prompt: `Reject '${selectedBot.name}' and permanently delete its evaluation evidence?`,
                          action: () => rejectCandidate(selectedBot.id),
                        }),
                    }
                  : selectedBot.role === "benchmark"
                    ? {
                        label: "Archive",
                        variant: "outline-warning",
                        onClick: () =>
                          confirmDialog.show({
                            prompt: `Archive benchmark '${selectedBot.name}'? Its history and rating evidence will be retained.`,
                            action: () => archiveBenchmark(selectedBot.id),
                          }),
                      }
                    : undefined
              }
              renameBot={() =>
                renameBotDialog.show({
                  botId: selectedBot.id,
                  currentName: selectedBot.name,
                  onSubmit: renameBot,
                })
              }
            />
          )}
        </Card.Body>
      </Card>

      {leaderboards.map((lb) => (
        <Leaderboard
          key={lb.id}
          lb={lb}
          bots={activeBots}
          selectedBotId={selectedBotId}
          patchLeaderboard={patchLeaderboard}
          deleteLeaderboard={deleteLeaderboard}
        />
      ))}

      <Container className="d-flex justify-content-center">
        <Button
          className="d-flex items-center"
          variant="outline-secondary"
          onClick={() =>
            createLeaderboardDialog.show({ onCreate: createLeaderboard })
          }
        >
          <FaPlus className="bi me-2" size={16} />
          <span>New leaderboard</span>
        </Button>
      </Container>
    </div>
  );
}
