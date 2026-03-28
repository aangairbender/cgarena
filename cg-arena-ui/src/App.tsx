import "./App.css";
import { Button, Card, Container } from "react-bootstrap";

import AppNavbar from "@components/AppNavbar";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";

import { useAppLogic } from "@hooks/useAppLogic";
import { FaPlus } from "react-icons/fa6";

import { useDialogs } from "@hooks/useDialogs";

function App() {
  const {
    bots,
    leaderboards,
    selectedBotId,
    selectBot,
    submitNewBot,
    loading,
    status,
    deleteBot,
    renameBot,
    matchmakingEnabled,
    enableMatchmaking,
    createLeaderboard,
    patchLeaderboard,
    deleteLeaderboard,
  } = useAppLogic();
  const {
    submitBotDialog,
    viewCodeDialog,
    confirmDialog,
    renameBotDialog,
    createLeaderboardDialog,
  } = useDialogs();

  const selectedBot = bots.find((b) => b.id == selectedBotId);

  return (
    <>
      <AppNavbar
        loading={loading}
        status={status}
        matchmakingEnabled={matchmakingEnabled}
        enableMatchmaking={enableMatchmaking}
        openSubmitDialog={() =>
          submitBotDialog.show({ onSubmit: submitNewBot })
        }
      />
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
    </>
  );
}

export default App;
