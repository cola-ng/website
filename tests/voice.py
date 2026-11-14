"""Request cancellation through the real API/DB, with a loopback-only provider."""
import base64
import io
import wave
import json
import os
import time
import uuid
from urllib.request import Request, urlopen
from urllib.error import HTTPError
from urllib.parse import urlsplit

base = os.environ['QA_API']
assert urlsplit(base).hostname in ('127.0.0.1', 'localhost')
token = None

def call(path, body, expected=200, auth=True, request_id=None, voice_mode=None, method="POST"):
    headers = {'Content-Type': 'application/json'}
    if auth and token:
        headers['Authorization'] = 'Bearer ' + token
    if request_id:
        headers['X-Conversation-Request'] = request_id
    if voice_mode: headers["X-Conversation-Voice"] = voice_mode
    try:
        response = urlopen(Request(base + path, data=json.dumps(body).encode(), headers=headers,method=method), timeout=20)
    except HTTPError as error:
        response = error
    if path.endswith('/stream') and response.status == 200:
        assert expected == 200, (path, 'unexpected successful stream')
        return response
    raw = response.read().decode()
    assert response.status == expected, (path, response.status, raw[:300])
    return json.loads(raw)

account = call('/api/register', {'name': 'Voice QA', 'email': f'voice-{uuid.uuid4().hex}@example.test', 'password': 'VoiceQATest123!'})
token = account['access_token']
personal=call('/api/ai-providers', {'provider_type': 'zhipu', 'settings': {'api_key': 'qa-slow-test-key', 'chat_model':'glm-4-flash'}, 'set_active': True}, 201)
chat = call('/api/learn/chats', {'title': 'Voice cancellation QA'}, 201)['id']
path = f'/api/learn/chats/{chat}'
body = {'type': 'text', 'message': 'Hello'}
call(path + '/cancel', {'request_id': uuid.uuid4().hex}, 401, auth=False)
call(f'/api/learn/chats/{chat + 1000000}/cancel', {'request_id': uuid.uuid4().hex}, 404)
call(path + '/cancel', {'request_id': 'invalid'}, 400)
# Cancel may arrive before the initial POST: it must not call an upstream or save turns.
early = uuid.uuid4().hex
call(path + '/cancel', {'request_id': early})
call(path + '/send/stream', body, 409, request_id=early)
# ASR can be interrupted before the user/assistant pair exists.
wave_bytes = io.BytesIO()
with wave.open(wave_bytes, 'wb') as wav:
    wav.setparams((1, 2, 16000, 0, 'NONE', 'not compressed'))
    wav.writeframes(b'\0' * 32000)
asr_id = uuid.uuid4().hex
with call(path + '/send/stream', {'type':'audio', 'audio_base64':base64.b64encode(wave_bytes.getvalue()).decode()}, request_id=asr_id) as stream:
    for line in stream:
        if line.startswith(b'data: '):
            frame = json.loads(line[6:])
            if frame['event'] == 'phase': break
    call(path + '/cancel', {'request_id':asr_id})
    remaining = [json.loads(line[6:]) for line in stream if line.startswith(b'data: ')]
    assert not any(frame['event']=='accepted' for frame in remaining)
# Cancel before a delayed upstream returns; wait for cancellation commit before next send.
request_id = uuid.uuid4().hex
response = call(path + '/send/stream', body, request_id=request_id)
ai_id = None
for line in response:
    if line.startswith(b'data: '):
        frame = json.loads(line[6:])
        if frame['event'] == 'accepted':
            ai_id = frame['data']['ai_turn_id']
            break
assert ai_id
call(path + '/cancel', {'request_id': request_id})
response.close()
def turn():
    return json.load(urlopen(Request(base + f'/api/learn/chats/turns/{ai_id}', headers={'Authorization': 'Bearer ' + token})))
assert turn()['status'] == 'cancelled'
second = uuid.uuid4().hex
with call(path + '/send/stream', body, request_id=second) as stream:
    frames = [json.loads(line[6:]) for line in stream if line.startswith(b'data: ')]
assert any(frame['event'] == 'done' for frame in frames), frames
# A late repeat cancel targets the old request, leaving the next reply intact.
call(path + '/cancel', {'request_id': request_id})
assert turn()['status'] == 'cancelled'
print('PASS voice API: auth, ownership, validation, cancel-before-start, ASR cancellation, in-flight cancellation, immediate next turn, stale cancellation')

# Real provider credentials, configured voice/model, segmented SSE and no duplicate whole TTS.
pid=personal['id']
call(f'/api/ai-providers/{pid}', {'settings':{'api_key':'qa-personal-key','tts_voice':'jam'}}, method='PUT')
upstream=os.environ['QA_PROVIDER_URL']
def counts():return json.load(urlopen(upstream))
before=counts()
with call(path+'/send/stream', {'type':'text','message':'Voice ordered test'},request_id=uuid.uuid4().hex,voice_mode='provider') as response:
    frames=[json.loads(line[6:]) for line in response if line.startswith(b'data: ')]
assert frames[-1]['event']=='voice_done' and frames[-1]['data']['segments']==2
assert next(i for i,f in enumerate(frames) if f['event']=='audio_start') < next(i for i,f in enumerate(frames) if f['event']=='done')
starts=[f for f in frames if f['event']=='audio_start'];assert [f['data']['sequence'] for f in starts]==[0,1]
for start in starts:
    sequence=start['data']['sequence'];parts=[f['data'] for f in frames if f['event']=='audio_chunk' and f['data']['sequence']==sequence]
    audio=b''
    for part in parts:
        assert part['offset']==len(audio);audio+=base64.b64decode(part['audio_base64'])
    assert len(audio)==start['data']['bytes'] and audio.startswith(b'RIFF')
after=counts()
assert after.get('personal:tts:jam:glm-tts',0)-before.get('personal:tts:jam:glm-tts',0)==2
assert after.get('platform:other',0)==before.get('platform:other',0)
assert after.get('personal:tts-input:First sentence. Second sentence.',0)==before.get('personal:tts-input:First sentence. Second sentence.',0)
# Explicit browser selection invokes no server TTS.
before=counts().get('personal:other',0)
with call(path+'/send/stream',body,request_id=uuid.uuid4().hex,voice_mode='browser') as stream:
    assert any(json.loads(line[6:])['event']=='done' for line in stream if line.startswith(b'data: '))
assert counts().get('personal:other',0)==before
# Provider denial retains completed text and reports voice_error, without platform fallback.
call(f'/api/ai-providers/{pid}',{'settings':{'api_key':'qa-voice-denied-key'}},method='PUT')
with call(path+'/send/stream',body,request_id=uuid.uuid4().hex,voice_mode='provider') as stream:
    frames=[json.loads(line[6:]) for line in stream if line.startswith(b'data: ')]
assert any(f['event']=='done' for f in frames) and any(f['event']=='voice_error' for f in frames)
assert not any(f['event']=='audio_start' for f in frames) and frames[-1]['event']=='voice_done'
# TTS disabled is rejected before model generation, but explicit browser voice still works.
call(f'/api/ai-providers/{pid}',{'activate_for':['chat','asr']},method='PUT')
before=counts().get('voice-denied:chat',0)
call(path+'/send/stream',body,400,request_id=uuid.uuid4().hex,voice_mode='provider')
assert counts().get('voice-denied:chat',0)==before
print('PASS configured TTS API: selected credentials/voice/model, early ordered audio, no duplicate synthesis, explicit browser mode, provider failure and disabled TTS')

# Dedicated account for the browser's real SSE + configured TTS/preview check.
call(f'/api/ai-providers/{pid}',{'settings':{'api_key':'qa-personal-key','tts_voice':'jam'},'activate_for':['chat','tts','asr']},method='PUT')
from pathlib import Path
Path(os.environ['QA_ARTIFACTS'],'qa-provider-voice-fixture.json').write_text(json.dumps({'auth':account,'chat_id':chat}))
