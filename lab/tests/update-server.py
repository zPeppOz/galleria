#!/usr/bin/env python3
"""The server of the version check for the smoke test (plan B6, T5, G11): it stands in for github.com on 127.0.0.1.

It writes every request it gets into a file, one JSON object per line (`line`: the request line; `headers`: the names and the values
as they came), and answers each with `302` and the `Location` it was given, with no body: what the page of the latest release of a
repository does. Nothing else of the answer is ever read by the app, and nothing here is ever followed.

    update-server.py --log FILE --port-file FILE [--location URL] [--status 302]

The port is the one the system gives (the file `--port-file` has it, written once the server listens). Stops on SIGTERM.
"""
import argparse
import json
import signal
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--log", required=True)
    parser.add_argument("--port-file", required=True)
    parser.add_argument("--location", default="https://github.com/zPeppOz/galleria/releases/tag/v9.9.9")
    parser.add_argument("--status", type=int, default=302)
    args = parser.parse_args()

    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def do_GET(self) -> None:  # noqa: N802
            entry = {"line": self.requestline, "headers": [[name, value] for name, value in self.headers.items()]}
            with open(args.log, "a", encoding="utf-8") as log:
                log.write(json.dumps(entry) + "\n")
            self.send_response(args.status)
            if 300 <= args.status < 400:
                self.send_header("Location", args.location)
            self.send_header("Content-Length", "0")
            self.send_header("Connection", "close")
            self.end_headers()

        def log_message(self, *_: object) -> None:  # the log of the requests is the file
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    with open(args.port_file + ".tmp", "w", encoding="utf-8") as out:
        out.write(str(server.server_address[1]))
    __import__("os").replace(args.port_file + ".tmp", args.port_file)
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
    server.serve_forever()
    return 0


if __name__ == "__main__":
    sys.exit(main())
