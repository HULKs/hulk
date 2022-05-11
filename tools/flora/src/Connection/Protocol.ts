import ImageSubscriptions from "./ImageSubscriptions";
import OutputSubscriptions from "./OutputSubscriptions";
import {
  ConnectionState,
  Cycler,
  OutputType,
  OutputHierarchy as ConnectionOutputHierarchy,
  Paths,
  ParameterHierarchy,
} from "./Connection";
import { WebSocketState } from "./StateMachine";
import {
  DataCallback as ParameterDataCallback,
  ErrorCallback as ParameterErrorCallback,
} from "./ParameterSubscriptions";
import {
  DataCallback as ImageDataCallback,
  ErrorCallback as ImageErrorCallback,
} from "./ImageSubscriptions";
import {
  DataCallback as OutputDataCallback,
  ErrorCallback as OutputErrorCallback,
} from "./OutputSubscriptions";
import ParameterSubscriptions from "./ParameterSubscriptions";

type SendCallback = (message: string) => void;
export enum ProtocolState {
  Disconnected = "Disconnected",
  GettingHierarchies = "GettingHierarchies",
  Subscribing = "Subscribing",
  Subscribed = "Subscribed",
}
type PendingRequestResponse = {
  id: number;
  type: string;
  ok: boolean;
  reason?: string;
  [key: string]: any;
};
type PendingRequestCallback = (response: PendingRequestResponse) => void;
type PendingRequests = {
  [id: number]: PendingRequestCallback;
};
type HierarchyType =
  | {
      type: "Primary";
      name: string;
    }
  | {
      type: "Struct";
      fields: {
        [name: string]: HierarchyType;
      };
    }
  | {
      type: "GenericStruct";
    }
  | {
      type: "Option";
      nested: HierarchyType;
    }
  | {
      type: "Vec";
      nested: HierarchyType;
    };
type CyclerOutputsHierarchy = {
  main: HierarchyType;
  additional: HierarchyType;
};
type OutputHierarchy = {
  audio: CyclerOutputsHierarchy;
  control: CyclerOutputsHierarchy;
  spl_network: CyclerOutputsHierarchy;
  vision_top: CyclerOutputsHierarchy;
  vision_bottom: CyclerOutputsHierarchy;
};
type OutputSubscription = {
  subscribe: boolean;
  cycler: Cycler;
  outputType: OutputType;
  path: string;
};
type ImageSubscription = {
  subscribe: boolean;
  cycler: Cycler;
};
type ParameterSubscription = {
  subscribe: boolean;
  path: string;
};
export type UnsubscribeOutputCallback = () => void;
export type UnsubscribeImageCallback = () => void;
export type UnsubscribeParameterCallback = () => void;
export type ParameterSuccessCallback = () => void;

export default class Protocol {
  private _sendCallback: SendCallback;
  private _nextPendingRequestId = 0;
  private _pendingRequests: PendingRequests = {};
  private _nextSubscriptionId = 0;
  private _pendingOutputSubscriptions: OutputSubscription[] = [];
  private _pendingImageSubscriptions: ImageSubscription[] = [];
  private _pendingParameterSubscriptions: ParameterSubscription[] = [];
  private _outputSubscriptions: OutputSubscriptions;
  private _outputHierarchy: ConnectionOutputHierarchy = {
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
  };
  onOutputHierarchyChange:
    | ((outputHierarchy: ConnectionOutputHierarchy) => void)
    | null = null;
  private _imageSubscriptions: ImageSubscriptions;
  private _pendingImages: { [imageId: number]: Blob } = {};
  private _cyclersForPendingImages: { [imageId: number]: Cycler } = {};
  private _parameterHierarchy: Paths = {};
  onParameterHierarchyChange:
    | ((parameterHierarchy: ParameterHierarchy) => void)
    | null = null;
  private _parameterSubscriptions: ParameterSubscriptions;
  private _webSocketState: WebSocketState | null = null;
  private _protocolState = ProtocolState.Disconnected;
  onStateChange: ((state: ProtocolState) => void) | null = null;

  constructor(sendCallback: SendCallback) {
    this._sendCallback = sendCallback;
    this._outputSubscriptions = new OutputSubscriptions();
    this._imageSubscriptions = new ImageSubscriptions();
    this._parameterSubscriptions = new ParameterSubscriptions();
  }

  simplifyState(state: ProtocolState): ConnectionState {
    return {
      [ProtocolState.Disconnected]: ConnectionState.Disconnected,
      [ProtocolState.GettingHierarchies]: ConnectionState.Connecting,
      [ProtocolState.Subscribing]: ConnectionState.Connected,
      [ProtocolState.Subscribed]: ConnectionState.Connected,
    }[state];
  }

  private emitStateChange() {
    if (typeof this.onStateChange === "function") {
      this.onStateChange(this._protocolState);
    }
  }

  get state(): ProtocolState {
    return this._protocolState;
  }

  set webSocketState(webSocketState: WebSocketState) {
    let lastWebSocketState = this._webSocketState;
    this._webSocketState = webSocketState;

    if (lastWebSocketState !== this._webSocketState) {
      if (this._webSocketState === "Connected") {
        this._protocolState = ProtocolState.GettingHierarchies;
        this.emitStateChange();
        const outputHierarchyRequestId = this._nextPendingRequestId++;
        const parameterHierarchyRequestId = this._nextPendingRequestId++;
        this._pendingRequests = {
          [outputHierarchyRequestId]: (response) => {
            delete this._pendingRequests[outputHierarchyRequestId];

            if (!response.ok) {
              console.error(
                `Failed to get output hierarchy: ${response.reason}`
              );
              alert(`Failed to get output hierarchy: ${response.reason}`);
              return;
            }

            this._outputHierarchy = Object.fromEntries(
              Object.entries(response.output_hierarchy as OutputHierarchy).map(
                ([cycler, outputTypes]) => [
                  {
                    audio: Cycler.Audio,
                    control: Cycler.Control,
                    spl_network: Cycler.SplNetwork,
                    vision_top: Cycler.VisionTop,
                    vision_bottom: Cycler.VisionBottom,
                  }[cycler],
                  Object.fromEntries(
                    Object.entries(outputTypes).map(([outputType, paths]) => [
                      {
                        main: OutputType.Main,
                        additional: OutputType.Additional,
                      }[outputType],
                      this.getPathsFromType("", paths),
                    ])
                  ),
                ]
              )
            );

            if (typeof this.onOutputHierarchyChange === "function") {
              this.onOutputHierarchyChange(this._outputHierarchy);
            }

            if (Object.keys(this._pendingRequests).length === 0) {
              this.generatePendingSubscriptions();
              this.subscribeAndUnsubscribe();
            }
          },
          [parameterHierarchyRequestId]: (response) => {
            delete this._pendingRequests[parameterHierarchyRequestId];

            if (!response.ok) {
              console.error(
                `Failed to get parameter hierarchy: ${response.reason}`
              );
              alert(`Failed to get parameter hierarchy: ${response.reason}`);
              return;
            }

            this._parameterHierarchy = this.getPathsFromType(
              "",
              response.parameter_hierarchy
            );

            if (typeof this.onParameterHierarchyChange === "function") {
              this.onParameterHierarchyChange(this._parameterHierarchy);
            }

            if (Object.keys(this._pendingRequests).length === 0) {
              this.generatePendingSubscriptions();
              this.subscribeAndUnsubscribe();
            }
          },
        };

        this._sendCallback(
          JSON.stringify({
            id: outputHierarchyRequestId,
            type: "GetOutputHierarchy",
          })
        );
        this._sendCallback(
          JSON.stringify({
            id: parameterHierarchyRequestId,
            type: "GetParameterHierarchy",
          })
        );
      } else {
        this._protocolState = ProtocolState.Disconnected;
        this.emitStateChange();
        this._pendingRequests = {};
      }
    }
  }

  private getPathsFromType(
    prefix: string,
    type: HierarchyType
  ): { [path: string]: string } {
    switch (type.type) {
      case "Primary": {
        return {
          [prefix]: type.name,
        };
      }
      case "Struct": {
        let paths =
          prefix.length === 0
            ? {}
            : {
                [prefix]: "GenericStruct",
              };
        for (let [name, fieldType] of Object.entries(type.fields)) {
          let fieldPrefix = prefix.length === 0 ? name : `${prefix}.${name}`;
          paths = {
            ...paths,
            ...this.getPathsFromType(fieldPrefix, fieldType),
          };
        }
        return paths;
      }
      case "GenericStruct": {
        return {
          [prefix]: "GenericStruct",
        };
      }
      case "Option": {
        return Object.fromEntries(
          Object.entries(this.getPathsFromType(prefix, type.nested)).map(
            ([path, type]) => [path, `Option<${type}>`]
          )
        );
      }
      case "Vec": {
        return Object.fromEntries(
          Object.entries(this.getPathsFromType(prefix, type.nested)).map(
            ([path, type]) => [path, `Vec<${type}>`]
          )
        );
      }
    }
  }

  private generatePendingSubscriptions() {
    this._pendingOutputSubscriptions = [];
    this._pendingImageSubscriptions = [];

    this._outputSubscriptions.forEachRegistered((cycler, outputType, path) => {
      this._pendingOutputSubscriptions.push({
        subscribe: true,
        cycler: cycler,
        outputType: outputType,
        path: path,
      });
    });

    this._imageSubscriptions.forEachRegistered((cycler) => {
      this._pendingImageSubscriptions.push({
        subscribe: true,
        cycler: cycler,
      });
    });

    this._parameterSubscriptions.forEachRegistered((path) => {
      this._pendingParameterSubscriptions.push({
        subscribe: true,
        path: path,
      });
    });
  }

  private subscribeAndUnsubscribe() {
    if (
      this._pendingOutputSubscriptions.length === 0 &&
      this._pendingImageSubscriptions.length === 0 &&
      this._pendingParameterSubscriptions.length === 0
    ) {
      this._protocolState = ProtocolState.Subscribed;
      this.emitStateChange();
      return;
    }

    this._protocolState = ProtocolState.Subscribing;
    this.emitStateChange();

    this._pendingRequests = {};

    for (let subscription of this._pendingOutputSubscriptions) {
      const requestId = this._nextPendingRequestId++;
      this._pendingRequests[requestId] = (response) => {
        delete this._pendingRequests[requestId];

        if (!response.ok) {
          console.error(
            `Failed to subscribe/unsubscribe output: ${response.reason}`
          );
          this._outputSubscriptions.callErrorCallbacks(
            subscription.cycler,
            subscription.outputType,
            subscription.path,
            `Failed to ${
              subscription.subscribe ? "subscribe" : "unsubscribe"
            } output: ${response.reason}`
          );
          return;
        }

        this.retriggerSubscribing();
      };

      this._sendCallback(
        JSON.stringify({
          id: requestId,
          type: subscription.subscribe
            ? "SubscribeOutput"
            : "UnsubscribeOutput",
          output: {
            cycler: subscription.cycler,
            output: {
              type: subscription.outputType,
              path: subscription.path,
            },
          },
        })
      );
    }

    for (let subscription of this._pendingImageSubscriptions) {
      const requestId = this._nextPendingRequestId++;
      this._pendingRequests[requestId] = (response) => {
        delete this._pendingRequests[requestId];

        if (!response.ok) {
          console.error(
            `Failed to subscribe/unsubscribe image: ${response.reason}`
          );
          if (subscription.subscribe) {
            this._imageSubscriptions.callErrorCallbacks(
              subscription.cycler,
              `Failed to ${
                subscription.subscribe ? "subscribe" : "unsubscribe"
              } image: ${response.reason}`
            );
          }
          return;
        }

        this.retriggerSubscribing();
      };

      this._sendCallback(
        JSON.stringify({
          id: requestId,
          type: subscription.subscribe
            ? "SubscribeOutput"
            : "UnsubscribeOutput",
          output: {
            cycler: subscription.cycler,
            output: {
              type: "Image",
            },
          },
        })
      );
    }

    for (let subscription of this._pendingParameterSubscriptions) {
      const requestId = this._nextPendingRequestId++;
      this._pendingRequests[requestId] = (response) => {
        delete this._pendingRequests[requestId];

        if (!response.ok) {
          console.error(
            `Failed to subscribe/unsubscribe parameter: ${response.reason}`
          );
          if (subscription.subscribe) {
            this._parameterSubscriptions.callErrorCallbacks(
              subscription.path,
              `Failed to ${
                subscription.subscribe ? "subscribe" : "unsubscribe"
              } parameter: ${response.reason}`
            );
          }
          return;
        }

        this.retriggerSubscribing();
      };

      this._sendCallback(
        JSON.stringify({
          id: requestId,
          type: subscription.subscribe
            ? "SubscribeParameter"
            : "UnsubscribeParameter",
          path: subscription.path,
        })
      );
    }

    this._pendingOutputSubscriptions = [];
    this._pendingImageSubscriptions = [];
    this._pendingParameterSubscriptions = [];
  }

  private retriggerSubscribing() {
    if (Object.keys(this._pendingRequests).length > 0) {
      return;
    }

    if (
      this._pendingOutputSubscriptions.length > 0 ||
      this._pendingImageSubscriptions.length > 0 ||
      this._pendingParameterSubscriptions.length > 0
    ) {
      this.subscribeAndUnsubscribe();
      return;
    }

    this._protocolState = ProtocolState.Subscribed;
    this.emitStateChange();
  }

  get outputHierarchy(): ConnectionOutputHierarchy {
    return this._outputHierarchy;
  }

  get parameterHierarchy(): ParameterHierarchy {
    return this._parameterHierarchy;
  }

  handleTextMessage(message: string) {
    type GenericMessage = {
      id: number;
      type: string;
    };
    type OutputsUpdatedMessage = {
      id: number;
      type: "OutputsUpdated";
      cycler: Cycler;
      outputs: {
        output: {
          type: OutputType;
          path: string;
        };
        data: any;
      }[];
      image_id?: number;
    };
    type ParameterUpdatedMessage = {
      id: number;
      type: "ParameterUpdated";
      path: string;
      data: any;
    };
    let parsedMessage: GenericMessage;
    try {
      parsedMessage = JSON.parse(message);
    } catch (error) {
      console.error(error, message);
      alert(`Failed to parse JSON message: ${error}`);
      return;
    }

    if (parsedMessage.id !== undefined) {
      let pendingRequestCallback = this._pendingRequests[parsedMessage.id];
      if (typeof pendingRequestCallback !== "function") {
        console.error(`Got unexpected message with ID ${parsedMessage.id}`);
        alert(`Got unexpected message with ID ${parsedMessage.id}`);
        return;
      }
      pendingRequestCallback(parsedMessage as PendingRequestResponse);
      return;
    }

    switch (parsedMessage.type) {
      case "OutputsUpdated": {
        const outputsUpdatedMessage = parsedMessage as OutputsUpdatedMessage;
        for (let output of outputsUpdatedMessage.outputs) {
          switch (output.output.type) {
            case "Main":
            case "Additional": {
              this._outputSubscriptions.callDataCallbacks(
                outputsUpdatedMessage.cycler,
                output.output.type,
                output.output.path,
                output.data
              );
              break;
            }
            default: {
              console.error(`Unexpected output type ${output.output.type}`);
              alert(`Unexpected output type ${output.output.type}`);
            }
          }
        }

        if (outputsUpdatedMessage.image_id !== undefined) {
          if (
            this._pendingImages[outputsUpdatedMessage.image_id] !== undefined
          ) {
            this._imageSubscriptions.callDataCallbacks(
              outputsUpdatedMessage.cycler,
              this._pendingImages[outputsUpdatedMessage.image_id]
            );
            delete this._pendingImages[outputsUpdatedMessage.image_id];
          } else {
            this._cyclersForPendingImages[outputsUpdatedMessage.image_id] =
              outputsUpdatedMessage.cycler;
          }
        }

        break;
      }
      case "ParameterUpdated": {
        const parameterUpdatedMessage =
          parsedMessage as ParameterUpdatedMessage;
        this._parameterSubscriptions.callDataCallbacks(
          parameterUpdatedMessage.path,
          parameterUpdatedMessage.data
        );

        break;
      }
      default: {
        console.error(`Ignoring message of type \`${parsedMessage.type}\``);
        alert(`Ignoring message of type \`${parsedMessage.type}\``);
        break;
      }
    }
  }

  private async splitBinaryMessage(
    message: Blob
  ): Promise<{ id: number; blob: Blob } | null> {
    if (message.size < 4 + 4) {
      console.error(`Received binary message is too short`);
      alert(`Received binary message is too short`);
      return null;
    }
    // ignore length
    let idSlice = message.slice(4, 8);
    return {
      id: new Uint32Array(await idSlice.arrayBuffer())[0],
      blob: message.slice(8),
    };
  }

  async handleBinaryMessage(message: Blob) {
    let splittedMessage = await this.splitBinaryMessage(message);
    if (splittedMessage === null) {
      return;
    }
    if (this._cyclersForPendingImages[splittedMessage.id]) {
      this._imageSubscriptions.callDataCallbacks(
        this._cyclersForPendingImages[splittedMessage.id],
        splittedMessage.blob
      );
      delete this._cyclersForPendingImages[splittedMessage.id];
    } else {
      this._pendingImages[splittedMessage.id] = splittedMessage.blob;
    }
  }

  subscribeOutput(
    cycler: Cycler,
    outputType: OutputType,
    path: string,
    dataCallback: OutputDataCallback,
    errorCallback: OutputErrorCallback
  ): UnsubscribeOutputCallback {
    let subscriptionId = this._nextSubscriptionId;
    this._nextSubscriptionId += 1;

    let requiresToSubscribe = this._outputSubscriptions.register(
      cycler,
      outputType,
      path,
      subscriptionId,
      dataCallback,
      errorCallback
    );
    if (
      ["Subscribing", "Subscribed"].includes(this._protocolState) &&
      requiresToSubscribe
    ) {
      let isUnsubscribe = (subscription: OutputSubscription) => {
        return (
          !subscription.subscribe &&
          subscription.cycler === cycler &&
          subscription.outputType === outputType &&
          subscription.path === path
        );
      };
      let previousLength = this._pendingOutputSubscriptions.length;
      this._pendingOutputSubscriptions =
        this._pendingOutputSubscriptions.filter(
          (subscription) => !isUnsubscribe(subscription)
        );
      let removedUnsubscribe =
        this._pendingOutputSubscriptions.length !== previousLength;
      if (!removedUnsubscribe) {
        this._pendingOutputSubscriptions.push({
          subscribe: true,
          cycler: cycler,
          outputType: outputType,
          path: path,
        });
        this.retriggerSubscribing();
      }
    }

    return () => {
      let requiresToUnsubscribe = this._outputSubscriptions.unregister(
        cycler,
        outputType,
        path,
        subscriptionId
      );
      if (
        ["Subscribing", "Subscribed"].includes(this._protocolState) &&
        requiresToUnsubscribe
      ) {
        let isSubscribe = (subscription: OutputSubscription) => {
          return (
            subscription.subscribe &&
            subscription.cycler === cycler &&
            subscription.outputType === outputType &&
            subscription.path === path
          );
        };
        let previousLength = this._pendingOutputSubscriptions.length;
        this._pendingOutputSubscriptions =
          this._pendingOutputSubscriptions.filter(
            (subscription) => !isSubscribe(subscription)
          );
        let removedSubscribe =
          this._pendingOutputSubscriptions.length !== previousLength;
        if (!removedSubscribe) {
          this._pendingOutputSubscriptions.push({
            subscribe: false,
            cycler: cycler,
            outputType: outputType,
            path: path,
          });
          this.retriggerSubscribing();
        }
      }
    };
  }

  subscribeImage(
    cycler: Cycler,
    dataCallback: ImageDataCallback,
    errorCallback: ImageErrorCallback
  ): UnsubscribeImageCallback {
    let subscriptionId = this._nextSubscriptionId;
    this._nextSubscriptionId += 1;

    let requiresToSubscribe = this._imageSubscriptions.register(
      cycler,
      subscriptionId,
      dataCallback,
      errorCallback
    );
    if (
      ["Subscribing", "Subscribed"].includes(this._protocolState) &&
      requiresToSubscribe
    ) {
      let isUnsubscribe = (subscription: ImageSubscription) => {
        return !subscription.subscribe && subscription.cycler === cycler;
      };
      let previousLength = this._pendingImageSubscriptions.length;
      this._pendingImageSubscriptions = this._pendingImageSubscriptions.filter(
        (subscription) => !isUnsubscribe(subscription)
      );
      let removedUnsubscribe =
        this._pendingImageSubscriptions.length !== previousLength;
      if (!removedUnsubscribe) {
        this._pendingImageSubscriptions.push({
          subscribe: true,
          cycler: cycler,
        });
        this.retriggerSubscribing();
      }
    }

    return () => {
      let requiresToUnsubscribe = this._imageSubscriptions.unregister(
        cycler,
        subscriptionId
      );
      if (
        ["Subscribing", "Subscribed"].includes(this._protocolState) &&
        requiresToUnsubscribe
      ) {
        let isSubscribe = (subscription: ImageSubscription) => {
          return subscription.subscribe && subscription.cycler === cycler;
        };
        let previousLength = this._pendingImageSubscriptions.length;
        this._pendingImageSubscriptions =
          this._pendingImageSubscriptions.filter(
            (subscription) => !isSubscribe(subscription)
          );
        let removedSubscribe =
          this._pendingImageSubscriptions.length !== previousLength;
        if (!removedSubscribe) {
          this._pendingImageSubscriptions.push({
            subscribe: false,
            cycler: cycler,
          });
          this.retriggerSubscribing();
        }
      }
    };
  }

  subscribeParameter(
    path: string,
    dataCallback: ParameterDataCallback,
    errorCallback: ParameterErrorCallback
  ): UnsubscribeParameterCallback {
    let subscriptionId = this._nextSubscriptionId;
    this._nextSubscriptionId += 1;

    let requiresToSubscribe = this._parameterSubscriptions.register(
      path,
      subscriptionId,
      dataCallback,
      errorCallback
    );
    if (
      ["Subscribing", "Subscribed"].includes(this._protocolState) &&
      requiresToSubscribe
    ) {
      let isUnsubscribe = (subscription: ParameterSubscription) => {
        return !subscription.subscribe && subscription.path === path;
      };
      let previousLength = this._pendingParameterSubscriptions.length;
      this._pendingParameterSubscriptions =
        this._pendingParameterSubscriptions.filter(
          (subscription) => !isUnsubscribe(subscription)
        );
      let removedUnsubscribe =
        this._pendingParameterSubscriptions.length !== previousLength;
      if (!removedUnsubscribe) {
        this._pendingParameterSubscriptions.push({
          subscribe: true,
          path: path,
        });
        this.retriggerSubscribing();
      }
    }

    return () => {
      let requiresToUnsubscribe = this._parameterSubscriptions.unregister(
        path,
        subscriptionId
      );
      if (
        ["Subscribing", "Subscribed"].includes(this._protocolState) &&
        requiresToUnsubscribe
      ) {
        let isSubscribe = (subscription: ParameterSubscription) => {
          return subscription.subscribe && subscription.path === path;
        };
        let previousLength = this._pendingParameterSubscriptions.length;
        this._pendingParameterSubscriptions =
          this._pendingParameterSubscriptions.filter(
            (subscription) => !isSubscribe(subscription)
          );
        let removedSubscribe =
          this._pendingParameterSubscriptions.length !== previousLength;
        if (!removedSubscribe) {
          this._pendingParameterSubscriptions.push({
            subscribe: false,
            path: path,
          });
          this.retriggerSubscribing();
        }
      }
    };
  }

  updateParameter(
    path: string,
    data: any,
    successCallback: ParameterSuccessCallback,
    errorCallback: ParameterErrorCallback
  ) {
    if (!["Subscribing", "Subscribed"].includes(this._protocolState)) {
      errorCallback(`Failed to update parameter: Not connected`);
      return;
    }

    const requestId = this._nextPendingRequestId++;
    this._pendingRequests[requestId] = (response) => {
      delete this._pendingRequests[requestId];

      if (!response.ok) {
        console.error(`Failed to update parameter: ${response.reason}`);
        errorCallback(`Failed to update parameter: ${response.reason}`);
        return;
      }

      successCallback();
    };

    this._sendCallback(
      JSON.stringify({
        id: requestId,
        type: "UpdateParameter",
        path,
        data,
      })
    );
  }
}

// function test_ConnectionProtocol_successfulStateTransitionsWithoutSubscriptions() {
//   const sentMessages: { [key: string]: any }[] = [];
//   const protocol = new Protocol((message) => {
//     sentMessages.push(JSON.parse(message));
//   });
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//   );
//   protocol.webSocketState = "NotConnected" as WebSocketState;
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//   );
//   for (let i = 0; i < 2; ++i) {
//     protocol.webSocketState = WebSocketState.Connected;
//     console.assert(
//       sentMessages.length === 1 &&
//         protocol.state === ProtocolState.GettingHierarchies
//     );
//     let message = sentMessages.shift();
//     console.assert(
//       message!.type === "GetOutputHierarchy" && typeof message!.id === "number"
//     );
//     protocol.handleTextMessage(
//       JSON.stringify({
//         type: "GetOutputHierarchyResult",
//         id: message!.id,
//         ok: true,
//         output_hierarchy: {},
//       })
//     );
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//     );
//     protocol.webSocketState = "NotConnected" as WebSocketState;
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//     );
//   }
// }

// function test_ConnectionProtocol_disconnectedAndConnectedSubscriptions() {
//   let sentMessages: { [key: string]: any }[] = [];
//   const protocol = new Protocol((message) => {
//     sentMessages.push(JSON.parse(message));
//   });
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path0",
//     () => {},
//     () => {}
//   );
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path0",
//     () => {},
//     () => {}
//   );
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path1",
//     () => {},
//     () => {}
//   );
//   if (sentMessages.length === 0) {
//     throw `sentMessages.length = ${sentMessages.length}`;
//   }
//   protocol.webSocketState = WebSocketState.Connected;
//   if (sentMessages.length === 1) {
//     throw `sentMessages.length = ${sentMessages.length}`;
//   }
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path2",
//     () => {},
//     () => {}
//   );
//   let message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "GetOutputHierarchyResult",
//       id: message!.id,
//       ok: true,
//       output_hierarchy: {},
//     })
//   );
//   console.assert(
//     sentMessages.length === 3 && protocol.state === ProtocolState.Subscribing
//   );
//   let pendingSubscriptions = new Set([
//     "Control.Main.path0",
//     "Control.Main.path1",
//     "Control.Main.path2",
//   ]);
//   let resultMessages = [];
//   for (let message of sentMessages) {
//     console.assert(
//       message.type === "SubscribeOutput" &&
//         typeof message.id === "number" &&
//         pendingSubscriptions.delete(
//           `${message.output.cycler}.${message.output.output.type}.${message.output.output.path}`
//         )
//     );
//     resultMessages.push(
//       JSON.stringify({
//         type: "SubscribeOutputResult",
//         id: message.id,
//         ok: true,
//       })
//     );
//   }
//   sentMessages = [];
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path3",
//     () => {},
//     () => {}
//   );
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribing
//   );
//   protocol.handleTextMessage(resultMessages.shift()!);
//   protocol.handleTextMessage(resultMessages.shift()!);
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribing
//   );
//   protocol.handleTextMessage(resultMessages.shift()!);
//   console.assert(
//     sentMessages.length === 1 && protocol.state === ProtocolState.Subscribing
//   );
//   message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "SubscribeOutputResult",
//       id: message!.id,
//       ok: true,
//     })
//   );
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//   );
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path4",
//     () => {},
//     () => {}
//   );
//   console.assert(
//     sentMessages.length === 1 && protocol.state === ProtocolState.Subscribing
//   );
//   message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "SubscribeOutputResult",
//       id: message!.id,
//       ok: true,
//     })
//   );
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//   );
//   protocol.webSocketState = WebSocketState.Disconnected;
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//   );
//   protocol.webSocketState = WebSocketState.Connected;
//   console.assert(
//     sentMessages.length === 1 &&
//       protocol.state === ProtocolState.GettingHierarchies
//   );
//   message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "GetOutputHierarchyResult",
//       id: message!.id,
//       ok: true,
//       output_hierarchy: {},
//     })
//   );
//   console.assert(protocol.state === ProtocolState.Subscribing);
//   pendingSubscriptions = new Set([
//     "Control.Main.path0",
//     "Control.Main.path1",
//     "Control.Main.path2",
//     "Control.Main.path3",
//     "Control.Main.path4",
//   ]);
//   for (let message of sentMessages) {
//     console.assert(
//       message.type === "SubscribeOutput" &&
//         typeof message.id === "number" &&
//         pendingSubscriptions.delete(
//           `${message.output.cycler}.${message.output.output.type}.${message.output.output.path}`
//         )
//     );
//     protocol.handleTextMessage(
//       JSON.stringify({
//         type: "SubscribeOutputResult",
//         id: message.id,
//         ok: true,
//       })
//     );
//   }
//   sentMessages = [];
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//   );
// }

// function test_ConnectionProtocol_mergeSubscriptionRequests() {
//   const sentMessages: { [key: string]: any }[] = [];
//   const protocol = new Protocol((message) => {
//     sentMessages.push(JSON.parse(message));
//   });
//   protocol.webSocketState = WebSocketState.Connected;
//   let message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "GetOutputHierarchyResult",
//       id: message!.id,
//       ok: true,
//       output_hierarchy: {},
//     })
//   );
//   let unsubscribe0 = protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     () => {},
//     () => {}
//   );
//   // while not calling unsubscribe0, merge new subscribe with unsubscribe1:
//   let unsubscribe1 = protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     () => {},
//     () => {}
//   );
//   unsubscribe1();
//   // fulfill subscription corresponding to unsubscribe0
//   message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "SubscribeOutputResult",
//       id: message!.id,
//       ok: true,
//     })
//   );
//   // nothing left to process because of merge
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//   );
//   let unsubscribe2 = protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "anotherPath",
//     () => {},
//     () => {}
//   );
//   // while not calling unsubscribe2, merge unsubscribe0 with new subscribe
//   unsubscribe0();
//   let unsubscribe3 = protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     () => {},
//     () => {}
//   );
//   // fulfill subscription corresponding to unsubscribe2
//   message = sentMessages.shift();
//   protocol.handleTextMessage(
//     JSON.stringify({
//       type: "SubscribeOutputResult",
//       id: message!.id,
//       ok: true,
//     })
//   );
//   // nothing left to process because of merge
//   console.assert(
//     sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//   );
// }

// function test_ConnectionProtocol_reconnectInAllStates() {
//   let sentMessages: { [key: string]: any }[] = [];
//   const protocol = new Protocol((message) => {
//     sentMessages.push(JSON.parse(message));
//   });
//   protocol.subscribeOutput(
//     Cycler.Control,
//     OutputType.Main,
//     "path",
//     () => {},
//     () => {}
//   );

//   for (let i = 0; i < 2; ++i) {
//     protocol.webSocketState = "NotConnected" as WebSocketState;
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//     );
//     protocol.webSocketState = "AlsoNotConnected" as WebSocketState;
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//     );
//   }

//   for (let i = 0; i < 2; ++i) {
//     protocol.webSocketState = WebSocketState.Connected;
//     console.assert(
//       sentMessages.length === 1 &&
//         protocol.state === ProtocolState.GettingHierarchies
//     );
//     sentMessages = [];
//     protocol.webSocketState = WebSocketState.Disconnected;
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//     );
//   }

//   for (let i = 0; i < 2; ++i) {
//     protocol.webSocketState = WebSocketState.Connected;
//     let message = sentMessages.shift();
//     protocol.handleTextMessage(
//       JSON.stringify({
//         type: "GetOutputHierarchyResult",
//         id: message!.id,
//         ok: true,
//         output_hierarchy: {},
//       })
//     );
//     console.assert(
//       sentMessages.length === 1 && protocol.state === ProtocolState.Subscribing
//     );
//     sentMessages = [];
//     protocol.webSocketState = WebSocketState.Disconnected;
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//     );
//   }

//   for (let i = 0; i < 2; ++i) {
//     protocol.webSocketState = WebSocketState.Connected;
//     let message = sentMessages.shift();
//     protocol.handleTextMessage(
//       JSON.stringify({
//         type: "GetOutputHierarchyResult",
//         id: message!.id,
//         ok: true,
//         output_hierarchy: {},
//       })
//     );
//     message = sentMessages.shift();
//     protocol.handleTextMessage(
//       JSON.stringify({
//         type: "SubscribeOutputResult",
//         id: message!.id,
//         ok: true,
//       })
//     );
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Subscribed
//     );
//     protocol.webSocketState = WebSocketState.Disconnected;
//     console.assert(
//       sentMessages.length === 0 && protocol.state === ProtocolState.Disconnected
//     );
//   }
// }

// function test_ConnectionProtocol() {
//   test_ConnectionProtocol_successfulStateTransitionsWithoutSubscriptions();
//   test_ConnectionProtocol_disconnectedAndConnectedSubscriptions();
//   test_ConnectionProtocol_mergeSubscriptionRequests();
//   test_ConnectionProtocol_reconnectInAllStates();
// }
