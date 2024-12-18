# Communication

Communication is the subcomponent in the framework that makes the [Cyclers](./cyclers.md) introspectable to the outside world.
Whereas the cyclers are required to run in a realtime manner, communication does not have this requirement.
Since it deals with external I/O and applications connected over the network which may influence the performance and responsiveness, it serves all its features in a best effort way and therefore decouples this influence from the cyclers.

At a high-level, communication allows applications connected from outside to...

- subscribe to databases from cyclers and receive selected fields from them (*database_subscription_manager*)
- subscribe to configuration parameters, receive changed ones, and update them (*parameter_modificator*)

## Asynchronous Channels and Tasks

Since communication deals with I/O and is idle most of the time waiting for I/O, it is implemented as an asynchronous subcomponent (using [the Tokio Rust crate](https://tokio.rs/)) with the [message passing](https://en.wikipedia.org/wiki/Message_passing) paradigm.
The parts of communication are executed as asynchronous tasks which are then connected together via message passing channels.
The following drawing shows all tasks in communication as square boxes (except the cycler threads, but they can be seen as task-like as well).

![communication_dataflow](./communication_dataflow.png)

Solid connections represent dataflow implemented with channels and dashed connections show the startup behavior of the tasks.

## Task Spawning and Connection Management

The entrypoint is the Communication Runtime which is a thread running a Tokio asynchronous runtime.
This thread is started from the framework's [Runtime](./runtime.md), similar to the cycler threads.
The communication runtime spawns three tasks and connects them with channels.
The _accepter_ task listens for new connections on the socket and spawns a new _connection_ task for each incoming connection.
The _connection_ task is a short-lived task which splits the connection socket into a sending and receiving half and spawns a long-lived task for each half, the _sender_ and _receiver_ tasks.
This splitting allows the _sender_ and _receiver_ to act as multiplexing/demultiplexing tasks if viewed in terms of their channel attachment points.
The _receiver_ interprets incoming messages from the socket and forwards them to the appropriate processing task (e.g. _database_subscription_manager_ or _parameter_modificator_).
The _sender_ gathers all messages from the connected tasks and sends them to the connected socket.

## Database Subscriptions

Communication allows connected clients to subscribe to databases from cyclers and receive selected fields from them.
Subscriptions are managed in the _database_subscription_manager_ task.
The _receiver_ task forwards (un-)subscription requests from the client to the _database_subscription_manager_.
If a connection is closed, the _receiver_ sends an `UnsubscribeEverything` request to the manager task.
Since all interaction between the tasks happens via channels, in some requests it is necessary to include other channel endpoints (e.g. for transferring back results).
Subscriptions always contain a cycler, output type, and data path.
If cyclers complete their execution of all modules, the written database is completed and freed.
Afterwards, the cycler notifies a [`Notify`](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html) which is shared between the cycler and the _database_subscription_manager_ task in communication.
This allows the manager task to wait for newly available databases from any cycler.
When a new database is ready, the manager task iterates all relevant subscriptions to extract subscribed types and images to construct messages for the subscribed clients.
Additional outputs that have been subscribed are sent to the cycler s.t. it can instruct modules to generate the additional outputs.

## Parameter Subscriptions & Updates

Communication allows connected clients to subscribe to configuration parameters, receive changed ones, and update them.
Similar to database subscriptions, parameter subscriptions are processed from the _receiver_ task.

TODO:

- (WebSocket) Protocol/(JSON) (De-)Serialization
    - Acceptor
    - Connection Setup (WebSocket handshake)
    - Sender/Receiver
    - Message Format
