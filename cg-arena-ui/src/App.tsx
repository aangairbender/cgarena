import "./App.css";
import { Container, Stack } from "react-bootstrap";

import SubmitBotDialog from "@components/SubmitBotDialog";
import AppNavbar from "@components/AppNavbar";
import BotSelector from "@components/BotSelector";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import { useAppLogic } from "@hooks/useAppLogic";
import ViewContentDialog from "@components/ViewContentDialog";
import { useDialog } from "@hooks/useDialog";
import ConfirmDialog from "@components/ConfirmDialog";

function App() {
  const {
    selectedBotId,
    bots,
    leaderboardData,
    selectBot,
    submitNewBot,
    loading,
    refreshLeaderboard,
    deleteBot,
  } = useAppLogic();
  const submitBotDialog = useDialog({ onSubmit: submitNewBot });
  const viewContentDialog = useDialog({ title: "", content: "" });
  const confirmDialog = useDialog({ prompt: "", action: () => {} });

  return (
    <>
      <AppNavbar
        loading={loading}
        refreshLeaderboard={refreshLeaderboard}
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
              deleteBot={() => confirmDialog.show({
                prompt: `Are you sure you want to delete bot '${leaderboardData.bot_overview.name}'?`,
                action: () => { deleteBot(leaderboardData.bot_overview.id) },
              })}
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
    </>
  );
}

export default App;
