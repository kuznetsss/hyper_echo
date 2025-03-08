Hyper_echo is an echo server empowered by [tokio](https://docs.rs/tokio/latest/tokio/), [tower](https://docs.rs/tower/latest/tower/), [hyper](https://docs.rs/hyper/latest/hyper/index.html) and [fastwebsockets](https://docs.rs/fastwebsockets/latest/fastwebsockets/index.html).
It supports both HTTP and WebSocket protocols, making it ideal for testing and debugging network applications.

## 💡Features
### HTTP
- Echoes back any received request (except websocket upgrade)
- Supports both HTTP/1.1 and HTTP/2
- Customizable logging levels:
  - `0`: No logging (default)
  - `1`: Log the request URI
  - `2`: Log the request URI and headers
  - `3`: Log the request URI, headers and body

### WebSocket
- Echoes received message back to the client
- Logging of received messages (off by default)
- Sends periodic pings (every 5 seconds by default) to keep connections alive and disconnects inactive clients

### Other
- Colorful log output when the output is a terminal
- Choose your desired port or let `hyper_echo` automatically find a free one
- Supports multi-threading, but efficient enough to use only one thread by default
- Graceful shutdown on `Ctrl-C` and force exit on the second `Ctrl-C`

Use the flag `--help` to discover CLI options for customizing the behavior of `hyper_echo`.

## 🌐 Links
- crates.io: [https://crates.io/crates/hyper_echo](https://crates.io/crates/hyper_echo)
- docs.rs: [https://docs.rs/hyper_echo](https://docs.rs/hyper_echo)

## ⚙️  HTTP logging implementation
There are two crate's features controlling HTTP logging:
- `tower_trace` (default) is based on [Trace](https://docs.rs/tower-http/latest/tower_http/trace/struct.Trace.html) from [tower_http](https://docs.rs/tower-http/latest/tower_http/index.html) crate
- `custom_trace` is a layer for tower service written from scratch

Both implementations are almost identical from a user perspective.
There is no real reason to use logging implementation provided by `custom_trace` feature.
It was created to learn how to create a custom tower layer and how to handle multiple features in one crate.
But if in some case you want to use it, please don't forget to add `default-features = false` if you are using `custom_trace` because
it is possible to use only one logging implementation at a time.

## 🙏 Acknowledgements
Thanks to David Peterson for the [Tower deep dive video](https://www.youtube.com/watch?v=16sU1q8OeeI) explained for me how to use tower.
