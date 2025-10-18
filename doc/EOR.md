# End of Record (EOR)

## The TELNET EOR option

The TELNET EOR (End of Record) option allows a MUD server to mark the end of a prompt.

EOR is implemented as a Telnet
option [RFC854](https://tintin.mudhalla.net/rfc/rfc854), [RFC855](https://tintin.mudhalla.net/rfc/rfc855). The server
and client negotiate the use of the EOR option as they would any other telnet option as detailed
in [RFC885](https://tintin.mudhalla.net/rfc/rfc885). Once agreement has been reached on the use of the option, IAC EOR
is used to mark prompts. A prompt is considered any line that does not end with \r\n.

### Server Commands

```
IAC WILL TELOPT_EOR    Indicates the server wants to enable EOR support.
IAC WONT TELOPT_EOR    Indicates the server wants to disable EOR support.
```

### Client Commands

```
IAC DO   TELOPT_EOR    Indicates the client supports EOR.
IAC DONT TELOPT_EOR    Indicates the client does not support EOR.
```

### Handshake

When a client connects to a server the server should send IAC WILL TELOPT_EOR. The client should respond with either IAC
DO TELOPT_EOR or IAC DONT TELOPT_EOR. If the server receives IAC DO TELOPT_EOR it can begin appending IAC EOR to the end
of prompts.

### EOR definitions

```
TELOPT_EOR  25
EOR        239
```

## Links

If you want a link added, you can email me at mudclient@gmail.com.

### Clients supporting EOR

- [Comparison of MUD clients](http://en.wikipedia.org/wiki/Comparison_of_MUD_clients) - Wikipedia contains an up to date
  list of the EOR support of MUD clients.

### Snippets

- [Scandum's MUD Telopt Handler](https://github.com/scandum/mth) - Handles CHARSET, EOR, MCCP2, MCCP3, MSDP, MSSP, MTTS,
  NAWS, NEW-ENVIRON, TTYPE, and xterm 256 colors.

