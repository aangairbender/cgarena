import "./App.css";
import { Container, Stack } from "react-bootstrap";

import SubmitBotDialog from "@components/SubmitBotDialog";
import AppNavbar from "@components/AppNavbar";
import BotSelector from "@components/BotSelector";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import ViewContentDialog from "@components/ViewContentDialog";
import ConfirmDialog from "@components/ConfirmDialog";
import RenameBotDialog from "@components/RenameBotDialog";
import { useAppLogic } from "@hooks/useAppLogic";
import { useDialog } from "@hooks/useDialog";

function App() {
  const {
    selectedBotId,
    bots,
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
      <Container className="mt-3">
        <BotSelector
          selectedId={selectedBotId}
          onSelected={(id) => selectBot(id)}
          items={bots}
        />
        <Stack className="mt-3">
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
          {leaderboardData && (
            <Leaderboard data={leaderboardData} selectBot={selectBot} />
          )}
        </Stack>
      </Container>
      <SubmitBotDialog {...submitBotDialog} />
      <ViewContentDialog {...viewContentDialog} />
      <ConfirmDialog {...confirmDialog} />
      <RenameBotDialog {...renameBotDialog} />
    </>
  );
}

export default App;
