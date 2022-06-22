import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./ProjectedLimbs.css";

export default function ProjectedLimbs({
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
  const [imageData, setImageData] = useState<Blob | undefined>(undefined);
  const [projectedLimbsData, setProjectedLimbsData] = useState<
    { pixel_polygon: number[][] }[] | null | undefined
  >(undefined);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribeImage = connection.subscribeImage(
      cycler,
      (data) => {
        setImageData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeProjectedLimbs = connection.subscribeOutput(
      cycler,
      OutputType.Main,
      "projected_limbs",
      (data) => {
        setProjectedLimbsData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeImage();
      unsubscribeProjectedLimbs();
    };
  }, [connection, cycler]);
  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (imageData !== undefined) {
      const imageUrl = URL.createObjectURL(imageData);
      setImageUrl(imageUrl);
      return () => {
        URL.revokeObjectURL(imageUrl);
      };
    }
  }, [imageData]);
  const projectedLimbs =
    projectedLimbsData !== undefined && projectedLimbsData !== null
      ? projectedLimbsData.map(drawLimb)
      : null;
  return (
    <div className="projectedLimbs">
      <div className="header">
        <div className="panelType">ProjectedLimbs:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {imageUrl !== undefined ? (
        <>
          <img className="content" src={imageUrl} alt="" />
          <svg className="overlay" viewBox="0 0 640 480">
            {projectedLimbs}
          </svg>
        </>
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  );
}

function drawLimb(limb: { pixel_polygon: number[][] }) {
  const lines = [];
  for (let i = 0; i < limb.pixel_polygon.length - 1; i++) {
    lines.push(
      <line
        x1={limb.pixel_polygon[i][0]}
        y1={limb.pixel_polygon[i][1]}
        x2={limb.pixel_polygon[i + 1][0]}
        y2={limb.pixel_polygon[i + 1][1]}
        stroke="#f0f"
        strokeWidth={3}
      />
    );
  }
  return (
    <>
      {lines}
      {limb.pixel_polygon.map((point, index) => (
        <>
          <circle cx={point[0]} cy={point[1]} r={10} fill="#f0f" />{" "}
          <text x={point[0]} y={point[1]}>
            {index}
          </text>
        </>
      ))}
    </>
  );
}
