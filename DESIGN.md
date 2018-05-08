# Design
The SimProto has two layers:

## Dialog Layer
Dialog layer provides basic request-response communication model to the protocol.
This layer is necessary for a more complex upper layer communication of SimProto.
It only assumes the underlying layer (stream) to be `Readable+Writable+Closeable`.
This layer is responsible for closing and error handling of the stream. Any protocol violation from other peer results in closing of stream.
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

`id` - Id of the message. All the messages of the same dialog must have same id. Initiator peer is responsible to choose a unique id.

`len` - Length of the payload in eight bytes.

`payload` - Data sent by upper layer (Sim Layer).

## Sim Layer
This layer needs its underlying protocol to provide a way to
- send request and receive response and
- receive request and send response.

The dialog layer is usually used as its underlying protocol.
This layer provides many types of communication and all use the same format for messages:

`T`|`len`|`topic`|`data`
:-:|:---:|:-----:|:--------:
 1 |  2  |  len  |SIZE - len

T - Type of message

len - Length of topic

topic - Use to identify topic of the message. It can specify topic to subscribe/unsubscribe.

data - Accompanying data with the topic. It can contain data for body of request/response, information regarding session, information for authentication etc.

SIZE - The size of the whole message. Lower layer frames the whole message so this is not added to the message.
