import createVAD, {VADMode, VADEvent} from './vendor/lib/index.js';

// Audio device adapter. Chat requests, history and cancellation remain in Rust.
// All pre-roll and VAD stay on this device; only completed utterances are emitted.
export function wavBase64(frames) {
  const samples=frames.reduce((n,f)=>n+f.length,0), buffer=new ArrayBuffer(44+samples*2), view=new DataView(buffer);
  const word=(offset,text)=>[...text].forEach((ch,i)=>view.setUint8(offset+i,ch.charCodeAt(0)));
  word(0,'RIFF');view.setUint32(4,36+samples*2,true);word(8,'WAVE');word(12,'fmt ');
  view.setUint32(16,16,true);view.setUint16(20,1,true);view.setUint16(22,1,true);view.setUint32(24,16000,true);
  view.setUint32(28,32000,true);view.setUint16(32,2,true);view.setUint16(34,16,true);word(36,'data');view.setUint32(40,samples*2,true);
  let offset=44;for(const frame of frames)for(const sample of frame){view.setInt16(offset,sample,true);offset+=2;}
  const bytes=new Uint8Array(buffer);let binary='';for(let i=0;i<bytes.length;i+=8192)binary+=String.fromCharCode(...bytes.subarray(i,i+8192));
  return btoa(binary);
}

export class VoiceSession {
  constructor(callback,output='provider') {
    this.output=output;this.expectedSequence=0;this.audioBytes=0;
    this.callback=callback;this.closed=false;this.phase='opening';this.epoch=0;
    this.preRoll=[];this.frames=[];this.voiced=0;this.speechFrames=0;this.lastFrame=0;
    this.spoken='';this.sentences=[];this.speaking=false;this.replyDone=false;
    this.visibility=()=>{if(document.hidden)this.close();};this.pagehide=()=>this.close();
    this.key=event=>{if(event.key==='Escape')this.close();};
    document.addEventListener('visibilitychange',this.visibility);window.addEventListener('pagehide',this.pagehide);window.addEventListener('keydown',this.key,true);
  }
  emit(type,value='') { if(!this.closed)this.callback(type,value); }
  status(phase,text) { this.phase=phase;this.emit('status',text); }
  async open() {
    let stream;
    try {
      if(!navigator.mediaDevices?.getUserMedia||!globalThis.AudioWorkletNode||(this.output==='browser'&&!globalThis.speechSynthesis))throw new Error('当前浏览器不支持连续语音，请使用录音按钮');
      this.context=new AudioContext();await this.context.resume();
      const VAD=await createVAD();if(this.closed)return;
      this.vad=new VAD(VADMode.AGGRESSIVE,16000);
      stream=await navigator.mediaDevices.getUserMedia({audio:{channelCount:1,echoCancellation:true,noiseSuppression:true,autoGainControl:true}});
      if(this.closed)return;
      this.stream=stream;stream=null;
      this.echoCancellation=this.stream.getAudioTracks()[0].getSettings().echoCancellation===true;
      this.stream.getAudioTracks()[0].addEventListener('ended',()=>this.close());
      await this.context.audioWorklet.addModule('/voice/recorder.worklet.js');if(this.closed)return;
      this.source=this.context.createMediaStreamSource(this.stream);this.node=new AudioWorkletNode(this.context,'colang-recorder');
      this.silent=this.context.createGain();this.silent.gain.value=0;
      this.source.connect(this.node);this.node.connect(this.silent);this.silent.connect(this.context.destination);
      this.node.port.onmessage=({data})=>this.frame(new Int16Array(data.pcm));
      this.context.onstatechange=()=>{if(this.context.state==='suspended')this.close();};
      this.awaitReply();
    } catch(error) { this.emit('error',error.message||'无法开启麦克风');this.close(); }
    finally {stream?.getTracks().forEach(track=>track.stop());}
  }
  frame(pcm) {
    if(this.closed)return;
    const now=performance.now();
    if(this.lastFrame&&now-this.lastFrame>200) {this.preRoll=[];this.voiced=0;if(this.phase==='recording'){this.emit('error','录音发生中断，请重新开启语音');this.close();return;}}
    this.lastFrame=now;
    const result=this.vad.processFrame(pcm);
    if(result===VADEvent.ERROR){this.emit('error','语音检测失败，请重新开启语音');this.close();return;}
    const speech=result===VADEvent.VOICE;
    if(this.phase==='recording') {
      this.frames.push(pcm);if(speech){this.lastSpeech=now;this.speechFrames++;}
      if((this.speechFrames>=8&&now-this.lastSpeech>1300)||this.frames.length>=750)this.finish();
      return;
    }
    this.preRoll.push(pcm);if(this.preRoll.length>25)this.preRoll.shift();
    const playing=this.speaking||this.sentences.length>0;
    // No energy-only playback interruption; require browser AEC and actual VAD.
    const allowed=!playing||this.echoCancellation;
    this.voiced=speech&&allowed&&now>=this.guard ? this.voiced+1 : 0;
    if(this.voiced>=(playing?8:6)) {
      const frames=this.preRoll.slice();this.interrupt();
      this.frames=frames;this.speechFrames=this.voiced;this.lastSpeech=now;this.voiced=0;this.preRoll=[];
      clearTimeout(this.idle);this.status('recording','正在听，停顿后自动发送');
    }
  }
  finish() {
    if(this.phase!=='recording')return;
    const frames=this.frames;this.frames=[];
    if(this.speechFrames<8){this.awaitReply();return;}
    this.status('thinking','正在识别与回复，可继续说话打断');this.guard=performance.now()+400;this.preRoll=[];
    this.emit('audio',wavBase64(frames));
  }
  stopSound() {
    this.epoch++;this.sentences=[];this.speaking=false;this.spoken='';this.replyDone=false;
    this.assembly=null;this.expectedSequence=0;this.audioBytes=0;
    if(this.playingSource){this.playingSource.onended=null;try{this.playingSource.stop();}catch{}this.playingSource.disconnect();this.playingSource=null;}
    if(this.output==='browser')globalThis.speechSynthesis?.cancel();
  }
  waiting() { if(this.closed)return;this.stopSound();clearTimeout(this.idle);this.status('thinking','正在回复，可继续说话打断'); }
  interrupt() { this.stopSound();this.emit('interrupt'); }
  reply(text,done) {
    if(this.output!=='browser'||this.closed||this.phase==='recording')return;
    if(!text.startsWith(this.spoken)){this.failAudio('回复文字已变更，已停止朗读');return;}
    let rest=text.slice(this.spoken.length), end;
    while((end=rest.search(/[.!?。！？](?:\s|$)/))>=0) {
      const phrase=rest.slice(0,end+1);this.spoken+=phrase;this.sentences.push(phrase);rest=rest.slice(end+1);
    }
    if(done&&rest.trim()){this.spoken+=rest;this.sentences.push(rest);}
    this.replyDone=done;this.speakNext();
  }
  audioEvent(frame) {
    if(this.output!=='provider'||this.closed||this.phase==='recording')return;
    const data=frame.data;
    try {
      switch(frame.event) {
        case 'audio_start':
          if(this.assembly||data.sequence!==this.expectedSequence||!Number.isInteger(data.bytes)||data.bytes<=0||data.bytes>2*1024*1024||this.audioBytes+data.bytes>10*1024*1024)throw new Error('回复音频顺序或大小无效');
          this.audioBytes+=data.bytes;this.assembly={sequence:data.sequence,bytes:new Uint8Array(data.bytes),received:0};break;
        case 'audio_chunk': {
          const part=this.assembly;
          if(!part||part.sequence!==data.sequence||data.offset!==part.received)throw new Error('回复音频分片顺序无效');
          const bytes=Uint8Array.from(atob(data.audio_base64),c=>c.charCodeAt(0));
          if(!bytes.length||bytes.length>24*1024||part.received+bytes.length>part.bytes.length)throw new Error('回复音频分片大小无效');
          part.bytes.set(bytes,part.received);part.received+=bytes.length;break;
        }
        case 'audio_end': {
          const part=this.assembly;
          if(!part||part.sequence!==data.sequence||part.received!==part.bytes.length)throw new Error('回复音频不完整');
          this.assembly=null;this.expectedSequence++;this.sentences.push(part.bytes.buffer);this.speakNext();break;
        }
        case 'voice_done':
          if(this.assembly||data.segments!==this.expectedSequence)throw new Error('回复语音未完整返回');
          this.replyDone=true;this.speakNext();break;
        case 'voice_error': this.failAudio(data.message||'用户 TTS 服务未完成语音合成');break;
      }
    } catch(error) { this.failAudio(error.message||'回复音频无效'); }
  }
  failAudio(message) { this.emit('error',message);this.close(false); }
  speakNext() {
    if(this.closed||this.speaking)return;
    const item=this.sentences.shift();
    if(item===undefined){if(this.replyDone){this.spoken='';this.replyDone=false;this.awaitReply();}return;}
    const epoch=this.epoch;this.speaking=true;
    if(this.output==='provider'){void this.playSegment(item,epoch);return;}
    const utterance=new SpeechSynthesisUtterance(item);utterance.lang='en-US';this.speakingStatus();
    utterance.onend=()=>{if(this.closed||epoch!==this.epoch)return;this.speaking=false;this.speakNext();};
    utterance.onerror=event=>{if(this.closed||epoch!==this.epoch)return;this.failAudio(`朗读未完成：${event.error}`);};
    speechSynthesis.speak(utterance);
  }
  speakingStatus() {
    this.guard=performance.now()+200;this.status('speaking',this.echoCancellation?'正在朗读，可直接说话打断':'正在朗读，可点击打断；此设备未确认回声消除');
  }
  async playSegment(bytes,epoch) {
    try {
      const buffer=await this.context.decodeAudioData(bytes);
      if(this.closed||epoch!==this.epoch)return;
      if(!buffer.length||buffer.duration>30)throw new Error('回复音频为空或过长');
      const source=this.context.createBufferSource();source.buffer=buffer;source.connect(this.context.destination);this.playingSource=source;
      source.onended=()=>{source.disconnect();if(this.closed||epoch!==this.epoch)return;this.playingSource=null;this.speaking=false;this.speakNext();};
      this.speakingStatus();source.start();
    } catch(error) { if(!this.closed&&epoch===this.epoch)this.failAudio(error.message||'用户 TTS 音频无法播放'); }
  }
  awaitReply() {
    this.guard=performance.now()+400;this.voiced=0;this.preRoll=[];this.lastFrame=0;
    this.status('awaiting','我在听，直接说话即可');
    clearTimeout(this.idle);this.idle=setTimeout(()=>this.close(),30000);
  }
  close(notify=true) {
    if(this.closed)return;
    if(notify)this.interrupt();else this.stopSound();this.closed=true;this.epoch++;clearTimeout(this.idle);
    document.removeEventListener('visibilitychange',this.visibility);window.removeEventListener('pagehide',this.pagehide);window.removeEventListener('keydown',this.key,true);
    if(this.node)this.node.port.onmessage=null;
    this.stream?.getTracks().forEach(track=>track.stop());this.source?.disconnect();this.node?.disconnect();this.silent?.disconnect();
    if(this.context){this.context.onstatechange=null;void this.context.close();}this.vad?.destroy();this.frames=[];this.preRoll=[];
    this.callback('closed','连续语音已结束');
  }
}
