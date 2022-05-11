import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./Horizon.css";

export default function Horizon({
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
  const [horizonData, setHorizonData] = useState<
    { left_horizon_y: number; right_horizon_y: number } | null | undefined
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
    const unsubscribeHorizon = connection.subscribeOutput(
      cycler,
      OutputType.Main,
      "camera_matrix.horizon",
      (data) => {
        setHorizonData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeImage();
      unsubscribeHorizon();
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
  const horizonLine =
    horizonData !== undefined && horizonData !== null ? (
      <line
        x1={0}
        y1={horizonData.left_horizon_y}
        x2={640}
        y2={horizonData.right_horizon_y}
        stroke="#ff0"
        strokeWidth="3"
      />
    ) : null;
  return (
    <div className="horizon">
      <div className="header">
        <div className="panelType">Horizon:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {imageUrl !== undefined ? (
        <>
          <img className="content" src={imageUrl} alt="" />
          <svg className="overlay" viewBox="0 0 640 480">
            {horizonLine}
          </svg>
        </>
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  );
}
