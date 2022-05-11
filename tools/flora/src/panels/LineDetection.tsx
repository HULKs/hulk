import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./LineDetection.css";

export default function LineDetection({
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
  const [lineData, setLineData] = useState<{
    lines: Array<Array<Array<number>>>;
    points: Array<Array<number>>;
  } | undefined
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
    const unsubscribeLineData = connection.subscribeOutput(
      cycler,
      OutputType.Additional,
      "lines_in_image",
      (data) => {
        setLineData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeImage();
      unsubscribeLineData();
    };
  }, [connection, cycler]);
  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (imageData !== undefined) {
      const imageUrl = URL.createObjectURL(imageData);
      setImageUrl(imageUrl);
      return () => {
        URL.revokeObjectURL(imageUrl);
      }
    }
  }, [imageData]);
  const points =
    lineData !== undefined && lineData !== null ? (
      lineData.points.map((point, index) => {
        return (
          <circle
            key={index}
            cx={point[0] * 2}
            cy={point[1]}
            r="3"
            stroke="red"
            strokeWidth="3"
          />)
      })
    ) : null;
  const lines =
    lineData !== undefined && lineData !== null ? (
      lineData.lines.map((line, index) => {
        return (
          <line
            key={index}
            x1={line[0][0] * 2}
            y1={line[0][1]}
            x2={line[1][0] * 2}
            y2={line[1][1]}
            stroke="blue"
            strokeWidth="3"
          />)
      })
    ) : null;
  return (
    <div className="lineDetection">
      <div className="header">
        <div className="panelType">LineDetection:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {imageUrl !== undefined ? (
        <>
          <img className="content" src={imageUrl} alt="" />
          <svg className="overlay" viewBox="0 0 640 480">
            {points}
            {lines}
          </svg>
        </>
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  )
}
