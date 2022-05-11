import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "../Connection/Connection";
import "./RawOutput.css";

export default function RawOutput({
  selector,
  connector,
  connection,
  cycler,
  outputType,
  path,
  type,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
  cycler: Cycler;
  outputType: OutputType;
  path: string;
  type: string;
}) {
  const [data, setData] = useState(undefined);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribe = connection.subscribeOutput(
      cycler,
      outputType,
      path,
      (data) => {
        setData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return unsubscribe;
  }, [connection, cycler, outputType, path]);
  const content =
    data === undefined ? (
      <div className="content noData">NAO has not sent any data yet</div>
    ) : (
      <pre className="content">{JSON.stringify(data, null, 2)}</pre>
    );
  return (
    <div className="rawOutput">
      <div className="header">
        <div className="panelType">RawOutput:</div>
        <div className="cycler">{cycler}:</div>
        <div className="outputType">{outputType}:</div>
        <div className="path">{path}</div>
        <div className="type">{type}</div>
        {selector}
        {connector}
      </div>
      {content}
    </div>
  );
}
