import { DialogProps } from "@hooks/useDialog";
import { useTheme } from "@hooks/useTheme";
import { Modal } from "react-bootstrap";
import { LightAsync as SyntaxHighlighter } from 'react-syntax-highlighter';
import { atomOneLight, atomOneDark } from 'react-syntax-highlighter/dist/esm/styles/hljs';

export interface ViewCodeDialogData {
  title: string;
  content: string;
}

const ViewCodeDialog = (dialog: DialogProps<ViewCodeDialogData>) => {
  const { theme } = useTheme();
  const style = theme === 'dark' ? atomOneDark : atomOneLight;

  const data = dialog.data;
  if (data === undefined) return null;

  return (
    <Modal show={dialog.isOpen} onHide={dialog.hide} scrollable size="xl">
      <Modal.Header closeButton>
        <Modal.Title>{data.title}</Modal.Title>
      </Modal.Header>
      <Modal.Body className="p-0">
        <SyntaxHighlighter language="text" style={style} showLineNumbers>{data.content}</SyntaxHighlighter>
      </Modal.Body>
    </Modal>
  );
};

export default ViewCodeDialog;
