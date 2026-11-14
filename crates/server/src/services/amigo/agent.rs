use serde::{Deserialize, Serialize};

use super::model::{Memory, clean};

pub const DEFAULT_AGENT_ID: &str = "amigo";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Strategy {
    pub warmth: f32,
    pub humor: f32,
    pub initiative: f32,
    pub directness: f32,
    pub verbosity: f32,
    pub teaching_intensity: f32,
    pub question_rate: f32,
}

impl Default for Strategy {
    fn default() -> Self {
        Self {
            warmth: 0.7,
            humor: 0.35,
            initiative: 0.55,
            directness: 0.5,
            verbosity: 0.4,
            teaching_intensity: 0.45,
            question_rate: 0.3,
        }
    }
}

impl Strategy {
    pub fn validate(&self) -> Result<(), String> {
        let values = [
            self.warmth,
            self.humor,
            self.initiative,
            self.directness,
            self.verbosity,
            self.teaching_intensity,
            self.question_rate,
        ];
        if values
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            Ok(())
        } else {
            Err("strategy values must be finite numbers between 0 and 1".into())
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceProvider {
    #[default]
    Preset,
    VoiceStudio,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VoiceConfig {
    pub provider: VoiceProvider,
    pub voice_id: Option<String>,
    pub consent_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role_prompt: String,
    pub background: String,
    pub strategy: Strategy,
    pub voice: VoiceConfig,
    pub share_profile: bool,
    pub builtin: bool,
    pub character_disclosure: String,
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            id: DEFAULT_AGENT_ID.into(),
            name: "Amigo".into(),
            role_prompt: "A warm, curious English learning companion who has opinions, shares observations and does not turn every exchange into an interview.".into(),
            background: String::new(),
            strategy: Strategy::default(),
            voice: VoiceConfig::default(),
            share_profile: true,
            builtin: true,
            character_disclosure: String::new(),
        }
    }
}

impl Agent {
    pub fn custom(name: &str, request: &str) -> Result<Self, String> {
        let name = clean(name, 80);
        let request = clean(request, 2000);
        if name.is_empty() || request.is_empty() {
            return Err("agent name and character request are required".into());
        }
        let agent = Self {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name,
            role_prompt: request,
            background: String::new(),
            strategy: Strategy::default(),
            voice: VoiceConfig::default(),
            share_profile: true,
            builtin: false,
            character_disclosure: "This is an AI role-play companion, not the actor or a real person. Any preset voice is not presented as the actor's cloned voice.".into(),
        };
        agent.validate()?;
        Ok(agent)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            || self.name.trim().is_empty()
            || self.name.len() > 80
            || self.role_prompt.len() > 2000
            || self.background.len() > 4000
            || self.character_disclosure.len() > 500
            || self
                .voice
                .voice_id
                .as_ref()
                .is_some_and(|id| id.len() > 200)
            || (self.voice.provider == VoiceProvider::VoiceStudio
                && (self.voice.voice_id.as_deref().is_none_or(str::is_empty)
                    || !self.voice.consent_confirmed))
        {
            return Err("invalid agent identity, character, voice or consent settings".into());
        }
        self.strategy.validate()
    }
}

pub fn default_agents() -> Vec<Agent> {
    vec![Agent::default()]
}

pub fn default_agent_id() -> String {
    DEFAULT_AGENT_ID.into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentState {
    pub learning: super::workflow::Session,
    pub memories: Vec<Memory>,
    pub recent_acts: Vec<ResponseAct>,
    pub turns: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseAct {
    #[default]
    Respond,
    Support,
    Reflect,
    Share,
    Encourage,
    Challenge,
    Teach,
    Review,
    Ask,
    Introduce,
}

impl ResponseAct {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Respond => "respond",
            Self::Support => "support",
            Self::Reflect => "reflect",
            Self::Share => "share",
            Self::Encourage => "encourage",
            Self::Challenge => "challenge",
            Self::Teach => "teach",
            Self::Review => "review",
            Self::Ask => "ask",
            Self::Introduce => "introduce",
        }
    }
}

pub enum Route {
    Talk(String),
    Create(Agent),
}

pub fn route(agents: &[Agent], active: &str, text: &str) -> Route {
    if let Some(agent) = creation_request(text) {
        return Route::Create(agent);
    }
    let lower = text.to_lowercase();
    for agent in agents {
        let name = agent.name.to_lowercase();
        let at = format!("@{}", agent.id.to_lowercase());
        let explicit = lower.contains(&at)
            || lower.starts_with(&format!("{name},"))
            || lower.starts_with(&format!("{name}，"))
            || lower.starts_with(&format!("{name}:"))
            || lower.starts_with(&format!("{name}："))
            || (lower.contains(&name)
                && ["叫", "请", "让", "过来", "在吗", "talk to", "call"]
                    .iter()
                    .any(|marker| lower.contains(marker)));
        if explicit {
            return Route::Talk(agent.id.clone());
        }
    }
    Route::Talk(
        agents
            .iter()
            .find(|agent| agent.id == active)
            .unwrap_or(&agents[0])
            .id
            .clone(),
    )
}

fn creation_request(text: &str) -> Option<Agent> {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    let creation = (lower.contains("创建") || lower.contains("新建") || lower.contains("create"))
        && (lower.contains("角色") || lower.contains("agent") || lower.contains("伙伴"));
    if !creation {
        return None;
    }
    let name = ["名叫", "名字叫", "叫做", "叫", "named"]
        .iter()
        .find_map(|marker| {
            let start = lower.find(marker)? + marker.len();
            let tail = trimmed.get(start..)?.trim_start();
            let phrase: String = tail
                .chars()
                .take_while(|c| !matches!(c, '，' | ',' | '。' | '.' | '；' | ';'))
                .collect();
            let name = phrase
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches("的角色")
                .trim()
                .to_owned();
            (!name.is_empty() && name.len() <= 80).then_some(name)
        })?;
    Agent::custom(&name, trimmed).ok()
}

pub fn select_act(
    agent: &Agent,
    state: &AgentState,
    text: &str,
    has_due_review: bool,
    proactive: bool,
    introducing: bool,
) -> ResponseAct {
    if introducing {
        return ResponseAct::Introduce;
    }
    let lower = text.to_lowercase();
    if !proactive
        && [
            "难过",
            "焦虑",
            "崩溃",
            "压力好大",
            "depressed",
            "anxious",
            "overwhelmed",
            "upset",
        ]
        .iter()
        .any(|word| lower.contains(word))
    {
        return ResponseAct::Support;
    }
    if !proactive && is_direct_question(&lower) {
        return ResponseAct::Respond;
    }
    if has_due_review && review_due_in_conversation(agent, state) {
        return ResponseAct::Review;
    }
    if state.recent_acts.last() == Some(&ResponseAct::Ask) {
        return if state.turns.is_multiple_of(2) {
            ResponseAct::Reflect
        } else {
            ResponseAct::Share
        };
    }
    if proactive {
        return ResponseAct::Share;
    }
    if agent.strategy.question_rate > 0.0 && question_turn(agent, state) {
        return ResponseAct::Ask;
    }
    const ACTS: [ResponseAct; 5] = [
        ResponseAct::Reflect,
        ResponseAct::Share,
        ResponseAct::Encourage,
        ResponseAct::Teach,
        ResponseAct::Challenge,
    ];
    let mut index =
        (state.turns as usize + (agent.strategy.initiative * 4.0) as usize) % ACTS.len();
    for _ in 0..ACTS.len() {
        let act = ACTS[index];
        if state.recent_acts.last() != Some(&act) {
            return act;
        }
        index = (index + 1) % ACTS.len();
    }
    ResponseAct::Respond
}

fn is_direct_question(text: &str) -> bool {
    text.trim_end().ends_with(['?', '？'])
        || [
            "what ",
            "why ",
            "how ",
            "when ",
            "where ",
            "can you",
            "could you",
            "请告诉",
            "怎么办",
            "为什么",
            "是什么",
        ]
        .iter()
        .any(|prefix| text.trim_start().starts_with(prefix))
}

fn review_due_in_conversation(agent: &Agent, state: &AgentState) -> bool {
    let cadence = (8.0 - agent.strategy.teaching_intensity * 6.0).round() as u64;
    state.turns > 0 && state.turns.is_multiple_of(cadence.max(2))
}

fn question_turn(agent: &Agent, state: &AgentState) -> bool {
    let budget = (agent.strategy.question_rate * 10.0).round() as u64;
    budget > 0 && state.turns % 10 < budget
}

pub fn remember_act(state: &mut AgentState, act: ResponseAct) {
    state.recent_acts.push(act);
    if state.recent_acts.len() > 8 {
        state.recent_acts.remove(0);
    }
}

pub fn private_memories<'a>(
    states: &'a std::collections::HashMap<String, AgentState>,
    id: &str,
) -> &'a [Memory] {
    states
        .get(id)
        .map(|state| state.memories.as_slice())
        .unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_is_explicit_and_sticky() {
        let mut joey = Agent::custom("Joey", "A humorous friend").unwrap();
        joey.id = "joey".into();
        let agents = vec![Agent::default(), joey];
        assert!(
            matches!(route(&agents, "amigo", "Joey，过来聊聊"), Route::Talk(id) if id == "joey")
        );
        assert!(matches!(route(&agents, "joey", "我们继续"), Route::Talk(id) if id == "joey"));
        assert!(matches!(route(&agents, "amigo", "Joey 是谁"), Route::Talk(id) if id == "amigo"));
    }

    #[test]
    fn creation_has_role_play_boundary() {
        let Route::Create(agent) = route(
            &default_agents(),
            DEFAULT_AGENT_ID,
            "创建一个名叫 Joey 的角色，模仿《老友记》中 Joey 的风格",
        ) else {
            panic!("expected agent creation");
        };
        assert_eq!(agent.name, "Joey");
        assert!(!agent.builtin);
        assert!(agent.character_disclosure.contains("not the actor"));
    }

    #[test]
    fn questions_have_a_budget_and_never_repeat_consecutively() {
        let agent = Agent::default();
        let mut state = AgentState::default();
        let first = select_act(&agent, &state, "I finished my project", false, false, false);
        remember_act(&mut state, first);
        state.turns += 1;
        let second = select_act(&agent, &state, "It went well", false, false, false);
        assert_eq!(first, ResponseAct::Ask);
        assert_ne!(second, ResponseAct::Ask);
        state.recent_acts.push(ResponseAct::Ask);
        assert_ne!(
            select_act(&agent, &state, "More news", false, false, false),
            ResponseAct::Ask
        );
        let mut quiet = agent.clone();
        quiet.strategy.question_rate = 0.0;
        assert_ne!(
            select_act(
                &quiet,
                &AgentState::default(),
                "A story",
                false,
                false,
                false
            ),
            ResponseAct::Ask
        );
    }

    #[test]
    fn user_intent_precedes_review() {
        let agent = Agent::default();
        let state = AgentState {
            turns: 5,
            ..Default::default()
        };
        assert_eq!(
            select_act(&agent, &state, "How does this work?", true, false, false),
            ResponseAct::Respond
        );
        assert_eq!(
            select_act(&agent, &state, "我今天压力好大", true, false, false),
            ResponseAct::Support
        );
        assert_eq!(
            select_act(&agent, &state, "继续聊项目", true, false, false),
            ResponseAct::Review
        );
    }

    #[test]
    fn invalid_strategy_and_unconsented_clone_are_rejected() {
        let mut agent = Agent::default();
        agent.strategy.question_rate = f32::NAN;
        assert!(agent.validate().is_err());
        let mut agent = Agent::default();
        agent.voice.provider = VoiceProvider::VoiceStudio;
        agent.voice.voice_id = Some("private-voice".into());
        assert!(agent.validate().is_err());
        agent.voice.consent_confirmed = true;
        assert!(agent.validate().is_ok());
    }
}
