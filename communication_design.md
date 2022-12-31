# Communication Design

## Messages

- Requests are always textual, binary requests are protocol errors
- At least within textual stream, messages are in order
- Each sent data frame contains all textual data of this frame and binary references
  - This allows to have the binary stream separate and only the order in the textual stream matters
- When the client subscribes, the request's id is used to store the subscription, unsubscribe in the future, and receive updated data from the server
- Clients must specify with which cycler instance they want to speak

## Databases

- Clients choose subscription IDs, they need to be prefixed with a unique client identifier (e.g. peer address)
- Two asynchronous tasks exist: `database_provider n--1 databases 1--1 receiver/sender`
  - Single `databases` demultiplexes to `database_provider`
  - Each connected cycler instance spawns a `database_provider` task
- Each `provider` task stores subscriptions
- All `provider` tasks and the `receiver` tasks can send requests to the `router`
- The `provider` first registers itself at the `router`
- `GetNext` and `Subscribe` ids are shared and need to be unique
