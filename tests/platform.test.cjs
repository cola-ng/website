const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm'),fs=require('node:fs'),path=require('node:path');
function platform() {
  let started=0, stopped=0; const listeners={},grants=[];
  const sandbox={console,setTimeout,clearTimeout,
    navigator:{mediaDevices:{getUserMedia:()=>new Promise(resolve=>{grants.push(resolve);})}},
    MediaRecorder:class {static isTypeSupported(){return true;} start(){started++;this.state='recording';} stop(){this.state='inactive';}},
    addEventListener:(name,fn)=>listeners[name]=fn};
  vm.createContext(sandbox);
  const source=fs.readFileSync(path.join(__dirname,'../front/src/browser/platform.js'),'utf8')
    .replace(/^import .*;$/m,'').replace(/export /g,'');
  vm.runInContext(source,sandbox);
  return {sandbox,listeners,grant:(index=0)=>grants.splice(index,1)[0]({getTracks:()=>[{stop(){stopped++;}}]}),counts:()=>({started,stopped})};
}
test('late microphone permission after pagehide cannot start recording',async()=>{
  const h=platform(),recording=h.sandbox.startRecording();
  h.listeners.pagehide(); h.grant();
  await assert.rejects(recording,/取消/);
  assert.deepEqual(h.counts(),{started:0,stopped:1});
});
test('ordinary microphone grant starts once and cancellation stops its tracks',async()=>{
  const h=platform(),recording=h.sandbox.startRecording(); h.grant();
  const owner=await recording;h.sandbox.cancelRecording(owner);
  assert.deepEqual(h.counts(),{started:1,stopped:1});
});

test('a cancelled pending grant cannot clear a newer recorder',async()=>{
  const h=platform(); const first=h.sandbox.startRecording();
  h.listeners.pagehide();
  const second=h.sandbox.startRecording(); h.grant(1);
  const owner=await second;
  h.grant(0); await assert.rejects(first,/取消/);
  assert.deepEqual(h.counts(),{started:1,stopped:1});
  h.sandbox.cancelRecording(owner);
  assert.deepEqual(h.counts(),{started:1,stopped:2});
});
