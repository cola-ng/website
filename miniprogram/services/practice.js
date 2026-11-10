const api = require("./api");
const media = require("./media");
function normalize(text) {
  return String(text || "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, " ");
}
async function evaluate(page, audio, reference) {
  const version = page._version;
  const value = await api.request("/api/learn/pronunciation/evaluate", "POST", {
    audio_base64: audio,
    reference_text: reference,
  });
  if (!page.active(version)) return null;
  page.setData({ evaluation: value });
  return value;
}
function record(page, reference, onEvaluated) {
  if (page.data.recording) return media.finish(page);
  if (page.data.busy || !reference) return;
  page.setData({ evaluation: null });
  media.record(page, (audio) =>
    page.run(async () => {
      const value = await evaluate(page, audio, reference);
      if (value && onEvaluated) await onEvaluated(value);
    }),
  );
}
function practiceId() {
  return `wx-${Date.now()}-${Math.random().toString(36).slice(2, 12)}`;
}
module.exports = { normalize, evaluate, record, practiceId };
