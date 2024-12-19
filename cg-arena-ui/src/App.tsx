import "./App.css";
import { useState } from "react";
import { Container, Stack } from "react-bootstrap";

import SubmitBotDialog from "@components/SubmitBotDialog";
import AppNavbar from "@components/AppNavbar";
import BotSelector from "@components/BotSelector";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import { useAppLogic } from "@hooks/useAppLogic";
import ViewContentDialog from "@components/ViewContentDialog";

function App() {
  const {
    selectedBotId,
    bots,
    leaderboardData,
    selectBot,
    submitNewBot,
    loading,
    refreshLeaderboard,
  } = useAppLogic();
  const [submitDialogOpen, setSubmitDialogOpen] = useState(false);
  const [viewContentDialogOpen, setViewContentDialogOpen] = useState(false);
  const [viewContentData, setViewContentData] = useState({
    title: "",
    content: "",
  });

  const showContentDialog = (title: string, content: string) => {
    setViewContentData({ title, content });
    setViewContentDialogOpen(true);
  };

  return (
    <>
      <AppNavbar
        loading={loading}
        refreshLeaderboard={refreshLeaderboard}
        openSubmitDialog={() => setSubmitDialogOpen(true)}
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
              showContentDialog={showContentDialog}
            />
          )}
          {leaderboardData && (
            <Leaderboard data={leaderboardData} selectBot={selectBot} />
          )}
        </Stack>
      </Container>
      <SubmitBotDialog
        open={submitDialogOpen}
        onClose={() => setSubmitDialogOpen(false)}
        onSubmit={submitNewBot}
      />
      <ViewContentDialog
        open={viewContentDialogOpen}
        onClose={() => setViewContentDialogOpen(false)}
        title={viewContentData.title}
        content={viewContentData.content}
      />
    </>
  );
}

export default App;
