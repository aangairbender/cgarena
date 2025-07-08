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
    leaderboardData,
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
            {leaderboardData && (
              <BotOverview
                bot={leaderboardData.bot_overview}
                showContentDialog={viewContentDialog.show}
                deleteBot={() =>
                  confirmDialog.show({
                    prompt: `Are you sure you want to delete bot '${leaderboardData.bot_overview.name}'?`,
                    action: () => {
                      deleteBot(leaderboardData.bot_overview.id);
                    },
                  })
                }
                renameBot={() =>
                  renameBotDialog.show({
                    botId: leaderboardData.bot_overview.id,
                    currentName: leaderboardData.bot_overview.name,
                    onSubmit: renameBot,
                  })
                }
              />
            )}
          </Card.Body>
        </Card>

        <Card className="mt-4">
          <Card.Header>Global Leaderboard</Card.Header>
          <Card.Body>
            {leaderboardData && (
              <Leaderboard data={leaderboardData} selectBot={selectBot} />
            )}
          </Card.Body>
        </Card>

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
