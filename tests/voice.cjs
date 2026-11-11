// Real WASM UI + AudioWorklet + local WebRTC VAD. Only AI/speaker output is stubbed.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');

exports.run=async({browser,fixture,base,artifacts})=>{
  const context=await browser.newContext({permissions:['microphone'],viewport:{width:390,height:844}});
  const page=await context.newPage(), errors=[];context.setDefaultTimeout(15000);
  page.on('pageerror',e=>errors.push(e.message));
  await page.addInitScript(()=>{
    window.qaUtterances=[];window.qaSpeechAuto=true;
    Object.defineProperty(window,'speechSynthesis',{value:{
      cancel(){window.qaCurrentSpeech=null;},
      speak(utterance){window.qaCurrentSpeech=utterance;window.qaUtterances.push(utterance.text);if(window.qaSpeechAuto)setTimeout(()=>{if(window.qaCurrentSpeech===utterance)utterance.onend?.(new Event('end'));},50);}
    }});
    navigator.mediaDevices.getUserMedia=async()=>{
      const context=new AudioContext(),buffer=await context.decodeAudioData(await (await fetch('/qa-voice-speech.wav')).arrayBuffer());
      const source=context.createBufferSource(),gain=context.createGain(),destination=context.createMediaStreamDestination();
      source.buffer=buffer;source.loop=true;gain.gain.value=0;source.connect(gain);gain.connect(destination);source.start();
      const track=destination.stream.getAudioTracks()[0];
      track.getSettings=()=>({echoCancellation:true});
      const stop=track.stop.bind(track);track.stop=()=>{stop();source.stop();void context.close();};
      window.qaMic={context,source,gain,track};return destination.stream;
    };
  });
  await page.route('**/qa-voice-speech.wav',route=>route.fulfill({contentType:'audio/wav',body:fs.readFileSync(path.join(artifacts,'qa-voice-speech.wav'))}));
  const id=fixture.chat.id;let sends=0,cancels=[],bodies=[],headers=[],hold=false,late;
  const user={id:910001,speaker:'user',content_en:'A voice input',status:'completed'};
  const ai={id:910002,speaker:'assistant',content_en:'Hello there! Let us practice.',content_zh:'你好',status:'completed'};
  const frames=[{event:'accepted',data:{user_turn:user,ai_turn_id:ai.id}},{event:'delta',data:{reply_en:'Hello there!',reply_zh:''}},{event:'done',data:{user_turn:user,ai_turn:ai}}];
  await page.route(`**/api/learn/chats/${id}/turns?*`,route=>route.fulfill({json:{items:[],has_prev:false}}));
  await page.route(`**/api/learn/chats/${id}/cancel`,route=>{cancels.push(route.request().postDataJSON().request_id);return route.fulfill({json:{ok:true}});});
  await page.route(`**/api/learn/chats/${id}/send/stream`,async route=>{
    const payload=route.request().postDataJSON();
    const responseFrames=payload.message==='Old input'?[{event:'done',data:{user_turn:user,ai_turn:{...ai,content_en:'STALE OLD REPLY'}}}]:frames;
    sends++;bodies.push(payload);headers.push(route.request().headers()['x-conversation-request']);
    if(hold){await new Promise(resolve=>late=resolve);}
    await route.fulfill({contentType:'text/event-stream',body:responseFrames.map(v=>'data: '+JSON.stringify(v)+'\n\n').join('')}).catch(()=>{});
  });
  try {
    await page.goto(base);await page.locator('main').waitFor();
    await page.evaluate(auth=>{localStorage.setItem('colang.token',auth.access_token);localStorage.setItem('colang.user',JSON.stringify(auth.user));},fixture.learner);
    await page.goto(`${base}/chat/${id}`);
    await page.getByRole('button',{name:'语音设置',exact:true}).click();
    assert.equal(await page.getByRole('button',{name:'用户配置的 TTS',exact:true}).getAttribute('aria-pressed'),'true');
    await page.getByRole('button',{name:'浏览器系统语音',exact:true}).click();await page.keyboard.press('Escape');
    await page.reload();
    assert.equal(await page.evaluate(()=>localStorage.getItem('chat.voice_output')),'browser');
    await page.getByRole('button',{name:'开启连续语音',exact:true}).click();
    await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();
    await page.waitForTimeout(650);assert.equal(sends,0,'silence must stay local');
    await page.evaluate(()=>window.qaMic.gain.gain.value=1);await page.waitForTimeout(1200);
    await page.evaluate(()=>window.qaMic.gain.gain.value=0);
    await page.waitForFunction(()=>window.qaUtterances.length>=2,{},{timeout:15000});
    await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();
    assert.equal(sends,1);assert.equal(bodies[0].type,'audio');
    const wav=Buffer.from(bodies[0].audio_base64,'base64');assert.equal(wav.toString('ascii',0,4),'RIFF');assert.equal(wav.readUInt32LE(24),16000);assert.equal(wav.readUInt16LE(22),1);assert.equal(wav.readUInt16LE(34),16);assert(wav.length>32000);
    assert.match(headers[0],/^web-[\d-]+$/);assert.deepEqual(await page.evaluate(()=>window.qaUtterances),['Hello there!',' Let us practice.']);
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
    await page.screenshot({path:path.join(artifacts,'qa-screenshots/voice-mobile.png'),fullPage:true});
    await page.setViewportSize({width:1280,height:900});await page.screenshot({path:path.join(artifacts,'qa-screenshots/voice-desktop.png'),fullPage:true});await page.setViewportSize({width:390,height:844});
    // A second utterance uses the same open microphone and cancels only the old ID.
    await page.evaluate(()=>{window.qaSpeechAuto=false;window.qaMic.gain.gain.value=1;});await page.waitForTimeout(1100);await page.evaluate(()=>window.qaMic.gain.gain.value=0);
    await page.waitForFunction(()=>window.qaUtterances.length>=3,{},{timeout:15000});
    assert.equal(sends,2);assert(cancels.includes(headers[0]));assert.notEqual(headers[0],headers[1]);
    // True VAD during simulated speaker playback: keep onset and stop old speech.
    await page.evaluate(()=>window.qaMic.gain.gain.value=1);await page.waitForTimeout(1100);await page.evaluate(()=>window.qaMic.gain.gain.value=0);
    await page.waitForFunction(()=>window.qaUtterances.length>=4,{},{timeout:15000});
    assert.equal(sends,3);assert(cancels.includes(headers[1]));
    await page.keyboard.press('Escape');await page.getByRole('button',{name:'开启连续语音',exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.qaMic.track.readyState),'ended');
    const stopped=sends;await page.waitForTimeout(400);assert.equal(sends,stopped);
    // Late old POST completion cannot clear or replace a newer reply.
    hold=true;await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('Old input');await page.getByRole('button',{name:'发送消息',exact:true}).click();
    await page.waitForFunction(()=>document.querySelector('.streaming-reply'));
    await page.getByRole('button',{name:'打断回复',exact:true}).click();
    await page.getByRole('button',{name:'发送消息',exact:true}).waitFor({state:'visible'});
    await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent==='发送消息'&&!b.disabled));
    const old=headers.at(-1);assert(cancels.includes(old));hold=false;
    await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('New input');await page.getByRole('button',{name:'发送消息',exact:true}).click();
    await page.getByText(ai.content_en,{exact:true}).waitFor();late();await page.waitForTimeout(300);
    assert.equal(await page.locator('.streaming-reply').count(),0);assert.equal(await page.getByText('页面已关闭',{exact:true}).count(),0);assert.equal(await page.getByText('STALE OLD REPLY',{exact:true}).count(),0);
    // Navigate away after permission is granted: all capture tracks must be released.
    await page.getByRole('button',{name:'开启连续语音',exact:true}).click();await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();
    await page.setViewportSize({width:1280,height:900});
    await page.getByRole('link',{name:'词典翻译',exact:true}).first().click();await page.getByRole('heading',{name:'词典翻译',exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.qaMic.track.readyState),'ended');
    await page.getByRole('link',{name:'天天唠嗑',exact:true}).first().click();
    await page.goto(`${base}/chat/${id}`);
    await page.getByRole('button',{name:'开启连续语音',exact:true}).click();await page.getByText('我在听，直接说话即可',{exact:true}).waitFor();
    await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));delete document.hidden;});
    await page.getByRole('button',{name:'开启连续语音',exact:true}).waitFor();assert.equal(await page.evaluate(()=>window.qaMic.track.readyState),'ended');
    const engine=await page.evaluate(async()=>{
      const {VoiceSession}=await import('/voice/session.js');
      const events=[], original=performance.now.bind(performance);let now=0;
      Object.defineProperty(performance,'now',{configurable:true,value:()=>now});
      const voice=new VoiceSession((type,value)=>events.push({type,value}));
      let speech=true;voice.vad={processFrame:()=>speech?1:0,destroy(){}};
      voice.phase='speaking';voice.speaking=true;voice.echoCancellation=false;voice.guard=200;
      const frame=n=>{now+=20;voice.frame(new Int16Array(320).fill(n));};
      try {
        for(let n=1;n<=30;n++)frame(n);
        const withoutAEC=voice.phase;
        voice.echoCancellation=true;voice.preRoll=[];voice.voiced=0;voice.guard=now+200;
        for(let n=1;n<=18;n++)frame(n);
        const onset=voice.frames[0][0],recording=voice.phase;
        for(let n=0;n<15;n++)frame(100);
        speech=false;for(let n=0;n<72;n++)frame(0);
        const audio=events.find(e=>e.type==='audio');
        const pcm=audio?new DataView(Uint8Array.from(atob(audio.value),c=>c.charCodeAt(0)).buffer):null;
        return {withoutAEC,recording,onset,wavOnset:pcm?.getInt16(44,true),uploads:events.filter(e=>e.type==='audio').length};
      } finally { voice.close();Object.defineProperty(performance,'now',{configurable:true,value:original}); }
    });
    assert.deepEqual(engine,{withoutAEC:'speaking',recording:'recording',onset:1,wavOnset:1,uploads:1});
    assert.deepEqual(errors,[]);console.log('PASS voice: real VAD/worklet, WAV, follow-up, playback interruption, cancellation, stale replies, onset preservation, AEC gate and cleanup');
  } finally { late?.();await context.close(); }
};
