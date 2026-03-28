import { ChartDialogData } from "@components/ChartDialog";
import { ConfirmDialogData } from "@components/ConfirmDialog";
import { CreateLeaderboardDialogData } from "@components/CreateLeaderboardDialog";
import { ExampleSeedsDialogData } from "@components/ExampleSeedsDialog";
import { PatchLeaderboardDialogData } from "@components/PatchLeaderboardDialog";
import { RenameBotDialogData } from "@components/RenameBotDialog";
import { SubmitBotDialogData } from "@components/SubmitBotDialog";
import { ViewCodeDialogData } from "@components/ViewCodeDialog";
import { DialogProps } from "@hooks/useDialog";
import { createContext } from "react";

const DialogsContext = createContext<DialogsContextType | undefined>(undefined);

interface DialogsContextType {
  submitBotDialog: DialogProps<SubmitBotDialogData>;
  viewCodeDialog: DialogProps<ViewCodeDialogData>;
  confirmDialog: DialogProps<ConfirmDialogData>;
  renameBotDialog: DialogProps<RenameBotDialogData>;
  createLeaderboardDialog: DialogProps<CreateLeaderboardDialogData>;
  patchLeaderboardDialog: DialogProps<PatchLeaderboardDialogData>;
  exampleSeedsDialog: DialogProps<ExampleSeedsDialogData>;
  chartDialog: DialogProps<ChartDialogData>;
}

export default DialogsContext;
