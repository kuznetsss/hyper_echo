Hyper_echo is an echo server empowered by [tokio](https://docs.rs/tokio/latest/tokio/), [tower](https://docs.rs/tower/latest/tower/) and [hyper](https://docs.rs/hyper/latest/hyper/index.html).

## 💡Features
- Customizable logging level: none, uri, uri + headers, uri + headers + body
- Colorful log when output is a terminal
- Use the port you want or let `hyper_echo` find some free port
- Use as many threads as you want, but `hyper_echo` is async and very efficient, so only 1 thread is used by default
- HTTP and WebSocket support

Please use the flag `--help` to see how to provide the options you want.

## 🌐 Links
- crates.io:
- docs.rs:

## 🙏 Acknowledgements
Thanks to David Peterson for the [Tower deep dive](https://www.youtube.com/watch?v=16sU1q8OeeI) video explained for me how to use tower.
