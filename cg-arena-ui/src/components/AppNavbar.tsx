import React from "react";
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
  autoRefresh: boolean,
  setAutoRefresh: (v: boolean) => void;
}

const AppNavbar: React.FC<AppNavbarProps> = ({
  loading,
  openSubmitDialog,
  autoRefresh,
  setAutoRefresh,
}) => {
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
