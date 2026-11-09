"""Loopback-only AI upstream for checking credential selection without paid calls."""
import argparse
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading
import time

parser = argparse.ArgumentParser()
parser.add_argument('--port', type=int, required=True)
args = parser.parse_args()
calls = Counter()
lock = threading.Lock()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def answer(self, status, value):
        data = json.dumps(value).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        with lock:
            self.answer(200, dict(calls))

    def do_POST(self):
        key = self.headers.get('Authorization', '').removeprefix('Bearer ')
        actor = {'qa-personal-key': 'personal', 'qa-platform-key': 'platform'}.get(key, 'invalid')
        chat = self.path.endswith('/chat/completions')
        with lock:
            calls[f'{actor}:{"chat" if chat else "other"}'] += 1
        payload = self.rfile.read(min(int(self.headers.get('Content-Length', 0)), 1024 * 1024))
        if actor == 'invalid':
            return self.answer(401, {'error': {'message': 'Invalid test credentials', 'code': 'invalid_api_key'}})
        if not chat:
            return self.answer(400, {'error': {'message': 'Voice is tested separately', 'code': 'unsupported'}})
        body = json.loads(payload)
        structured = {'use_lang': 'en', 'original_en': body['messages'][-1]['content'], 'original_zh': '测试输入', 'reply_en': 'Hello from your own provider.', 'reply_zh': '来自个人服务的回复。', 'issues': []}
        content = json.dumps(structured, ensure_ascii=False) if body.get('response_format') else f'{actor} translation'
        if body.get('stream'):
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Connection', 'close')
            self.end_headers()
            for chunk in [content[:30], content[30:]]:
                response = {'id': 'qa', 'object': 'chat.completion.chunk', 'created': 1, 'model': 'qa', 'choices': [{'index': 0, 'delta': {'content': chunk}, 'finish_reason': None}]}
                self.wfile.write(('data: ' + json.dumps(response) + '\n\n').encode())
                self.wfile.flush()
                time.sleep(0.03)
            response = {'id': 'qa', 'object': 'chat.completion.chunk', 'created': 1, 'model': 'qa', 'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}]}
            self.wfile.write(('data: ' + json.dumps(response) + '\n\ndata: [DONE]\n\n').encode())
            self.close_connection = True
        else:
            self.answer(200, {'id': 'qa', 'object': 'chat.completion', 'created': 1, 'model': 'qa', 'choices': [{'index': 0, 'message': {'role': 'assistant', 'content': content}, 'finish_reason': 'stop'}], 'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}})


ThreadingHTTPServer(('127.0.0.1', args.port), Handler).serve_forever()
