import { Cycler } from "./Connection";

export type DataCallback = (data: Blob) => void;
export type ErrorCallback = (error: any) => void;
type Subscriptions = {
  [cycler in Cycler]?: {
    [id: number]: [DataCallback, ErrorCallback];
  };
};

export default class ImageSubscriptions {
  private subscriptions: Subscriptions = {};

  register(
    cycler: Cycler,
    id: number,
    dataCallback: DataCallback,
    errorCallback: ErrorCallback
  ) {
    let firstRegisteredOutputForImage = false;
    if (this.subscriptions[cycler] === undefined) {
      firstRegisteredOutputForImage = true;
      this.subscriptions[cycler] = {};
    }
    this.subscriptions[cycler]![id] = [dataCallback, errorCallback];
    return firstRegisteredOutputForImage;
  }

  unregister(cycler: Cycler, id: number) {
    let lastRegisteredOutputForImage = false;
    if (this.subscriptions[cycler] === undefined) {
      return;
    }
    delete this.subscriptions[cycler]![id];
    if (Object.keys(this.subscriptions[cycler]!).length === 0) {
      lastRegisteredOutputForImage = true;
      delete this.subscriptions[cycler];
    }
    return lastRegisteredOutputForImage;
  }

  callDataCallbacks(cycler: Cycler, data: Blob) {
    if (this.subscriptions[cycler] === undefined) {
      return;
    }
    for (const callbacks of Object.values(this.subscriptions[cycler]!)) {
      callbacks[0](data);
    }
  }

  callErrorCallbacks(cycler: Cycler, error: any) {
    if (this.subscriptions[cycler] === undefined) {
      return;
    }
    for (const callbacks of Object.values(this.subscriptions[cycler]!)) {
      callbacks[1](error);
    }
  }

  forEachRegistered(callback: (cycler: Cycler) => void) {
    for (const cycler of Object.keys(this.subscriptions) as Cycler[]) {
      callback(cycler);
    }
  }
}

// function test_ConnectionImageSubscriptions_onlyFirstRegistrationAndLastUnregistrationTriggerChange() {
//   const subscriptions = new ImageSubscriptions();
//   console.assert(
//     subscriptions.register(
//       Cycler.Control,
//       0,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     !subscriptions.register(
//       Cycler.Control,
//       1,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     subscriptions.register(
//       Cycler.VisionTop,
//       2,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(
//     !subscriptions.register(
//       Cycler.Control,
//       3,
//       () => {},
//       () => {}
//     )
//   );
//   console.assert(!subscriptions.unregister(Cycler.Control, 0));
//   console.assert(!subscriptions.unregister(Cycler.Control, 1));
//   console.assert(subscriptions.unregister(Cycler.VisionTop, 2));
//   console.assert(subscriptions.unregister(Cycler.Control, 3));
// }

// function test_ConnectionImageSubscriptions_allRegisteredCallbacksAreCalled() {
//   const subscriptions = new ImageSubscriptions();
//   let dataCallbackCounter = 0;
//   let errorCallbackCounter = 0;
//   const dataCallback = () => {
//     dataCallbackCounter += 1;
//   };
//   const errorCallback = () => {
//     errorCallbackCounter += 1;
//   };
//   subscriptions.register(Cycler.Control, 0, dataCallback, errorCallback);
//   subscriptions.register(Cycler.VisionTop, 1, dataCallback, errorCallback);
//   console.assert(dataCallbackCounter === 0 && errorCallbackCounter === 0);
//   subscriptions.callDataCallbacks(Cycler.Control, new Blob([]));
//   console.assert(dataCallbackCounter === 1 && errorCallbackCounter === 0);
//   subscriptions.callDataCallbacks(Cycler.VisionTop, new Blob([]));
//   console.assert(dataCallbackCounter === 2 && errorCallbackCounter === 0);
//   subscriptions.callErrorCallbacks(Cycler.Control, new Blob([]));
//   console.assert(dataCallbackCounter === 2 && errorCallbackCounter === 1);
//   subscriptions.callErrorCallbacks(Cycler.VisionTop, new Blob([]));
//   console.assert(dataCallbackCounter === 2 && errorCallbackCounter === 2);
//   subscriptions.callDataCallbacks("nonExistingCycler" as Cycler, new Blob([]));
//   console.assert(dataCallbackCounter === 2 && errorCallbackCounter === 2);
//   subscriptions.callErrorCallbacks("nonExistingCycler" as Cycler, new Blob([]));
//   console.assert(dataCallbackCounter === 2 && errorCallbackCounter === 2);
// }

// function test_ConnectionImageSubscriptions_allRegistrationsAreIterated() {
//   let subscriptions = new ImageSubscriptions();
//   subscriptions.register(
//     Cycler.Control,
//     0,
//     () => {},
//     () => {}
//   );
//   subscriptions.register(
//     Cycler.Control,
//     1,
//     () => {},
//     () => {}
//   );
//   subscriptions.register(
//     Cycler.VisionTop,
//     2,
//     () => {},
//     () => {}
//   );
//   let pendingSubscriptions = new Set([Cycler.Control, Cycler.VisionTop]);
//   let unexpectedDeletion = false;
//   subscriptions.forEachRegistered((cycler) => {
//     if (!pendingSubscriptions.delete(cycler)) {
//       unexpectedDeletion = true;
//     }
//   });
//   if (pendingSubscriptions.size !== 0 || unexpectedDeletion) {
//     throw `pendingSubscriptions.size = ${pendingSubscriptions.size}, unexpectedDeletion = ${unexpectedDeletion}`;
//   }
// }

// function test_ConnectionImageSubscriptions() {
//   test_ConnectionImageSubscriptions_onlyFirstRegistrationAndLastUnregistrationTriggerChange();
//   test_ConnectionImageSubscriptions_allRegisteredCallbacksAreCalled();
//   test_ConnectionImageSubscriptions_allRegistrationsAreIterated();
// }
