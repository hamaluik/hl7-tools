# HL7-Tools

## HQ

HQ is a an HL7 parser for the command line with some basic features to enable
using and manipulating HL7 messages via a composable command-line workflow.

```bash
# Read sample_adt_a01.hl and before printing it, set MSH.10 (control ID) of the
# message to `1234`
hq -m MSH.10=1234 assets/sample_adt_a01.hl7
```

```bash
# Query PID.5 (patient name) from assets/sample_adt_a01.hl7 and print it
hq -q PID.5 assets/sample_adt_a01.hl7
```

```bash
# Parse the message from stdin and print a JSON version to stdout
cat assets/sample_adt_a01.hl7 | hq -o json
```

## HS

HS is a tool for sending and receiving HL7 messages over the MLLP protocol.

```bash
# Listen on localhost port 10500 for 3 messages and print them as they come in,
# returning error ACKs for each
hs listen --message-count 3 --ack-mode error --bind localhost:10500 
```

```bash
# Send a message to localhost port 10500
cat assets/sample_adt_a01.hl7 | hs send localhost:10500
```

```bash
# Send a message, automatically generating the control id
cat assets/sample_adt_a01.hl7 | hq -m 'MSH.10=<auto>' | hs send localhost:10500
```
