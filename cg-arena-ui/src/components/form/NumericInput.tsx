import { Form, FormGroup } from "react-bootstrap";

interface NumericInputProps {
  label: string;
  value: number;
  errors: string[];
  onChange: (v: number) => void;
  onBlur: () => void;
}

export default function NumbericInput({
  label,
  value,
  errors,
  onChange,
  onBlur,
}: NumericInputProps) {
  const hasErrors = errors.length > 0;

  return (
    <FormGroup>
      <Form.Label>{label}</Form.Label>
      <Form.Control
        type="text"
        pattern="[0-9]*"
        value={value}
        onChange={(e) => onChange(Number(e.target.value.replace(/\D/g, "")))}
        onBlur={onBlur}
        isInvalid={hasErrors}
      />
      {hasErrors && (
        <Form.Control.Feedback type="invalid">
          <ul>
            {errors.map((e) => (
              <li>{e}</li>
            ))}
          </ul>
        </Form.Control.Feedback>
      )}
    </FormGroup>
  );
}
