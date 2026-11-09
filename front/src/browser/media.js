// Browser platform adapters only. Application state and UI live in Rust.
let recorder, stream, chunks = [], pending, timer, audio, objectUrl, starting = false, audioGeneration = 0;
let recordingSequence=0, recordingOwner, rejectRecording;
function release() {
  clearTimeout(timer);
  stream?.getTracks().forEach(track => track.stop());
  stream = undefined;
}
export async function startRecording() {
  if (recorder || starting || pending) throw new Error('请先停止并提交正在进行的录音');
  if (!navigator.mediaDevices?.getUserMedia || !globalThis.MediaRecorder) throw new Error('当前浏览器不支持录音，请使用 HTTPS 或 localhost');
  starting = true;
  const owner=++recordingSequence;
  recordingOwner=owner;
  try {
    stream = await navigator.mediaDevices.getUserMedia({audio: true});
    const mimeType = ['audio/webm;codecs=opus', 'audio/mp4', 'audio/webm'].find(t => MediaRecorder.isTypeSupported(t));
    recorder = new MediaRecorder(stream, mimeType ? {mimeType} : {});
    chunks = [];
    pending = new Promise((resolve, reject) => {
      rejectRecording=reject;
      recorder.ondataavailable = event => { if (event.data.size) chunks.push(event.data); };
      recorder.onerror = () => { release(); recorder = undefined; reject(new Error('录音失败，请重试')); };
      recorder.onstop = async () => {
        const blob = new Blob(chunks, {type: recorder?.mimeType || mimeType});
        release(); recorder = undefined;
        try {
          if (!blob.size) throw new Error('没有录到声音，请重试');
          const context = new AudioContext();
          let decoded;
          try { decoded = await context.decodeAudioData(await blob.arrayBuffer()); }
          finally { await context.close(); }
          // Downmix and resample to mono 16k PCM, the ASR services' WAV format.
          const rate = 16000, length = Math.floor(decoded.duration * rate);
          if (!length) throw new Error('录音太短，请重试');
          const result = new ArrayBuffer(44 + length * 2), view = new DataView(result);
          const word = (offset, str) => [...str].forEach((ch, i) => view.setUint8(offset + i, ch.charCodeAt(0)));
          word(0, 'RIFF'); view.setUint32(4, 36 + length * 2, true); word(8, 'WAVE'); word(12, 'fmt ');
          view.setUint32(16, 16, true); view.setUint16(20, 1, true); view.setUint16(22, 1, true);
          view.setUint32(24, rate, true); view.setUint32(28, rate * 2, true); view.setUint16(32, 2, true); view.setUint16(34, 16, true);
          word(36, 'data'); view.setUint32(40, length * 2, true);
          const channels = Array.from({length: decoded.numberOfChannels}, (_, i) => decoded.getChannelData(i));
          for (let i = 0; i < length; i++) {
            const position = i * decoded.sampleRate / rate, left = Math.floor(position), fraction = position - left;
            let sample = 0;
            for (const channel of channels) sample += (channel[left] || 0) * (1 - fraction) + (channel[Math.min(left + 1, channel.length - 1)] || 0) * fraction;
            sample = Math.max(-1, Math.min(1, sample / channels.length));
            view.setInt16(44 + i * 2, sample < 0 ? sample * 32768 : sample * 32767, true);
          }
          const base64 = await new Promise((res, rej) => { const reader = new FileReader(); reader.onload = () => res(reader.result.split(',')[1]); reader.onerror = rej; reader.readAsDataURL(new Blob([result], {type: 'audio/wav'})); });
          resolve(base64);
        } catch (error) { reject(error); }
      };
    });
    // Prevent an unhandled rejection before the user presses Stop.
    pending.catch(() => {});
    recorder.start();
    timer = setTimeout(() => { if (recorder?.state === 'recording') recorder.stop(); }, 60000);
    return owner;
  } catch (error) { release(); recorder = undefined; pending = undefined; recordingOwner=undefined; rejectRecording=undefined; throw error; }
  finally { starting = false; }
}
export async function stopRecording() {
  if (!pending) throw new Error('尚未开始录音');
  if (recorder?.state === 'recording') recorder.stop();
  const completion=pending;
  try { return await completion; }
  finally { if (pending===completion) { pending = undefined; recordingOwner=undefined; rejectRecording=undefined; } }
}
export function cancelRecording(owner) {
  if (owner!==recordingOwner) return;
  if (recorder) { recorder.onstop=null; recorder.onerror=null; recorder.ondataavailable=null; if(recorder.state==='recording')recorder.stop(); }
  release(); recorder=undefined; chunks=[];
  rejectRecording?.(new Error('录音已取消'));
  pending=undefined; recordingOwner=undefined; rejectRecording=undefined;
}
export function stopAudio() {
  audioGeneration++;
  audio?.pause(); audio = undefined;
  if (objectUrl) URL.revokeObjectURL(objectUrl);
  objectUrl = undefined;
  globalThis.speechSynthesis?.cancel();
}
export async function playAudio(path, token) {
  stopAudio();
  const generation=audioGeneration;
  const response = await fetch(path, {headers: token ? {Authorization: `Bearer ${token}`} : {}});
  if (!response.ok) throw new Error('音频暂时不可用');
  const blob=await response.blob();
  if (generation!==audioGeneration) return;
  objectUrl = URL.createObjectURL(blob);
  audio = new Audio(objectUrl);
  audio.onended = stopAudio;
  await audio.play();
}
export function speakText(text) {
  stopAudio();
  if (!globalThis.speechSynthesis) throw new Error('当前浏览器不支持朗读');
  const utterance = new SpeechSynthesisUtterance(text);
  utterance.lang = /[\u3400-\u9fff]/.test(text) ? 'zh-CN' : 'en-US';
  speechSynthesis.speak(utterance);
}
export function downloadText(filename, text) {
  const url=URL.createObjectURL(new Blob([text],{type:'text/plain;charset=utf-8'}));
  const link=document.createElement('a'); link.href=url; link.download=filename;
  link.click(); setTimeout(()=>URL.revokeObjectURL(url),1000);
}
export async function uploadAvatar(token) {
  const input = document.createElement('input'); input.type = 'file'; input.accept = 'image/png,image/jpeg,image/webp';
  const file = await new Promise(resolve => { input.onchange = () => resolve(input.files?.[0]); input.oncancel = () => resolve(); input.click(); });
  if (!file) return null;
  if (file.size > 5 * 1024 * 1024) throw new Error('头像不能超过 5 MB');
  const bitmap = await createImageBitmap(file);
  const canvas = document.createElement('canvas'); canvas.width = canvas.height = 256;
  const side = Math.min(bitmap.width, bitmap.height);
  canvas.getContext('2d').drawImage(bitmap, (bitmap.width-side)/2, (bitmap.height-side)/2, side, side, 0, 0, 256, 256); bitmap.close();
  const blob = await new Promise(resolve => canvas.toBlob(resolve, 'image/png'));
  const body = new FormData(); body.append('image', blob, 'avatar.png');
  const response = await fetch('/api/me/avatar', {method:'POST', headers:{Authorization:`Bearer ${token}`}, body});
  if (!response.ok) throw new Error('头像上传失败，请重新登录后重试');
  return await response.text();
}
export async function avatarData(token) {
  const response = await fetch('/api/me/avatar', {headers:{Authorization:`Bearer ${token}`}});
  if (!response.ok) return '/colang-logo.svg';
  const blob = await response.blob();
  return await new Promise((resolve,reject) => { const reader=new FileReader(); reader.onload=()=>resolve(reader.result); reader.onerror=reject; reader.readAsDataURL(blob); });
}
addEventListener('pagehide', () => { cancelRecording(recordingOwner); release(); stopAudio(); });
