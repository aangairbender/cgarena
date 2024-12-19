import { DialogProps } from "@hooks/useDialog";
import { Button, Modal } from "react-bootstrap";

interface Data {
  prompt: string;
  action: () => void;
}

const ConfirmDialog = (dialog: DialogProps<Data>) => {
  const handleConfirm = () => {
    dialog.data.action();
    dialog.hide();
  };

  return (
    <Modal show={dialog.isOpen} onHide={dialog.hide} centered>
      <Modal.Header closeButton>
        <Modal.Title>Confirmation</Modal.Title>
      </Modal.Header>
      <Modal.Body>{dialog.data.prompt}</Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={dialog.hide}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleConfirm}>
          Confirm
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default ConfirmDialog;
