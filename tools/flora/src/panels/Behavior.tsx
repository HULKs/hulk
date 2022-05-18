import React, { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./Behavior.css";

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

export default function Behavior({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const [ballPosition, setBallPosition] = useState<Array<number> | undefined>(
    undefined
  );
  const [fallState, setFallState] = useState<Object | undefined>(undefined);
  const [fieldDimensions, setFieldDimensions] = useState<
    FieldDimensions | undefined
  >(undefined);
  const [filteredGameState, setFilteredGameState] = useState<
    Object | undefined
  >(undefined);
  const [gameControllerState, setGameControllerState] = useState<
    Object | undefined
  >(undefined);
  const [headYaw, setHeadYaw] = useState<number | undefined>(undefined);
  const [motionCommand, setMotionCommand] = useState<Object | undefined>(
    undefined
  );
  const [primaryState, setPrimaryState] = useState<Object | undefined>(
    undefined
  );
  const [robotPose, setRobotPose] = useState<Isometry | undefined>(undefined);
  const [role, setRole] = useState<string | undefined>(undefined);
  const [walkTarget, setWalkTarget] = useState<Isometry | undefined>(undefined);

  useEffect(() => {
    if (connection === null) {
      return;
    }

    const unsubscribeBall = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "world_state.ball.position",
      (ball) => {
        setBallPosition(ball);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeFallState = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "world_state.robot.fall_state",
      (fallState) => {
        setFallState(fallState);
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

    const unsubscribeFilteredGameState = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "filtered_game_state",
      (filteredGameState) => {
        setFilteredGameState(filteredGameState);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeGameControllerState = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "game_controller_state",
      (gameControllerState) => {
        setGameControllerState(gameControllerState);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeHeadYaw = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "positions.head.yaw",
      (headYaw) => {
        setHeadYaw(headYaw);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeMotionCommand = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "motion_command.motion",
      (motionCommand) => {
        setMotionCommand(motionCommand);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribePrimaryState = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "world_state.robot.primary_state",
      (primaryState) => {
        setPrimaryState(primaryState);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeRobotPose = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "world_state.robot.pose",
      (robotPose) => {
        setRobotPose(robotPose);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeRole = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "world_state.robot.role",
      (role) => {
        setRole(role);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    const unsubscribeWalkTarget = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Main,
      "world_state.robot.walk_target_pose",
      (walkTarget) => {
        setWalkTarget(walkTarget);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    return () => {
      unsubscribeBall();
      unsubscribeFallState();
      unsubscribeFieldDimensions();
      unsubscribeFilteredGameState();
      unsubscribeGameControllerState();
      unsubscribeHeadYaw();
      unsubscribeMotionCommand();
      unsubscribePrimaryState();
      unsubscribeRobotPose();
      unsubscribeRole();
      unsubscribeWalkTarget();
    };
  }, [connection]);

  const header = (
    <div className="header">
      <div className="panelType">Behavior</div>
      <div className="outputType"></div>
      <div className="path"></div>
      <div className="type"></div>
      {selector}
      {connector}
    </div>
  );
  let content = undefined;
  if (
    ballPosition === undefined ||
    fallState === undefined ||
    fieldDimensions === undefined ||
    filteredGameState === undefined ||
    gameControllerState === undefined ||
    headYaw === undefined ||
    motionCommand === undefined ||
    primaryState === undefined ||
    robotPose === undefined ||
    role === undefined ||
    walkTarget === undefined
  ) {
    content = (
      <div className="content noData">NAO has not sent all data yet</div>
    );
  } else {
    let motionCommandText = "";
    if (typeof motionCommand === "string") {
      motionCommandText = motionCommand;
    } else {
      motionCommandText = Object.keys(motionCommand)[0];
    }

    let fallStateText = "";
    if (typeof fallState === "string") {
      fallStateText = fallState;
    } else {
      fallStateText = Object.keys(fallState)[0];
    }

    let gameControllerContent = undefined;
    if (gameControllerState === null) {
      gameControllerContent = (
        <tspan x="0" dy=".2pt">
          Game controller: not connected
        </tspan>
      );
    } else {
      gameControllerContent = (
        <tspan x="0" dy=".2pt">
          Game controller: connected
        </tspan>
      );
    }

    let filteredGameStateContent = undefined;
    if (filteredGameState !== null) {
      let filteredGameStateText = undefined;
      if (typeof filteredGameState === "string") {
        filteredGameStateText = filteredGameState;
      } else {
        filteredGameStateText = Object.keys(filteredGameState)[0];
      }
      filteredGameStateContent = (
        <tspan x="0" dy=".2pt">
          Filtered game state: {filteredGameStateText}
        </tspan>
      );
    }

    const robotMarker = (
      <Transform isometry={robotPose}>
        <g transform={`rotate(${28.15 + (headYaw * 180.0) / Math.PI}) `}>
          <line
            x1="0"
            y1="0"
            x2="1.5"
            y2="0"
            stroke="yellow"
            strokeWidth="0.01"
          />
        </g>
        <g transform={`rotate(${-28.15 + (headYaw * 180.0) / Math.PI}) `}>
          <line
            x1="0"
            y1="0"
            x2="1.5"
            y2="0"
            stroke="yellow"
            strokeWidth="0.01"
          />
        </g>
        <circle
          r="0.13665"
          fill="red"
          fillOpacity="1.0"
          stroke="black"
          strokeWidth="0.01"
        />
        <line x1="0" y1="0" x2="0.2" y2="0" stroke="black" strokeWidth="0.01" />
      </Transform>
    );

    let walkTargetPose = undefined;
    if (motionCommandText === "Walk") {
      walkTargetPose = (
        <Transform isometry={walkTarget}>
          <circle
            r="0.13665"
            fill="red"
            fillOpacity="0.3"
            stroke="black"
            strokeWidth="0.01"
          />
          <line
            x1="0"
            y1="0"
            x2="0.2"
            y2="0"
            stroke="black"
            strokeWidth="0.01"
          />
        </Transform>
      );
    }

    let ball = undefined;
    if (ballPosition !== null) {
      ball = (
        <Transform isometry={robotPose}>
          <g transform={`translate(${ballPosition[0]},${ballPosition[1]}) `}>
            <circle
              r={fieldDimensions.ball_radius}
              fill="white"
              fillOpacity="1.0"
              stroke="black"
              strokeWidth="0.01"
            />
          </g>
        </Transform>
      );
    }

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
          {walkTargetPose}
          {robotMarker}
          {ball}
        </g>
        <g
          transform={`translate(${
            -(fieldDimensions.length + fieldDimensions.border_strip_width) / 2
          },${
            -(fieldDimensions.width + fieldDimensions.border_strip_width) / 2
          }) `}
        >
          <text className="robotText" dy="0">
            <tspan x="0" dy=".2pt">
              Primary state: {primaryState}
            </tspan>
            <tspan x="0" dy=".2pt">
              Fall state: {fallStateText}
            </tspan>
            {gameControllerContent}
            {filteredGameStateContent}
            <tspan x="0" dy=".2pt">
              Role: {role}
            </tspan>
            <tspan x="0" dy=".3pt">
              Action: {motionCommandText}
            </tspan>
          </text>
        </g>
      </svg>
    );
  }

  return (
    <div className="behavior">
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
