export type DataCallback = (data: any) => void;
export type ErrorCallback = (error: any) => void;
type Subscriptions = {
  [path: string]: {
    [id: number]: [DataCallback, ErrorCallback];
  };
};

export default class ParameterSubscriptions {
  private subscriptions: Subscriptions = {};

  register(
    path: string,
    id: number,
    dataCallback: DataCallback,
    errorCallback: ErrorCallback
  ) {
    let firstRegisteredPathForParameter = false;
    if (this.subscriptions[path] === undefined) {
      firstRegisteredPathForParameter = true;
      this.subscriptions[path] = {};
    }
    this.subscriptions[path]![id] = [dataCallback, errorCallback];
    return firstRegisteredPathForParameter;
  }

  unregister(path: string, id: number) {
    let lastRegisteredPathForParameter = false;
    if (this.subscriptions[path] === undefined) {
      return;
    }
    delete this.subscriptions[path]![id];
    if (Object.keys(this.subscriptions[path]!).length === 0) {
      lastRegisteredPathForParameter = true;
      delete this.subscriptions[path];
    }
    return lastRegisteredPathForParameter;
  }

  callDataCallbacks(path: string, data: Blob) {
    if (this.subscriptions[path] === undefined) {
      return;
    }
    for (const callbacks of Object.values(this.subscriptions[path]!)) {
      callbacks[0](data);
    }
  }

  callErrorCallbacks(path: string, error: any) {
    if (this.subscriptions[path] === undefined) {
      return;
    }
    for (const callbacks of Object.values(this.subscriptions[path]!)) {
      callbacks[1](error);
    }
  }

  forEachRegistered(callback: (path: string) => void) {
    for (const cycler of Object.keys(this.subscriptions) as string[]) {
      callback(cycler);
    }
  }
}
