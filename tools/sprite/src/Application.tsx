import { useState } from "react";
import recording from "./recording.json";
import "./Application.css";
import { useFieldDimensions } from "./useFieldDimensions";
import { Circle, Line, useMarks } from "./useMarks";
import { State, useAnimation } from "./useAnimation";

export function Application() {
  const [frameIndex, setFrameIndex] = useState(0);
  const intervalMillisecondsRealtime =
    recording.simulation_configuration.time_step.secs * 1000 +
    recording.simulation_configuration.time_step.nanos / 1000000;
  const [state, setState] = useAnimation(
    intervalMillisecondsRealtime,
    intervalMillisecondsRealtime / 5,
    () => {
      if (frameIndex < recording.frames.length - 1) {
        setFrameIndex(frameIndex + 1);
      } else {
        setState(State.Pause);
      }
    },
    () => {
      if (frameIndex > 0) {
        setFrameIndex(frameIndex - 1);
      } else {
        setState(State.Pause);
      }
    }
  );
  const fieldDimensions = useFieldDimensions();
  const marks = useMarks(fieldDimensions);
  const frame = recording.frames[frameIndex];
  return (
    <div className="Application">
      <svg
        viewBox={`${
          -(fieldDimensions.length + fieldDimensions.border_strip_width) / 2
        } ${
          -(fieldDimensions.width + fieldDimensions.border_strip_width) / 2
        } ${fieldDimensions.length + fieldDimensions.border_strip_width} ${
          fieldDimensions.width + fieldDimensions.border_strip_width
        }`}
      >
        <g transform="scale(1, -1)">
          {marks.map((mark) => {
            switch (mark.type) {
              case "Line":
                return drawLine(mark, "white", fieldDimensions.line_width);
              case "Circle":
                return drawCircle(
                  mark,
                  "white",
                  fieldDimensions.line_width,
                  "none"
                );
            }
            return null;
          })}
          <circle
            cx={frame.ball_position[0]}
            cy={frame.ball_position[1]}
            r={fieldDimensions.ball_radius}
            fill="red"
          />
          {frame.robots.map((robot, robotIndex) => (
            <Transform isometry={robot.robot_to_field}>
              <Robot
                configuration={recording.robot_configurations[robotIndex]}
                database={robot.database}
              />
            </Transform>
          ))}
        </g>
      </svg>
      <div className="output">
        <div>GameState: {frame.game_state}</div>
        <div>
          MotionCommand:{" "}
          {frame.robots
            .map((robot) => {
              const motion = robot.database.main_outputs.motion_command.motion;
              if (typeof motion === "string") {
                return motion;
              } else if (typeof motion === "object") {
                return Object.keys(motion)[0];
              }
              return "???";
            })
            .join(", ")}
        </div>
      </div>
      <div className="time">
        <input
          className="frameIndexRange"
          type="range"
          min={0}
          max={Math.max(0, recording.frames.length - 1)}
          value={frameIndex}
          onChange={(event) => {
            setFrameIndex(parseInt(event.target.value));
          }}
        />
        <button
          onClick={() => {
            if (state === State.BackwardFast) {
              setState(State.Pause);
            } else {
              setState(State.BackwardFast);
            }
          }}
        >
          {state === State.BackwardFast ? "Pause" : "<<"}
        </button>
        <button
          onClick={() => {
            if (state === State.BackwardRealtime) {
              setState(State.Pause);
            } else {
              setState(State.BackwardRealtime);
            }
          }}
        >
          {state === State.BackwardRealtime ? "Pause" : "<"}
        </button>
        <button
          onClick={() => {
            if (state === State.ForwardRealtime) {
              setState(State.Pause);
            } else {
              setState(State.ForwardRealtime);
            }
          }}
        >
          {state === State.ForwardRealtime ? "Pause" : ">"}
        </button>
        <button
          onClick={() => {
            if (state === State.ForwardFast) {
              setState(State.Pause);
            } else {
              setState(State.ForwardFast);
            }
          }}
        >
          {state === State.ForwardFast ? "Pause" : ">>"}
        </button>
        <span className="frameLabel">Frame:</span>
        <input
          className="frameIndexNumber"
          type="number"
          size={6}
          min={0}
          max={Math.max(0, recording.frames.length - 1)}
          value={frameIndex}
          onChange={(event) => {
            setFrameIndex(parseInt(event.target.value));
          }}
        />
        <span className="timeLabel">
          {`Time: ${(
            frame.now.secs_since_epoch +
            frame.now.nanos_since_epoch / 1000000000
          ).toFixed(2)}s`}
        </span>
      </div>
    </div>
  );
}

function drawLine(
  line: Line,
  strokeColor: string,
  strokeWidth: number
): JSX.Element {
  return (
    <line
      x1={line.point0[0]}
      y1={line.point0[1]}
      x2={line.point1[0]}
      y2={line.point1[1]}
      stroke={strokeColor}
      strokeWidth={strokeWidth}
      fill="none"
    />
  );
}

function drawCircle(
  circle: Circle,
  strokeColor: string,
  strokeWidth: number,
  fillColor: string
): JSX.Element {
  return (
    <circle
      cx={circle.center[0]}
      cy={circle.center[1]}
      r={circle.radius}
      stroke={strokeColor}
      strokeWidth={strokeWidth}
      fill={fillColor}
    />
  );
}

type Isometry = {
  rotation: number[];
  translation: number[];
};

function Transform({
  isometry,
  children,
}: {
  isometry: Isometry;
  children: React.ReactNode | React.ReactNode[];
}) {
  const angle = toAngle(isometry.rotation);
  return (
    <g
      transform={`translate(${isometry.translation[0]},${isometry.translation[1]}) rotate(${angle})`}
    >
      {children}
    </g>
  );
}

function toAngle(rotation: number[]): number {
  return (180 / Math.PI) * Math.atan2(rotation[1], rotation[0]);
}

function Robot({
  configuration,
  database,
}: {
  configuration: any;
  database: any;
}): JSX.Element {
  const motion = database.main_outputs.motion_command.motion;
  let walkTarget = null;
  let plannedPathTarget = null;
  if (typeof motion === "object" && "Walk" in motion) {
    walkTarget = drawLine(
      {
        type: "Line",
        point0: [0, 0],
        point1: motion.Walk.target_pose.translation,
      },
      "red",
      0.0125
    );
    plannedPathTarget = drawLine(
      {
        type: "Line",
        point0: [0, 0],
        point1: database.main_outputs.planned_path.end_pose.translation,
      },
      "blue",
      0.0125
    );
  }
  return (
    <>
      {drawCircle(
        { type: "Circle", center: [0, 0], radius: 0.15 },
        "black",
        0.0125,
        "rgba(255, 255, 255, 0.75)"
      )}
      {drawLine(
        { type: "Line", point0: [0, 0], point1: [0.15, 0] },
        "black",
        0.0125
      )}
      {walkTarget}
      {plannedPathTarget}
      <g transform="scale(-1, 1)">
        <text x={0} y={-0.025} fontSize={0.15} textAnchor="middle">
          {configuration.player_number}
        </text>
      </g>
      <g transform="scale(1, -1)">
        <text x={0} y={-0.025} fontSize={0.15} textAnchor="middle">
          {configuration.player_number}
        </text>
      </g>
    </>
  );
}
