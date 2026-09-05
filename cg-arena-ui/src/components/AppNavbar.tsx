import {
  Badge,
  Button,
  Container,
  Form,
  Nav,
  Navbar,
  Spinner,
  Stack,
} from "react-bootstrap";
import ThemeSwitcher from "./ThemeSwitcher";
import { useDialogs } from "@/hooks/useDialogs";
import { useAppStore } from "@/hooks/useAppStore";
import { Link } from "@tanstack/react-router";

function AppNavbar({ runtimeAvailable }: { runtimeAvailable: boolean }) {
  const { submitBotDialog } = useDialogs();
  const loading = useAppStore((state) => state.loading);
  const status = useAppStore((state) => state.status);
  const matchmakingEnabled = useAppStore((state) => state.matchmakingEnabled);
  const enableMatchmaking = useAppStore((state) => state.enableMatchmaking);
  const submitNewBot = useAppStore((state) => state.submitNewBot);

  const openSubmitDialog = () => {
    submitBotDialog.show({ onSubmit: submitNewBot });
  };
  const connected = runtimeAvailable && status === "connected";
  const statusText = runtimeAvailable ? status : "setup";

  return (
    <Navbar className="bg-body-tertiary">
      <Container>
        <Link
          to="/"
          className="navbar-brand"
          search={(prev) => ({ selectedBotId: prev.selectedBotId })}
        >
          CG Arena
        </Link>
        <Navbar.Toggle aria-controls="basic-navbar-nav" />
        <Navbar.Collapse id="basic-navbar-nav">
          <Nav className="me-auto">
            {runtimeAvailable && (
              <>
                <Link
                  to="/"
                  className="nav-link"
                  search={(prev) => ({ selectedBotId: prev.selectedBotId })}
                >
                  Home
                </Link>
                <Link to="/matches" className="nav-link">
                  Matches
                </Link>
              </>
            )}
            <Link to="/config" className="nav-link">
              Config
            </Link>
          </Nav>
        </Navbar.Collapse>

        <Stack direction="horizontal" gap={3}>
          {loading && <Spinner animation="border" />}
          <Badge
            pill
            bg={connected ? "success" : "warning"}
            text={connected ? "light" : "dark"}
          >
            {statusText}
          </Badge>
          {runtimeAvailable && (
            <>
              <Form.Switch
                checked={matchmakingEnabled}
                onChange={(event) => enableMatchmaking(event.target.checked)}
                label="Matchmaking"
              />
              <Button variant="primary" onClick={openSubmitDialog}>
                Submit a new bot
              </Button>
            </>
          )}
          <ThemeSwitcher />
        </Stack>
      </Container>
    </Navbar>
  );
}

export default AppNavbar;
