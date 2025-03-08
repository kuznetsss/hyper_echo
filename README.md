Hyper_echo is an echo server empowered by [tokio](https://docs.rs/tokio/latest/tokio/), [tower](https://docs.rs/tower/latest/tower/), [hyper](https://docs.rs/hyper/latest/hyper/index.html) and [fastwebsockets](https://docs.rs/fastwebsockets/latest/fastwebsockets/index.html).
It supports both HTTP and WebSocket protocols, making it ideal for testing and debugging network applications.

## 💡Features
### Http:
- Echoes back any the received request (except websocket upgrade)
- Customizable logging levels:
  - `0`: No logging (default)
  - `1`: Log the request URI
  - `2`: Log the request URI and headers
  - `3`: Log the request URI, headers, and body

### WebSocket:
- Echoes received message back to the client
- Logging of received messages (off by default)
- Sends periodic pings (every 5 seconds by default) to keep connections alive and disconnects inactive clients

### Other
- Colorful log output when the output is a terminal
- Choose your desired port or let `hyper_echo` automatically find a free one
- Supports multi-threading, but is optimized for single-threaded use by default.

Use the flag `--help` to discover CLI options for customizing the behavior of `hyper_echo`.

## 🌐 Links
- crates.io:
- docs.rs:

## 🙏 Acknowledgements
Thanks to David Peterson for the [Tower deep dive video](https://www.youtube.com/watch?v=16sU1q8OeeI) explained for me how to use tower.
