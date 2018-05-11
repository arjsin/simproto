# Design
The SimProto has two layers:

## Dialog Layer
Dialog layer provides basic request-response communication model to the protocol.
This layer is necessary for a more complex upper layer communication of SimProto.
It only assumes the underlying layer (stream) to be `Readable+Writable+Closeable`.
This layer is responsible for closing and error handling of the stream.
Any protocol violation from other peer results in closing of stream.
The communication over this layer happens in a form of dialog and little endian byte order is used.
Generally any of the two peers on either side of the stream can initiate a dialog with a request message and the other peer terminates the dialog with a response message.
There is usually timeout for receiving response after which the stream is assumed to be broken and it is closed immediately.
Each message has the following format:

`T`|`id`|`len`|`payload`
:-:|:--:|:---:|:-------:
 1 | 8  |  8  |   len

`T` - Type of message. It is one byte indicating 

 - 0: Request - This is used by upper layer for making request.
 - 1: Response - This is used by upper layer for making response.
 - 2: Ping - This is used to check active connection.
 - 3: Pong - This is used internally to reply ping.

`id` - Id of the message. All the messages of the same dialog must have same id.
Initiator peer is responsible to choose a unique id.

`len` - Length of the payload in eight bytes.

`payload` - Data sent by upper layer (Sim Layer).

## Sim Layer
The dialog layer is usually used as its underlying protocol.
Type of communication over the protocol:
1. Subscribe
2. Unsubscribe
3. Notify
4. RPC

### Request packet:

`T`|`len`|`topic`|`data`
:-:|:---:|:-----:|:----------------:
 1 |  2  | `len` |`SIZE` - `len` - 3

T - Type of request
Type of message changes with request and response. Type of message with request:
- 0: RPC
- 1: Subscription
- 2: Unsubscription
- 3: Notification

len - Length of the topic.

topic - The topic of the request. The topic is matched exactly.

data - The body of the request. It can be any serializable format which the application can use.

SIZE - The size of the whole message. Lower layer frames the whole message so this is not added to the message.

### Response packet:
`T`|`data`
:-:|:----------------:
 1 |`SIZE` - 1

Type of response depends on the type of request. Any application level error handling messages are provided by the application and a protocol is never sufficient. Only types 0 and 1 are common. Only 0 is success and everything else is failure:
 - 0: Accepted or success (data in case of RPC and subscription).
 Success is only from protocol side and it does not say success or failure for application. Application is supposed to use data to communicate further.
 - 1: Topic not found (no data)

RPC has no special types.

Subscription
 - 2: Double subscription(Rejected) (no data)
 - 3: Rejected (data)

Unsubscription
 - 2: Not subscribed (no data)

Notification
 - 2: Not subscribed (no data)
 
data - Accompanying data. It is optional and depends on the type of the response message.

SIZE - The size of the whole message. Lower layer frames the whole message so this is not added to the message.
