import React, { useEffect, useState } from "react";
import {
  Button,
  Container,
  Form,
  Navbar,
  Spinner,
  Stack,
} from "react-bootstrap";
import ThemeSwitcher from "./ThemeSwitcher";

interface AppNavbarProps {
  loading: boolean;
  openSubmitDialog: () => void;
  refreshLeaderboard: () => void;
}

const AppNavbar: React.FC<AppNavbarProps> = ({
  loading,
  openSubmitDialog,
  refreshLeaderboard,
}) => {
  const [autoRefresh, setAutoRefresh] = useState(true);

  useEffect(() => {
    if (!autoRefresh) return;
    
    const interval = setInterval(refreshLeaderboard, 3000); // in ms
    return () => clearInterval(interval);
  }, [refreshLeaderboard, autoRefresh]);

  return (
    <Navbar className="bg-body-tertiary">
      <Container>
        <Navbar.Brand href="#home">CG Arena</Navbar.Brand>

        <Stack direction="horizontal" gap={3}>
          {loading && <Spinner animation="border" />}
          <Form.Switch
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
            label="Auto Refresh"
          />
          <Button variant="primary" onClick={openSubmitDialog}>
            Submit a new bot
          </Button>
          <ThemeSwitcher />
        </Stack>
      </Container>
    </Navbar>
  );
};

export default AppNavbar;
