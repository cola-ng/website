// Capture only. Main thread decides when audio may be retained or uploaded.
class LangbotRecorder extends AudioWorkletProcessor {
  constructor() { super(); this.ratio = sampleRate / 16000; this.weight = 0; this.sum = 0; this.frame = new Int16Array(320); this.offset = 0; this.energy = 0 }
  process(inputs) {
    const input = inputs[0]?.[0]
    if (!input) return true
    for (const value of input) {
      let remaining = 1
      while (remaining > 1e-8) {
        const amount = Math.min(remaining, this.ratio - this.weight)
        this.sum += value * amount; this.weight += amount; remaining -= amount
        if (this.weight >= this.ratio - 1e-8) {
          const sample = Math.max(-1, Math.min(1, this.sum / this.ratio))
          this.frame[this.offset++] = Math.round(sample * (sample < 0 ? 32768 : 32767)); this.energy += sample * sample
          this.weight = 0; this.sum = 0
          if (this.offset === 320) {
            this.port.postMessage({ pcm: this.frame.buffer, rms: Math.sqrt(this.energy / 320) }, [this.frame.buffer])
            this.frame = new Int16Array(320); this.offset = 0; this.energy = 0
          }
        }
      }
    }
    return true
  }
}
registerProcessor('colang-recorder', LangbotRecorder)

