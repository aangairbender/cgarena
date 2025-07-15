import { useEffect, useMemo, useState } from "react";
import { Alert, Button, Form, Modal, Spinner, ToggleButton, ToggleButtonGroup } from "react-bootstrap";
import { DialogProps } from "@hooks/useDialog";
import { BotOverviewResponse, ChartOverviewResponse, ChartRequest, ChartTurnDataResponse } from "@models";
import * as api from "@api";
import { AxisOptions, Chart, UserSerie } from "react-charts";
import { useTheme } from "@hooks/useTheme";

export interface ChartDialogData {
  bots: BotOverviewResponse[];
  filter: string;
}

const ChartDialog = (dialog: DialogProps<ChartDialogData>) => {
  const [filter, setFilter] = useState("");
  const [attr, setAttr] = useState("");
  const [error, setError] = useState("");
  const [chart, setChart] = useState<ChartOverviewResponse>();
  const [loading, setLoading] = useState(false);
  const [metric, setMetric] = useState<"avg" | "min" | "max">("avg");
  const { theme } = useTheme();

  const data = dialog.data;

  useEffect(() => {
    if (dialog.isOpen && data) {
      setFilter(data.filter);
    }
  }, [dialog.isOpen, data]);

  const primaryAxis = useMemo(
    (): AxisOptions<ChartTurnDataResponse> => ({
      getValue: datum => datum.turn,
    }),
    []
  );

  const secondaryAxes = useMemo(
    (): AxisOptions<ChartTurnDataResponse>[] => [
      {
        getValue: datum => datum[metric],
      },
    ],
    [metric]
  );

  const chartData = useMemo(() => {
    if (chart === undefined || data === undefined) {
      return undefined;
    }

    if (chart.items.length == 0) {
      setError("No matches with such attribute")
      return undefined;
    }

    const mapped: UserSerie<ChartTurnDataResponse>[] = chart.items.map(item => ({
      label: data.bots.find(b => b.id == item.bot_id)?.name ?? "unknown",
      data: item.data,
    }));

    setMetric("avg");

    return mapped;
  }, [chart, data]);

  if (data === undefined) return null;

  const canCreate = attr.length > 0;

  const closeDialog = () => {
    setAttr("");
    setFilter("");
    setError("");
    setChart(undefined);
    dialog.hide();
  };

  const handleCreate = async () => {
    const req: ChartRequest = { filter, attribute_name: attr };
    try {
      setError("");
      setLoading(true);
      setChart(undefined);
      const res = await api.chart(req);
      setLoading(false);
      setChart(res);
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      } else {
        setError(String(e));
      }
    }
  };

  return (
    <Modal show={dialog.isOpen} onHide={closeDialog} centered size="xl">
      <Modal.Header closeButton>
        <Modal.Title>Visualize turn data</Modal.Title>
      </Modal.Header>
      <Modal.Body>
        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Bot turn attribute name</Form.Label>
          <Form.Control
            placeholder=""
            value={attr}
            onChange={(e) => setAttr(e.target.value)}
          />
          <Form.Text className="text-muted">
            The "key" of the attribute recorded with "[PDATA][turn] key = value". Only last 1000 matches matching the filter would be used for visualization.
          </Form.Text>
        </Form.Group>

        <Form.Group controlId="formName" className="mb-3">
          <Form.Label>Match filter</Form.Label>
          <Form.Control
            placeholder=""
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <Form.Text className="text-muted">
            e.g. match.player_count == 2
          </Form.Text>
        </Form.Group>

        {error && <Alert variant="danger">{error}</Alert>}

        {loading && <Spinner animation="border" />}

        {chartData && <div className="mb-3" style={{height: "400px"}}>
          <div>
            <ToggleButtonGroup type="radio" value={metric} name="metric" onChange={(v) => setMetric(v)}>
              <ToggleButton id="tbg-btn-1" variant="secondary" size="sm" value="avg">
                avg
              </ToggleButton>
              <ToggleButton id="tbg-btn-2" variant="secondary" size="sm" value="min">
                min
              </ToggleButton>
              <ToggleButton id="tbg-btn-3" variant="secondary" size="sm" value="max">
                max
              </ToggleButton>
            </ToggleButtonGroup>
          </div>
          <Chart
            options={{
              data: chartData,
              primaryAxis,
              secondaryAxes,
              padding: {
                bottom: 16
              },
              dark: theme === "dark"
            }}
          />
        </div>}
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={closeDialog}>
          Cancel
        </Button>
        <Button variant="primary" onClick={handleCreate} disabled={!canCreate || loading}>
          Visualize
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default ChartDialog;
