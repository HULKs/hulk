import { useState } from "react";
import { ConnectionState } from "../Connection/Connection";
import "./Connector.css";

export default function Connector({
  connectionState,
  connect,
  setConnect,
  setWebSocketUrl,
}: {
  connectionState: ConnectionState;
  connect: boolean;
  setConnect: React.Dispatch<React.SetStateAction<boolean>>;
  setWebSocketUrl: React.Dispatch<React.SetStateAction<string>>;
}) {
  const [open, setOpen] = useState(false);
  const [temporaryWebSocketUrl, setTemporaryWebSocketUrl] = useState<string>(
    "ws://localhost:1337"
  );
  return (
    <div className="connector">
      <button
        onClick={() => {
          setOpen(true);
        }}
      >
        Connection ({connectionState})
      </button>
      <div
        className={open ? "modal" : "modal hidden"}
        onClick={(event) => {
          if (event.target === event.currentTarget) {
            setOpen(false);
          }
        }}
      >
        <div className="inner">
          <label>
            <input
              type="checkbox"
              checked={connect}
              onChange={() => setConnect(!connect)}
            />
            Connect
          </label>
          <input
            type="text"
            value={temporaryWebSocketUrl}
            onChange={(event) => setTemporaryWebSocketUrl(event.target.value)}
            onKeyPress={(event) => {
              if (event.key === "Enter") {
                setWebSocketUrl(temporaryWebSocketUrl);
              }
            }}
            onBlur={() => {
              setWebSocketUrl(temporaryWebSocketUrl);
            }}
          />
          <div className="connectionState">{connectionState}</div>
        </div>
      </div>
    </div>
  );
}
