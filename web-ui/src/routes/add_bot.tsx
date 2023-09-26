import { useState } from "react";
import { Button, Form } from "react-bootstrap";
import { CreateBotRequest, createBot } from "../services/api_service";
import { useNavigate } from "react-router-dom";

export default function AddBot() {
    const navigate = useNavigate();
    const [name, setName] = useState("");
    const [language, setLanguage] = useState("cpp");
    const [sourceCode, setSourceCode] = useState("");

    const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
        e.preventDefault();
        const data: CreateBotRequest = {
            name,
            language,
            source_code: sourceCode
        };
        const result = await createBot(data);
        if (result) {
            navigate("/bots");
        } else {
            alert("Error creating a bot");
        }
    };

    return (
        <Form onSubmit={handleSubmit}>
            <Form.Group className="mb-3" controlId="formBotName">
                <Form.Label>Bot name</Form.Label>
                <Form.Control
                    type="input"
                    placeholder="Enter bot name"
                    value={name}
                    onChange={e => setName(e.target.value)} />
                <Form.Text className="text-muted">
                    Bot name must be unique.
                </Form.Text>
            </Form.Group>

            <Form.Group className="mb-3">
                <Form.Label>Bot language</Form.Label>
                <Form.Select aria-label="Bot language selection" value={language} onChange={e => setLanguage(e.target.value)}>
                    <option value="cpp">C++</option>
                    <option value="rust">Rust</option>
                    <option value="python3">Python3</option>
                </Form.Select>
            </Form.Group>

            <Form.Group className="mb-3" controlId="formBotSourceCode">
                <Form.Label>Source code</Form.Label>
                <Form.Control as="textarea" rows={24} value={sourceCode} onChange={e => setSourceCode(e.target.value)} />
            </Form.Group>

            <Button variant="primary" type="submit">
                Submit
            </Button>
        </Form>
    );
}
