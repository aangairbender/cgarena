import { useState } from "react";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";

interface Data {
  onCreate: () => Promise<void>;
}

const CreateLeaderboardDialog = (dialog: DialogProps<Data>) => {
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("");
  const [error, setError] = useState("");

  const canCreate = name.length > 0;

  const closeDialog = () => {
    setName("");
    setError("");
    dialog.hide();
  };

  const handleCreate = async () => {
    try {
      await dialog.data.onCreate();
      closeDialog();
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      } else {
        setError(String(e));
      }
    }
  };

  return (
    <Modal show={dialog.isOpen} onHide={closeDialog} centered>
      <Modal.Header closeButton>
        <Modal.Title>Create a new leaderboard</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Name</Form.Label>
          <Form.Control
            placeholder="Leaderboard's name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <Form.Text className="text-muted">
            Non-empty string up to 64 characters long.
          </Form.Text>
        </Form.Group>

        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Match filter</Form.Label>
          <Form.Control
            placeholder=""
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <Form.Text className="text-muted">
            e.g. match.player_count == 2
          </Form.Text>
        </Form.Group>

        {error && <Alert variant="danger">{error}</Alert>}
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={closeDialog}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleCreate} disabled={!canCreate}>
          Create
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default CreateLeaderboardDialog;
