import { useState } from "react";
import recording from "./recording.json";
import "./Application.css";
import { useFieldDimensions } from "./useFieldDimensions";
import { Circle, Line, useMarks } from "./useMarks";
import { State, useAnimation } from "./useAnimation";
import { Vector } from "vecti";

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

  let filtered_game_state_string = "";
  if (typeof frame.filtered_game_state === "string") {
    filtered_game_state_string = frame.filtered_game_state;
  } else if (typeof frame.filtered_game_state === "object") {
    filtered_game_state_string = Object.keys(frame.filtered_game_state)[0];
  }

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
                simulation_configuration={recording.simulation_configuration}
                angle={toAngle(robot.robot_to_field.rotation)}
                head_yaw={toAngle(robot.head_yaw)}
              />
            </Transform>
          ))}
        </g>
        <text x={-4.5} y={-3.1} fontSize={0.15}>
          Messages: {frame.broadcasted_spl_message_counter} total. Amounting to{" "}
          {(
            frame.broadcasted_spl_message_counter /
            (frame.now.secs_since_epoch +
              frame.now.nanos_since_epoch / 1000000000)
          ).toFixed(2)}{" "}
          msg/sec on average.
        </text>
      </svg>
      <div className="output">
        <div>FilteredGameState: {filtered_game_state_string}</div>
        <div>
          Role:{" "}
          {frame.robots
            .map((robot) => {
              return robot.database.main_outputs.world_state.robot.role;
            })
            .join(", ")}
        </div>
        <div>
          MotionCommand:{" "}
          {frame.robots
            .map((robot) => {
              const motion = robot.database.main_outputs.motion_command;
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

function drawFieldOfView(
  head_yaw: number,
  maximum_angle: number,
  maximum_distance: number,
  strokeColor: string,
  strokeWidth: number
): JSX.Element {
  const x = maximum_distance * Math.cos(maximum_angle);
  const y = maximum_distance * Math.sin(maximum_angle);
  return (
    <g transform={`rotate(${head_yaw})`}>
      <path
        d={`M 0 0 L ${x} ${y} A ${maximum_distance} ${maximum_distance} 0 0 0 ${x} ${-y} L 0 0 Z`}
        fill="transparent"
        stroke={strokeColor}
        strokeOpacity={0.15}
        strokeWidth={strokeWidth}
      />
    </g>
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

type Point = [number, number];
type LineSegment = { LineSegment: Array<Point> };
type Direction = "Clockwise" | "Counterclockwise";
type Arc = { circle: Circle; start: Point; end: Point };
type ArcSegment = { Arc: [Arc, Direction] };
type PathSegment = LineSegment | ArcSegment;

function toAngle(rotation: number[]): number {
  return (180 / Math.PI) * Math.atan2(rotation[1], rotation[0]);
}

function determine_long_arc_flag(arc: Arc, sweep: number): number {
  const center = Vector.of(arc.circle.center);
  const start = Vector.of(arc.start).subtract(center);
  const end = Vector.of(arc.end).subtract(center);
  const long_arc = start.rotateByDegrees(90).dot(end) > 0 ? 0 : 1;
  return sweep === 1 ? long_arc : 1 - long_arc;
}

function Robot({
  configuration,
  database,
  simulation_configuration,
  angle,
  head_yaw,
}: {
  configuration: any;
  database: any;
  simulation_configuration: any;
  angle: any;
  head_yaw: any;
}): JSX.Element {
  const motion = database.main_outputs.motion_command;
  let plannedPathTarget = null;
  if (typeof motion === "object" && "Walk" in motion) {
    plannedPathTarget = motion.Walk.path?.map((segment: PathSegment) => {
      if ("LineSegment" in segment) {
        const line_segment = segment.LineSegment;
        return (
          <line
            x1={`${line_segment[0][0]}`}
            y1={`${line_segment[0][1]}`}
            x2={`${line_segment[1][0]}`}
            y2={`${line_segment[1][1]}`}
            stroke="orange"
            strokeWidth="0.0125"
          />
        );
      }
      if ("Arc" in segment) {
        const [arc, direction] = segment.Arc;
        const x_axis_rotation = 0;
        const sweep_flag = direction === "Clockwise" ? 0 : 1;
        const long_arc_flag = determine_long_arc_flag(arc, sweep_flag);
        return (
          <path
            d={`
            M ${arc.start[0]} ${arc.start[1]}
            A ${arc.circle.radius} ${arc.circle.radius} ${x_axis_rotation} ${long_arc_flag} ${sweep_flag}  ${arc.end[0]} ${arc.end[1]}
          `}
            fill="none"
            stroke="red"
            strokeWidth="0.0125"
            strokeLinecap="round"
          />
        );
      }
      return null;
    });
  }
  return (
    <>
      {drawFieldOfView(
        head_yaw,
        simulation_configuration.maximum_field_of_view_angle,
        simulation_configuration.maximum_field_of_view_distance,
        "yellow",
        0.0125
      )}
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
      {plannedPathTarget}

      <g transform={`scale(1, -1) rotate(${angle})`}>
        <text x={0} y={-0.025} fontSize={0.15} textAnchor="middle">
          {configuration.player_number}
        </text>
        <text x={0} y={0.1} fontSize={0.15} textAnchor="middle">
          {database.main_outputs.world_state.robot.role}
        </text>
      </g>
    </>
  );
}
