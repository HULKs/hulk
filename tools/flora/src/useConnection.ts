import React, { useEffect, useState } from "react";
import Connection, {
  ConnectionState,
  Cycler,
  OutputHierarchy,
  OutputType,
  ParameterHierarchy,
} from "./Connection/Connection";

export default function useConnection(): [
  boolean,
  React.Dispatch<React.SetStateAction<boolean>>,
  string,
  React.Dispatch<React.SetStateAction<string>>,
  OutputHierarchy,
  ParameterHierarchy,
  ConnectionState,
  Connection | null
] {
  const [connect, setConnect] = useState<boolean>(true);
  const [webSocketUrl, setWebSocketUrl] = useState<string>(
    "ws://localhost:1337"
  );
  const [outputHierarchy, setOutputHierarchy] = useState<OutputHierarchy>({
    [Cycler.Audio]: {
      [OutputType.Main]: {},
      [OutputType.Additional]: {},
    },
    [Cycler.Control]: {
      [OutputType.Main]: {},
      [OutputType.Additional]: {},
    },
    [Cycler.SplNetwork]: {
      [OutputType.Main]: {},
      [OutputType.Additional]: {},
    },
    [Cycler.VisionTop]: {
      [OutputType.Main]: {},
      [OutputType.Additional]: {},
    },
    [Cycler.VisionBottom]: {
      [OutputType.Main]: {},
      [OutputType.Additional]: {},
    },
  });
  const [parameterHierarchy, setParameterHierarchy] =
    useState<ParameterHierarchy>({});
  const [connectionState, setConnectionState] = useState<ConnectionState>(
    ConnectionState.Disconnected
  );
  const [connection, setConnection] = useState<Connection | null>(null);
  useEffect(() => {
    const connection = new Connection();
    connection.onOutputHierarchyChange = (outputHierarchy) => {
      setOutputHierarchy(outputHierarchy);
    };
    connection.onParameterHierarchyChange = (parameterHierarchy) => {
      setParameterHierarchy(parameterHierarchy);
    };
    connection.onStateChange = (connectionState) => {
      setConnectionState(connectionState);
    };
    setConnection(connection);

    return () => {
      connection.connect = false;
    };
  }, []);
  useEffect(() => {
    if (connection) {
      connection.connect = connect;
    }
  }, [connection, connect]);
  useEffect(() => {
    if (connection) {
      connection.webSocketUrl = webSocketUrl;
    }
  }, [connection, webSocketUrl]);

  return [
    connect,
    setConnect,
    webSocketUrl,
    setWebSocketUrl,
    outputHierarchy,
    parameterHierarchy,
    connectionState,
    connection,
  ];
}
