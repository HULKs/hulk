import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./ProjectedFieldLines.css";

export default function ProjectedFieldLines({
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
  const [projectedFieldLines, setProjectedFieldLines] = useState<
    | {
        top: Array<Array<Array<number>>>;
        bottom: Array<Array<Array<number>>>;
      }
    | undefined
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
    const unsubscribeProjectedFieldLines = connection.subscribeOutput(
      Cycler.Control,
      OutputType.Additional,
      "projected_field_lines",
      (data) => {
        setProjectedFieldLines(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return () => {
      unsubscribeImage();
      unsubscribeProjectedFieldLines();
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
  const lines =
    projectedFieldLines !== undefined && projectedFieldLines !== null
      ? (cycler === Cycler.VisionTop
          ? projectedFieldLines.top
          : projectedFieldLines.bottom
        ).map((line, index) => {
          return (
            <line
              key={index}
              x1={line[0][0]}
              y1={line[0][1]}
              x2={line[1][0]}
              y2={line[1][1]}
              stroke="blue"
              strokeWidth="3"
            />
          );
        })
      : null;
  return (
    <div className="projectedFieldLines">
      <div className="header">
        <div className="panelType">ProjectedFieldLines:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {imageUrl !== undefined ? (
        <>
          <img className="content" src={imageUrl} alt="" />
          <svg className="overlay" viewBox="0 0 640 480">
            {lines}
          </svg>
        </>
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  );
}
