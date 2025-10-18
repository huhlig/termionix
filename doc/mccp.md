# Mud Client Compression Protocol

## The MCCP2 Protocol

MCCP2 is a compression protocol which allows a MUD server to compress output to the receiving client using the zlib
compression library. This typically reduces the amount of bandwidth used by the server by 75 to 90%.

MCCP2 is implemented as a Telnet option RFC854, RFC855. The server and client negotiate the use of MCCP2 as they would
any other telnet option. Once agreement has been reached on the use of the option, option sub-negotiation is used to
start compression.

### Server Commands

```
IAC WILL MCCP2    Indicates the server supports MCCP2.
```

### Client Commands

```
IAC DO   MCCP2    Indicates the client supports MCCP2.
IAC DONT MCCP2    Indicates the client does not support MCCP2. If the server has enabled compression it should disable it.
```

### Handshake

When a client connects to a server the server should send IAC WILL MCCP2. The client should respond with either IAC DO
MCCP2 or IAC DONT MCCP2. If the server receives IAC DO MCCP2 it should respond with: IAC SB MCCP2 IAC SE and immediately
afterwards start compressing data.

### MCCP2 definitions

```
MCCP2 86
```

### Compression Format

Immediately after the server sends IAC SB MCCP2 IAC SE the server should create a zlib stream RFC1950.
Once compression is established, all server side communication, including telnet negotiations, takes place within the
compressed stream.
The server may terminate compression at any point by sending an orderly stream end (Z_FINISH). Following this, the
connection continues as a normal telnet connection.

### Compression Errors

If a decompression error is reported by zlib on the client side, the client can stop decompressing and send IAC DONT
MCCP2. The server in turn should disable compression upon receiving IAC DONT MCCP2, the connection continues as a normal
telnet connection.

Alternatively, the client can close the connection when a stream error is detected.

## The MCCP3 Protocol

MCCP3 is a compression protocol which allows a MUD client to compress output to the receiving server using the zlib
compression library. In some usecases this can significantly reduce bandwidth, while in the typical usecase it can
provide security through obscurity because passwords and messages are no longer sent in plain text.

MCCP3 is implemented as a Telnet option RFC854, RFC855. The server and client negotiate the use of MCCP3 as they would
any other telnet option. Once agreement has been reached on the use of the option, option sub-negotiation is used to
start compression.

### Server Commands

```
IAC WILL MCCP3    Indicates the server supports MCCP3.
IAC WONT MCCP3    Indicates the server wants to disable MCCP3.
```

### Client Commands

```
IAC DO   MCCP3    Indicates the client supports MCCP3.
IAC DONT MCCP3    Indicates the client does not support MCCP3.
```

### Handshake

When a client connects to a server the server should send `IAC WILL MCCP3`. The client should respond with either
`IAC DO MCCP3 IAC SB MCCP3 IAC SE` or `IAC DONT MCCP3`. If the client sends `IAC DO MCCP3 IAC SB MCCP3 IAC SE` it should
immediately afterwards start compressing data.

### MCCP3 definitions

```
MCCP3 87
```

### Compression Format

Immediately after the client sends IAC SB MCCP3 IAC SE the client should create a zlib
stream [RFC1950](https://tintin.mudhalla.net/rfc/rfc1950).

Once compression is established, all client side communication, including telnet negotiations, takes place within the
compressed stream.

The client may terminate compression at any point by sending an orderly stream end (Z_FINISH). Following this, the
connection continues as a normal telnet connection.

The server may request ending compression at any point by sending IAC WONT MCCP3. For example right before initiating a
copyover.

### Compression Errors

If a decompression error is reported by zlib on the server side, the server can stop decompressing and send IAC WONT
MCCP3. The client in turn should disable compression upon receiving IAC WONT MCCP3, the connection continues as a normal
telnet connection.

Alternatively the server can close the connection when a stream error is detected.

### MCCP versions

In 1998 MCCP used TELOPT 85 and the protocol defined an invalid subnogation sequence (IAC SB 85 WILL SE) to start
compression. Subsequently MCCP version 2 was created in 2000 using TELOPT 86 and a valid subnogation (IAC SB 86 IAC SE).

As of 2004 virtually every MCCP enabled MUD client supports version 2, making version 1 obsolete. As such this
specification only deals with version 2, and it is strongly discouraged for MUD servers to implement version 1.

In 2019 MCCP version 3 was created. This version does not replace MCCP 2 but is a separate protocol that allows the
client to send compressed data to the server.

## Links

If you want a link added, you can email me at [mudclient@gmail.com](mudclient@gmail.com).

### Clients supporting MCCP

[Comparison of MUD clients](http://en.wikipedia.org/wiki/Comparison_of_MUD_clients) - Wikipedia contains an up to date
list of the MCCP2 status of MUD clients.

As of this moment only [TinTin++ 2.01.8](https://tintin.mudhalla.net/) and later supports MCCP3.

### Servers supporting MCCP3

endenskeep.com:4000

### Snippets

- [Scandum's MUD Telopt Handler](https://github.com/scandum/mth) - Handles CHARSET, EOR, MCCP2, MCCP3, MSDP, MSSP, MTTS,
  NAWS, NEW-ENVIRON, TTYPE, and xterm 256 colors.



