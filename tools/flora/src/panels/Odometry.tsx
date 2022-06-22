import Connection, { Cycler, OutputType } from "../Connection/Connection";
import { FieldDimensions } from "../FieldDimensions";
import { Isometry2 } from "../Isometry2";
import Field from "../shared/Field";
import Transform from "../shared/Transform";
import {
  useOutputSubscription,
  useParameterSubscription,
} from "../useSubscription";
import "./Odometry.css";

export default function Odometry({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const accumulatedOdometry = useOutputSubscription<Isometry2 | null>(
    connection,
    Cycler.Control,
    OutputType.Additional,
    "accumulated_odometry"
  );
  const fieldDimensions = useParameterSubscription<FieldDimensions>(
    connection,
    "field_dimensions"
  );
  let robot = null;
  if (accumulatedOdometry !== undefined && accumulatedOdometry !== null) {
    robot = (
      <Transform isometry={accumulatedOdometry}>
        <circle
          r="0.2"
          fill="yellow"
          fillOpacity="0.4"
          stroke="black"
          strokeWidth="0.01"
        />
        <line x1="0" y1="0" x2="0.3" y2="0" stroke="black" strokeWidth="0.01" />
      </Transform>
    );
  }
  return (
    <div className="odometry">
      <div className="header">
        <div className="panelType">Odometry</div>
        {selector}
        {connector}
      </div>
      <Field fieldDimensions={fieldDimensions}>{robot}</Field>
    </div>
  );
}
