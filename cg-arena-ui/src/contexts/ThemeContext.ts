import { createContext } from "react";

export type Theme = "light" | "dark";

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

interface ThemeContextType {
  theme: Theme;
  toggleTheme: () => void;
}

export default ThemeContext;
