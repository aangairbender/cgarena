import { PropsWithChildren, useEffect, useState } from "react";
import ThemeContext, { Theme } from "src/contexts/ThemeContext";

const ThemeProvider: React.FC<PropsWithChildren> = ({children}) => {
  const [theme,  setTheme] = useState<Theme>(currentTheme());

  useEffect(() => {
    document.documentElement.setAttribute(ThemeAttribute, theme);
    localStorage.setItem(ThemeAttribute, theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme(t => t == "light" ? "dark" : "light");
  };

  return (
    <ThemeContext.Provider value={{theme, toggleTheme}}>
      {children}
    </ThemeContext.Provider>
  )
};


const ThemeAttribute = "data-bs-theme";

const preferredTheme = (): Theme | undefined => {
  if (!window.matchMedia) return undefined;

  if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return "dark";
  } else if (window.matchMedia('(prefers-color-scheme: light)').matches) {
    return "light";
  } else {
    return undefined;
  }
}

const localStoredTheme = (): Theme | undefined => {
  const stored = localStorage.getItem(ThemeAttribute);
  if (stored) return stored as Theme;
  else return undefined;
};

const currentTheme = (): Theme => {
  const current = document.documentElement.dataset[ThemeAttribute];
  if (current) return current as Theme;

  const stored = localStoredTheme();
  if (stored) return stored;

  const preferred = preferredTheme();
  if (preferred) return preferred;

  return "light";
};

export default ThemeProvider;