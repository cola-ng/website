use std::collections::HashMap;

use chrono::{DateTime, Duration, Timelike, Utc};
use amigo::protocol::Source;
use serde::{Deserialize, Serialize};

use super::agent::{
    Agent, AgentState, ResponseAct, Route, default_agent_id, default_agents, private_memories,
    remember_act, route, select_act,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMode {
    #[default]
    Standard,
    Realtime,
}

fn realtime_model() -> String { "gpt-realtime-2.1-mini".into() }
fn realtime_voice() -> String { "marin".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub conversation_mode: ConversationMode,
    pub realtime_provider_id: Option<i64>,
    pub realtime_model: String,
    pub realtime_voice: String,
    pub camera_enabled: bool,
    pub ambient_audio_enabled: bool,
    pub proactive_enabled: bool,
    pub observation_interval_secs: u64,
    pub proactive_cooldown_secs: u64,
    pub idle_secs: u64,
    pub daily_outreach_limit: u32,
    pub quiet_start_minute: u32,
    pub quiet_end_minute: u32,
    pub utc_offset_minutes: i32,
    pub camera_target: Source,
    pub english_level: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            conversation_mode: ConversationMode::Standard,
            realtime_provider_id: None,
            realtime_model: realtime_model(),
            realtime_voice: realtime_voice(),
            camera_enabled: true,
            ambient_audio_enabled: true,
            proactive_enabled: true,
            observation_interval_secs: 300,
            proactive_cooldown_secs: 1200,
            idle_secs: 60,
            daily_outreach_limit: 8,
            quiet_start_minute: 22 * 60 + 30,
            quiet_end_minute: 8 * 60,
            utc_offset_minutes: 480,
            camera_target: Source::Book,
            english_level: "B1".into(),
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !crate::services::openai_realtime::is_model(&self.realtime_model)
            || self.realtime_model.len() > 120
            || !self.realtime_model.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b))
            || !crate::services::voices::openai_voice_supported(&self.realtime_model, &self.realtime_voice)
            || self.realtime_voice.len() > 200
        {
            return Err("请选择有效的 OpenAI Realtime 模型与音色");
        }
        if !(10..=86400).contains(&self.observation_interval_secs)
            || !(10..=86400).contains(&self.proactive_cooldown_secs)
            || !(1..=3600).contains(&self.idle_secs)
            || self.daily_outreach_limit > 24
            || self.quiet_start_minute >= 1440
            || self.quiet_end_minute >= 1440
            || !(-720..=840).contains(&self.utc_offset_minutes)
            || self.camera_target == Source::Speech
            || !["A1", "A2", "B1", "B2", "C1", "C2"].contains(&self.english_level.as_str())
        {
            return Err("invalid observation, outreach, timezone or learner settings");
        }
        Ok(())
    }

    pub fn quiet(&self, now: DateTime<Utc>) -> bool {
        let local = now + Duration::minutes(i64::from(self.utc_offset_minutes));
        let minute = local.hour() * 60 + local.minute();
        let (start, end) = (self.quiet_start_minute, self.quiet_end_minute);
        if start == end {
            false
        } else if start < end {
            minute >= start && minute < end
        } else {
            minute >= start || minute < end
        }
    }

    pub fn day(&self, now: DateTime<Utc>) -> String {
        (now + Duration::minutes(i64::from(self.utc_offset_minutes)))
            .date_naive()
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    #[serde(default)]
    pub user_confirmed: bool,
    pub subject: String,
    pub summary: String,
    pub evidence_count: u32,
    pub confidence: f32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weakness {
    #[serde(default)]
    pub user_corrected: bool,
    pub key: String,
    pub kind: String,
    pub original: String,
    pub corrected: String,
    pub failures: u32,
    pub successful_recalls: u32,
    pub interval_days: u32,
    pub next_review_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub source: String,
    pub text: String,
    pub topics: Vec<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dialogue {
    #[serde(default)]
    pub learning_mode: String,
    #[serde(default)]
    pub fictional: bool,
    pub role: String,
    pub text: String,
    #[serde(default = "default_agent_id")]
    pub agent_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_agent: Option<Agent>,
    pub settings: Settings,
    pub memories: Vec<Memory>,
    pub weaknesses: Vec<Weakness>,
    pub observations: Vec<Context>,
    pub history: Vec<Dialogue>,
    pub assigned_targets: Vec<String>,
    pub last_observation_at: Option<DateTime<Utc>>,
    pub last_outreach_at: Option<DateTime<Utc>>,
    pub outreach_day: String,
    pub outreaches_today: u32,
    pub dismissals: u32,
    /// Deleted items cannot be immediately relearned from old evidence.
    pub forgotten: Vec<String>,
    #[serde(default = "default_agents")]
    pub agents: Vec<Agent>,
    #[serde(default = "default_agent_id")]
    pub active_agent_id: String,
    pub agent_states: HashMap<String, AgentState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryEvidence {
    #[serde(default)]
    pub evidence: String,
    pub subject: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub key: String,
    pub kind: String,
    pub original: String,
    pub corrected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recall {
    pub key: String,
    pub evidence: String,
    pub success: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Analysis {
    pub learning_evidence: Vec<super::workflow::EvidenceProposal>,
    pub next_task: Option<super::workflow::PresentedTask>,
    pub reply_en: String,
    pub hint_zh: String,
    pub topics: Vec<String>,
    pub memories: Vec<MemoryEvidence>,
    pub issues: Vec<Issue>,
    pub recalls: Vec<Recall>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<super::workflow::Execution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual: Option<VisualContext>,
    pub situation: String,
    pub targets: Vec<Weakness>,
    pub level: String,
    pub proactive: bool,
    pub agent: Agent,
    pub response_act: ResponseAct,
    pub recent_acts: Vec<ResponseAct>,
    pub private_memories: Vec<Memory>,
    pub shared_memories: Vec<Memory>,
    pub history: Vec<Dialogue>,
    pub previous_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualContext {
    pub status: String,
    pub observation: Option<Context>,
}

impl Profile {
    pub fn ensure_agents(&mut self) {
        if let Some(agent) = self.device_agent.clone() {
            if let Some(existing) = self.agents.iter_mut().find(|a| a.id == agent.id) { *existing = agent; }
            else { self.agents.push(agent); }
        }
        if self.agents.is_empty() {
            self.agents = default_agents();
        }
        if !self
            .agents
            .iter()
            .any(|agent| agent.id == self.active_agent_id)
        {
            self.active_agent_id = default_agent_id();
        }
    }

    pub fn route(&self, text: &str) -> Route {
        route(&self.agents, &self.active_agent_id, text)
    }

    pub fn activate(&mut self, agent: Agent) -> Result<(), String> {
        agent.validate()?;
        if self.agents.len() >= 20 && !self.agents.iter().any(|item| item.id == agent.id) {
            return Err("a user can have at most 20 agents".into());
        }
        if self
            .agents
            .iter()
            .any(|item| item.id != agent.id && item.name.eq_ignore_ascii_case(&agent.name))
        {
            return Err("agent names must be unique".into());
        }
        self.active_agent_id = agent.id.clone();
        if let Some(current) = self.agents.iter_mut().find(|item| item.id == agent.id) {
            if current.builtin && !agent.builtin {
                return Err("built-in agent identity cannot be changed".into());
            }
            *current = agent;
        } else {
            self.agents.push(agent);
        }
        Ok(())
    }

    pub fn archive_agent(&mut self, id: &str) -> bool {
        if id == default_agent_id() || !self.agents.iter().any(|agent| agent.id == id) {
            return false;
        }
        self.agents.retain(|agent| agent.id != id);
        self.agent_states.remove(id);
        self.history.retain(|line| line.agent_id != id);
        if self.active_agent_id == id {
            self.active_agent_id = default_agent_id();
        }
        true
    }

    pub fn plan(&self, now: DateTime<Utc>, proactive: bool) -> Plan {
        self.plan_for(&self.active_agent_id.clone(), now, proactive, "", false)
    }

    pub fn plan_for(
        &self,
        agent_id: &str,
        now: DateTime<Utc>,
        proactive: bool,
        text: &str,
        introducing: bool,
    ) -> Plan {
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .cloned()
            .unwrap_or_default();
        let state = self
            .agent_states
            .get(&agent.id)
            .cloned()
            .unwrap_or_default();
        let situation = self
            .observations
            .iter()
            .rev()
            .find(|o| now.signed_duration_since(o.observed_at) <= Duration::hours(2))
            .map(|o| format!("{}: {}", o.source, o.text))
            .unwrap_or_else(|| "A casual conversation about the learner's current activity".into());
        let lower = situation.to_lowercase();
        let mut targets: Vec<_> = self
            .weaknesses
            .iter()
            .filter(|w| w.next_review_at <= now && !self.forgotten.contains(&w.key))
            .cloned()
            .collect();
        targets.sort_by_key(|w| {
            (
                !lower.contains(&w.key.to_lowercase()),
                w.next_review_at,
                std::cmp::Reverse(w.failures),
            )
        });
        targets.truncate(2);
        let response_act = select_act(
            &agent,
            &state,
            text,
            !targets.is_empty(),
            proactive,
            introducing,
        );
        let history = self
            .history
            .iter()
            .filter(|line| line.agent_id == agent.id)
            .cloned()
            .collect();
        let previous_questions = self
            .history
            .iter()
            .filter(|line| {
                line.agent_id == agent.id && line.role == "assistant" && line.text.contains('?')
            })
            .rev()
            .take(3)
            .map(|line| line.text.clone())
            .collect();
        Plan {
            workflow: None,
            visual: None,
            situation,
            targets,
            level: self.settings.english_level.clone(),
            proactive,
            private_memories: private_memories(&self.agent_states, &agent.id).to_vec(),
            shared_memories: if agent.share_profile {
                self.memories.clone()
            } else {
                vec![]
            },
            history,
            previous_questions,
            recent_acts: state.recent_acts,
            response_act,
            agent,
        }
    }

    pub fn may_outreach(&self, now: DateTime<Utc>, idle: bool) -> bool {
        let count = if self.outreach_day == self.settings.day(now) {
            self.outreaches_today
        } else {
            0
        };
        self.settings.proactive_enabled
            && idle
            && !self.settings.quiet(now)
            && count < self.settings.daily_outreach_limit
            && self.last_outreach_at.is_none_or(|last| {
                now.signed_duration_since(last).num_seconds()
                    >= (self.settings.proactive_cooldown_secs
                        * (1 + u64::from(self.dismissals.min(3)))) as i64
            })
            && (!self.plan(now, true).targets.is_empty()
                || super::workflow::recommendation(self, now).is_some()
                || self
                    .agent_states
                    .get(&self.active_agent_id)
                    .is_some_and(|state| {
                        state
                            .learning
                            .items
                            .iter()
                            .any(|item| item.next_review_at.is_some_and(|due| due <= now))
                    })
                || self.observations.last().is_some_and(|o| {
                    now.signed_duration_since(o.observed_at) < Duration::minutes(30)
                        && self
                            .last_outreach_at
                            .is_none_or(|last| o.observed_at > last)
                }))
    }

    pub fn mark_outreach(&mut self, now: DateTime<Utc>) {
        let day = self.settings.day(now);
        if self.outreach_day != day {
            self.outreaches_today = 0;
            self.outreach_day = day;
        }
        self.outreaches_today = self.outreaches_today.saturating_add(1);
        self.last_outreach_at = Some(now);
    }

    pub fn observe(&mut self, context: Context, evidence: &[MemoryEvidence]) {
        self.last_observation_at = Some(context.observed_at);
        self.learn_memories(evidence, context.observed_at);
        self.observations.push(context);
        trim(&mut self.observations, 20);
    }

    fn learn_memories(&mut self, evidence: &[MemoryEvidence], now: DateTime<Utc>) {
        learn_memory_list(&mut self.memories, evidence, &self.forgotten, now);
    }

    pub fn learn(
        &mut self,
        text: &str,
        analysis: &Analysis,
        targets: &[Weakness],
        now: DateTime<Utc>,
    ) {
        self.learn_inner(text, analysis, targets, now, default_agent_id(), None);
    }

    pub fn learn_for_agent(
        &mut self,
        text: &str,
        analysis: &Analysis,
        targets: &[Weakness],
        now: DateTime<Utc>,
        agent_id: &str,
        act: ResponseAct,
    ) {
        self.learn_inner(text, analysis, targets, now, agent_id.to_owned(), Some(act));
    }

    pub fn record_behavior(&mut self, agent_id: &str, act: ResponseAct) {
        let state = self.agent_states.entry(agent_id.to_owned()).or_default();
        state.turns = state.turns.saturating_add(1);
        remember_act(state, act);
    }

    fn learn_inner(
        &mut self,
        text: &str,
        analysis: &Analysis,
        targets: &[Weakness],
        now: DateTime<Utc>,
        agent_id: String,
        act: Option<ResponseAct>,
    ) {
        let share = self
            .agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .is_none_or(|agent| agent.share_profile);
        if share {
            self.learn_memories(&analysis.memories, now);
        }
        let forgotten = self.forgotten.clone();
        let state = self.agent_states.entry(agent_id.clone()).or_default();
        learn_memory_list(&mut state.memories, &analysis.memories, &forgotten, now);
        if let Some(act) = act {
            state.turns = state.turns.saturating_add(1);
            remember_act(state, act);
        }
        let lower = text.to_lowercase();
        let mut seen_issues = std::collections::HashSet::new();
        for issue in analysis.issues.iter().take(5) {
            let key = clean(&issue.key, 80).to_lowercase();
            let original = clean(&issue.original, 240);
            let corrected = clean(&issue.corrected, 240);
            if key.is_empty()
                || original.is_empty()
                || corrected.is_empty()
                || original.eq_ignore_ascii_case(&corrected)
                || !lower.contains(&original.to_lowercase())
                || !["vocabulary", "grammar", "sentence"].contains(&issue.kind.as_str())
                || self.forgotten.contains(&key)
                || !seen_issues.insert(key.clone())
            {
                continue;
            }
            if let Some(weakness) = self.weaknesses.iter_mut().find(|w| w.key == key) {
                weakness.failures = weakness.failures.saturating_add(1);
                weakness.original = original;
                if !weakness.user_corrected {
                    weakness.corrected = corrected;
                }
                weakness.interval_days = 0;
                weakness.next_review_at = now;
                weakness.updated_at = now;
            } else {
                self.weaknesses.push(Weakness {
                    user_corrected: false,
                    key,
                    kind: issue.kind.clone(),
                    original,
                    corrected,
                    failures: 1,
                    successful_recalls: 0,
                    interval_days: 0,
                    next_review_at: now,
                    updated_at: now,
                });
            }
        }
        let mut seen_recalls = std::collections::HashSet::new();
        for recall in analysis.recalls.iter().take(2) {
            if recall.evidence.trim().is_empty()
                || !lower.contains(&recall.evidence.to_lowercase())
                || !targets.iter().any(|t| t.key == recall.key)
                || analysis
                    .issues
                    .iter()
                    .any(|i| i.key.eq_ignore_ascii_case(&recall.key))
                || !seen_recalls.insert(recall.key.clone())
            {
                continue;
            }
            if let Some(weakness) = self.weaknesses.iter_mut().find(|w| w.key == recall.key) {
                if recall.success {
                    if weakness.next_review_at > now {
                        continue;
                    }
                    weakness.successful_recalls = weakness.successful_recalls.saturating_add(1);
                    weakness.interval_days = if weakness.interval_days == 0 {
                        1
                    } else {
                        (weakness.interval_days * 2).min(30)
                    };
                    weakness.next_review_at =
                        now + Duration::days(i64::from(weakness.interval_days));
                } else {
                    weakness.failures = weakness.failures.saturating_add(1);
                    weakness.interval_days = 0;
                    weakness.next_review_at = now;
                }
                weakness.updated_at = now;
            }
        }
        trim(&mut self.weaknesses, 200);
        let learning_mode = self
            .agent_states
            .get(&agent_id)
            .map(|state| state.learning.mode.clone())
            .unwrap_or_default();
        let fictional =
            super::workflow::playbook(&learning_mode).is_some_and(|book| book.fictional);
        self.history.push(Dialogue {
            learning_mode: learning_mode.clone(),
            fictional,
            role: "user".into(),
            text: clean(text, 2000),
            agent_id: agent_id.clone(),
        });
        self.history.push(Dialogue {
            learning_mode,
            fictional,
            role: "assistant".into(),
            text: analysis.reply_en.clone(),
            agent_id,
        });
        trim(&mut self.history, 64);
    }
}

pub fn clean(text: &str, max_chars: usize) -> String {
    super::privacy::redact_sensitive_text(text.trim())
        .text
        .chars()
        .take(max_chars)
        .collect()
}

fn learn_memory_list(
    memories: &mut Vec<Memory>,
    evidence: &[MemoryEvidence],
    forgotten: &[String],
    now: DateTime<Utc>,
) {
    let mut seen = std::collections::HashSet::new();
    for item in evidence.iter().take(5) {
        let subject = clean(&item.subject, 80).to_lowercase();
        let summary = clean(&item.summary, 240);
        if subject.is_empty()
            || summary.is_empty()
            || forgotten.contains(&subject)
            || !seen.insert(subject.clone())
        {
            continue;
        }
        if let Some(memory) = memories.iter_mut().find(|m| m.subject == subject) {
            memory.evidence_count = memory.evidence_count.saturating_add(1);
            if !memory.user_confirmed {
                memory.confidence = (0.25 + memory.evidence_count.min(10) as f32 * 0.06).min(0.85);
                memory.summary = summary;
            }
            memory.updated_at = now;
        } else {
            memories.push(Memory {
                user_confirmed: false,
                subject,
                summary,
                evidence_count: 1,
                confidence: 0.31,
                updated_at: now,
            });
        }
    }
    trim(memories, 100);
}

fn trim<T>(items: &mut Vec<T>, max: usize) {
    if items.len() > max {
        items.drain(..items.len() - max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn subsequent_inferences_preserve_human_corrections() {
        let mut profile = Profile::default();
        let evidence = Analysis {
            memories: vec![MemoryEvidence {
                subject: "plants".into(),
                summary: "AI guess".into(),
                ..Default::default()
            }],
            issues: vec![Issue {
                key: "past tense".into(),
                kind: "grammar".into(),
                original: "I go yesterday".into(),
                corrected: "AI correction".into(),
            }],
            ..Default::default()
        };
        profile.learn("I go yesterday", &evidence, &[], Utc::now());
        profile.memories[0].user_confirmed = true;
        profile.memories[0].summary = "Human description".into();
        profile.memories[0].confidence = 1.0;
        profile.weaknesses[0].user_corrected = true;
        profile.weaknesses[0].corrected = "Human correction".into();
        profile.learn("I go yesterday", &evidence, &[], Utc::now());
        assert_eq!(profile.memories[0].summary, "Human description");
        assert_eq!(profile.memories[0].confidence, 1.0);
        assert_eq!(profile.weaknesses[0].corrected, "Human correction");
        assert_eq!(profile.weaknesses[0].failures, 2);
    }

    #[test]
    fn timezone_quiet_hours_and_daily_boundary() {
        let settings = Settings::default();
        assert!(settings.quiet(Utc.with_ymd_and_hms(2026, 10, 30, 15, 0, 0).unwrap()));
        assert!(!settings.quiet(Utc.with_ymd_and_hms(2026, 10, 30, 0, 0, 0).unwrap()));
        assert_eq!(
            settings.day(Utc.with_ymd_and_hms(2026, 10, 30, 17, 0, 0).unwrap()),
            "2026-10-31"
        );
    }

    #[test]
    fn evidence_drives_weaknesses_and_spaced_recall() {
        let now = Utc::now();
        let mut profile = Profile::default();
        let analysis = Analysis {
            issues: vec![Issue {
                key: "past tense".into(),
                kind: "grammar".into(),
                original: "I go yesterday".into(),
                corrected: "I went yesterday".into(),
            }],
            ..Default::default()
        };
        profile.learn("I go yesterday", &analysis, &[], now);
        assert_eq!(profile.weaknesses[0].failures, 1);
        let plan = profile.plan(now, false);
        let recall = Analysis {
            recalls: vec![Recall {
                key: "past tense".into(),
                evidence: "I went yesterday".into(),
                success: true,
            }],
            ..Default::default()
        };
        profile.learn("I went yesterday", &recall, &plan.targets, now);
        assert_eq!(profile.weaknesses[0].interval_days, 1);
        assert!(profile.plan(now, true).targets.is_empty());
        profile.learn("unrelated", &analysis, &[], now);
        assert_eq!(profile.weaknesses[0].failures, 1);
    }

    #[test]
    fn outreach_respects_context_idle_cooldown_and_rejection() {
        let now = Utc.with_ymd_and_hms(2026, 10, 30, 4, 0, 0).unwrap();
        let mut profile = Profile::default();
        profile.observe(
            Context {
                source: "book".into(),
                text: "photosynthesis".into(),
                topics: vec![],
                observed_at: now,
            },
            &[],
        );
        assert!(profile.may_outreach(now, true));
        assert!(!profile.may_outreach(now, false));
        profile.mark_outreach(now);
        assert!(!profile.may_outreach(now + Duration::minutes(1), true));
        profile.settings.proactive_enabled = false;
        assert!(!profile.may_outreach(now + Duration::hours(1), true));
    }

    #[test]
    fn old_profiles_gain_amigo_and_agent_memories_respect_sharing() {
        let mut profile: Profile = serde_json::from_value(serde_json::json!({
            "history": [{"role":"assistant","text":"Old reply"}]
        }))
        .unwrap();
        profile.ensure_agents();
        assert_eq!(profile.agents[0].id, default_agent_id());
        assert_eq!(profile.history[0].agent_id, default_agent_id());
        let mut private = Agent::custom("Private", "A private work companion").unwrap();
        private.share_profile = false;
        profile.activate(private.clone()).unwrap();
        let analysis = Analysis {
            memories: vec![MemoryEvidence {
                subject: "project".into(),
                summary: "Working on a private project".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        profile.learn_for_agent(
            "I am working on a project",
            &analysis,
            &[],
            Utc::now(),
            &private.id,
            ResponseAct::Share,
        );
        assert!(profile.memories.is_empty());
        assert_eq!(profile.agent_states[&private.id].memories.len(), 1);
        assert!(
            profile
                .plan(now_for_test(), false)
                .shared_memories
                .is_empty()
        );
        assert_eq!(profile.history.last().unwrap().agent_id, private.id);
        profile.activate(profile.agents[0].clone()).unwrap();
        assert!(profile.archive_agent(&private.id));
        assert_eq!(profile.active_agent_id, default_agent_id());
        assert!(
            profile
                .history
                .iter()
                .all(|line| line.agent_id != private.id)
        );
    }

    fn now_for_test() -> DateTime<Utc> {
        Utc::now()
    }
}
