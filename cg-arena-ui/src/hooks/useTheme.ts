import { useContext } from "react";
import ThemeContext from "@/contexts/ThemeContext";

export const useTheme = () => {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error("ThemeProvider is not found");
  }

  return ctx;
};
