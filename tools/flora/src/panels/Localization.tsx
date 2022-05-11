import React, { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./Localization.css";

type FieldDimensions = {
  ball_radius: number;
  length: number;
  width: number;
  line_width: number;
  penalty_marker_size: number;
  goal_box_area_length: number;
  goal_box_area_width: number;
  penalty_area_length: number;
  penalty_area_width: number;
  penalty_marker_distance: number;
  center_circle_diameter: number;
  border_strip_width: number;
  goal_inner_width: number;
  goal_post_diameter: number;
  goal_depth: number;
};

export default function Localization({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const [poseEstimation, setPoseEstimation] = useState<
    | {
        hypotheses: {
          score: number;
          state_mean: number[];
          state_covariance: number[];
        }[];
      }
    | undefined
  >(undefined);
  const [lineInfosTop, setLineDataTop] = useState<
    Array<Array<Array<number>>> | undefined
  >(undefined);
  const [robotToField, setRobotToField] = useState<Isometry | undefined>(
    undefined
  );
  const [lineInfosBottom, setLineDataBottom] = useState<
    Array<Array<Array<number>>> | undefined
  >(undefined);
  const [fieldDimensions, setFieldDimensions] = useState<
    FieldDimensions | undefined
  >(undefined);

  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribePoseEstimation = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Additional,
      "pose_estimation",
      (pose_estimation) => {
        setPoseEstimation(pose_estimation);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeRobotToField = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "robot_to_field",
      (robotToField) => {
        setRobotToField(robotToField);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeLineDataTop = connection.subscribeOutput(
      Cycler.VisionTop,
      OutputType.Main,
      "line_data.lines_in_robot",
      (lineData) => {
        setLineDataTop(lineData);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeLineDataBottom = connection.subscribeOutput(
      Cycler.VisionBottom,
      OutputType.Main,
      "line_data.lines_in_robot",
      (lineData) => {
        setLineDataBottom(lineData);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeFieldDimensions = connection.subscribeParameter(
      "field_dimensions",
      (fieldDimensions) => {
        setFieldDimensions(fieldDimensions);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    return () => {
      unsubscribePoseEstimation();
      unsubscribeRobotToField();
      unsubscribeLineDataTop();
      unsubscribeLineDataBottom();
      unsubscribeFieldDimensions();
    };
  }, [connection]);
  const header = (
    <div className="header">
      <div className="panelType">Localization</div>
      <div className="outputType"></div>
      <div className="path"></div>
      <div className="type"></div>
      {selector}
      {connector}
    </div>
  );
  let robotHypotheses = undefined;
  let perceivedLines = undefined;
  let content = undefined;
  if (
    poseEstimation === undefined ||
    robotToField === undefined ||
    lineInfosTop === undefined ||
    lineInfosBottom === undefined ||
    fieldDimensions === undefined
  ) {
    content = (
      <div className="content noData">NAO has not sent all data yet</div>
    );
  } else {
    let linesTop = lineInfosTop.map((lineInfo) => (
      <line
        x1={`${lineInfo[0][0]}`}
        y1={`${lineInfo[0][1]}`}
        x2={`${lineInfo[1][0]}`}
        y2={`${lineInfo[1][1]}`}
        stroke="red"
        strokeWidth={fieldDimensions.line_width}
      />
    ));
    let linesBottom = lineInfosBottom.map((lineInfo) => (
      <line
        x1={`${lineInfo[0][0]}`}
        y1={`${lineInfo[0][1]}`}
        x2={`${lineInfo[1][0]}`}
        y2={`${lineInfo[1][1]}`}
        stroke="red"
        strokeWidth={fieldDimensions.line_width}
      />
    ));
    perceivedLines = (
      <Transform isometry={robotToField}>
        {linesTop}
        {linesBottom}
      </Transform>
    );
    robotHypotheses = poseEstimation.hypotheses.map((hypothesis) => {
      let positionCovariance = [
        hypothesis.state_covariance[0],
        hypothesis.state_covariance[1],
        hypothesis.state_covariance[3],
        hypothesis.state_covariance[4],
      ];
      return (
        <g
          transform={`translate(${hypothesis.state_mean[0]},${
            hypothesis.state_mean[1]
          })rotate(${(180 / Math.PI) * hypothesis.state_mean[2]}) `}
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
    content = (
      <svg
        className="overlay"
        viewBox={`${
          -(fieldDimensions.length + fieldDimensions.border_strip_width) / 2
        } ${
          -(fieldDimensions.width + fieldDimensions.border_strip_width) / 2
        } ${fieldDimensions.length + fieldDimensions.border_strip_width} ${
          fieldDimensions.width + fieldDimensions.border_strip_width
        }`}
      >
        <g transform="scale(1,-1)">
          <rect
            x={-fieldDimensions.length / 2}
            y={-fieldDimensions.width / 2}
            width={fieldDimensions.length}
            height={fieldDimensions.width}
            stroke="white"
            fill="none"
            strokeWidth={fieldDimensions.line_width}
          />
          <rect
            x={-fieldDimensions.length / 2}
            y={-fieldDimensions.penalty_area_width / 2}
            width={fieldDimensions.penalty_area_length}
            height={fieldDimensions.penalty_area_width}
            stroke="white"
            fill="none"
            strokeWidth={fieldDimensions.line_width}
          />
          <rect
            x={-fieldDimensions.length / 2}
            y={-fieldDimensions.goal_box_area_width / 2}
            width={fieldDimensions.goal_box_area_length}
            height={fieldDimensions.goal_box_area_width}
            stroke="white"
            fill="none"
            strokeWidth={fieldDimensions.line_width}
          />
          <rect
            x={fieldDimensions.length / 2 - fieldDimensions.penalty_area_length}
            y={-fieldDimensions.penalty_area_width / 2}
            width={fieldDimensions.penalty_area_length}
            height={fieldDimensions.penalty_area_width}
            stroke="white"
            fill="none"
            strokeWidth={fieldDimensions.line_width}
          />
          <rect
            x={
              fieldDimensions.length / 2 - fieldDimensions.goal_box_area_length
            }
            y={-fieldDimensions.goal_box_area_width / 2}
            width={fieldDimensions.goal_box_area_length}
            height={fieldDimensions.goal_box_area_width}
            stroke="white"
            fill="none"
            strokeWidth={fieldDimensions.line_width}
          />
          <line
            x1="0"
            y1={-fieldDimensions.width / 2}
            x2="0"
            y2={fieldDimensions.width / 2}
            stroke="white"
            strokeWidth={fieldDimensions.line_width}
          />
          <circle
            cx="0"
            cy="0"
            r={fieldDimensions.center_circle_diameter / 2}
            stroke="white"
            strokeWidth={fieldDimensions.line_width}
            fill="none"
          />
          <line
            x1={
              fieldDimensions.length / 2 -
              fieldDimensions.penalty_marker_distance -
              fieldDimensions.penalty_marker_size / 2
            }
            y1="0"
            x2={
              fieldDimensions.length / 2 -
              fieldDimensions.penalty_marker_distance +
              fieldDimensions.penalty_marker_size / 2
            }
            y2="0"
            stroke="white"
            strokeWidth={fieldDimensions.line_width * 0.5}
          />
          <line
            x1={
              fieldDimensions.length / 2 -
              fieldDimensions.penalty_marker_distance
            }
            y1={fieldDimensions.penalty_marker_size / 2}
            x2={
              fieldDimensions.length / 2 -
              fieldDimensions.penalty_marker_distance
            }
            y2={-fieldDimensions.penalty_marker_size / 2}
            stroke="white"
            strokeWidth={fieldDimensions.line_width * 0.5}
          />

          <line
            x1={
              -fieldDimensions.length / 2 +
              fieldDimensions.penalty_marker_distance -
              fieldDimensions.penalty_marker_size / 2
            }
            y1="0"
            x2={
              -fieldDimensions.length / 2 +
              fieldDimensions.penalty_marker_distance +
              fieldDimensions.penalty_marker_size / 2
            }
            y2="0"
            stroke="white"
            strokeWidth={fieldDimensions.line_width * 0.5}
          />
          <line
            x1={
              -fieldDimensions.length / 2 +
              fieldDimensions.penalty_marker_distance
            }
            y1={fieldDimensions.penalty_marker_size / 2}
            x2={
              -fieldDimensions.length / 2 +
              fieldDimensions.penalty_marker_distance
            }
            y2={-fieldDimensions.penalty_marker_size / 2}
            stroke="white"
            strokeWidth={fieldDimensions.line_width * 0.5}
          />
          {perceivedLines}
          {robotHypotheses}
        </g>
      </svg>
    );
  }
  return (
    <div className="candidates">
      {header}
      {content}
    </div>
  );
}

type Isometry = {
  rotation: Array<number>;
  translation: Array<number>;
};

function Transform({
  isometry,
  children,
}: {
  isometry: Isometry;
  children: React.ReactNode | React.ReactNode[];
}) {
  let angle = toAngle(isometry.rotation);
  return (
    <g
      transform={`translate(${isometry.translation[0]},${isometry.translation[1]})rotate(${angle}) `}
    >
      {children}
    </g>
  );
}

function toAngle(rotation: Array<number>): number {
  return (180 / Math.PI) * Math.atan2(rotation[1], rotation[0]);
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
      transform={`rotate${angle}`}
      color=""
      opacity="0.5"
    />
  );
}
