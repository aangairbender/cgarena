import "./App.css";
import { Button, Card, Container } from "react-bootstrap";

import SubmitBotDialog, {
  SubmitBotDialogData,
} from "@components/SubmitBotDialog";
import AppNavbar from "@components/AppNavbar";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import ViewCodeDialog, { ViewCodeDialogData } from "@components/ViewCodeDialog";
import ConfirmDialog, { ConfirmDialogData } from "@components/ConfirmDialog";
import RenameBotDialog, {
  RenameBotDialogData,
} from "@components/RenameBotDialog";
import { useAppLogic } from "@hooks/useAppLogic";
import { useDialog } from "@hooks/useDialog";
import { FaPlus } from "react-icons/fa6";
import CreateLeaderboardDialog, {
  CreateLeaderboardDialogData,
} from "@components/CreateLeaderboardDialog";
import PatchLeaderboardDialog, {
  PatchLeaderboardDialogData,
} from "@components/PatchLeaderboardDialog";
import ExampleSeedsDialog, {
  ExampleSeedsDialogData,
} from "@components/ExampleSeedsDialog";
import ChartDialog, { ChartDialogData } from "@components/ChartDialog";

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
  const submitBotDialog = useDialog<SubmitBotDialogData>();
  const viewCodeDialog = useDialog<ViewCodeDialogData>();
  const confirmDialog = useDialog<ConfirmDialogData>();
  const renameBotDialog = useDialog<RenameBotDialogData>();
  const createLeaderboardDialog = useDialog<CreateLeaderboardDialogData>();
  const patchLeaderboardDialog = useDialog<PatchLeaderboardDialogData>();
  const exampleSeedsDialog = useDialog<ExampleSeedsDialogData>();
  const chartDialog = useDialog<ChartDialogData>();

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
            chartDialog={chartDialog}
            exampleSeedsDialog={exampleSeedsDialog}
            patchLeaderboardDialog={patchLeaderboardDialog}
            confirmDialog={confirmDialog}
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

      <SubmitBotDialog {...submitBotDialog} />
      <ViewCodeDialog {...viewCodeDialog} />
      <ConfirmDialog {...confirmDialog} />
      <RenameBotDialog {...renameBotDialog} />
      <CreateLeaderboardDialog {...createLeaderboardDialog} />
      <PatchLeaderboardDialog {...patchLeaderboardDialog} />
      <ExampleSeedsDialog {...exampleSeedsDialog} />
      <ChartDialog {...chartDialog} />
    </>
  );
}

export default App;
