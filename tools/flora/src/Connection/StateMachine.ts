import { ConnectionState } from "./Connection";

export enum WebSocketState {
  WantsToStayDisconnectedWithUnsetUrl = "WantsToStayDisconnectedWithUnsetUrl",
  WantsToConnectWithUnsetUrl = "WantsToConnectWithUnsetUrl",
  Connecting = "Connecting",
  Connected = "Connected",
  BackOff = "BackOff",
  Disconnecting = "Disconnecting",
  ConnectAfterDisconnect = "ConnectAfterDisconnect",
  Disconnected = "Disconnected",
}

export default class ConnectionStateMachine {
  private _state = WebSocketState.WantsToStayDisconnectedWithUnsetUrl;
  private _webSocketType: new (url: string) => WebSocket;
  private _webSocketUrl: string | null = null;
  private _connect = false;
  private _webSocket: WebSocket | null = null;
  private _backOffTimeout: NodeJS.Timeout | null = null;
  private _backOffTimeoutSeconds = 1;
  onStateChange: ((state: WebSocketState) => void) | null = null;
  onMessage: ((message: string | Blob) => void) | null = null;

  constructor(webSocketType: new (url: string) => WebSocket) {
    this._webSocketType = webSocketType;
  }

  simplifyState(state: WebSocketState): ConnectionState {
    return {
      [WebSocketState.WantsToStayDisconnectedWithUnsetUrl]:
        ConnectionState.Disconnected,
      [WebSocketState.WantsToConnectWithUnsetUrl]: ConnectionState.Disconnected,
      [WebSocketState.Connecting]: ConnectionState.Connecting,
      [WebSocketState.Connected]: ConnectionState.Connected,
      [WebSocketState.BackOff]: ConnectionState.Connecting,
      [WebSocketState.Disconnecting]: ConnectionState.Disconnecting,
      [WebSocketState.ConnectAfterDisconnect]: ConnectionState.Connecting,
      [WebSocketState.Disconnected]: ConnectionState.Disconnected,
    }[state];
  }

  private emitStateChange() {
    if (typeof this.onStateChange === "function") {
      this.onStateChange(this._state);
    }
  }

  get state() {
    return this._state;
  }

  get webSocketUrl() {
    return this._webSocketUrl;
  }

  set webSocketUrl(webSocketUrl) {
    if (webSocketUrl === this._webSocketUrl) {
      return;
    }
    this._webSocketUrl = webSocketUrl;

    switch (this._state) {
      case WebSocketState.WantsToStayDisconnectedWithUnsetUrl: {
        this._state = WebSocketState.Disconnected;
        this.emitStateChange();
        break;
      }
      case WebSocketState.WantsToConnectWithUnsetUrl: {
        this._state = WebSocketState.Connecting;
        this.connectWebSocket();
        this.emitStateChange();
        break;
      }
      case WebSocketState.Connecting:
      case WebSocketState.Connected: {
        this._state = WebSocketState.ConnectAfterDisconnect;
        this.disconnectWebSocket();
        this.emitStateChange();
        break;
      }
      case WebSocketState.BackOff: {
        this._state = WebSocketState.Connecting;
        if (this._backOffTimeout !== null) {
          clearTimeout(this._backOffTimeout);
        }
        this._backOffTimeoutSeconds = 1;
        this.connectWebSocket();
        this.emitStateChange();
        break;
      }
      case WebSocketState.Disconnected:
      case WebSocketState.ConnectAfterDisconnect:
      case WebSocketState.Disconnecting: {
        break;
      }
      default: {
        console.error(
          `Setting webSocketUrl in state ${this._state} is not implemented`
        );
        alert(
          `Setting webSocketUrl in state ${this._state} is not implemented`
        );
        break;
      }
    }
  }

  get connect() {
    return this._connect;
  }

  set connect(connect) {
    if (connect === this._connect) {
      return;
    }
    this._connect = connect;

    if (this._connect) {
      switch (this._state) {
        case WebSocketState.WantsToStayDisconnectedWithUnsetUrl: {
          this._state = WebSocketState.WantsToConnectWithUnsetUrl;
          this.emitStateChange();
          break;
        }
        case WebSocketState.Disconnected: {
          this._state = WebSocketState.Connecting;
          this.connectWebSocket();
          this.emitStateChange();
          break;
        }
        case WebSocketState.Disconnecting: {
          this._state = WebSocketState.ConnectAfterDisconnect;
          this.emitStateChange();
          break;
        }
        case WebSocketState.WantsToConnectWithUnsetUrl:
        case WebSocketState.Connecting:
        case WebSocketState.Connected:
        case WebSocketState.BackOff:
        case WebSocketState.ConnectAfterDisconnect: {
          break;
        }
        default: {
          console.error(
            `Setting connect = true in state ${this._state} is not implemented`
          );
          alert(
            `Setting connect = true in state ${this._state} is not implemented`
          );
          break;
        }
      }
    } else {
      switch (this._state) {
        case WebSocketState.WantsToConnectWithUnsetUrl: {
          this._state = WebSocketState.WantsToStayDisconnectedWithUnsetUrl;
          this.emitStateChange();
          break;
        }
        case WebSocketState.Connecting:
        case WebSocketState.Connected: {
          this._state = WebSocketState.Disconnecting;
          this.disconnectWebSocket();
          this.emitStateChange();
          break;
        }
        case WebSocketState.ConnectAfterDisconnect: {
          this._state = WebSocketState.Disconnecting;
          this.emitStateChange();
          break;
        }
        case WebSocketState.BackOff: {
          this._state = WebSocketState.Disconnected;
          if (this._backOffTimeout !== null) {
            clearTimeout(this._backOffTimeout);
          }
          this._backOffTimeoutSeconds = 1;
          this.emitStateChange();
          break;
        }
        case WebSocketState.WantsToStayDisconnectedWithUnsetUrl:
        case WebSocketState.Disconnecting:
        case WebSocketState.Disconnected: {
          break;
        }
        default: {
          console.error(
            `Setting connect = false in state ${this._state} is not implemented`
          );
          alert(
            `Setting connect = false in state ${this._state} is not implemented`
          );
          break;
        }
      }
    }
  }

  private onOpen() {
    switch (this._state) {
      case WebSocketState.Connecting: {
        this._state = WebSocketState.Connected;
        if (this._backOffTimeout !== null) {
          clearTimeout(this._backOffTimeout);
        }
        this._backOffTimeoutSeconds = 1;
        this.emitStateChange();
        break;
      }
      default: {
        console.error(
          `Handling onOpen in state ${this._state} is not implemented`
        );
        alert(`Handling onOpen in state ${this._state} is not implemented`);
        break;
      }
    }
  }

  private onClose() {
    switch (this._state) {
      case WebSocketState.Connecting:
      case WebSocketState.Connected: {
        this._state = WebSocketState.BackOff;
        this._backOffTimeout = setTimeout(() => {
          this._state = WebSocketState.Connecting;
          this.connectWebSocket();
          this.emitStateChange();
        }, Math.min(10, this._backOffTimeoutSeconds) * 1000);
        this._backOffTimeoutSeconds *= 1.5;
        this.emitStateChange();
        break;
      }
      case WebSocketState.ConnectAfterDisconnect: {
        this._state = WebSocketState.Connecting;
        this.connectWebSocket();
        this.emitStateChange();
        break;
      }
      case WebSocketState.Disconnecting: {
        this._state = WebSocketState.Disconnected;
        this.emitStateChange();
        break;
      }
      default: {
        console.error(
          `Handling onClose in state ${this._state} is not implemented`
        );
        alert(`Handling onClose in state ${this._state} is not implemented`);
        break;
      }
    }
  }

  private connectWebSocket() {
    this._webSocket = new this._webSocketType(this._webSocketUrl!);
    this._webSocket.onopen = () => {
      this.onOpen();
    };
    this._webSocket.onerror = (error) => {
      console.error(error);
    };
    this._webSocket.onclose = () => {
      this.onClose();
    };
    this._webSocket.onmessage = (message) => {
      if (typeof this.onMessage === "function") {
        this.onMessage(message.data);
      }
    };
  }

  private disconnectWebSocket() {
    this._webSocket!.close();
  }

  send(message: string) {
    if (this._state === WebSocketState.Connected) {
      this._webSocket!.send(message);
    }
  }
}

// class MockedWebSocket {
//   static instances: MockedWebSocket[] = [];
//   webSocketUrl: string;
//   onopen = () => {};
//   onerror = () => {};
//   onclose = () => {};
//   onmessage = () => {};
//   sentMessages: string[] = [];
//   closed = false;

//   constructor(webSocketUrl: string) {
//     MockedWebSocket.instances.push(this);
//     this.webSocketUrl = webSocketUrl;
//   }

//   send(message: string) {
//     this.sentMessages.push(message);
//   }

//   close() {
//     this.closed = true;
//   }
// }

// function test_ConnectionStateMachine_walkAllTransitionsExceptTimeouts() {
//   MockedWebSocket.instances = [];
//   let stateMachine = new ConnectionStateMachine(
//     MockedWebSocket as unknown as new (url: string) => WebSocket
//   );
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToStayDisconnectedWithUnsetUrl"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToStayDisconnectedWithUnsetUrl"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToConnectWithUnsetUrl"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToConnectWithUnsetUrl"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToStayDisconnectedWithUnsetUrl"
//   );
//   stateMachine.webSocketUrl = "ws://eins:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "Disconnected"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "Disconnected"
//   );
//   stateMachine.webSocketUrl = "ws://zwei:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "Disconnected"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Connecting"
//   );
//   console.assert(
//     MockedWebSocket.instances[0].webSocketUrl === stateMachine.webSocketUrl
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Connecting"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Disconnecting"
//   );
//   console.assert(!MockedWebSocket.instances[0].closed);
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Disconnecting"
//   );
//   stateMachine.webSocketUrl = "ws://drei:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Disconnecting"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "ConnectAfterDisconnect"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "ConnectAfterDisconnect"
//   );
//   stateMachine.webSocketUrl = "ws://vier:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "ConnectAfterDisconnect"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Disconnecting"
//   );
//   MockedWebSocket.instances[0].onclose();
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Disconnected"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 2 &&
//       stateMachine.state === "Connecting"
//   );
//   console.assert(
//     MockedWebSocket.instances[1].webSocketUrl === stateMachine.webSocketUrl
//   );
//   stateMachine.webSocketUrl = "ws://fünf:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 2 &&
//       stateMachine.state === "ConnectAfterDisconnect"
//   );
//   console.assert(!MockedWebSocket.instances[1].closed);
//   MockedWebSocket.instances[1].onclose();
//   console.assert(
//     MockedWebSocket.instances.length === 3 &&
//       stateMachine.state === "Connecting"
//   );
//   console.assert(
//     MockedWebSocket.instances[2].webSocketUrl === stateMachine.webSocketUrl
//   );
//   MockedWebSocket.instances[2].onclose();
//   console.assert(
//     MockedWebSocket.instances.length === 3 && stateMachine.state === "BackOff"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 3 && stateMachine.state === "BackOff"
//   );
//   stateMachine.webSocketUrl = "ws://sechs:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 4 &&
//       stateMachine.state === "Connecting"
//   );
//   console.assert(
//     MockedWebSocket.instances[3].webSocketUrl === stateMachine.webSocketUrl
//   );
//   MockedWebSocket.instances[3].onopen();
//   console.assert(
//     MockedWebSocket.instances.length === 4 && stateMachine.state === "Connected"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 4 && stateMachine.state === "Connected"
//   );
//   MockedWebSocket.instances[3].onclose();
//   console.assert(
//     MockedWebSocket.instances.length === 4 && stateMachine.state === "BackOff"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 4 &&
//       stateMachine.state === "Disconnected"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 5 &&
//       stateMachine.state === "Connecting"
//   );
//   console.assert(
//     MockedWebSocket.instances[4].webSocketUrl === stateMachine.webSocketUrl
//   );
//   MockedWebSocket.instances[4].onopen();
//   console.assert(
//     MockedWebSocket.instances.length === 5 && stateMachine.state === "Connected"
//   );
//   stateMachine.webSocketUrl = "ws://sieben:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 5 &&
//       stateMachine.state === "ConnectAfterDisconnect"
//   );
//   console.assert(!MockedWebSocket.instances[4].closed);
//   MockedWebSocket.instances[4].onclose();
//   console.assert(
//     MockedWebSocket.instances.length === 6 &&
//       stateMachine.state === "Connecting"
//   );
//   console.assert(
//     MockedWebSocket.instances[5].webSocketUrl === stateMachine.webSocketUrl
//   );
//   MockedWebSocket.instances[5].onopen();
//   console.assert(
//     MockedWebSocket.instances.length === 6 && stateMachine.state === "Connected"
//   );
//   stateMachine.connect = false;
//   console.assert(
//     MockedWebSocket.instances.length === 6 &&
//       stateMachine.state === "Disconnecting"
//   );

//   MockedWebSocket.instances = [];
//   stateMachine = new ConnectionStateMachine(
//     MockedWebSocket as unknown as new (url: string) => WebSocket
//   );
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToStayDisconnectedWithUnsetUrl"
//   );
//   stateMachine.connect = true;
//   console.assert(
//     MockedWebSocket.instances.length === 0 &&
//       stateMachine.state === "WantsToConnectWithUnsetUrl"
//   );
//   stateMachine.webSocketUrl = "ws://acht:1337";
//   console.assert(
//     MockedWebSocket.instances.length === 1 &&
//       stateMachine.state === "Connecting"
//   );
// }

// function test_ConnectionStateMachine() {
//   test_ConnectionStateMachine_walkAllTransitionsExceptTimeouts();
// }
