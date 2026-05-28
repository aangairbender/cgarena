import { useDialog } from "@/hooks/useDialog";
import { PropsWithChildren } from "react";
import ChartDialog, { ChartDialogData } from "./ChartDialog";
import ConfirmDialog, { ConfirmDialogData } from "./ConfirmDialog";
import CreateLeaderboardDialog, {
  CreateLeaderboardDialogData,
} from "./CreateLeaderboardDialog";
import ExampleSeedsDialog, {
  ExampleSeedsDialogData,
} from "./ExampleSeedsDialog";
import PatchLeaderboardDialog, {
  PatchLeaderboardDialogData,
} from "./PatchLeaderboardDialog";
import RenameBotDialog, { RenameBotDialogData } from "./RenameBotDialog";
import SubmitBotDialog, { SubmitBotDialogData } from "./SubmitBotDialog";
import ViewCodeDialog, { ViewCodeDialogData } from "./ViewCodeDialog";
import DialogsContext from "@/contexts/DialogsContext";
import ReplayDialog, { ReplayDialogData } from "./ReplayDialog";

const DialogsProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const submitBotDialog = useDialog<SubmitBotDialogData>();
  const viewCodeDialog = useDialog<ViewCodeDialogData>();
  const confirmDialog = useDialog<ConfirmDialogData>();
  const renameBotDialog = useDialog<RenameBotDialogData>();
  const createLeaderboardDialog = useDialog<CreateLeaderboardDialogData>();
  const patchLeaderboardDialog = useDialog<PatchLeaderboardDialogData>();
  const exampleSeedsDialog = useDialog<ExampleSeedsDialogData>();
  const chartDialog = useDialog<ChartDialogData>();
  const replayDialog = useDialog<ReplayDialogData>();

  const dialogs = {
    submitBotDialog,
    viewCodeDialog,
    confirmDialog,
    renameBotDialog,
    createLeaderboardDialog,
    patchLeaderboardDialog,
    exampleSeedsDialog,
    chartDialog,
    replayDialog,
  };

  return (
    <DialogsContext.Provider value={dialogs}>
      <>
        {children}

        <SubmitBotDialog {...submitBotDialog} />
        <ViewCodeDialog {...viewCodeDialog} />
        <ConfirmDialog {...confirmDialog} />
        <RenameBotDialog {...renameBotDialog} />
        <CreateLeaderboardDialog {...createLeaderboardDialog} />
        <PatchLeaderboardDialog {...patchLeaderboardDialog} />
        <ExampleSeedsDialog {...exampleSeedsDialog} />
        <ChartDialog {...chartDialog} />
        <ReplayDialog {...replayDialog} />
      </>
    </DialogsContext.Provider>
  );
};

export default DialogsProvider;
