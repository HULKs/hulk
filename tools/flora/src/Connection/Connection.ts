import Protocol, {
  ParameterSuccessCallback,
  UnsubscribeImageCallback,
  UnsubscribeOutputCallback,
  UnsubscribeParameterCallback,
} from "./Protocol";
import ConnectionStateMachine from "./StateMachine";
import {
  DataCallback as ImageDataCallback,
  ErrorCallback as ImageErrorCallback,
} from "./ImageSubscriptions";
import {
  DataCallback as ParameterDataCallback,
  ErrorCallback as ParameterErrorCallback,
} from "./ParameterSubscriptions";
import {
  DataCallback as OutputDataCallback,
  ErrorCallback as OutputErrorCallback,
} from "./OutputSubscriptions";

export enum ConnectionState {
  Disconnected = "Disconnected",
  Connecting = "Connecting",
  Connected = "Connected",
  Disconnecting = "Disconnecting",
}
export enum Cycler {
  Audio = "Audio",
  Control = "Control",
  SplNetwork = "SplNetwork",
  VisionTop = "VisionTop",
  VisionBottom = "VisionBottom",
}
export enum OutputType {
  Main = "Main",
  Additional = "Additional",
}
export type Paths = { [path: string]: string };
export type OutputTypes = {
  [OutputType.Main]: Paths;
  [OutputType.Additional]: Paths;
};
export type OutputHierarchy = {
  [Cycler.Audio]: OutputTypes;
  [Cycler.Control]: OutputTypes;
  [Cycler.SplNetwork]: OutputTypes;
  [Cycler.VisionTop]: OutputTypes;
  [Cycler.VisionBottom]: OutputTypes;
};
export type ParameterHierarchy = Paths;

export default class Connection {
  private _stateMachine: ConnectionStateMachine;
  private _protocol: Protocol;
  private _onStateChange: ((state: ConnectionState) => void) | null = null;
  private _state = ConnectionState.Disconnected;

  constructor() {
    this._stateMachine = new ConnectionStateMachine(WebSocket);
    this._protocol = new Protocol((message) => {
      this._stateMachine.send(message);
    });
    this._protocol.webSocketState = this._stateMachine.state;
    this._stateMachine.onStateChange = (webSocketState) => {
      let nextState = this.mergeStates(
        this._stateMachine.simplifyState(webSocketState),
        this._protocol.simplifyState(this._protocol.state)
      );
      if (
        nextState !== this._state &&
        typeof this._onStateChange === "function"
      ) {
        this._state = nextState;
        this._onStateChange(this._state);
      }
      this._protocol.webSocketState = webSocketState;
    };
    this._stateMachine.onMessage = (message) => {
      if (typeof message === "string") {
        this._protocol.handleTextMessage(message);
      } else {
        this._protocol.handleBinaryMessage(message);
      }
    };
    this._protocol.onStateChange = (protocolState) => {
      let nextState = this.mergeStates(
        this._stateMachine.simplifyState(this._stateMachine.state),
        this._protocol.simplifyState(protocolState)
      );
      if (
        nextState !== this._state &&
        typeof this._onStateChange === "function"
      ) {
        this._state = nextState;
        this._onStateChange(this._state);
      }
    };
  }

  private mergeStates(
    webSocketState: ConnectionState,
    protocolState: ConnectionState
  ) {
    if (webSocketState !== "Connected") {
      return webSocketState;
    }
    return protocolState;
  }

  get webSocketUrl(): string {
    return this._stateMachine.webSocketUrl!;
  }

  set webSocketUrl(webSocketUrl: string) {
    this._stateMachine.webSocketUrl = webSocketUrl;
  }

  get connect(): boolean {
    return this._stateMachine.connect;
  }

  set connect(connect: boolean) {
    this._stateMachine.connect = connect;
  }

  get state(): ConnectionState {
    return this.mergeStates(
      this._stateMachine.simplifyState(this._stateMachine.state),
      this._protocol.simplifyState(this._protocol.state)
    );
  }

  set onStateChange(onStateChange: (state: ConnectionState) => void) {
    this._onStateChange = onStateChange;
  }

  get outputHierarchy(): OutputHierarchy {
    return this._protocol.outputHierarchy;
  }

  set onOutputHierarchyChange(
    onOutputHierarchyChange: (outputHierarchy: OutputHierarchy) => void
  ) {
    this._protocol.onOutputHierarchyChange = onOutputHierarchyChange;
  }

  get parameterHierarchy(): ParameterHierarchy {
    return this._protocol.parameterHierarchy;
  }

  set onParameterHierarchyChange(
    onParameterHierarchyChange: (parameterHierarchy: ParameterHierarchy) => void
  ) {
    this._protocol.onParameterHierarchyChange = onParameterHierarchyChange;
  }

  subscribeOutput(
    cycler: Cycler,
    outputType: OutputType,
    path: string,
    dataCallback: OutputDataCallback,
    errorCallback: OutputErrorCallback
  ): UnsubscribeOutputCallback {
    return this._protocol.subscribeOutput(
      cycler,
      outputType,
      path,
      dataCallback,
      errorCallback
    );
  }

  subscribeImage(
    cycler: Cycler,
    dataCallback: ImageDataCallback,
    errorCallback: ImageErrorCallback
  ): UnsubscribeImageCallback {
    return this._protocol.subscribeImage(cycler, dataCallback, errorCallback);
  }

  subscribeParameter(
    path: string,
    dataCallback: ParameterDataCallback,
    errorCallback: ParameterErrorCallback
  ): UnsubscribeParameterCallback {
    return this._protocol.subscribeParameter(path, dataCallback, errorCallback);
  }

  updateParameter(
    path: string,
    data: any,
    successCallback: ParameterSuccessCallback,
    errorCallback: ParameterErrorCallback
  ) {
    return this._protocol.updateParameter(
      path,
      data,
      successCallback,
      errorCallback
    );
  }
}
