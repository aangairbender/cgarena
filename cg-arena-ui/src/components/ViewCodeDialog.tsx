import { DialogProps } from "@hooks/useDialog";
import { useTheme } from "@hooks/useTheme";
import { Button, Modal } from "react-bootstrap";
import { FaRegClipboard } from "react-icons/fa6";
import { Light as SyntaxHighlighter } from "react-syntax-highlighter";
import {
  atomOneLight,
  atomOneDark,
} from "react-syntax-highlighter/dist/esm/styles/hljs";

export interface ViewCodeDialogData {
  title: string;
  content: string;
}

const ViewCodeDialog = (dialog: DialogProps<ViewCodeDialogData>) => {
  const { theme } = useTheme();
  const style = theme === "dark" ? atomOneDark : atomOneLight;

  const copyToClipboard = async () => {
    const text = dialog.data?.content;
    if (text) {
      await navigator.clipboard.writeText(text);
    }
  };

  const data = dialog.data;
  if (data === undefined) return null;

  return (
    <Modal show={dialog.isOpen} onHide={dialog.hide} scrollable size="xl">
      <Modal.Header closeButton>
        <Modal.Title>{data.title}</Modal.Title>
        <Button
          style={{ marginLeft: 16 }}
          variant="outline-info"
          onClick={copyToClipboard}
        >
          <FaRegClipboard className="bi" />
        </Button>
      </Modal.Header>
      <Modal.Body className="p-0">
        <SyntaxHighlighter language="text" style={style} showLineNumbers>
          {data.content}
        </SyntaxHighlighter>
      </Modal.Body>
    </Modal>
  );
};

export default ViewCodeDialog;
