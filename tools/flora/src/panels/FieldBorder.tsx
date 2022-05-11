import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./FieldBorder.css";

export default function FieldBorder({
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
  const [fieldBorder, setFieldBorder] = useState<{
    border_lines: Array<Array<Array<number>>>;
  } | undefined
  >(undefined);
  const [borderPoints, setBorderPoints] = useState<Array<Array<number>>| undefined>(undefined);
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
    const unsubscribeFieldBorder = connection.subscribeOutput(
      cycler,
      OutputType.Main,
      "field_border",
      (data) => {
        setFieldBorder(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    const unsubscribeBorderPoints = connection.subscribeOutput(
      cycler,
      OutputType.Additional,
      "field_border_points",
      (data) => {
        setBorderPoints(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeImage();
      unsubscribeFieldBorder();
      unsubscribeBorderPoints();
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
  const lines =
    fieldBorder !== undefined && fieldBorder !== null ? (
      fieldBorder.border_lines.map((line, index) => {
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
  const points =
    borderPoints !== undefined && borderPoints !== null ? (
      borderPoints.map((point, index) => {
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
  return (
    <div className="fieldBorder">
      <div className="header">
        <div className="panelType">FieldBorder:</div>
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
