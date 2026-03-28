import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import { useAppStore } from "@hooks/useAppStore";
import { useDialogs } from "@hooks/useDialogs";
import { useEffect } from "react";
import { Container, Card, Button } from "react-bootstrap";
import { FaPlus } from "react-icons/fa6";

export default function HomePage() {
  const {
    viewCodeDialog,
    confirmDialog,
    renameBotDialog,
    createLeaderboardDialog,
  } = useDialogs();

  const selectedBotId = useAppStore((state) => state.selectedBotId);
  const bots = useAppStore((state) => state.bots);
  const leaderboards = useAppStore((state) => state.leaderboards);
  const deleteBot = useAppStore((state) => state.deleteBot);
  const renameBot = useAppStore((state) => state.renameBot);
  const selectBot = useAppStore((state) => state.selectBot);
  const patchLeaderboard = useAppStore((state) => state.patchLeaderboard);
  const deleteLeaderboard = useAppStore((state) => state.deleteLeaderboard);
  const createLeaderboard = useAppStore((state) => state.createLeaderboard);

  const selectedBot = bots.find((b) => b.id == selectedBotId);

  useEffect(() => {
    const fetch = useAppStore.getState().fetchStatus;

    fetch(); // initial load

    const interval = setInterval(() => {
      useAppStore.getState().refreshLeaderboard();
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return (
    <Container>
      <Card className="mt-4">
        <Card.Header>Selected bot</Card.Header>
        <Card.Body>
          {selectedBot && (
            <BotOverview
              bot={selectedBot}
              showCodeDialog={viewCodeDialog.show}
              deleteBot={() =>
                confirmDialog.show({
                  prompt: `Are you sure you want to delete bot '${selectedBot.name}'?`,
                  action: () => {
                    deleteBot(selectedBot.id);
                  },
                })
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
          lb={lb}
          bots={bots}
          selectedBotId={selectedBotId}
          selectBot={selectBot}
          patchLeaderboard={patchLeaderboard}
          deleteLeaderboard={deleteLeaderboard}
        />
      ))}

      <Container className="my-4 d-flex justify-content-center">
        <Button
          className="mx-1"
          variant="outline-secondary"
          onClick={() =>
            createLeaderboardDialog.show({ onCreate: createLeaderboard })
          }
        >
          <FaPlus className="bi me-2" size={16} />
          New leaderboard
        </Button>
      </Container>
    </Container>
  );
}
