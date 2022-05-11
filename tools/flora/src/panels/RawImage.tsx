import { useEffect, useState } from "react";
import Connection, { Cycler } from "../Connection/Connection";
import "./RawImage.css";

export default function RawImage({
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
  const [data, setData] = useState<Blob | undefined>(undefined);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribe = connection.subscribeImage(
      cycler,
      (data) => {
        setData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return unsubscribe;
  }, [connection, cycler]);
  const [imageUrl, setImageUrl] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (data !== undefined) {
      const imageUrl = URL.createObjectURL(data);
      setImageUrl(imageUrl);
      return () => {
        URL.revokeObjectURL(imageUrl);
      };
    }
  }, [data]);
  return (
    <div className="rawImage">
      <div className="header">
        <div className="panelType">RawImage:</div>
        <div className="cycler">{cycler}</div>
        {selector}
        {connector}
      </div>
      {imageUrl !== undefined ? (
        <img className="content" src={imageUrl} alt="" />
      ) : (
        <div className="content noData">NAO has not sent any data yet</div>
      )}
    </div>
  );
}
