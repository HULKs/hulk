import { useEffect, useState } from "react";
import Connection, { Cycler, OutputType } from "./Connection/Connection";

export function useOutputSubscription<T>(
  connection: Connection | null,
  cycler: Cycler,
  outputType: OutputType,
  path: string
): T | undefined {
  const [data, setData] = useState<T | undefined>(undefined);
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
  return data;
}

export function useImageSubscription(
  connection: Connection | null,
  cycler: Cycler
): string | undefined {
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
  const [url, setUrl] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (data !== undefined) {
      const url = URL.createObjectURL(data);
      setUrl(url);
      return () => {
        URL.revokeObjectURL(url);
      };
    }
  }, [data]);
  return url;
}

export function useParameterSubscription<T>(
  connection: Connection | null,
  path: string
): T | undefined {
  const [data, setData] = useState<T | undefined>(undefined);
  useEffect(() => {
    if (connection === null) {
      return;
    }
    const unsubscribe = connection.subscribeParameter(
      path,
      (data) => {
        setData(data);
      },
      (error) => {
        alert(`Error: ${error}`);
      }
    );
    return unsubscribe;
  }, [connection, path]);
  return data;
}
