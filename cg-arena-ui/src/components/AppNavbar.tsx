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
import { useDialogs } from "@hooks/useDialogs";
import { useAppStore } from "@hooks/useAppStore";
import { Link } from "@tanstack/react-router";

function AppNavbar() {
  const { submitBotDialog } = useDialogs();
  const loading = useAppStore((state) => state.loading);
  const status = useAppStore((state) => state.status);
  const matchmakingEnabled = useAppStore((state) => state.matchmakingEnabled);
  const enableMatchmaking = useAppStore((state) => state.enableMatchmaking);
  const submitNewBot = useAppStore((state) => state.submitNewBot);

  const openSubmitDialog = () => {
    submitBotDialog.show({ onSubmit: submitNewBot });
  };
  const pillBg = status == "connected" ? "success" : "warning";
  const pillText = status == "connected" ? "light" : "dark";

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
            <Link
              to="/"
              className="nav-link"
              search={(prev) => ({ selectedBotId: prev.selectedBotId })}
            >
              Home
            </Link>
            {/* <Link to="/config" className="nav-link" search={{}}>
              Config
            </Link> */}
          </Nav>
        </Navbar.Collapse>

        <Stack direction="horizontal" gap={3}>
          {loading && <Spinner animation="border" />}
          <Badge pill bg={pillBg} text={pillText}>
            {status}
          </Badge>
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
}

export default AppNavbar;
