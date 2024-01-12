# HS

HS is a tool for sending and receiving HL7 messages over the MLLP protocol.

## Features

- [X] Send a HL7 message over MLLP (message sourced from a file or stdin).
- [X] Open a server to receive HL7 messages over MLLP and print them to stdout.
- [X] Generate and return reasonably formed HL7 ACKs
- [X] Limit the number of received messages before stopping the server (or run indefinitely).

## Non-Goals

* Send more than one message at a time (TBD, maybe will become a feature)
* Provide a TUI
* Anything fancy

