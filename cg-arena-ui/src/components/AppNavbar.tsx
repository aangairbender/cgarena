import React from "react";
import {
  Badge,
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
  status: "connected" | "connecting";
  openSubmitDialog: () => void;
  matchmakingEnabled: boolean,
  enableMatchmaking: (v: boolean) => void;
}

const AppNavbar: React.FC<AppNavbarProps> = ({
  loading,
  status,
  openSubmitDialog,
  matchmakingEnabled,
  enableMatchmaking,
}) => {
  const pillBg = status == "connected" ? "success" : "warning";
  const pillText = status == "connected" ? "light" : "dark";

  return (
    <Navbar className="bg-body-tertiary">
      <Container>
        <Navbar.Brand href="#home">CG Arena</Navbar.Brand>

        <Stack direction="horizontal" gap={3}>
          {loading && <Spinner animation="border" />}
          <Badge pill bg={pillBg} text={pillText}>{status}</Badge>
          <Form.Switch
            checked={matchmakingEnabled}
            onChange={(e) => enableMatchmaking(e.target.checked)}
            label="Matchmaking"
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
