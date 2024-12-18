import { BotMinimalResponse } from "@models";
import { Form, Stack } from "react-bootstrap";

interface BotSelectorProps {
  selectedId: string | undefined;
  onSelected: (botId: string) => void;
  items: BotMinimalResponse[];
}

const BotSelector: React.FC<BotSelectorProps> = ({
  selectedId,
  onSelected,
  items,
}) => {
  return (
    <Stack>
      <Form.Group>
        <Form.Label>Select bot</Form.Label>
        <Form.Select
          value={selectedId}
          onChange={(e) => onSelected(e.target.value)}
        >
          {items.map((item) => (
            <option value={item.id} key={item.id}>
              {item.name}
            </option>
          ))}
        </Form.Select>
      </Form.Group>
    </Stack>
  );
};

export default BotSelector;
