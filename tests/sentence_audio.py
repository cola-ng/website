"""Real local HTTP/Postgres/file/provider flow. No real keys or external AI calls."""
import concurrent.futures
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import wave
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from urllib.parse import urlsplit

base,database,upstream=(os.environ[k] for k in ['QA_API','QA_DATABASE_URL','QA_PROVIDER_URL'])
assert all(urlsplit(u).hostname in ('127.0.0.1','localhost') for u in [base,database,upstream])
artifacts=Path(os.environ['QA_ARTIFACTS']);space=Path(os.environ['QA_SPACE_PATH'])
fixture=json.loads((artifacts/'qa-fixture.json').read_text())
token=fixture['learner']['access_token'];admin=fixture['admin']['access_token'];uid=fixture['learner']['user']['id'];checks=0
def sql(query):
    return subprocess.check_output([shutil.which('psql'),database,'-XAt','-v','ON_ERROR_STOP=1','-c',query],text=True).splitlines()[0]
def call(method,path,body=None,auth=token,expected=200,binary=False):
    global checks
    headers={'Content-Type':'application/json'}
    if auth:headers['Authorization']='Bearer '+auth
    request=Request(base+path,data=json.dumps(body).encode() if body is not None else None,headers=headers,method=method)
    try:r=urlopen(request,timeout=25)
    except HTTPError as e:r=e
    raw=r.read();assert r.status==expected,(method,path,r.status,raw[:250]);checks+=1
    return raw if binary else json.loads(raw)
def new_sentence(text):
    assert "'" not in text
    return int(sql(f"INSERT INTO dict_sentences(language,sentence) VALUES('en','{text}') RETURNING id"))
def path(id):return f'/api/sentence-audio/dict/{id}'
def generate(id,expected=200):return call('POST',path(id)+'/generate',{},expected=expected)
def review(id,action):return call('POST',f'/api/admin/sentence-audio/{id}/review',{'action':action,'note':'QA review'},auth=admin)
def allow(enabled):call('PUT','/api/admin/sentence-audio/whitelist',{'provider_type':'zhipu','enabled':enabled},auth=admin)
def counts():return json.load(urlopen(upstream))

catalog=call('GET','/api/ai-providers/catalog')
assert catalog==json.loads((Path(__file__).parent/'provider-catalog.json').read_text())
assert {'qwen','deepseek','minimax','kimi','openai','anthropic','gemini','siliconflow','baidu','hunyuan'}<=set(p['id'] for p in catalog)
call('GET','/api/ai-providers/catalog',auth=None,expected=401)
pid=int(sql(f"SELECT id FROM user_ai_providers WHERE user_id={uid} AND provider_type='zhipu'"))
call('PUT',f'/api/ai-providers/{pid}',{'activate_for':[]})
missing=new_sentence('A sentence without a recording.')
assert call('GET',path(missing),auth=None)['audio_url'] is None
assert call('GET',path(missing))['can_generate'] is False
before=counts();generate(missing,400);assert counts()==before
call('POST',path(missing)+'/generate',{},auth=None,expected=401)

# Existing platform files are playable by an anonymous/free user and reusable by exact text.
folder=space/'qa-platform-audio';folder.mkdir(parents=True,exist_ok=True)
with wave.open(str(folder/'existing.wav'),'wb') as w:w.setparams((1,2,16000,0,'NONE','not compressed'));w.writeframes(b'\0'*3200)
subject=int(sql("INSERT INTO asset_read_subjects(code,title_en,title_zh) VALUES('qa-audio','Audio test','音频测试') RETURNING id"))
reading=int(sql(f"INSERT INTO asset_read_sentences(subject_id,sentence_order,content_en,content_zh,audio_path) VALUES({subject},1,'Existing platform recording.','测试','qa-platform-audio/existing.wav') RETURNING id"))
existing=new_sentence('Existing platform recording.')
meta=call('GET',path(existing),auth=None);assert meta['official'] and not meta['can_generate']
assert call('GET',meta['audio_url'],auth=None,binary=True).startswith(b'RIFF')
assert counts()==before

# A separate text-only provider must not replace the personal speech provider.
call('PUT',f'/api/ai-providers/{pid}',{'settings':{'api_key':'qa-personal-key'},'activate_for':['tts','asr']})
text=call('POST','/api/ai-providers',{'provider_type':'deepseek','settings':{'api_key':'qa-invalid-key'},'activate_for':['chat']},expected=201)
access=call('GET','/api/ai-providers/access');assert access['chat_provider_id']==text['id'] and access['tts_provider_id']==pid and access['asr_provider_id']==pid
call('PUT',f"/api/ai-providers/{text['id']}",{'settings':{'tts_model':'speech-2.8-hd'}},expected=400)
call('PUT',f'/api/ai-providers/{pid}',{'settings':{'chat_model':'glm-tts'}},expected=400)
call('PUT',f'/api/ai-providers/{pid}',{'activate_for':['video']},expected=400)
assert call('GET',path(missing))['can_generate']
allowed=generate(missing);assert allowed['status']=='approved'
assert call('GET',allowed['audio_url'],auth=None,binary=True).startswith(b'RIFF')
before=counts();generate(missing,409);assert counts()==before,'existing audio must never spend a key again'
assert call('GET',path(missing),auth=None)['official']
assert sql(f"SELECT provider_type||':'||model||':'||voice FROM sentence_audio_candidates WHERE id={allowed['id']}")=='zhipu:glm-tts:tongtong'

# Admin-controlled whitelist; pending files are private to their contributor and admins.
call('PUT','/api/admin/sentence-audio/whitelist',{'provider_type':'zhipu','enabled':False},expected=403)
call('PUT','/api/admin/sentence-audio/whitelist',{'provider_type':'deepseek','enabled':True},auth=admin,expected=400)
allow(False)
pending_sentence=new_sentence('A candidate awaiting review.')
pending=generate(pending_sentence);assert pending['status']=='pending'
call('GET',pending['audio_url'],auth=None,expected=404)
assert call('GET',pending['audio_url'],auth=admin,binary=True).startswith(b'RIFF')
assert call('GET',pending['audio_url'],binary=True).startswith(b'RIFF')
assert not call('GET',path(pending_sentence),auth=None)['official']
assert call('GET',path(pending_sentence))['status']=='pending'
before=counts();generate(pending_sentence,409);assert counts()==before
call('GET','/api/admin/sentence-audio',expected=403)
queue=call('GET','/api/admin/sentence-audio',auth=admin);assert any(c['id']==pending['id'] and c['sentence']=='A candidate awaiting review.' for c in queue['items'])
call('POST',f"/api/admin/sentence-audio/{pending['id']}/review",{'action':'approve'},expected=403)
assert review(pending['id'],'approve')['status']=='approved'
assert call('GET',path(pending_sentence),auth=None)['official']
call('POST',f"/api/admin/sentence-audio/{pending['id']}/review",{'action':'reject'},auth=admin,expected=409)

rejected=generate(new_sentence('A rejected candidate.'));assert review(rejected['id'],'reject')['status']=='rejected'
assert sql(f"SELECT review_note FROM sentence_audio_candidates WHERE id={rejected['id']}")=='QA review'
stale_id=new_sentence('A sentence before editing.');stale=generate(stale_id)
sql(f"UPDATE dict_sentences SET sentence='A sentence after editing.' WHERE id={stale_id}")
assert review(stale['id'],'approve')['status']=='superseded'
assert call('GET',path(stale_id),auth=None)['audio_url'] is None

# Recheck a whitelist change during generation, and reserve duplicate clicks before the paid call.
slow=new_sentence('Slow audio generation.')
before=counts().get('personal:other',0)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    first=pool.submit(generate,slow);time.sleep(.15)
    generate(slow,409);allow(True);result=first.result()
assert result['status']=='approved' and counts().get('personal:other',0)==before+1

# Active text key alone must not unlock generation or fall back to a platform TTS key.
call('PUT',f'/api/ai-providers/{pid}',{'activate_for':[]})
before=counts();generate(new_sentence('No personal speech provider.'),400);assert counts()==before
call('DELETE',f"/api/ai-providers/{text['id']}")
call('PUT',f'/api/ai-providers/{pid}',{'is_active':True})
(artifacts/'qa-sentence-audio.json').write_text(json.dumps({'passed':True,'checks':checks,'upstream':counts()}))
print(f'{checks} sentence audio/provider catalogue checks passed: public playback, BYOK, modality separation, whitelist, review, stale text and duplicate clicks.')
