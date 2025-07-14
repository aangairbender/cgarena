import { useEffect, useState } from "react";
import { BotId, RenameBotRequest } from "@models";
import { Alert, Button, Form, Modal } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";

export interface RenameBotDialogData {
  botId: BotId;
  currentName: string;
  onSubmit: (id: BotId, req: RenameBotRequest) => Promise<void>;
}

const RenameBotDialog = (dialog: DialogProps<RenameBotDialogData>) => {
  const [name, setName] = useState("");
  const [error, setError] = useState("");

  const data = dialog.data;

  useEffect(() => {
    if (dialog.isOpen && data) {
      setName(data.currentName);
    }
  }, [dialog.isOpen, data]);

  if (data === undefined) return null;

  const canSubmit = name.length > 0 && name != data.currentName;

  const closeDialog = () => {
    setName("");
    setError("");
    dialog.hide();
  };

  const handleSubmit = async () => {
    const req: RenameBotRequest = { name };
    try {
      await data.onSubmit(data.botId, req);
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
        <Modal.Title>Rename bot</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Name</Form.Label>
          <Form.Control
            placeholder="Bot's name"
            defaultValue={data.currentName}
            onChange={(e) => setName(e.target.value)}
          />
          <Form.Text className="text-muted">
            Non-empty string up to 32 characters long.
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

export default RenameBotDialog;
