import { useMemo, useState } from "react";
import {
  Alert,
  Button,
  Form,
  Modal,
  Spinner,
  ToggleButton,
  ToggleButtonGroup,
} from "react-bootstrap";
import { DialogProps } from "@/hooks/useDialog";
import {
  BotOverviewResponse,
  ChartOverviewResponse,
  ChartRequest,
  ChartTurnDataResponse,
} from "@/models";
import * as api from "@/api";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { useTheme } from "@/hooks/useTheme";

export interface ChartDialogData {
  bots: BotOverviewResponse[];
  filter: string;
}

type ChartMetric = "avg" | "min" | "max";

type ChartPoint = {
  turn: number;
  [seriesKey: string]: number;
};

const SERIES_COLORS = [
  "#0d6efd",
  "#dc3545",
  "#198754",
  "#fd7e14",
  "#6f42c1",
  "#0dcaf0",
];

const ChartDialog = (dialog: DialogProps<ChartDialogData>) => {
  const [filter, setFilter] = useState(dialog.data?.filter ?? "");
  const [attr, setAttr] = useState("");
  const [error, setError] = useState("");
  const [chart, setChart] = useState<ChartOverviewResponse>();
  const [loading, setLoading] = useState(false);
  const [metric, setMetric] = useState<ChartMetric>("avg");
  const { theme } = useTheme();

  const data = dialog.data;


  const chartView = useMemo(() => {
    if (chart === undefined || data === undefined || chart.items.length === 0) {
      return undefined;
    }

    const pointsByTurn = new Map<number, ChartPoint>();
    const series = chart.items.map((item, index) => {
      const key = `bot-${item.bot_id}`;

      item.data.forEach((datum: ChartTurnDataResponse) => {
        const point = pointsByTurn.get(datum.turn) ?? { turn: datum.turn };
        point[key] = datum[metric];
        pointsByTurn.set(datum.turn, point);
      });

      return {
        key,
        label:
          data.bots.find((bot) => bot.id === item.bot_id)?.name ?? "unknown",
        color: SERIES_COLORS[index % SERIES_COLORS.length],
      };
    });

    return {
      points: Array.from(pointsByTurn.values()).sort(
        (left, right) => left.turn - right.turn,
      ),
      series,
    };
  }, [chart, data, metric]);

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
    setError("");
    setLoading(true);
    setChart(undefined);
    setMetric("avg");

    try {
      const response = await api.chart(req);
      if (response.items.length === 0) {
        setError("No matches with such attribute");
      } else {
        setChart(response);
      }
    } catch (e) {
      if (e instanceof Error) {
        setError(e.message);
      } else {
        setError(String(e));
      }
    } finally {
      setLoading(false);
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
            The "key" of the attribute recorded with "[PDATA][turn] key =
            value". Only last 1000 matches matching the filter would be used for
            visualization.
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

        {chartView && (
          <div className="mb-3" style={{ height: "400px" }}>
            <div>
              <ToggleButtonGroup
                type="radio"
                value={metric}
                name="metric"
                onChange={(value: ChartMetric) => setMetric(value)}
              >
                <ToggleButton
                  id="tbg-btn-1"
                  variant="secondary"
                  size="sm"
                  value="avg"
                >
                  avg
                </ToggleButton>
                <ToggleButton
                  id="tbg-btn-2"
                  variant="secondary"
                  size="sm"
                  value="min"
                >
                  min
                </ToggleButton>
                <ToggleButton
                  id="tbg-btn-3"
                  variant="secondary"
                  size="sm"
                  value="max"
                >
                  max
                </ToggleButton>
              </ToggleButtonGroup>
            </div>
            <ResponsiveContainer width="100%" height="100%">
              <LineChart
                data={chartView.points}
                margin={{ top: 16, right: 24, bottom: 16, left: 8 }}
              >
                <CartesianGrid
                  stroke={theme === "dark" ? "#495057" : "#dee2e6"}
                  strokeDasharray="3 3"
                />
                <XAxis dataKey="turn" type="number" allowDecimals={false} />
                <YAxis />
                <Tooltip />
                <Legend />
                {chartView.series.map((item) => (
                  <Line
                    key={item.key}
                    type="monotone"
                    dataKey={item.key}
                    name={item.label}
                    stroke={item.color}
                    dot={false}
                    connectNulls
                  />
                ))}
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}
      </Modal.Body>
      <Modal.Footer>
        <Button variant="secondary" onClick={closeDialog}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onClick={handleCreate}
          disabled={!canCreate || loading}
        >
          Visualize
        </Button>
      </Modal.Footer>
    </Modal>
  );
};

export default ChartDialog;
