import { DialogProps } from "@hooks/useDialog";
import { Modal } from "react-bootstrap";

export interface ViewContentDialogData {
  title: string;
  content: string;
}

const ViewContentDialog = (dialog: DialogProps<ViewContentDialogData>) => {
  const data = dialog.data;
  if (data === undefined) return null;

  return (
    <Modal show={dialog.isOpen} onHide={dialog.hide} scrollable fullscreen>
      <Modal.Header closeButton>
        <Modal.Title>{data.title}</Modal.Title>
      </Modal.Header>
      <Modal.Body className="bg-secondary text-light">
        {data.content}
      </Modal.Body>
    </Modal>
  );
};

export default ViewContentDialog;
