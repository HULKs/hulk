import Connection, { Cycler, OutputType } from "../Connection/Connection";
import { FieldDimensions } from "../FieldDimensions";
import { Isometry2 } from "../Isometry2";
import Field from "../shared/Field";
import Transform from "../shared/Transform";
import {
  useOutputSubscription,
  useParameterSubscription,
} from "../useSubscription";
import "./Localization.css";

export default function Localization({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const hypotheses = useOutputSubscription<
    | {
        pose_filter: {
          mean: number[];
          covariance: number[];
        };
        score: number;
      }[]
    | null
  >(
    connection,
    Cycler.Control,
    OutputType.Additional,
    "localization.pose_hypotheses"
  );
  const correspondenceLines = useOutputSubscription<number[][][] | null>(
    connection,
    Cycler.Control,
    OutputType.Additional,
    "localization.correspondence_lines"
  );
  const measuredLinesInField = useOutputSubscription<number[][][] | null>(
    connection,
    Cycler.Control,
    OutputType.Additional,
    "localization.measured_lines_in_field"
  );
  const localizationUpdates = useOutputSubscription<
    | {
        robot_to_field: Isometry2;
        line_center_point: number[];
        fit_error: number;
        line_distance_to_robot: number;
        line_length_weight: number;
      }[][]
    | null
  >(connection, Cycler.Control, OutputType.Additional, "localization.updates");
  const fieldDimensions = useParameterSubscription<FieldDimensions>(
    connection,
    "field_dimensions"
  );

  const state = (() => {
    const states = {
      hypotheses: hypotheses,
      correspondenceLines: correspondenceLines,
      measuredLinesInField: measuredLinesInField,
      localizationUpdates: localizationUpdates,
      fieldDimensions: fieldDimensions,
    };
    const undefinedStates = Object.entries(states)
      .filter(([_stateName, state]) => state === undefined)
      .map(([stateName, _state]) => stateName);
    const nullStates = Object.entries(states)
      .filter(([_stateName, state]) => state === null)
      .map(([stateName, _state]) => stateName);
    if (undefinedStates.length > 0 || nullStates.length > 0) {
      const stateStringParts = [];
      if (undefinedStates.length > 0) {
        stateStringParts.push(
          `${undefinedStates.join(", ")} ${
            undefinedStates.length === 1 ? "is" : "are"
          } undefined`
        );
      }
      if (nullStates.length > 0) {
        stateStringParts.push(
          `${nullStates.join(", ")} ${
            nullStates.length === 1 ? "is" : "are"
          } null`
        );
      }
      return `${stateStringParts.join(", ")}`;
    }
    return `${hypotheses!.length} hypotheses (${hypotheses!
      .map((hypothesis) => hypothesis.score.toFixed(2))
      .join(", ")}), ${measuredLinesInField!.length} measured lines (red)`;
  })();
  const header = (
    <div className="header">
      <div className="panelType">Localization:</div>
      <div className="state">{state}</div>
      {selector}
      {connector}
    </div>
  );
  let renderedCorrespondenceLines = null;
  if (correspondenceLines !== undefined && correspondenceLines !== null) {
    renderedCorrespondenceLines = correspondenceLines.map(
      (correspondenceLine) => (
        <line
          x1={correspondenceLine[0][0]}
          y1={correspondenceLine[0][1]}
          x2={correspondenceLine[1][0]}
          y2={correspondenceLine[1][1]}
          stroke="yellow"
          strokeWidth="0.025"
        />
      )
    );
  }
  let renderedMeasuredLinesInField = null;
  if (measuredLinesInField !== undefined && measuredLinesInField !== null) {
    renderedMeasuredLinesInField = measuredLinesInField.map(
      (measuredLineInField) => (
        <line
          x1={measuredLineInField[0][0]}
          y1={measuredLineInField[0][1]}
          x2={measuredLineInField[1][0]}
          y2={measuredLineInField[1][1]}
          stroke="red"
          strokeWidth="0.025"
        />
      )
    );
  }
  let renderedLocalizationUpdates = null;
  if (localizationUpdates !== undefined && localizationUpdates !== null) {
    renderedLocalizationUpdates = localizationUpdates.map(
      (LocalizationUpdate) =>
        LocalizationUpdate.map((localizationUpdate) => (
          <>
            <Transform isometry={localizationUpdate.robot_to_field}>
              <line
                x1="0"
                y1="0"
                x2="0.3"
                y2="0"
                stroke="blue"
                strokeWidth="0.01"
              />
            </Transform>
            <text
              x={0}
              y={0}
              fontSize={0.15}
              transform={`scale(1, -1) translate(${
                localizationUpdate.line_center_point[0]
              }, ${-localizationUpdate.line_center_point[1]}) rotate(10)`}
            >{`${localizationUpdate.fit_error.toFixed(
              3
            )}, ${localizationUpdate.line_distance_to_robot.toFixed(
              3
            )}, ${localizationUpdate.line_length_weight.toFixed(3)}, ${(
              localizationUpdate.fit_error *
              localizationUpdate.line_distance_to_robot *
              localizationUpdate.line_length_weight
            ).toFixed(10)}`}</text>
          </>
        ))
    );
  }
  let renderedHypotheses = null;
  if (hypotheses !== undefined && hypotheses !== null) {
    renderedHypotheses = hypotheses.map((hypothesis) => {
      let positionCovariance = [
        hypothesis.pose_filter.covariance[0],
        hypothesis.pose_filter.covariance[1],
        hypothesis.pose_filter.covariance[3],
        hypothesis.pose_filter.covariance[4],
      ];
      return (
        <g
          transform={`translate(${hypothesis.pose_filter.mean[0]},${
            hypothesis.pose_filter.mean[1]
          }) rotate(${(180 / Math.PI) * hypothesis.pose_filter.mean[2]}) `}
        >
          {ellipseFromCovariance(positionCovariance)}
          <circle
            r="0.2"
            fill="yellow"
            fillOpacity="0.4"
            stroke="black"
            strokeWidth="0.01"
          />
          <line
            x1="0"
            y1="0"
            x2="0.3"
            y2="0"
            stroke="black"
            strokeWidth="0.01"
          />
        </g>
      );
    });
  }
  let content = (
    <Field fieldDimensions={fieldDimensions}>
      {renderedCorrespondenceLines}
      {renderedMeasuredLinesInField}
      {renderedLocalizationUpdates}
      {renderedHypotheses}
    </Field>
  );

  return (
    <div className="localization">
      {header}
      {content}
    </div>
  );
}

function ellipseFromCovariance(covariance: Array<number>) {
  const radiusX = Math.sqrt(
    (covariance[0] + covariance[3]) / 2 +
      Math.sqrt(((covariance[0] - covariance[3]) / 2) ** 2 + covariance[1] ** 2)
  );
  const radiusY = Math.sqrt(
    (covariance[0] + covariance[3]) / 2 -
      Math.sqrt(((covariance[0] - covariance[3]) / 2) ** 2 + covariance[1] ** 2)
  );
  let angle;
  if (covariance[1] === 0 && covariance[0] >= covariance[3]) {
    angle = 0;
  } else if (covariance[1] === 0 && covariance[0] < covariance[3]) {
    angle = Math.PI / 2;
  } else {
    angle = Math.atan2(radiusX - covariance[0], covariance[1]);
  }
  return (
    <ellipse
      rx={radiusX}
      ry={radiusY}
      transform={`rotate(${angle})`}
      fill="none"
      stroke="purple"
      strokeWidth={0.01}
    />
  );
}
