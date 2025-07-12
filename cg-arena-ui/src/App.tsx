import "./App.css";
import { Button, Card, Container } from "react-bootstrap";

import SubmitBotDialog from "@components/SubmitBotDialog";
import AppNavbar from "@components/AppNavbar";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import ViewContentDialog from "@components/ViewContentDialog";
import ConfirmDialog from "@components/ConfirmDialog";
import RenameBotDialog from "@components/RenameBotDialog";
import { useAppLogic } from "@hooks/useAppLogic";
import { useDialog } from "@hooks/useDialog";
import { FaPlus } from "react-icons/fa6";
import CreateLeaderboardDialog from "@components/CreateLeaderboardDialog";

function App() {
  const {
    bots,
    leaderboards,
    selectedBotId,
    selectBot,
    submitNewBot,
    loading,
    deleteBot,
    renameBot,
    autoRefresh,
    setAutoRefresh,
  } = useAppLogic();
  const submitBotDialog = useDialog({ onSubmit: submitNewBot });
  const viewContentDialog = useDialog({ title: "", content: "" });
  const confirmDialog = useDialog({ prompt: "", action: () => {} });
  const renameBotDialog = useDialog({
    botId: "",
    currentName: "",
    onSubmit: renameBot,
  });
  const createLeaderboardDialog = useDialog({ onCreate: async () => {} });

  const selectedBot = bots.find(b => b.id == selectedBotId);

  return (
    <>
      <AppNavbar
        loading={loading}
        autoRefresh={autoRefresh}
        setAutoRefresh={setAutoRefresh}
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
                showContentDialog={viewContentDialog.show}
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

        {leaderboards.map(lb => (
          <Card className="mt-4" key={lb.id}>
            <Card.Header>{lb.id == "0" ? "Global Leaderboard" : lb.name}</Card.Header>
            <Card.Body>
              <Leaderboard data={lb} selectedBotId={selectedBotId} selectBot={selectBot} />
            </Card.Body>
          </Card>
        ))}

        <Container className="mt-4 d-flex justify-content-center">
          <Button
            className="mx-1"
            variant="outline-secondary"
            onClick={() => createLeaderboardDialog.show({ onCreate: async () => {} })}
          >
            <FaPlus className="bi me-2" size={16} />
            New leaderboard
          </Button>
        </Container>
      </Container>

      <SubmitBotDialog {...submitBotDialog} />
      <ViewContentDialog {...viewContentDialog} />
      <ConfirmDialog {...confirmDialog} />
      <RenameBotDialog {...renameBotDialog} />
      <CreateLeaderboardDialog {...createLeaderboardDialog} />
    </>
  );
}

export default App;
