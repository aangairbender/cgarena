import { DialogProps } from "@hooks/useDialog";
import { Modal } from "react-bootstrap";

interface Data {
  title: string;
  content: string;
}

const ViewContentDialog = (dialog: DialogProps<Data>) => {
  return (
    <Modal show={dialog.isOpen} onHide={dialog.hide} scrollable fullscreen>
      <Modal.Header closeButton>
        <Modal.Title>{dialog.data.title}</Modal.Title>
      </Modal.Header>
      <Modal.Body className="bg-secondary text-light">
        {dialog.data.content}
      </Modal.Body>
    </Modal>
  );
};

export default ViewContentDialog;
