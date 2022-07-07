import Connection, { Cycler, OutputType } from "../Connection/Connection";
import { FieldDimensions } from "../FieldDimensions";
import { Isometry2 } from "../Isometry2";
import Field from "../shared/Field";
import Transform from "../shared/Transform";
import {
  useImageSubscription,
  useOutputSubscription,
  useParameterSubscription,
} from "../useSubscription";
import "./RobotDetection.css";

const COLORS = ["red", "blue", "green", "yellow", "purple", "orange"];
const segmentPointRadius = 3;

type ScoredClusterPoint = {
  point: [number, number];
  amount_score: number;
  luminance_score: number;
};
type ScoredCluster = {
  center: [number, number];
  radius: number;
  score: number;
};
type ClusterCone = {
  left: [number, number];
  right: [number, number];
};
type RobotPosition = {
  position: number[];
  last_seen: {
    nanos_since_epoch: number;
    secs_since_epoch: number;
  };
};

export default function RobotDetection({
  selector,
  connector,
  connection,
  cycler,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
  cycler: Cycler;
}) {
  const imageUrl = useImageSubscription(connection, cycler);
  const clusterPointsInPixel = useOutputSubscription<
    ScoredClusterPoint[] | null
  >(
    connection,
    cycler,
    OutputType.Additional,
    "robot_detection.cluster_points_in_pixel"
  );
  const clusteredClusterPointsInGround = useOutputSubscription<
    ScoredClusterPoint[][] | null
  >(
    connection,
    cycler,
    OutputType.Additional,
    "robot_detection.clustered_cluster_points_in_ground"
  );
  const detectedRobots = useOutputSubscription<{
    robot_positions: ScoredCluster[];
  } | null>(connection, cycler, OutputType.Main, "detected_robots");
  const filteredRobots = useOutputSubscription<RobotPosition[] | null>(
    connection,
    Cycler.Control,
    OutputType.Main,
    "robot_positions"
  );
  const clusterCones = useOutputSubscription<ClusterCone[] | null>(
    connection,
    cycler,
    OutputType.Additional,
    "robot_detection.cluster_cones"
  );
  const fieldDimensions = useParameterSubscription<FieldDimensions>(
    connection,
    "field_dimensions"
  );
  const robotToField = useOutputSubscription<Isometry2 | null>(
    connection,
    Cycler.Control,
    OutputType.Main,
    "robot_to_field"
  );
  const amountScoreExponent = useParameterSubscription<number>(
    connection,
    `${
      cycler === Cycler.VisionTop ? "vision_top" : "vision_bottom"
    }.robot_detection.amount_score_exponent`
  );
  const luminanceScoreExponent = useParameterSubscription<number>(
    connection,
    `${
      cycler === Cycler.VisionTop ? "vision_top" : "vision_bottom"
    }.robot_detection.luminance_score_exponent`
  );
  const renderedClusterPointsInPixel =
    clusterPointsInPixel !== undefined &&
    clusterPointsInPixel !== null &&
    amountScoreExponent !== undefined &&
    luminanceScoreExponent !== undefined
      ? clusterPointsInPixel.map((point) => {
          const score =
            Math.pow(point.amount_score, amountScoreExponent) *
            Math.pow(point.luminance_score, luminanceScoreExponent);
          const scoreAngle = score * 2 * Math.PI;
          const center = [point.point[0] * 2, point.point[1]];
          const top = [center[0], center[1] - segmentPointRadius];
          const value = [
            center[0] + Math.sin(scoreAngle) * segmentPointRadius,
            center[1] - Math.cos(scoreAngle) * segmentPointRadius,
          ];
          return (
            <>
              <circle
                cx={center[0]}
                cy={center[1]}
                r={segmentPointRadius}
                fill="black"
              />
              <path
                d={`M ${top[0]} ${
                  top[1]
                } A ${segmentPointRadius} ${segmentPointRadius} 0 ${
                  score > 0.5 ? 1 : 0
                } 1 ${value[0]} ${value[1]} L ${center[0]} ${center[1]} Z`}
                fill="red"
              />
            </>
          );
        })
      : null;
  const renderedClusteredClusterPointsInGround =
    clusteredClusterPointsInGround !== undefined &&
    clusteredClusterPointsInGround !== null
      ? clusteredClusterPointsInGround.map((cluster, clusterIndex) =>
          cluster.map((point) => (
            <circle
              cx={point.point[0]}
              cy={point.point[1]}
              r={0.025}
              fill={COLORS[clusterIndex % COLORS.length]}
            />
          ))
        )
      : null;
  const filteredRobotPositionCircles =
    filteredRobots !== undefined && filteredRobots !== null
      ? filteredRobots.map((robot) => (
          <circle
            cx={robot.position[0]}
            cy={robot.position[1]}
            r="0.1"
            fill="orange"
            stroke="white"
            strokeWidth={0.01}
          />
        ))
      : null;
  const renderedClusterCircles =
    detectedRobots !== undefined && detectedRobots !== null
      ? detectedRobots.robot_positions.map((cluster) => (
          <circle
            cx={cluster.center[0]}
            cy={cluster.center[1]}
            r={cluster.radius}
            fill="none"
            stroke="black"
            strokeWidth={0.01}
          />
        ))
      : null;
  const renderedClusterCones =
    clusterCones !== undefined && clusterCones !== null
      ? clusterCones.map((cone, clusterIndex) => {
          const leftLength = Math.sqrt(
            cone.left[0] * cone.left[0] + cone.left[1] * cone.left[1]
          );
          const left = [
            (cone.left[0] / leftLength) * 10,
            (cone.left[1] / leftLength) * 10,
          ];
          const rightLength = Math.sqrt(
            cone.right[0] * cone.right[0] + cone.right[1] * cone.right[1]
          );
          const right = [
            (cone.right[0] / rightLength) * 10,
            (cone.right[1] / rightLength) * 10,
          ];
          return (
            <>
              <line
                x1={0}
                y1={0}
                x2={left[0]}
                y2={left[1]}
                fill="none"
                stroke={COLORS[clusterIndex % COLORS.length]}
                strokeWidth={0.01}
              />
              <line
                x1={0}
                y1={0}
                x2={right[0]}
                y2={right[1]}
                fill="none"
                stroke={COLORS[clusterIndex % COLORS.length]}
                strokeWidth={0.01}
              />
            </>
          );
        })
      : null;
  const thingsOnTheField =
    robotToField !== undefined && robotToField !== null ? (
      <Transform isometry={robotToField}>
        <circle
          r="0.2"
          fill="yellow"
          fillOpacity="0.4"
          stroke="black"
          strokeWidth="0.01"
        />
        <line x1="0" y1="0" x2="0.3" y2="0" stroke="black" strokeWidth="0.01" />
        {renderedClusteredClusterPointsInGround}
        {renderedClusterCircles}
        {renderedClusterCones}
        {filteredRobotPositionCircles}
      </Transform>
    ) : (
      <>
        {renderedClusteredClusterPointsInGround}
        {renderedClusterCircles}
        {renderedClusterCones}
        {filteredRobotPositionCircles}
      </>
    );
  return (
    <div className="robotDetection">
      <div className="header">
        <div className="panelType">RobotDetection:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      <div className="content">
        {imageUrl !== undefined ? (
          <img className="image" src={imageUrl} alt="" />
        ) : (
          <div className="image">NAO has not sent any image yet</div>
        )}
        <svg className="overlay" viewBox="0 0 640 480">
          {renderedClusterPointsInPixel}
        </svg>
        <Field fieldDimensions={fieldDimensions}>{thingsOnTheField}</Field>
      </div>
    </div>
  );
}
