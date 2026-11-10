"""Loopback-only AI upstream for checking credential selection without paid calls."""
import argparse
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import threading
import time
import io
import wave

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
        actor = {'qa-personal-key': 'personal', 'qa-platform-key': 'platform', 'qa-voice-denied-key': 'voice-denied', 'qa-slow-test-key': 'slow-test'}.get(key, 'invalid')
        chat = self.path.endswith('/chat/completions')
        with lock:
            calls[f'{actor}:{"chat" if chat else "other"}'] += 1
        payload = self.rfile.read(min(int(self.headers.get('Content-Length', 0)), 1024 * 1024))
        if actor == 'invalid':
            return self.answer(401, {'error': {'message': 'Invalid test credentials', 'code': 'invalid_api_key'}})
        if self.path.endswith('/audio/transcriptions'):
            assert b'glm-asr-2512' in payload and b'RIFF' in payload
            return self.answer(200, {'id':'qa-asr','created':1,'model':'glm-asr-2512','text':''})
        if self.path.endswith('/audio/speech'):
            if actor == 'voice-denied':
                return self.answer(403, {'error': {'message':'Voice model permission denied','code':'permission_denied'}})

            body = json.loads(payload)
            assert body['model'] == 'glm-tts' and body['voice'] in ['tongtong','chuichui','xiaochen','jam','kazi','douji','luodo']
            if body['input'] == 'Slow audio generation.': time.sleep(0.5)
            output = io.BytesIO()
            with wave.open(output, 'wb') as w:
                w.setparams((1,2,16000,0,'NONE','not compressed')); w.writeframes(b'\0' * 3200)
            data=output.getvalue()
            self.send_response(200);self.send_header('Content-Type','audio/wav');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data);return
        if not chat:
            return self.answer(400, {'error': {'message': 'Unsupported test endpoint', 'code': 'unsupported'}})
        body = json.loads(payload)
        if actor == 'slow-test': time.sleep(0.5)
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
