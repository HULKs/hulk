import React, { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./BallFilter.css";

export default function BallFilter({
  selector,
  connector,
  connection,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
}) {
  const [ballFilterHypotheses, setBallFilterHypotheses] = useState<
    | {
        filter: {
          state: number[];
          covariance: number[];
        };
        validity: number;
        last_update: {
          nanos_since_epoch: number;
          secs_since_epoch: number;
        };
      }[]
    | undefined
  >(undefined);

  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribeBallFilter = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Additional,
      "ball_filter_hypotheses",
      (ball_filter_hypotheses) => {
        setBallFilterHypotheses(ball_filter_hypotheses);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );

    return () => {
      unsubscribeBallFilter();
    };
  }, [connection]);
  const header = (
    <div className="header">
      <div className="panelType">BallFilter</div>
      <div className="outputType"></div>
      <div className="path"></div>
      <div className="type"></div>
      {selector}
      {connector}
    </div>
  );
  let ballHypotheses = undefined;
  let content = undefined;
  if (ballFilterHypotheses === undefined) {
    content = (
      <div className="content noData">NAO has not sent all data yet</div>
    );
  } else {
    ballHypotheses = ballFilterHypotheses.map((hypothesis) => {
      let positionCovariance = [
        hypothesis.filter.covariance[0],
        hypothesis.filter.covariance[1],
        hypothesis.filter.covariance[4],
        hypothesis.filter.covariance[5],
      ];
      return (
        <g
          transform={`translate(${hypothesis.filter.state[0]},${hypothesis.filter.state[1]}) `}
        >
          {ellipseFromCovariance(positionCovariance)}
          <circle r="0.08" fill="white" stroke="black" strokeWidth="0.01" />
          <line
            x1="0"
            y1="0"
            x2={hypothesis.filter.state[2]}
            y2={hypothesis.filter.state[3]}
            stroke="red"
            strokeWidth="0.03"
            marker-end="url(#triangle)"
          />
        </g>
      );
    });
    content = (
      <svg className="overlay" viewBox="-4 -4 8 8">
        <defs>
          <marker
            id="triangle"
            viewBox="0 0 10 10"
            refX="1"
            refY="5"
            markerUnits="strokeWidth"
            markerWidth="3"
            markerHeight="3"
            orient="auto"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="#f00" />
          </marker>
        </defs>
        <g transform="scale(1,-1)rotate(90)">
          <line
            x1="-2"
            y1="0"
            x2="2"
            y2="0"
            stroke="black"
            strokeWidth="0.01"
          />
          <line
            x1="0"
            y1="-2"
            x2="0"
            y2="2"
            stroke="black"
            strokeWidth="0.01"
          />
          {ballHypotheses}
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
