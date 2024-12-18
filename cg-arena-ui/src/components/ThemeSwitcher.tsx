import { useTheme } from "@hooks/useTheme";
import { Button } from "react-bootstrap";
import { MdDarkMode, MdLightMode } from "react-icons/md";


const ThemeSwitcher: React.FC = () => {
  const { theme, toggleTheme } = useTheme();

  if (theme == "dark") {
    return (
      <Button variant="outline-light" onClick={toggleTheme}>
        <MdLightMode />
      </Button>
    )
  } else {
    return (
      <Button variant="outline-dark" onClick={toggleTheme}>
        <MdDarkMode />
      </Button>
    )
  };
};

export default ThemeSwitcher;
