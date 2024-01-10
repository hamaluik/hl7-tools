# HQ

HQ is a an HL7 parser for the command line with some basic features to enable
using and manipulating HL7 messages via a composable command-line workflow.

## Features

- [X] Parse HL7 messages
- [ ] Validate HL7 message structure
- [X] Map field values to new values (ex: set `MSH.10` to "1234")
- [ ] Query field values (ex: "what is the value of `PID.5`?)
- [X] Read from a file or stdin
- [X] Map newlines to HL7 `\r` segment separators
- [X] Print a (minimally) syntax-highlighted version of the message to stdout

## Non-Goals

* Send or receive HL7 messages
* Process more than one message at a time (TBD, maybe will become a feature)
* Provide a TUI

