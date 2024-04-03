# Swec

Swec is a minimal status page API written in Rust, which means it could be used as the back-end for more minimal alternatives to projects like [Gatus](https://github.com/TwiN/gatus) or [HertzBeat](https://github.com/dromara/hertzbeat).

It aims to provide a minimal and efficient REST API that can be used by *checkers* and *clients*:
- A *checker* determine what the status of a service is periodically and sends that information to the API, which stores it in RAM and dumps it to disk too (kind of like an in memory database).
- *Clients* can then ask the public facing API what the status of an API is. Obviously, the public API is read-only.

A checker spec as stored on the API server has the following attributes:
- A human-readable description
- An optional URL
- An optional group (in the form of a free string)

A status captured at a certain time has the following attributes:
- Whether the checked service is up
- A message, indicating why it is considered in that state

## Features

Implemented:
- Basic API to read and modify statuses and checkers
- Websockets api to watch for new statuses

Planned:
- Web client
- Various checkers
