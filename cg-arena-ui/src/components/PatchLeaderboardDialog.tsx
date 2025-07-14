import { useEffect, useState } from "react";
import { LeaderboardId, LeaderboardOverviewResponse, PatchLeaderboardRequest } from "@models";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";

export interface PatchLeaderboardDialogData {
  leaderboard: LeaderboardOverviewResponse;
  onSubmit: (id: LeaderboardId, req: PatchLeaderboardRequest) => Promise<void>;
}

const PatchLeaderboardDialog = (dialog: DialogProps<PatchLeaderboardDialogData>) => {
  const [name, setName] = useState("");
  const [filter, setFilter] = useState("");
  const [error, setError] = useState("");

  const data = dialog.data;

  useEffect(() => {
    if (dialog.isOpen && data) {
      setName(data.leaderboard.name);
      setFilter(data.leaderboard.filter);
    }
  }, [dialog.isOpen, data]);

  if (data === undefined) return null;

  const canSubmit = (name.length > 0 && name != data.leaderboard.name)
    || (filter.length > 0 && filter != data.leaderboard.filter);

  const closeDialog = () => {
    setName("");
    setFilter("");
    setError("");
    dialog.hide();
  };

  const handleSubmit = async () => {
    const req: PatchLeaderboardRequest = { name, filter };
    try {
      await data.onSubmit(data.leaderboard.id, req);
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
        <Modal.Title>Change leaderboard</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Name</Form.Label>
          <Form.Control
            placeholder="Leaderboard's name"
            defaultValue={data.leaderboard.name}
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
            defaultValue={data.leaderboard.filter}
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
        <Button variant="primary" onClick={handleSubmit} disabled={!canSubmit}>
          Submit
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default PatchLeaderboardDialog;
