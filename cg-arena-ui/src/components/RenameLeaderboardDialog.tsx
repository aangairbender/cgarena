import { useState } from "react";
import { LeaderboardId, RenameLeaderboardRequest } from "@models";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";

export interface RenameLeaderboardDialogData {
  leaderboardId: LeaderboardId;
  currentName: string;
  onSubmit: (id: LeaderboardId, req: RenameLeaderboardRequest) => Promise<void>;
}

const RenameLeaderboardDialog = (dialog: DialogProps<RenameLeaderboardDialogData>) => {
  const [name, setName] = useState("");
  const [error, setError] = useState("");

  const data = dialog.data;
  if (data === undefined) return null;


  const canSubmit = name.length > 0;

  const closeDialog = () => {
    setName("");
    setError("");
    dialog.hide();
  };

  const handleSubmit = async () => {
    const req: RenameLeaderboardRequest = { name };
    try {
      await data.onSubmit(data.leaderboardId, req);
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
        <Modal.Title>Rename leaderboard</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Name</Form.Label>
          <Form.Control
            placeholder="Leaderboard's name"
            defaultValue={data.currentName}
            onChange={(e) => setName(e.target.value)}
          />
          <Form.Text className="text-muted">
            Non-empty string up to 64 characters long.
          </Form.Text>
        </Form.Group>

        {error && <Alert variant="danger">{error}</Alert>}
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={closeDialog}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleSubmit} disabled={!canSubmit}>
          Rename
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default RenameLeaderboardDialog;
