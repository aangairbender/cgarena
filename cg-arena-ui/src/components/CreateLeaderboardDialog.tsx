import { useState } from "react";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";
import { CreateLeaderboardRequest } from "@models";

export interface CreateLeaderboardDialogData {
  onCreate: (req: CreateLeaderboardRequest) => Promise<void>;
}

const CreateLeaderboardDialog = (dialog: DialogProps<CreateLeaderboardDialogData>) => {
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("");
  const [error, setError] = useState("");

  const data = dialog.data;
  if (data === undefined) return null;

  const canCreate = name.length > 0;

  const closeDialog = () => {
    setName("");
    setFilter("");
    setError("");
    dialog.hide();
  };

  const handleCreate = async () => {
    const req: CreateLeaderboardRequest = { name, filter };
    try {
      await data.onCreate(req);
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
