"""API regression against an explicitly isolated local PostgreSQL database.

Run after starting the backend with QA_DATABASE_URL as DATABASE_URL. Tests create
only QA records; never use a production database for this suite.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen

base = os.environ.get('QA_API', 'http://127.0.0.1:8129')
database = os.environ['QA_DATABASE_URL']
url = urlsplit(database)
assert url.hostname in ('127.0.0.1', 'localhost') and url.path == '/colang_qa', 'An isolated colang_qa database is required'
assert urlsplit(base).hostname in ('127.0.0.1', 'localhost'), 'QA_API must be local'
count = 0
artifacts = Path(json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'], text=True))['target_directory'])

def call(method, path, body=None, token=None, expected=200):
    global count
    headers = {'Accept': 'application/json'}
    if token:
        headers['Authorization'] = f'Bearer {token}'
    data = None
    if body is not None:
        headers['Content-Type'] = 'application/json'
        data = json.dumps(body).encode()
    request = Request(base + path, data, headers, method=method)
    try:
        response = urlopen(request, timeout=10)
    except HTTPError as error:
        response = error
    raw = response.read().decode()
    assert response.status in ([expected] if isinstance(expected, int) else expected), (method, path, response.status, raw[:500])
    count += 1
    return json.loads(raw) if raw.strip() else None

suffix = str(time.time_ns())
password = 'Qa-password-2026!'
email = f'learner-{suffix}@example.test'
learner = call('POST', '/api/register', {'name': 'QA Learner ' + suffix, 'email': email, 'password': password})
admin = call('POST', '/api/register', {'name': 'QA Admin ' + suffix, 'email': f'admin-{suffix}@example.test', 'password': password})
learner_token = learner['access_token']
psql = shutil.which('psql') or r'C:\Program Files\PostgreSQL\18\bin\psql.exe'
subprocess.run([psql, database, '-v', 'ON_ERROR_STOP=1', '-c', f"INSERT INTO base_admins (user_id) VALUES ({int(admin['user']['id'])});"], check=True, capture_output=True)
admin_token = admin['access_token']
admin['user'] = call('GET', '/api/me', token=admin_token)
assert admin['user']['is_admin']
call('POST', '/api/login', {'email': email, 'password': 'wrong'}, expected=401)
login = call('POST', '/api/login', {'email': email, 'password': password})
assert login['user']['id'] == learner['user']['id']
call('POST', '/api/dict/words', {'word': 'forbidden'}, expected=401)
call('POST', '/api/dict/words', {'word': 'forbidden'}, token=learner_token, expected=403)
call('POST', '/api/asset/domains', {'code': 'forbidden', 'name_en': 'X', 'name_zh': 'X'}, expected=401)
call('POST', '/api/asset/categories', {'code': 'forbidden'}, token=learner_token, expected=403)
call('GET', '/api/admin/stats', token=learner_token, expected=403)
word = call('POST', '/api/dict/words', {'word': f'qaword{suffix}', 'language': 'en', 'difficulty': 1, 'is_active': True}, token=admin_token, expected=[200, 201])
word_id = word['id']
call('GET', f'/api/dict/words/{word_id}')
definition = call('POST', f'/api/dict/words/{word_id}/definitions', {'language': 'zh', 'definition': '回归测试单词', 'is_summary': True}, token=admin_token, expected=[200, 201])
lookup = call('GET', '/api/dict/lookup?word=' + word['word'])
assert lookup['definitions'][0]['definition'] == '回归测试单词'
call('POST', '/api/dict/words/searched', {'word': word['word']}, token=learner_token, expected=[200, 201])
favorite = call('POST', '/api/learn/favorites/toggle', {'entity': 'word', 'record_id': word_id}, token=learner_token)
assert favorite['is_favorited']
assert call('GET', '/api/learn/favorites?entity=word', token=learner_token)[0]['record_id'] == word_id
call('DELETE', f'/api/learn/favorites?entity=word&record_id={word_id}', token=learner_token)
vocabulary = call('POST', '/api/learn/vocabulary', {'word': word['word'], 'word_zh': '回归测试单词'}, token=learner_token, expected=[200, 201])
review = call('GET', '/api/learn/review/items?type=recognition', token=learner_token)
assert any(item['word'] == word['word'] for item in review['items'])
call('POST', '/api/learn/review/submit', {'source': 'vocabulary', 'source_id': vocabulary['id'], 'review_type': 'recognition', 'result': 'correct', 'time_spent_ms': 1000}, token=learner_token)
domain = call('POST', '/api/asset/domains', {'code': f'qa{suffix}', 'name_en': 'QA ' + suffix, 'name_zh': '测试领域 ' + suffix}, token=admin_token)
stage = call('POST', '/api/admin/stages', {'code': f'qa{suffix}', 'name_en': 'QA Stage ' + suffix, 'name_zh': '测试场景 ' + suffix, 'is_active': True, 'difficulty': 1}, token=admin_token)
call('GET', f"/api/admin/stages/{stage['id']}", token=admin_token)
call('PUT', f"/api/admin/stages/{stage['id']}", {'description_zh': '已更新'}, token=admin_token)
subject = call('POST', '/api/admin/read-subjects', {'code': f'qa{suffix}', 'title_en': 'QA Reading ' + suffix, 'title_zh': '测试跟读 ' + suffix}, token=admin_token)
call('GET', f"/api/admin/read-subjects/{subject['id']}", token=admin_token)
sentence = call('POST', '/api/admin/read-sentences', {'subject_id': subject['id'], 'sentence_order': 1, 'content_en': 'Hello world.', 'content_zh': '你好，世界。', 'difficulty': 1}, token=admin_token)
call('PUT', f"/api/admin/read-sentences/{sentence['id']}", {'difficulty': 2}, token=admin_token)
call('POST', '/api/asset/read/practice', {'sentence_id': sentence['id'], 'practice_id': 'qa-' + suffix, 'overall_score': 85, 'pronunciation_score': 85}, token=learner_token)
chat = call('POST', '/api/learn/chats', {'title': 'QA Chat'}, token=learner_token, expected=[200, 201])
call('PUT', f"/api/learn/chats/{chat['id']}", {'title': 'QA Chat Updated'}, token=learner_token)
call('GET', f"/api/learn/chats/{chat['id']}/turns", token=learner_token)
call('POST', f"/api/learn/chats/{chat['id']}/reset", token=learner_token)
call('POST', '/api/learn/heartbeat', {'page_type': 'dictionary'}, token=learner_token)
call('GET', '/api/learn/study-time', token=learner_token)
for path in ['/api/learn/daily', '/api/learn/summary', '/api/me/account-status', '/api/achievements/my', '/api/subscription/plans', '/api/subscription/current', '/api/wallet', '/api/wallet/transactions']:
    call('GET', path, token=learner_token)
for path in ['/api/admin/stats', '/api/admin/stats/finance', '/api/admin/users', f"/api/admin/users/{learner['user']['id']}", '/api/admin/orders', '/api/admin/subscriptions']:
    call('GET', path, token=admin_token)
code = call('POST', '/api/auth/code', {'redirect_uri': 'http://127.0.0.1:43210/callback', 'state': suffix}, token=learner_token)
call('POST', '/api/auth/consume', {'code': code['code'], 'redirect_uri': code['redirect_uri']})
call('POST', '/api/auth/consume', {'code': code['code'], 'redirect_uri': code['redirect_uri']}, expected=401)
call('POST', '/api/auth/code', {'redirect_uri': 'https://example.test/steal', 'state': suffix}, token=learner_token, expected=400)
call('GET', '/api/does-not-exist', expected=404)
fixture = {'learner': learner, 'admin': admin, 'password': password, 'email': email, 'word': word, 'stage': stage, 'subject': subject, 'sentence': sentence, 'chat': chat}
(artifacts / 'qa-fixture.json').write_text(json.dumps(fixture), encoding='utf-8')
print(f'{count} API regression checks passed. Fixtures: {artifacts / "qa-fixture.json"}')
