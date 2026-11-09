"""OAuth protocol/state regressions. No requests leave the isolated QA server."""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
from urllib.error import HTTPError
from urllib.parse import parse_qs, urlsplit
from urllib.request import build_opener, HTTPRedirectHandler, Request

base = os.environ['QA_API']
database = os.environ['QA_DATABASE_URL']
assert urlsplit(base).hostname in ('127.0.0.1', 'localhost')
assert urlsplit(database).hostname in ('127.0.0.1', 'localhost') and urlsplit(database).path == '/colang_qa'
artifacts = Path(json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1'], text=True))['target_directory'])
fixture = json.loads((artifacts / 'qa-fixture.json').read_text())
psql = shutil.which('psql')
count = 0

class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *args):
        return None

def call(method, path, body=None, cookie=None, token=None, expected=200):
    global count
    headers = {'Accept': 'application/json'}
    if cookie:
        headers['Cookie'] = cookie
    if token:
        headers['Authorization'] = 'Bearer ' + token
    if body is not None:
        headers['Content-Type'] = 'application/json'
    try:
        response = build_opener(NoRedirect).open(Request(base + path, None if body is None else json.dumps(body).encode(), headers, method=method), timeout=10)
    except HTTPError as error:
        response = error
    data = response.read().decode()
    assert response.status in ([expected] if isinstance(expected, int) else expected), (method, path, response.status, data[:200])
    count += 1
    return response, json.loads(data) if data.startswith('{') else data

def sql(statement):
    return subprocess.check_output([psql, database, '-XAt', '-v', 'ON_ERROR_STOP=1', '-c', statement], text=True).strip()

def quoted(value):
    return "'" + value.replace("'", "''") + "'"

def hashed(value):
    return hashlib.sha256(value.encode()).hexdigest()

response, providers = call('GET', '/api/auth/providers')
assert {p['id'] for p in providers['providers']} == {'google', 'github'}
assert response.headers['Cache-Control'] == 'no-store'
for provider, host in [('github', 'github.com'), ('google', 'accounts.google.com')]:
    response, _ = call('GET', f'/api/auth/{provider}/start?next=//evil.example', expected=302)
    target = urlsplit(response.headers['Location'])
    assert target.hostname == host
    params = parse_qs(target.query)
    assert params['code_challenge_method'] == ['S256']
    assert 'secret' not in params
    state = params['state'][0]
    cookie = response.headers['Set-Cookie'].split(';', 1)[0]
    assert 'HttpOnly' in response.headers['Set-Cookie'] and 'SameSite=Lax' in response.headers['Set-Cookie']
    record = json.loads(sql('SELECT redirect_uri FROM oauth_login_sessions WHERE state=' + quoted(hashed(state))))
    assert record['next'] == '/me'
    assert record['browser_hash'] == hashed(cookie.split('=', 1)[1])
    assert record['verifier'] != params['code_challenge'][0]
    # A stolen state cannot be used from another browser or for another provider.
    response, _ = call('GET', f'/api/auth/{provider}/callback?state={state}&error=access_denied', cookie='colang_oauth=wrong', expected=302)
    assert 'invalid_state' in response.headers['Location']
    other = 'google' if provider == 'github' else 'github'
    response, _ = call('GET', f'/api/auth/{other}/callback?state={state}&error=access_denied', cookie=cookie, expected=302)
    assert 'invalid_state' in response.headers['Location']
    response, _ = call('GET', f'/api/auth/{provider}/callback?state={state}&error=access_denied', cookie=cookie, expected=302)
    assert 'access_denied' in response.headers['Location']
    response, _ = call('GET', f'/api/auth/{provider}/callback?state={state}&error=access_denied', cookie=cookie, expected=302)
    assert 'invalid_state' in response.headers['Location']

call('POST', '/api/me/oauth/github/start', {}, expected=401)
call('POST', '/api/me/oauth', {'provider': 'github', 'provider_user_id': 'forged'}, token=fixture['learner']['access_token'], expected=400)
response, linked = call('POST', '/api/me/oauth/github/start', {}, token=fixture['learner']['access_token'])
state = parse_qs(urlsplit(linked['authorization_url']).query)['state'][0]
record = json.loads(sql('SELECT redirect_uri FROM oauth_login_sessions WHERE state=' + quoted(hashed(state))))
assert record['link_user'] == fixture['learner']['user']['id'] and record['next'] == '/me'

def completion(user, expired=False):
    code, browser = secrets.token_urlsafe(32), secrets.token_urlsafe(32)
    metadata = {'browser_hash': hashed(browser), 'verifier': '', 'next': '/review', 'link_user': None, 'complete_user': user}
    expiry = "now() - interval '1 minute'" if expired else "now() + interval '1 minute'"
    sql("INSERT INTO oauth_login_sessions(provider,state,redirect_uri,expires_at) VALUES ('complete'," + quoted(hashed(code)) + ',' + quoted(json.dumps(metadata)) + ',' + expiry + ')')
    return code, 'colang_oauth=' + browser

user = fixture['learner']['user']['id']
code, cookie = completion(user)
call('POST', '/api/auth/complete', {'code': code}, cookie='colang_oauth=other', expected=401)
response, auth = call('POST', '/api/auth/complete', {'code': code}, cookie=cookie)
assert auth['user']['id'] == user and auth['access_token'] and auth['next'] == '/review'
assert response.headers['Cache-Control'] == 'no-store' and 'Max-Age=0' in response.headers['Set-Cookie']
call('POST', '/api/auth/complete', {'code': code}, cookie=cookie, expected=401)
code, cookie = completion(user, True)
call('POST', '/api/auth/complete', {'code': code}, cookie=cookie, expected=401)
code, cookie = completion(user)
def race(_):
    return call('POST', '/api/auth/complete', {'code': code}, cookie=cookie, expected=(200, 401))[0].status
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
    results = list(executor.map(race, range(2)))
assert sorted(results) == [200, 401], 'Completion codes must be single-use even concurrently'

# Social-only users cannot remove their final login method; simultaneous unlinks
# of two methods must leave one available.
suffix = secrets.token_hex(8)
social_user = int(sql("INSERT INTO base_users(name,email) VALUES (" + quoted('OAuth unlink ' + suffix) + ',' + quoted(suffix + '@example.test') + ') RETURNING id').splitlines()[0])
code, cookie = completion(social_user)
_, social = call('POST', '/api/auth/complete', {'code': code}, cookie=cookie)
ids = []
for provider in ['google', 'github']:
    ids.append(int(sql("INSERT INTO oauth_identities(provider,provider_user_id,user_id,verified_at) VALUES (" + quoted(provider) + ',' + quoted(suffix) + ',' + str(social_user) + ',now()) RETURNING id').splitlines()[0]))
def unlink(identity):
    return call('DELETE', '/api/me/oauth/' + str(identity), token=social['access_token'], expected=(200, 400))[0].status
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
    results = list(executor.map(unlink, ids))
assert sorted(results) == [200, 400]
assert sql('SELECT count(*) FROM oauth_identities WHERE user_id=' + str(social_user)) == '1'
_, status = call('GET', '/api/me/account-status', token=social['access_token'])
assert not status['has_password'] and len(status['linked_identities']) == 1
remaining = status['linked_identities'][0]['id']
call('DELETE', '/api/me/oauth/' + str(remaining), token=social['access_token'], expected=400)
new_password = 'QA-social-password-2026!'
call('POST', '/api/me/change-password', {'new_password': new_password}, token=social['access_token'])
call('DELETE', '/api/me/oauth/' + str(remaining), token=social['access_token'])
_, status = call('GET', '/api/me/account-status', token=social['access_token'])
assert status['has_password'] and status['linked_identities'] == []
assert sql('SELECT count(*) FROM oauth_identities WHERE user_id=' + str(social_user)) == '0'
_, password_login = call('POST', '/api/login', {'email': suffix + '@example.test', 'password': new_password})
assert password_login['user']['id'] == social_user
print(f'{count} OAuth HTTP checks passed, including state/browser binding, PKCE, single-use completion and unlink races.')
