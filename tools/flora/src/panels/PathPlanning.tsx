import React, { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./PathPlanning.css";
import { Vector } from 'vecti'

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
type Point = [number, number];
type Isometry = {
  rotation: [number, number];
  translation: Point;
};
type Circle = { center: Point, radius: number };
type LineSegment = { LineSegment: [Point, Point] };
type Direction = "Clockwise" | "Counterclockwise";
type Arc = { circle: Circle, start: Point, end: Point };
type ArcSegment = { Arc: [Arc, Direction] };
type PathSegment = LineSegment | ArcSegment;
type PathObstacle = { nodes: Array<number>, shape: PathObstacleShape };
type PathObstacleShape = { Circle: Circle } | { LineSegment: [Point, Point] };

export default function PathPlanning({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const [fieldDimensions, setFieldDimensions] = useState<
    FieldDimensions | undefined
  >(undefined);
  const [robotToField, setRobotToField] = useState<Isometry | undefined>(
    undefined
  );
  const [ballPosition, setBallPosition] = useState<{ position: Point } | null | undefined>(
    undefined
  );
  const [plannedPath, setPlannedPath] = useState<Array<PathSegment> | null | undefined>(
    undefined
  );
  const [pathObstacles, setPathObstacles] = useState<Array<PathObstacle> | undefined>(
    undefined
  );

  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribeFieldDimensions = connection.subscribeParameter(
      "field_dimensions",
      (fieldDimensions) => {
        setFieldDimensions(fieldDimensions);
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
    const unsubscribeBallPosition = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "ball_position",
      (ballPosition) => {
        setBallPosition(ballPosition);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribePlannedPath = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "motion_command",
      (motionCommand) => {
        if (typeof motionCommand !== "string" && "Walk" in motionCommand) {
          setPlannedPath(motionCommand.Walk.path);
        }
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribePathObstacles = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Additional,
      "path_obstacles",
      (pathObstacles) => {
        setPathObstacles(pathObstacles);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    return () => {
      unsubscribeFieldDimensions();
      unsubscribeRobotToField();
      unsubscribeBallPosition();
      unsubscribePlannedPath();
      unsubscribePathObstacles();
    };
  }, [connection]);
  const header = (
    <div className="header">
      <div className="panelType">PathPlanning</div>
      <div className="outputType"></div>
      <div className="path"></div>
      <div className="type"></div>
      {selector}
      {connector}
    </div>
  );
  let content = undefined;
  if (
    fieldDimensions === undefined ||
    fieldDimensions === null ||
    robotToField === undefined ||
    robotToField === null ||
    ballPosition === undefined ||
    plannedPath === undefined ||
    pathObstacles === undefined
  ) {
    content = (
      <div className="content noData">NAO has not sent all data yet</div>
    );
  } else {
    const obstacles = pathObstacles?.map((obstacle) => {
      if ("Circle" in obstacle.shape) {
        const circle = obstacle.shape.Circle;
        return (<circle
          cx={circle.center[0]}
          cy={circle.center[1]}
          r={circle.radius}
          stroke="blue"
          stroke-width="0.05"
          fill="none"
        />)
      }
      if ("LineSegment" in obstacle.shape) {
        const line_segment = obstacle.shape.LineSegment;
        return (
          <line
            x1={`${line_segment[0][0]}`}
            y1={`${line_segment[0][1]}`}
            x2={`${line_segment[1][0]}`}
            y2={`${line_segment[1][1]}`}
            stroke="blue"
            strokeWidth={fieldDimensions.line_width}
          />);
      }
      return null;
    });
    const path = plannedPath?.map((segment) => {
      if ("LineSegment" in segment) {
        const line_segment = segment.LineSegment;
        return (
          <line
            x1={`${line_segment[0][0]}`}
            y1={`${line_segment[0][1]}`}
            x2={`${line_segment[1][0]}`}
            y2={`${line_segment[1][1]}`}
            stroke="orange"
            strokeWidth={fieldDimensions.line_width}
          />);
      }
      if ("Arc" in segment) {
        const [arc, direction] = segment.Arc;
        const x_axis_rotation = 0;
        const sweep_flag = direction === "Clockwise" ? 0 : 1;
        const long_arc_flag = determine_long_arc_flag(arc, sweep_flag);
        return (<path d={`
            M ${arc.start[0]} ${arc.start[1]}
            A ${arc.circle.radius} ${arc.circle.radius} ${x_axis_rotation} ${long_arc_flag} ${sweep_flag}  ${arc.end[0]} ${arc.end[1]}
          `}
          fill="none"
          stroke="red"
          strokeWidth={fieldDimensions.line_width}
          strokeLinecap="round"
        />);
      }
      return null;
    });
    let ball = undefined;
    if (ballPosition !== null) {
      ball = (<circle
        cx={ballPosition.position[0]}
        cy={ballPosition.position[1]}
        r={fieldDimensions.ball_radius}
        stroke="black"
        strokeWidth="0.015"
        fill="white"
      />);
    }
    const pathSegments = (
      <Transform isometry={robotToField}>
        {obstacles}
        {ball}
        {path}
      </Transform>
    );
    content = (
      <svg
        className="overlay"
        viewBox={`${-(fieldDimensions.length + fieldDimensions.border_strip_width) / 2
          } ${-(fieldDimensions.width + fieldDimensions.border_strip_width) / 2
          } ${fieldDimensions.length + fieldDimensions.border_strip_width} ${fieldDimensions.width + fieldDimensions.border_strip_width
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
          {pathSegments}
        </g>
      </svg>
    );
  }
  return (
    <div className="paths">
      {header}
      {content}
    </div>
  );
}

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

function determine_long_arc_flag(arc: Arc, sweep: number): number {
  const center = Vector.of(arc.circle.center);
  const start = Vector.of(arc.start).subtract(center);
  const end = Vector.of(arc.end).subtract(center);
  const long_arc = start.rotateByDegrees(90).dot(end) > 0 ? 0 : 1;
  return sweep === 1 ? long_arc : 1 - long_arc;
}
