import { useEffect, useState } from "react";
import Connection from "../Connection/Connection";
import "./Parameter.css";

export default function Parameter({
  selector,
  connector,
  connection,
  path,
  type,
}: {
  selector: JSX.Element;
  connector: JSX.Element;
  connection: Connection | null;
  path: string;
  type: string;
}) {
  const [data, setData] = useState<string | undefined>(undefined);
  const [previousData, setPreviousData] = useState<string | undefined>(
    undefined
  );
  const [textAreaValue, setTextAreaValue] = useState("");
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribe = connection.subscribeParameter(
      path,
      (data) => {
        setData(JSON.stringify(data, null, 2));
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return unsubscribe;
  }, [connection, path]);
  useEffect(() => {
    if (data !== previousData) {
      if (previousData === textAreaValue || textAreaValue === "") {
        setTextAreaValue(data!);
      } else {
        alert(
          "Parameter updated while you edited.\nEither reset or update your changes."
        );
      }
      setPreviousData(data);
    }
  }, [data, previousData, textAreaValue]);
  const content =
    data === undefined ? (
      <div className="content noData">NAO has not sent any data yet</div>
    ) : (
      <textarea
        className="content"
        value={textAreaValue}
        onChange={(event) => {
          setTextAreaValue(event.target.value);
        }}
      />
    );
  return (
    <div className="parameter">
      <div className="header">
        <div className="panelType">Parameter:</div>
        <div className="path">{path}</div>
        <div className="type">{type}</div>
        <button
          className="reset"
          disabled={data === undefined || textAreaValue === data}
          onClick={() => {
            setTextAreaValue(data!);
          }}
        >
          Reset
        </button>
        <button
          className="update"
          disabled={data === undefined || textAreaValue === data}
          onClick={() => {
            if (connection === null) {
              return;
            }
            connection.updateParameter(
              path,
              JSON.parse(textAreaValue),
              () => {},
              () => {}
            );
            setData(textAreaValue);
            setPreviousData(textAreaValue);
          }}
        >
          Update
        </button>
        {selector}
        {connector}
      </div>
      {content}
    </div>
  );
}
