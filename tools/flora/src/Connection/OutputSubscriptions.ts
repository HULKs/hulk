import { Cycler, OutputType } from "./Connection";

export type DataCallback = (data: any) => void;
export type ErrorCallback = (error: any) => void;
type Paths = {
  [path: string]: {
    [id: number]: [DataCallback, ErrorCallback];
  };
};
type OutputTypes = {
  [outputType in OutputType]?: Paths;
};
type Subscriptions = {
  [cycler in Cycler]?: OutputTypes;
};

export default class OutputSubscriptions {
  private subscriptions: Subscriptions = {};

  register(
    cycler: Cycler,
    outputType: OutputType,
    path: string,
    id: number,
    dataCallback: DataCallback,
    errorCallback: ErrorCallback
  ) {
    let firstRegisteredOutputForPath = false;
    if (this.subscriptions[cycler] === undefined) {
      this.subscriptions[cycler] = {};
    }
    if (this.subscriptions[cycler]![outputType] === undefined) {
      this.subscriptions[cycler]![outputType] = {};
    }
    if (this.subscriptions[cycler]![outputType]![path] === undefined) {
      firstRegisteredOutputForPath = true;
      this.subscriptions[cycler]![outputType]![path] = {};
    }
    this.subscriptions[cycler]![outputType]![path][id] = [
      dataCallback,
      errorCallback,
    ];
    return firstRegisteredOutputForPath;
  }

  unregister(cycler: Cycler, outputType: OutputType, path: string, id: number) {
    let lastRegisteredOutputForPath = false;
    if (
      this.subscriptions[cycler] === undefined ||
      this.subscriptions[cycler]![outputType] === undefined ||
      this.subscriptions[cycler]![outputType]![path] === undefined
    ) {
      return;
    }
    delete this.subscriptions[cycler]![outputType]![path][id];
    if (
      Object.keys(this.subscriptions[cycler]![outputType]![path]).length === 0
    ) {
      lastRegisteredOutputForPath = true;
      delete this.subscriptions[cycler]![outputType]![path];
    }
    if (Object.keys(this.subscriptions[cycler]![outputType]!).length === 0) {
      delete this.subscriptions[cycler]![outputType];
    }
    if (Object.keys(this.subscriptions[cycler]!).length === 0) {
      delete this.subscriptions[cycler];
    }
    return lastRegisteredOutputForPath;
  }

  callDataCallbacks(
    cycler: Cycler,
    outputType: OutputType,
    path: string,
    data: any
  ) {
    if (
      this.subscriptions[cycler] === undefined ||
      this.subscriptions[cycler]![outputType] === undefined ||
      this.subscriptions[cycler]![outputType]![path] === undefined
    ) {
      return;
    }
    for (const callbacks of Object.values(
      this.subscriptions[cycler]![outputType]![path]
    )) {
      callbacks[0](data);
    }
  }

  callErrorCallbacks(
    cycler: Cycler,
    outputType: OutputType,
    path: string,
    error: any
  ) {
    if (
      this.subscriptions[cycler] === undefined ||
      this.subscriptions[cycler]![outputType] === undefined ||
      this.subscriptions[cycler]![outputType]![path] === undefined
    ) {
      return;
    }
    for (const callbacks of Object.values(
      this.subscriptions[cycler]![outputType]![path]
    )) {
      callbacks[1](error);
    }
  }

  forEachRegistered(
    callback: (cycler: Cycler, outputType: OutputType, path: string) => void
  ) {
    for (const [cycler, outputTypes] of Object.entries(this.subscriptions) as [
      Cycler,
      OutputTypes
    ][]) {
      for (const [outputType, paths] of Object.entries(outputTypes) as [
        OutputType,
        Paths
      ][]) {
        for (const path of Object.keys(paths)) {
          callback(cycler, outputType, path);
        }
      }
    }
  }
}

// function test_ConnectionOutputSubscriptions_onlyFirstRegistrationAndLastUnregistrationTriggerChange() {
//   const subscriptions = new OutputSubscriptions();
//   console.assert(
//     subscriptions.register(
//       Cycler.Control,
//       OutputType.Main,
//       "path",
//       0,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     !subscriptions.register(
//       Cycler.Control,
//       OutputType.Main,
//       "path",
//       1,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     subscriptions.register(
//       Cycler.Control,
//       OutputType.Main,
//       "path2",
//       2,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     !subscriptions.register(
//       Cycler.Control,
//       OutputType.Main,
//       "path",
//       3,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     !subscriptions.unregister(Cycler.Control, OutputType.Main, "path", 0)
//   );
//   console.assert(
//     !subscriptions.unregister(Cycler.Control, OutputType.Main, "path", 1)
//   );
//   console.assert(
//     subscriptions.unregister(Cycler.Control, OutputType.Main, "path2", 2)
//   );
//   console.assert(
//     subscriptions.unregister(Cycler.Control, OutputType.Main, "path", 3)
//   );
// }

// function test_ConnectionOutputSubscriptions_allRegisteredCallbacksAreCalled() {
//   const subscriptions = new OutputSubscriptions();
//   let dataCallbackCounter = 0;
//   let errorCallbackCounter = 0;
//   const dataCallback = () => {
//     dataCallbackCounter += 1;
//   };
//   const errorCallback = () => {
//     errorCallbackCounter += 1;
//   };
//   subscriptions.register(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     0,
//     dataCallback,
//     errorCallback
//   );
//   subscriptions.register(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     1,
//     dataCallback,
//     errorCallback
//   );
//   subscriptions.register(
//     Cycler.Control,
//     OutputType.Main,
//     "path2",
//     2,
//     dataCallback,
//     errorCallback
//   );
//   console.assert(dataCallbackCounter === 0 && errorCallbackCounter === 0);
//   subscriptions.callDataCallbacks(Cycler.Control, OutputType.Main, "path", 1);
//   console.assert(dataCallbackCounter === 2 && errorCallbackCounter === 0);
//   subscriptions.callDataCallbacks(Cycler.Control, OutputType.Main, "path2", 1);
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 0);
//   subscriptions.callErrorCallbacks(Cycler.Control, OutputType.Main, "path", 1);
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 2);
//   subscriptions.callErrorCallbacks(Cycler.Control, OutputType.Main, "path2", 1);
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 3);
//   subscriptions.callDataCallbacks(
//     "nonExistingCycler" as Cycler,
//     OutputType.Main,
//     "path2",
//     1
//   );
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 3);
//   subscriptions.callErrorCallbacks(
//     "nonExistingCycler" as Cycler,
//     OutputType.Main,
//     "path2",
//     1
//   );
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 3);
//   subscriptions.callDataCallbacks(
//     Cycler.Control,
//     "nonExistingOutputType" as OutputType,
//     "path2",
//     1
//   );
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 3);
//   subscriptions.callErrorCallbacks(
//     Cycler.Control,
//     "nonExistingOutputType" as OutputType,
//     "path2",
//     1
//   );
//   console.assert(dataCallbackCounter === 3 && errorCallbackCounter === 3);
// }

// function test_ConnectionOutputSubscriptions_allRegistrationsAreIterated() {
//   const subscriptions = new OutputSubscriptions();
//   subscriptions.register(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     0,
//     () => {},
//     () => {}
//   );
//   subscriptions.register(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     1,
//     () => {},
//     () => {}
//   );
//   subscriptions.register(
//     Cycler.Control,
//     OutputType.Main,
//     "path2",
//     2,
//     () => {},
//     () => {}
//   );
//   const pendingSubscriptions = new Set([
//     "cycler.outputType.path",
//     "cycler.outputType.path2",
//   ]);
//   let unexpectedDeletion = false;
//   subscriptions.forEachRegistered((cycler, outputType, path) => {
//     if (!pendingSubscriptions.delete(`${cycler}.${outputType}.${path}`)) {
//       unexpectedDeletion = true;
//     }
//   });
//   console.assert(pendingSubscriptions.size === 0 && !unexpectedDeletion);
// }

// function test_ConnectionOutputSubscriptions() {
//   test_ConnectionOutputSubscriptions_onlyFirstRegistrationAndLastUnregistrationTriggerChange();
//   test_ConnectionOutputSubscriptions_allRegisteredCallbacksAreCalled();
//   test_ConnectionOutputSubscriptions_allRegistrationsAreIterated();
// }
