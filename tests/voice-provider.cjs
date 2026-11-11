// Actual WebAudio decoding/playback and VAD. No system speech fallback is allowed.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
function wav(seconds,value){
  const samples=Math.floor(seconds*16000),b=Buffer.alloc(44+samples*2);
  b.write('RIFF');b.writeUInt32LE(b.length-8,4);b.write('WAVEfmt ',8);b.writeUInt32LE(16,16);b.writeUInt16LE(1,20);b.writeUInt16LE(1,22);b.writeUInt32LE(16000,24);b.writeUInt32LE(32000,28);b.writeUInt16LE(2,32);b.writeUInt16LE(16,34);b.write('data',36);b.writeUInt32LE(samples*2,40);
  for(let n=0;n<samples;n++)b.writeInt16LE(value,44+n*2);return b;
}
function segment(sequence,bytes){return [
  {event:'audio_start',data:{sequence,bytes:bytes.length,format:'wav'}},
  ...Array.from({length:Math.ceil(bytes.length/24576)},(_,i)=>({event:'audio_chunk',data:{sequence,offset:i*24576,audio_base64:bytes.subarray(i*24576,(i+1)*24576).toString('base64')}})),
  {event:'audio_end',data:{sequence}}
];}
exports.run=async({browser,fixture,base,artifacts})=>{
  const context=await browser.newContext({viewport:{width:390,height:844},permissions:['microphone']});context.setDefaultTimeout(15000);
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    window.qaBrowserSpeech=0;window.qaStarts=[];window.qaStops=0;
    Object.defineProperty(window,'speechSynthesis',{value:{cancel(){},speak(){window.qaBrowserSpeech++;throw new Error('System speech is forbidden in provider mode');}}});
    const start=AudioBufferSourceNode.prototype.start,stop=AudioBufferSourceNode.prototype.stop;
    AudioBufferSourceNode.prototype.start=function(...args){if(!this.loop)window.qaStarts.push({time:performance.now(),duration:this.buffer.duration,sample:this.buffer.getChannelData(0)[10]});return start.apply(this,args);};
    AudioBufferSourceNode.prototype.stop=function(...args){if(!this.loop)window.qaStops++;return stop.apply(this,args);};
    navigator.mediaDevices.getUserMedia=async()=>{
      const context=new AudioContext(),buffer=await context.decodeAudioData(await (await fetch('/qa-voice-speech.wav')).arrayBuffer());
      const source=context.createBufferSource(),gain=context.createGain(),destination=context.createMediaStreamDestination();source.buffer=buffer;source.loop=true;gain.gain.value=0;source.connect(gain);gain.connect(destination);source.start();
      const track=destination.stream.getAudioTracks()[0];track.getSettings=()=>({echoCancellation:true});
      const stop=track.stop.bind(track);track.stop=()=>{stop();source.stop();void context.close();};window.qaMic={gain,track};return destination.stream;
    };
  });
  await page.route('**/qa-voice-speech.wav',route=>route.fulfill({contentType:'audio/wav',body:fs.readFileSync(path.join(artifacts,'qa-voice-speech.wav'))}));
  const id=fixture.chat.id,requests=[],cancels=[];
  let scenario='normal';
  const user={id:920001,speaker:'user',content_en:'Provider voice input',status:'completed'};
  const ai={id:920002,speaker:'assistant',content_en:'Your configured voice is speaking.',content_zh:'使用配置的声音',status:'completed'};
  const done={event:'done',data:{user_turn:user,ai_turn:ai}};
  await page.route(`**/api/learn/chats/${id}/turns?*`,route=>route.fulfill({json:{items:[],has_prev:false}}));
  await page.route(`**/api/learn/chats/${id}/cancel`,route=>{cancels.push(route.request().postDataJSON().request_id);return route.fulfill({json:{ok:true}});});
  await page.route(`**/api/learn/chats/${id}/send/stream`,route=>{
    assert.equal(route.request().headers()['x-conversation-voice'],'provider');requests.push(route.request().headers()['x-conversation-request']);
    const frames=[{event:'accepted',data:{user_turn:user,ai_turn_id:ai.id}},{event:'delta',data:{reply_en:'Your configured voice is speaking.',reply_zh:''}}];
    if(scenario==='failure')frames.push(done,{event:'voice_error',data:{message:'用户 TTS 服务合成失败'}},{event:'voice_done',data:{segments:0}});
    else if(scenario==='invalid')frames.push(done,...segment(0,Buffer.from('invalid audio')),{event:'voice_done',data:{segments:1}});
    else if(scenario==='truncated')frames.push(done);
    else if(scenario==='long')frames.push(...segment(0,wav(4,2000)),done,{event:'voice_done',data:{segments:1}});
    else frames.push(...segment(0,wav(.25,1000)),done,...segment(1,wav(.35,3000)),{event:'voice_done',data:{segments:2}});
    return route.fulfill({contentType:'text/event-stream',body:frames.map(f=>'data: '+JSON.stringify(f)+'\n\n').join('')});
  });
  async function enable(){await page.getByRole('button',{name:'开启连续语音',exact:true}).click();await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();}
  async function send(){await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('Speak with my provider');await page.getByRole('button',{name:'发送消息',exact:true}).click();}
  try {
    await page.goto(base);await page.locator('main').waitFor();await page.evaluate(auth=>{localStorage.setItem('colang.token',auth.access_token);localStorage.setItem('colang.user',JSON.stringify(auth.user));},fixture.learner);
    await page.goto(`${base}/chat/${id}`);await enable();await send();
    await page.waitForFunction(()=>window.qaStarts.length===2);await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();
    const playback=await page.evaluate(()=>window.qaStarts);assert(playback[0].sample<playback[1].sample);assert(playback[1].time-playback[0].time>=200,'segments must play in order');
    assert.equal(await page.evaluate(()=>window.qaBrowserSpeech),0);
    await page.getByRole('button',{name:'语音设置',exact:true}).click();assert(await page.getByRole('button',{name:'浏览器系统语音',exact:true}).isDisabled());await page.keyboard.press('Escape');
    // Dismiss the settings overlay first, then Esc ends continuous voice.
    await page.keyboard.press('Escape');
    await page.getByRole('button',{name:'开启连续语音',exact:true}).waitFor();await enable();scenario='long';await send();await page.waitForFunction(()=>window.qaStarts.length>=3);
    const old=requests.at(-1);scenario='normal';await page.evaluate(()=>window.qaMic.gain.gain.value=1);await page.waitForTimeout(1100);await page.evaluate(()=>window.qaMic.gain.gain.value=0);
    await page.waitForFunction(()=>window.qaStarts.length>=5);await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();assert(cancels.includes(old));assert(await page.evaluate(()=>window.qaStops>=1));
    // Service errors retain completed text, release capture and never invoke system speech.
    scenario='failure';await send();await page.getByText('用户 TTS 服务合成失败',{exact:true}).waitFor();await page.getByRole('button',{name:'开启连续语音',exact:true}).waitFor();assert.equal(await page.evaluate(()=>window.qaMic.track.readyState),'ended');await page.getByText(ai.content_en,{exact:true}).waitFor();
    scenario='invalid';await enable();await send();await page.getByRole('button',{name:'开启连续语音',exact:true}).waitFor();assert.equal(await page.evaluate(()=>window.qaMic.track.readyState),'ended');
    scenario='truncated';await enable();await send();await page.getByText('回复未完整返回，请重试',{exact:true}).waitFor();await page.getByRole('button',{name:'开启连续语音',exact:true}).waitFor();
    // Use the real local API, provider SDK and preview SSE proxy for the final turn.
    const real=JSON.parse(fs.readFileSync(path.join(artifacts,'qa-provider-voice-fixture.json'),'utf8'));
    await page.evaluate(auth=>{localStorage.setItem('colang.token',auth.access_token);localStorage.setItem('colang.user',JSON.stringify(auth.user));},real.auth);
    await page.goto(`${base}/chat/${real.chat_id}`);await enable();
    await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('Voice ordered test');await page.getByRole('button',{name:'发送消息',exact:true}).click();
    await page.waitForFunction(()=>window.qaStarts.length>=1);assert.equal(await page.locator('.streaming-reply').count(),1,'real configured TTS must start before text finishes through the preview proxy');
    await page.waitForFunction(()=>window.qaStarts.length>=2);await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();
    await page.getByText('First sentence. Second sentence.',{exact:true}).last().waitFor();assert.equal(await page.evaluate(()=>window.qaBrowserSpeech),0);await page.keyboard.press('Escape');
    const late=await page.evaluate(async()=>{
      const {VoiceSession}=await import('/voice/session.js');let resolve,starts=0;
      const v=new VoiceSession(()=>{},'provider');v.context={decodeAudioData:()=>new Promise(r=>resolve=r),createBufferSource:()=>{starts++;return {};}};
      v.audioEvent({event:'audio_start',data:{sequence:0,bytes:1}});v.audioEvent({event:'audio_chunk',data:{sequence:0,offset:0,audio_base64:'AA=='}});v.audioEvent({event:'audio_end',data:{sequence:0}});
      v.interrupt();resolve({length:100,duration:.1});await Promise.resolve();v.context=null;v.close();return starts;
    });
    assert.equal(late,0,'late decoding must not revive interrupted audio');assert.equal(await page.evaluate(()=>window.qaBrowserSpeech),0);assert.deepEqual(errors,[]);
    console.log('PASS provider voice: configured mode, real API/SDK/SSE proxy, actual decoding, ordered playback, follow-up, barge-in, service/decode/transport errors, no fallback and stale decode isolation');
  } finally {await context.close();}
};
