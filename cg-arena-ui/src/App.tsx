import "./App.css";
import { Button, Card, Container } from "react-bootstrap";

import SubmitBotDialog, { SubmitBotDialogData } from "@components/SubmitBotDialog";
import AppNavbar from "@components/AppNavbar";
import BotOverview from "@components/BotOverview";
import Leaderboard from "@components/Leaderboard";
import ViewContentDialog, { ViewContentDialogData } from "@components/ViewContentDialog";
import ConfirmDialog, { ConfirmDialogData } from "@components/ConfirmDialog";
import RenameBotDialog, { RenameBotDialogData } from "@components/RenameBotDialog";
import { useAppLogic } from "@hooks/useAppLogic";
import { useDialog } from "@hooks/useDialog";
import { FaPencil, FaPlus, FaTrash } from "react-icons/fa6";
import CreateLeaderboardDialog, { CreateLeaderboardDialogData } from "@components/CreateLeaderboardDialog";
import RenameLeaderboardDialog, { RenameLeaderboardDialogData } from "@components/RenameLeaderboardDialog";
import { GLOBAL_LEADERBOARD_ID } from "@models";

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
    createLeaderboard,
    renameLeaderboard,
    deleteLeaderboard,
  } = useAppLogic();
  const submitBotDialog = useDialog<SubmitBotDialogData>();
  const viewContentDialog = useDialog<ViewContentDialogData>();
  const confirmDialog = useDialog<ConfirmDialogData>();
  const renameBotDialog = useDialog<RenameBotDialogData>();
  const createLeaderboardDialog = useDialog<CreateLeaderboardDialogData>();
  const renameLeaderboardDialog = useDialog<RenameLeaderboardDialogData>();

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
            <Card.Header className="d-flex justify-content-between align-items-center">
              <div>{lb.name}</div>
              <div className="d-flex gap-2">
                {lb.id != GLOBAL_LEADERBOARD_ID && (
                  <>
                    <Button
                      variant="outline-warning"
                      size="sm"
                      onClick={() => renameLeaderboardDialog.show({leaderboardId: lb.id, currentName: lb.name, onSubmit: renameLeaderboard})}
                    >
                      <FaPencil className="bi"/>
                    </Button>

                    <Button
                      variant="outline-danger"
                      size="sm"
                      onClick={() => confirmDialog.show({ prompt: `Do you really want to delete leaderboard {lb.name}?`, action: () => deleteLeaderboard(lb.id)})}
                    >
                      <FaTrash className="bi"/>
                    </Button>
                  </>
                )}
              </div>
            </Card.Header>
            <Card.Body>
              <Leaderboard bots={bots} data={lb} selectedBotId={selectedBotId} selectBot={selectBot} />
            </Card.Body>
          </Card>
        ))}

        <Container className="mt-4 d-flex justify-content-center">
          <Button
            className="mx-1"
            variant="outline-secondary"
            onClick={() => createLeaderboardDialog.show({ onCreate: createLeaderboard })}
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
      <RenameLeaderboardDialog {...renameLeaderboardDialog} />
    </>
  );
}

export default App;
