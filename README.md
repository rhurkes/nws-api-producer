# wx-nwsapi-loader
TODO

# Implemented products
Details of product codes and products can be found at: https://en.wikipedia.org/wiki/Specific_Area_Message_Encoding
- `AFD` Area Forecast Discussion
- `LSR` Local Storm Report
- `SEL` Severe Local Storm Watch and Watch Cancellation Msg. Issued when watches are issued. Has the watch text.
- `SVR` Severe Thunderstorm Warning
- `SVS` Severe Weather Statement (only PDS and tornado emergency)
- `SWO` Severe Storm Outlook Narrative. Includes the 1/2/3/4-8 day outlooks (ACUS01/02/03/48) and Mesoscale Discussions (ACUS11). MDs contain their own coordinates and do not have a corresponding PTS.
- `TOR` Tornado Warning
- `FFW` Flash Flood Warning

# Missing products (that should be implemented in order of priority)
- `SEV` Shows coordinates for all active watches.
- `PTS` Probabilistic Outlook Points. Contains coordinates for SWO outlooks (WUUS01/02/03/48).
- `FFA` Flash Flood Watch (need sample)

# Building
## OSX
- Need to install pkg-config: `brew install pkg-config`
- Need to install ZeroMQ: `brew install zmq`

# TODO
- handle multiple events in an LSR
- check on TSTM and no severe outlooks once they happen, to finish get_outlook_risk
- implement sev/pts once mapping client exists
- look into parser combinators
- make the main loop more performant with threading
