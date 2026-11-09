"""Real API/SQL/provider-SDK policy checks, using only a loopback AI upstream."""
import concurrent.futures
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
from urllib.error import HTTPError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen

base, database, upstream = os.environ['QA_API'], os.environ['QA_DATABASE_URL'], os.environ['QA_PROVIDER_URL']
assert urlsplit(database).hostname in ('127.0.0.1', 'localhost') and urlsplit(database).path == '/colang_qa'
assert all(urlsplit(u).hostname in ('127.0.0.1', 'localhost') for u in [base, upstream])
artifacts = Path(os.environ['QA_ARTIFACTS'])
fixture = json.loads((artifacts / 'qa-fixture.json').read_text())
token = fixture['learner']['access_token']
admin = fixture['admin']['access_token']
uid = int(fixture['learner']['user']['id'])
chat = int(fixture['chat']['id'])
psql = shutil.which('psql')
count = 0


def sql(query):
    return subprocess.check_output([psql, database, '-XAt', '-v', 'ON_ERROR_STOP=1', '-c', query], text=True).splitlines()[0]


def call(method, path, body=None, auth=token, expected=200):
    global count
    headers = {'Content-Type': 'application/json', 'Accept': 'text/event-stream' if path.endswith('/stream') else 'application/json'}
    if auth:
        headers['Authorization'] = 'Bearer ' + auth
    request = Request(base + path, data=json.dumps(body).encode() if body is not None else None, headers=headers, method=method)
    try:
        response = urlopen(request, timeout=30)
    except HTTPError as error:
        response = error
    raw = response.read().decode()
    assert response.status == expected, (method, path, response.status, raw[:250])
    count += 1
    return json.loads(raw) if raw.strip() else None


def accesses():
    return json.load(urlopen(upstream, timeout=5))


assert call('GET', '/api/ai-providers/access')['source'] == 'none'
for auth in [token, admin]:
    call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'}, auth=auth, expected=402)
call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'}, auth=None, expected=401)
for endpoint in ['send', 'send/stream']:
    call('POST', f'/api/learn/chats/{chat}/{endpoint}', {'type': 'text', 'message': 'Hello'}, expected=402)
call('POST', '/api/learn/pronunciation/evaluate', {'audio_base64': 'UklGRg==', 'reference_text': 'Hello'}, expected=402)
assert accesses().get('platform:chat', 0) == 0
call('POST', '/api/ai-providers', {'provider_type': 'zhipu', 'settings': {}}, expected=400)
personal = call('POST', '/api/ai-providers', {'provider_type': 'zhipu', 'settings': {'api_key': 'qa-personal-key'}, 'set_active': True}, expected=201)
pid = personal['id']
assert personal['settings']['api_key'] == '********' and 'qa-personal-key' not in json.dumps(personal)
assert call('GET', '/api/ai-providers/access')['source'] == 'personal'
for method in ['GET', 'PUT', 'DELETE']:
    call(method, f'/api/ai-providers/{pid}', {} if method == 'PUT' else None, auth=admin, expected=404)
call('POST', f'/api/ai-providers/{pid}/activate', auth=admin, expected=404)
call('POST', f'/api/ai-providers/{pid}/test', auth=admin, expected=404)
call('PUT', f'/api/ai-providers/{pid}', {'settings': {'chat_model': 'qa-model', 'api_key': '********'}})
assert sql(f"SELECT settings->>'api_key' FROM user_ai_providers WHERE id={pid}") == 'qa-personal-key'
assert call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'})['translated'] == 'personal translation'
request = Request(base + f'/api/learn/chats/{chat}/send/stream', data=json.dumps({'type': 'text', 'message': 'Hello from my own key'}).encode(), headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token})
with urlopen(request, timeout=30) as response:
    frames = [json.loads(line[6:]) for line in response if line.startswith(b'data: ')]
assert any(f['event'] == 'done' and f['data']['ai_turn']['content_en'] == 'Hello from your own provider.' for f in frames)
count += 1
assert accesses().get('personal:chat', 0) >= 2 and accesses().get('platform:chat', 0) == 0
assert call('POST', f'/api/ai-providers/{pid}/test')['success'] is True
doubao = call('POST', '/api/ai-providers', {'provider_type': 'doubao', 'settings': {'api_key': '中文密钥测试abcdefgh'}, 'set_active': False}, expected=201)
assert doubao['settings']['api_key'] == '********'
with concurrent.futures.ThreadPoolExecutor() as pool:
    list(pool.map(lambda id: call('POST', f'/api/ai-providers/{id}/activate'), [pid, doubao['id']]))
assert sql(f'SELECT count(*) FROM user_ai_providers WHERE user_id={uid} AND is_active') == '1'
call('POST', f'/api/ai-providers/{pid}/activate')
call('POST', '/api/ai-providers', {'provider_type': 'zhipu', 'settings': {'api_key': 'duplicate'}, 'set_active': True}, expected=409)
assert call('GET', '/api/ai-providers/access')['active_provider_id'] == pid
call('DELETE', f"/api/ai-providers/{doubao['id']}")
call('PUT', f'/api/ai-providers/{pid}', {'is_active': False})

suffix = str(time.time_ns())
plan = int(sql(f"INSERT INTO subscription_plans(code,name_en,name_zh,tier,price_credits) VALUES ('qa-policy-{suffix}','QA paid','测试付费','pro',100) RETURNING id"))
sub = int(sql(f"INSERT INTO user_subscriptions(user_id,plan_id,status,expires_at) VALUES ({uid},{plan},'trial',now()+interval '1 day') RETURNING id"))
assert not call('GET', '/api/ai-providers/access')['platform_allowed']
sql(f"UPDATE user_subscriptions SET status='active' WHERE id={sub}")
assert not call('GET', '/api/ai-providers/access')['platform_allowed'], 'Active plan without settled payment must not grant platform keys'
payment = int(sql(f"INSERT INTO payment_orders(user_id,order_no,order_type,amount_credits,amount_cents,currency,provider,status,paid_at) VALUES ({uid},'qa-{suffix}','subscription',100,100,'CNY','qa','paid',now()) RETURNING id"))
sql(f'UPDATE user_subscriptions SET last_payment_id={payment} WHERE id={sub}')
assert call('GET', '/api/ai-providers/access')['source'] == 'platform'
assert call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'})['translated'] == 'platform translation'
used = accesses().get('platform:chat', 0)
assert used == 1
call('PUT', f'/api/ai-providers/{pid}', {'settings': {'api_key': 'qa-invalid-key'}, 'is_active': True})
call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'}, expected=500)
assert accesses().get('platform:chat', 0) == used, 'Personal-key failures must never fall back to platform'
call('PUT', f'/api/ai-providers/{pid}', {'is_active': False})
for change in ["expires_at=now()-interval '1 second'", "expires_at=now()+interval '1 day',status='trial'", "status='cancelled'", "status='active',started_at=now()+interval '1 day'"]:
    sql(f'UPDATE user_subscriptions SET {change} WHERE id={sub}')
    assert not call('GET', '/api/ai-providers/access')['platform_allowed']
    call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'}, expected=402)
assert accesses().get('platform:chat', 0) == used
sql(f'DELETE FROM user_subscriptions WHERE id={sub}')
sql(f'DELETE FROM payment_orders WHERE id={payment}')
sql(f'DELETE FROM subscription_plans WHERE id={plan}')
call('PUT', f'/api/ai-providers/{pid}', {'settings': {'api_key': 'qa-personal-key'}, 'is_active': True})
assert not call('GET', '/api/ai-providers/access')['platform_allowed']
assert call('POST', '/api/dict/translate', {'text': 'Hello', 'target': 'zh'})['translated'] == 'personal translation'
(artifacts / 'qa-provider-policy.json').write_text(json.dumps({'passed': True, 'checks': count, 'upstream': accesses()}))
print(f'{count} provider policy checks passed: personal credentials, paid-only platform, no failure fallback, expiry and isolation.')
