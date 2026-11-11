"""Local production-artifact preview with same-origin API proxy and deep links."""
import argparse
import http.client
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

parser = argparse.ArgumentParser()
parser.add_argument('--port', type=int, default=5173)
parser.add_argument('--api', default='http://127.0.0.1:8119')
parser.add_argument('--dir', default='front/dist')
args = parser.parse_args()
root = Path(args.dir).resolve()
upstream = urlsplit(args.api)

class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=str(root), **kw)

    def do_GET(self):
        if self.path == '/api' or self.path.startswith('/api/'):
            return self.proxy()
        candidate = Path(self.translate_path(self.path))
        if not candidate.is_file() and not Path(urlsplit(self.path).path).suffix:
            self.path = '/index.html'
        return super().do_GET()

    def proxy(self):
        cls = http.client.HTTPSConnection if upstream.scheme == 'https' else http.client.HTTPConnection
        conn = cls(upstream.hostname, upstream.port, timeout=95)
        headers_sent = False
        try:
            body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
            headers = {k: v for k, v in self.headers.items() if k.lower() not in ('host', 'connection')}
            conn.request(self.command, self.path, body, headers)
            response = conn.getresponse()
            streaming = response.getheader('Content-Type', '').startswith('text/event-stream')
            data = None if streaming else response.read()
            self.send_response(response.status)
            for k, v in response.getheaders():
                if k.lower() not in ('transfer-encoding', 'connection', 'content-length'):
                    self.send_header(k, v)
            if streaming:
                # HTTP/1.0 close-delimited stream: never buffer a complete reply.
                self.send_header('Connection', 'close')
                self.close_connection = True
            else:
                self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            headers_sent = True
            if streaming:
                while chunk := response.read1(64 * 1024):
                    self.wfile.write(chunk)
                    self.wfile.flush()
            else:
                self.wfile.write(data)
        except (OSError, http.client.HTTPException):
            if not headers_sent:
                self.send_error(502, 'API upstream unavailable')
            self.close_connection = True
        finally:
            conn.close()

    do_POST = do_PUT = do_DELETE = do_PATCH = do_OPTIONS = proxy

print(f'Preview: http://127.0.0.1:{args.port}', flush=True)
ThreadingHTTPServer(('127.0.0.1', args.port), Handler).serve_forever()
