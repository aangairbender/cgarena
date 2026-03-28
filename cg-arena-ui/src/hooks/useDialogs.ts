import { useContext } from "react";
import DialogsContext from "src/contexts/DialogsContext";

export const useDialogs = () => {
  const ctx = useContext(DialogsContext);
  if (!ctx) {
    throw new Error("DialogsProvider is not found");
  }

  return ctx;
};
