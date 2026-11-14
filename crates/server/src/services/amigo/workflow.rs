//! Semantic planning contracts and evidence rules. No keyword intent routing.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

use super::agent::ResponseAct;
use super::model::{Analysis, Context, Profile, clean};
use super::pronunciation::Assessment;

pub const PRACTICE_ROUNDS: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub modes: Vec<Playbook>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    #[serde(default)]
    pub fast_stages: Vec<String>,
    pub id: String,
    pub title: String,
    pub description: String,
    pub entry: Vec<String>,
    pub stages: Vec<Stage>,
    pub fictional: bool,
    pub allow_proactive: bool,
    pub exposure_stage: Option<String>,
    pub tool_failure_stage: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub title: String,
    pub id: String,
    pub next: Vec<String>,
    pub strategy: String,
}
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("workflow_catalog.json"))
            .expect("validated workflow catalog")
    })
}
pub fn playbook(mode: &str) -> Option<&'static Playbook> {
    catalog().modes.iter().find(|p| p.id == mode)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub catalog_version: u32,
    pub id: String,
    pub revision: u64,
    pub mode: String,
    pub stage: String,
    pub goal: String,
    pub strategy: String,
    pub scene: String,
    pub targets: Vec<Target>,
    pub task: Option<Task>,
    pub material: Option<Material>,
    pub items: Vec<Item>,
    pub evidence: Vec<Evidence>,
    pub last_event: Option<Uuid>,
    pub practice_attempts: u8,
    pub awaiting_choice: bool,
    pub practice_ended_at: Option<DateTime<Utc>>,
    pub pronunciation_focus: Option<String>,
    pub last_pronunciation: Option<Assessment>,
}
impl Session {
    pub fn normalized(mut self) -> Self {
        if playbook(&self.mode).is_none() {
            self.mode = "conversation".into();
            self.stage = "converse".into();
            self.task = None;
        }
        self
    }
}
pub fn session(profile: &Profile, agent_id: &str) -> Session {
    profile
        .agent_states
        .get(agent_id)
        .map(|s| s.learning.clone())
        .unwrap_or_default()
        .normalized()
}
pub fn prompt_state(profile: &Profile, agent_id: &str) -> serde_json::Value {
    let current = session(profile, agent_id);
    let mut task = current.task.clone();
    if let Some(task) = &mut task {
        task.id = "current".into();
    }
    serde_json::json!({"id":current.id,"revision":current.revision,"mode":current.mode,"stage":current.stage,
        "goal":current.goal,"strategy":current.strategy,"scene":current.scene,"targets":current.targets,"task":task,"material":current.material,
        "items":current.items.iter().rev().take(8).collect::<Vec<_>>(),
        "due_items":current.items.iter().filter(|item|item.next_review_at.is_some_and(|at|at<=Utc::now())).take(4).collect::<Vec<_>>(),
        "recent_evidence":current.evidence.iter().rev().take(4).collect::<Vec<_>>(),
        "practice_attempts":current.practice_attempts,"awaiting_choice":current.awaiting_choice,
        "practice_rounds":PRACTICE_ROUNDS,"last_pronunciation":current.last_pronunciation})
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub context: Context,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub key: String,
    pub kind: String,
    pub meaning: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Item {
    pub key: String,
    pub kind: String,
    pub meaning: String,
    pub exposures: u32,
    pub imitations: u32,
    pub assisted: u32,
    pub independent: u32,
    pub failures: u32,
    pub next_review_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextRequest {
    #[default]
    None,
    FreshCamera,
    CurrentMaterial,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPolicy {
    #[default]
    Keep,
    Replace,
    Clear,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    #[default]
    None,
    Hint,
    Answer,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Exposure,
    Imitation,
    CuedRecall,
    IndependentUse,
    Transfer,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    #[serde(default)]
    pub resume_practice: bool,
    #[serde(skip_deserializing)]
    pub practice_attempt: bool,
    #[serde(skip_deserializing)]
    pub check_in: bool,
    #[serde(skip_deserializing)]
    pub pronunciation: Option<Assessment>,
    #[serde(default)]
    pub support_provided: Support,
    #[serde(default)]
    pub task_spec: Option<TaskSpec>,
    pub mode: String,
    pub stage: String,
    pub confidence: f32,
    pub switch_evidence: String,
    pub goal: String,
    pub strategy: String,
    pub scene: String,
    pub action: ResponseAct,
    pub context: ContextRequest,
    pub task_policy: TaskPolicy,
    pub assess_user: bool,
    pub targets: Vec<Target>,
    pub explanation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub base_revision: u64,
    pub decision: Option<Decision>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub target_keys: Vec<String>,
    pub kind: String,
    pub support: Support,
    pub objective: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub prompt: String,
    pub target_keys: Vec<String>,
    pub kind: String,
    pub support: Support,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentedTask {
    pub prompt: String,
    #[serde(default)]
    pub target_keys: Vec<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub support: Support,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProposal {
    pub task_id: String,
    pub target_key: String,
    pub user_span: String,
    pub kind: EvidenceKind,
    pub success: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub event_id: Uuid,
    pub session_id: String,
    pub task_id: String,
    pub mode: String,
    pub stage: String,
    pub target_key: String,
    pub kind: EvidenceKind,
    pub user_span: String,
    pub success: bool,
    pub support: Support,
    pub at: DateTime<Utc>,
}

pub fn validate(
    mut decision: Decision,
    current: &Session,
    text: &str,
    proactive: bool,
) -> Result<Decision, String> {
    let book = playbook(&decision.mode).ok_or("unknown learning mode")?;
    let stage = book
        .stages
        .iter()
        .find(|s| s.id == decision.stage)
        .ok_or("unknown learning stage")?;
    if !decision.confidence.is_finite() || decision.confidence < 0.75 || decision.confidence > 1.0 {
        return Err("uncertain workflow decision".into());
    }
    if decision.mode != current.mode {
        if !book.entry.contains(&decision.stage)
            || (!proactive
                && (decision.switch_evidence.trim().is_empty()
                    || !text.contains(decision.switch_evidence.trim())))
            || (proactive && (current.mode != "conversation" || current.task.is_some()))
        {
            return Err("mode switch needs current user evidence and an entry stage".into());
        }
        if !proactive {
            decision.assess_user = false;
        }
    } else {
        let previous = book
            .stages
            .iter()
            .find(|s| s.id == current.stage)
            .ok_or("invalid current learning stage")?;
        if !previous.next.contains(&stage.id) {
            return Err("invalid learning stage transition".into());
        }
    }
    if proactive
        && (decision.context != ContextRequest::None
            || (decision.mode == current.mode
                && (decision.task_policy != TaskPolicy::Keep || decision.task_spec.is_some()))
            || decision.assess_user
            || decision.resume_practice
            || decision.support_provided != Support::None)
    {
        return Err("proactive turn cannot assess, resume, request a photo or change a task in the current mode".into());
    }
    if decision.resume_practice
        && (!current.awaiting_choice
            || decision.mode != current.mode
            || decision.assess_user
            || decision.switch_evidence.trim().is_empty()
            || !text.contains(decision.switch_evidence.trim()))
    {
        return Err(
            "resuming needs an affirmative current user span after a practice check-in".into(),
        );
    }
    if decision.context == ContextRequest::CurrentMaterial && current.material.is_none() {
        return Err("no current learning material".into());
    }
    if decision.targets.len() > 3 {
        return Err("too many learning targets".into());
    }
    decision.goal = clean(&decision.goal, 240);
    decision.strategy = clean(&decision.strategy, 500);
    decision.scene = clean(&decision.scene, 1000);
    if book.fictional && decision.mode == current.mode && decision.scene.is_empty() {
        decision.scene = current.scene.clone();
    }
    decision.explanation = clean(&decision.explanation, 240);
    for target in &mut decision.targets {
        target.key = clean(&target.key, 80).to_lowercase();
        target.meaning = clean(&target.meaning, 240);
        if target.key.is_empty()
            || ![
                "vocabulary",
                "grammar",
                "sentence",
                "communication",
                "comprehension",
                "pronunciation",
            ]
            .contains(&target.kind.as_str())
        {
            return Err("invalid learning target".into());
        }
    }
    if decision.goal.is_empty() || decision.strategy.is_empty() {
        return Err("learning goal and strategy required".into());
    }
    if let Some(task) = &mut decision.task_spec {
        task.objective = clean(&task.objective, 500);
        for key in &mut task.target_keys {
            *key = clean(key, 80).to_lowercase();
        }
        if task.target_keys.len() > 3 || task.objective.is_empty() {
            return Err("task requires a nonempty objective and at most three target keys".into());
        }
        if ![
            "meaning_recall",
            "sentence_use",
            "communication",
            "comprehension",
            "transfer",
            "pronunciation_repeat",
        ]
        .contains(&task.kind.as_str())
        {
            return Err("unknown exercise kind".into());
        }
        let effective_targets = if decision.mode != current.mode || !decision.targets.is_empty() {
            &decision.targets
        } else {
            &current.targets
        };
        if task
            .target_keys
            .iter()
            .any(|key| !effective_targets.iter().any(|target| target.key == *key))
        {
            return Err("every task target_key must match decision.targets; only unchanged-mode empty targets inherit current_learning.targets".into());
        }
        if (task.kind == "pronunciation_repeat") != (decision.mode == "pronunciation")
            || (task.kind == "pronunciation_repeat"
                && (task.target_keys.len() != 1
                    || !effective_targets.iter().any(|target| {
                        target.key == task.target_keys[0] && target.kind == "pronunciation"
                    })))
        {
            return Err("a pronunciation exercise needs one pronunciation target".into());
        }
        decision.task_policy = TaskPolicy::Replace;
    } else if decision.task_policy == TaskPolicy::Replace {
        return Err("new task needs a planned specification".into());
    }
    Ok(decision)
}

/// The server selects the next suggested focus from recorded evidence. The
/// planner can decline it or choose how to teach it within the catalog.
pub fn recommendation(profile: &Profile, now: DateTime<Utc>) -> Option<(&'static str, Target)> {
    let current = session(profile, &profile.active_agent_id);
    if current.mode != "conversation" || current.task.is_some() {
        return None;
    }
    if current.practice_ended_at.is_some_and(|at| {
        now.signed_duration_since(at).num_seconds()
            < profile.settings.proactive_cooldown_secs as i64
    }) {
        return None;
    }
    if let Some(word) = current
        .pronunciation_focus
        .filter(|word| !profile.forgotten.contains(word))
    {
        return Some((
            "pronunciation",
            Target {
                key: word,
                kind: "pronunciation".into(),
                meaning: "Model and repeat the word in a short phrase".into(),
            },
        ));
    }
    if let Some(item) = current
        .items
        .iter()
        .filter(|item| {
            item.next_review_at.is_some_and(|at| at <= now)
                && matches!(item.kind.as_str(), "vocabulary" | "grammar" | "sentence")
                && !profile.forgotten.contains(&item.key)
        })
        .min_by_key(|item| item.next_review_at)
    {
        return Some((
            "review",
            Target {
                key: item.key.clone(),
                kind: item.kind.clone(),
                meaning: item.meaning.clone(),
            },
        ));
    }
    profile
        .weaknesses
        .iter()
        .filter(|item| {
            item.next_review_at <= now
                && item.failures.saturating_sub(item.successful_recalls) >= 2
                && matches!(item.kind.as_str(), "vocabulary" | "grammar" | "sentence")
                && !profile.forgotten.contains(&item.key)
        })
        .min_by_key(|item| item.next_review_at)
        .map(|item| {
            (
                "review",
                Target {
                    key: item.key.clone(),
                    kind: item.kind.clone(),
                    meaning: item.corrected.clone(),
                },
            )
        })
}

pub fn authorize_outreach(
    decision: &Decision,
    current: &Session,
    profile: &Profile,
) -> Result<(), String> {
    if decision.mode == current.mode {
        return Ok(());
    }
    let Some((mode, target)) = recommendation(profile, Utc::now()) else {
        return Err("proactive scene needs a recorded learning focus".into());
    };
    if decision.mode != mode
        || decision.targets.len() != 1
        || decision.targets[0].key != target.key
        || decision.targets[0].kind != target.kind
        || !decision.task_spec.as_ref().is_some_and(|task| {
            task.target_keys == [target.key]
                && (mode != "pronunciation" || task.kind == "pronunciation_repeat")
        })
    {
        return Err("proactive scene must use the recommended mode, focus and exercise".into());
    }
    Ok(())
}

pub fn prepare_practice(
    decision: &mut Decision,
    current: &Session,
    text: &str,
    spoken: bool,
    assessment: Option<Assessment>,
) {
    if decision.mode != current.mode || decision.mode == "conversation" {
        if decision.assess_user && decision.mode == "conversation" && current.mode == "conversation"
        {
            decision.pronunciation = assessment;
        }
        return;
    }
    if current.awaiting_choice && !decision.resume_practice {
        decision.check_in = true;
        decision.assess_user = false;
    } else if decision.assess_user && decision.context != ContextRequest::FreshCamera {
        if current.mode == "pronunciation" {
            let target = current
                .task
                .as_ref()
                .filter(|task| task.kind == "pronunciation_repeat")
                .and_then(|task| task.target_keys.first());
            decision.practice_attempt = spoken
                && target.is_some_and(|target| {
                    assessment
                        .as_ref()
                        .is_some_and(|score| score.reference == *target)
                        || super::pronunciation::contains_words(text, target)
                });
            if decision.practice_attempt {
                decision.pronunciation = assessment;
            }
        } else {
            decision.practice_attempt =
                current.task.is_some() || matches!(current.mode.as_str(), "roleplay" | "reading");
        }
        decision.check_in =
            decision.practice_attempt && current.practice_attempts >= PRACTICE_ROUNDS - 1;
    }
    if decision.check_in {
        if decision.mode == "pronunciation" {
            decision.stage = "feedback".into();
        }
        decision.task_policy = TaskPolicy::Clear;
        decision.task_spec = None;
        decision.context = ContextRequest::None;
        decision.support_provided = Support::None;
    }
}

pub fn parse(
    text: &str,
    current: &Session,
    user_text: &str,
    proactive: bool,
) -> Result<Decision, String> {
    if text.len() > 12000 {
        return Err("workflow response exceeds limit".into());
    }
    let json = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text.trim())
        .trim()
        .trim_end_matches("```")
        .trim();
    let decision =
        serde_json::from_str(json).map_err(|error| format!("invalid workflow JSON: {error}"))?;
    validate(decision, current, user_text, proactive)
}

/// An operational fallback has no teaching side effects and never calls a tool.
pub fn fallback(current: &Session) -> Execution {
    Execution {
        base_revision: current.revision,
        decision: None,
        status: "clarification".into(),
    }
}

/// Keep legacy issue extraction only when the workflow says the learner is
/// producing assessable language. Scripted dialogue never creates real-life memories.
pub fn filter_analysis(analysis: &Analysis, execution: &Execution, text: &str) -> Analysis {
    let mut filtered = analysis.clone();
    filtered.recalls.clear(); // Workflow evidence, not a model's unbound recall claim, owns this.
    filtered.memories.retain(|memory| {
        !memory.evidence.trim().is_empty()
            && text
                .to_lowercase()
                .contains(&memory.evidence.to_lowercase())
    });
    match &execution.decision {
        None => {
            filtered.issues.clear();
            filtered.memories.clear();
        }
        Some(d) => {
            if !d.assess_user || d.mode == "pronunciation" {
                filtered.issues.clear();
            }
            if !d.switch_evidence.is_empty()
                || d.mode == "pronunciation"
                || playbook(&d.mode).is_some_and(|book| book.fictional)
            {
                filtered.memories.clear();
            }
        }
    }
    filtered
}

/// Called inside the profile transaction. False means a newer learning state
/// won; no workflow or mastery evidence may be applied from this stale plan.
pub struct Commit<'a> {
    pub agent_id: &'a str,
    pub execution: &'a Execution,
    pub analysis: &'a Analysis,
    pub text: &'a str,
    pub event: Uuid,
    pub visual: Option<&'a Context>,
    pub now: DateTime<Utc>,
}
pub fn commit(profile: &mut Profile, update: Commit<'_>) -> bool {
    let Commit {
        agent_id,
        execution,
        analysis,
        text,
        event,
        visual,
        now,
    } = update;
    let mut current = session(profile, agent_id);
    if current.last_event == Some(event) {
        return true;
    }
    if current.revision != execution.base_revision {
        return false;
    }
    let Some(decision) = &execution.decision else {
        return true;
    };
    let switching = current.mode != decision.mode;
    if decision.check_in && !analysis.reply_en.contains('?') {
        return false;
    }
    let spoken_prompt = analysis
        .next_task
        .as_ref()
        .map(|task| clean(&task.prompt, 500))
        .filter(|prompt| !prompt.is_empty() && analysis.reply_en.contains(prompt));
    if let Some(spec) = &decision.task_spec
        && (switching || decision.mode == "pronunciation")
        && (spoken_prompt.is_none()
            || (decision.mode == "pronunciation"
                && !spec.target_keys.iter().all(|target| {
                    super::pronunciation::contains_words(&analysis.reply_en, target)
                })))
    {
        return false;
    }
    if current.id.is_empty() || switching {
        current.id = Uuid::new_v4().to_string();
    }
    let previous_task = if switching {
        None
    } else {
        current.task.clone()
    };
    let previous_assistant = profile
        .history
        .iter()
        .rev()
        .find(|line| line.role == "assistant" && line.agent_id == agent_id)
        .map(|line| line.text.to_lowercase())
        .unwrap_or_default();
    let mut assessed = false;
    if decision.assess_user
        && current.mode != "pronunciation"
        && let Some(task) = &previous_task
    {
        let mut seen = std::collections::HashSet::new();
        for proposal in analysis.learning_evidence.iter().take(5) {
            let span = clean(&proposal.user_span, 500);
            if (proposal.task_id != "current" && proposal.task_id != task.id)
                || span.is_empty()
                || !text.to_lowercase().contains(&span.to_lowercase())
                || !task.target_keys.contains(&proposal.target_key)
                || !seen.insert(proposal.target_key.clone())
                || profile.forgotten.contains(&proposal.target_key)
            {
                continue;
            }
            let mut kind = proposal.kind;
            if matches!(kind, EvidenceKind::Exposure) {
                continue;
            }
            if previous_assistant.contains(&span.to_lowercase())
                || current.material.as_ref().is_some_and(|material| {
                    material
                        .context
                        .text
                        .to_lowercase()
                        .contains(&span.to_lowercase())
                })
                || task.support == Support::Answer
            {
                kind = EvidenceKind::Imitation;
            } else if task.support == Support::Hint
                && matches!(kind, EvidenceKind::IndependentUse | EvidenceKind::Transfer)
            {
                kind = EvidenceKind::CuedRecall;
            }
            let Some(item) = current
                .items
                .iter_mut()
                .find(|item| item.key == proposal.target_key)
            else {
                continue;
            };
            match kind {
                EvidenceKind::Imitation => item.imitations += 1,
                EvidenceKind::CuedRecall => item.assisted += u32::from(proposal.success),
                EvidenceKind::IndependentUse | EvidenceKind::Transfer if proposal.success => {
                    item.independent += 1;
                    item.next_review_at =
                        Some(now + chrono::Duration::days(i64::from(item.independent.min(7))));
                    if let Some(weakness) = profile
                        .weaknesses
                        .iter_mut()
                        .find(|weakness| weakness.key == proposal.target_key)
                    {
                        weakness.successful_recalls = weakness.successful_recalls.saturating_add(1);
                        weakness.interval_days = item.independent.min(7);
                        weakness.next_review_at = item.next_review_at.unwrap();
                        weakness.updated_at = now;
                    }
                }
                _ => {}
            }
            if !proposal.success {
                item.failures += 1;
            }
            current.evidence.push(Evidence {
                id: format!("{event}:{}", proposal.target_key),
                event_id: event,
                session_id: current.id.clone(),
                task_id: task.id.clone(),
                mode: current.mode.clone(),
                stage: current.stage.clone(),
                target_key: proposal.target_key.clone(),
                kind,
                user_span: span,
                success: proposal.success,
                support: task.support,
                at: now,
            });
            assessed = true;
        }
    }
    if switching || assessed || decision.task_policy != TaskPolicy::Keep {
        current.task = None;
    }
    // Assistance is cumulative for an unanswered task. A later neutral turn
    // cannot erase an earlier hint and turn helped recall into independent use.
    if let Some(task) = &mut current.task {
        task.support = match (task.support, decision.support_provided) {
            (Support::Answer, _) | (_, Support::Answer) => Support::Answer,
            (Support::Hint, _) | (_, Support::Hint) => Support::Hint,
            _ => Support::None,
        };
    }
    current.mode = decision.mode.clone();
    current.stage = decision.stage.clone();
    current.goal = decision.goal.clone();
    current.strategy = decision.strategy.clone();
    current.scene = decision.scene.clone();
    if switching || decision.resume_practice {
        current.practice_attempts = 0;
        current.awaiting_choice = false;
    }
    if switching && decision.mode == "conversation" {
        current.practice_ended_at = Some(now);
    }
    if decision.practice_attempt {
        current.practice_attempts = current
            .practice_attempts
            .saturating_add(1)
            .min(PRACTICE_ROUNDS);
    }
    if decision.check_in {
        current.awaiting_choice = true;
    }
    if switching && decision.mode == "pronunciation" {
        current.pronunciation_focus = None;
        current.last_pronunciation = None;
    }
    if let Some(score) = &decision.pronunciation {
        if decision.mode == "conversation"
            && let Some(word) = &score.weak_word
            && !profile.forgotten.contains(word)
        {
            current.pronunciation_focus = Some(word.clone());
        }
        current.last_pronunciation = Some(score.clone());
    }
    if switching || !decision.targets.is_empty() {
        current.targets = decision.targets.clone();
    }
    for target in &current.targets {
        if profile.forgotten.contains(&target.key) {
            continue;
        }
        let first_exposure = current
            .items
            .iter()
            .find(|item| item.key == target.key)
            .is_none_or(|item| item.exposures == 0);
        if !current.items.iter().any(|item| item.key == target.key) && current.items.len() < 128 {
            current.items.push(Item {
                key: target.key.clone(),
                kind: target.kind.clone(),
                meaning: target.meaning.clone(),
                ..Default::default()
            });
        }
        if playbook(&decision.mode).is_some_and(|book| {
            book.exposure_stage
                .as_ref()
                .is_some_and(|stage| stage == &decision.stage || first_exposure)
        }) && analysis.reply_en.to_lowercase().contains(&target.key)
        {
            if let Some(item) = current.items.iter_mut().find(|item| item.key == target.key) {
                item.exposures += 1;
            }
            if let Some(item) = current.items.iter_mut().find(|item| item.key == target.key) {
                item.next_review_at
                    .get_or_insert(now + chrono::Duration::days(1));
            }
            current.evidence.push(Evidence {
                id: format!("{event}:exposure:{}", target.key),
                event_id: event,
                session_id: current.id.clone(),
                task_id: String::new(),
                mode: current.mode.clone(),
                stage: current.stage.clone(),
                target_key: target.key.clone(),
                kind: EvidenceKind::Exposure,
                user_span: String::new(),
                success: false,
                support: Support::Answer,
                at: now,
            });
        }
    }
    if decision.task_policy == TaskPolicy::Replace
        && let Some(prompt) = spoken_prompt
        && let Some(planned) = &decision.task_spec
    {
        let keys = &planned.target_keys;
        let kind = planned.kind.as_str();
        let support = planned.support;
        if keys.len() <= 3
            && keys.iter().all(|key| {
                current.targets.iter().any(|target| target.key == *key)
                    && current.items.iter().any(|item| item.key == *key)
            })
            && [
                "meaning_recall",
                "sentence_use",
                "communication",
                "comprehension",
                "transfer",
                "pronunciation_repeat",
            ]
            .contains(&kind)
        {
            current.task = Some(Task {
                id: format!("{event}:task"),
                prompt,
                target_keys: keys.clone(),
                kind: kind.into(),
                support,
            });
        }
    }
    if let Some(context) = visual {
        current.material = Some(Material {
            id: event.to_string(),
            context: context.clone(),
        });
    }
    if current.evidence.len() > 128 {
        current.evidence.drain(..current.evidence.len() - 128);
    }
    current.revision += 1;
    current.last_event = Some(event);
    current.catalog_version = catalog().version;
    profile
        .agent_states
        .entry(agent_id.to_owned())
        .or_default()
        .learning = current;
    true
}

#[cfg(test)]
mod tests {
    use super::super::model::Dialogue;
    use super::*;
    #[test]
    fn pronunciation_check_in_waits_for_three_spoken_attempts_and_a_choice() {
        let mut profile = Profile::default();
        let now = Utc::now();
        profile
            .agent_states
            .entry("amigo".into())
            .or_default()
            .learning = Session {
            mode: "pronunciation".into(),
            stage: "repeat".into(),
            task: Some(Task {
                id: "practice".into(),
                prompt: "Repeat three".into(),
                target_keys: vec!["three".into()],
                kind: "pronunciation_repeat".into(),
                support: Support::Answer,
            }),
            ..Default::default()
        };
        let mut analysis = Analysis {
            reply_en: "Try it once more.".into(),
            ..Default::default()
        };
        for attempt in 0..PRACTICE_ROUNDS {
            let current = session(&profile, "amigo");
            let mut next = decision("pronunciation", "repeat");
            next.assess_user = true;
            prepare_practice(&mut next, &current, "Three.", false, None);
            assert!(!next.practice_attempt);
            prepare_practice(&mut next, &current, "Three.", true, None);
            assert_eq!(next.check_in, attempt == PRACTICE_ROUNDS - 1);
            if next.check_in {
                analysis.reply_en = "Would you like another round or stop here?".into();
            }
            let execution = Execution {
                base_revision: current.revision,
                decision: Some(next),
                status: "planned".into(),
            };
            assert!(commit(
                &mut profile,
                Commit {
                    agent_id: "amigo",
                    execution: &execution,
                    analysis: &analysis,
                    text: "Three.",
                    event: Uuid::new_v4(),
                    visual: None,
                    now
                }
            ));
        }
        let current = session(&profile, "amigo");
        assert_eq!(current.practice_attempts, PRACTICE_ROUNDS);
        assert!(current.awaiting_choice && current.task.is_none());
        assert!(current.evidence.is_empty() && current.last_pronunciation.is_none());
        let mut next = decision("pronunciation", "model");
        next.resume_practice = true;
        assert!(validate(next.clone(), &current, "Yes, let's continue", false).is_err());
        next.switch_evidence = "Yes, let's continue".into();
        let next = validate(next, &current, "Yes, let's continue", false).unwrap();
        let execution = Execution {
            base_revision: current.revision,
            decision: Some(next),
            status: "planned".into(),
        };
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &execution,
                analysis: &analysis,
                text: "Yes, let's continue",
                event: Uuid::new_v4(),
                visual: None,
                now
            }
        ));
        let resumed = session(&profile, "amigo");
        assert!(!resumed.awaiting_choice);
        assert_eq!(resumed.practice_attempts, 0);
    }
    fn decision(mode: &str, stage: &str) -> Decision {
        Decision {
            resume_practice: false,
            practice_attempt: false,
            check_in: false,
            pronunciation: None,
            support_provided: Support::None,
            task_spec: None,
            mode: mode.into(),
            stage: stage.into(),
            confidence: 0.95,
            switch_evidence: String::new(),
            goal: "Use resilient naturally".into(),
            strategy: "Give one example then invite original production".into(),
            scene: String::new(),
            action: ResponseAct::Teach,
            context: ContextRequest::None,
            task_policy: TaskPolicy::Keep,
            assess_user: false,
            targets: vec![],
            explanation: "按当前任务继续".into(),
        }
    }
    #[test]
    fn catalog_entries_and_transitions_are_real_stages() {
        let mut ids = std::collections::HashSet::new();
        for book in &catalog().modes {
            assert!(ids.insert(book.id.clone()));
            assert!(!book.entry.is_empty());
            for entry in &book.entry {
                assert!(book.stages.iter().any(|stage| stage.id == *entry));
            }
            for stage in &book.stages {
                for next in &stage.next {
                    assert!(book.stages.iter().any(|stage| stage.id == *next));
                }
            }
        }
        assert_eq!(ids.len(), 6);
        assert!(ids.contains("pronunciation"));
    }
    #[test]
    fn invalid_low_confidence_and_proactive_decisions_have_no_authority() {
        let state = Session::default().normalized();
        let mut d = decision("vocabulary", "explain");
        assert!(validate(d.clone(), &state, "teach resilient", false).is_err());
        d.switch_evidence = "teach resilient".into();
        assert!(validate(d.clone(), &state, "teach resilient", false).is_ok());
        d.stage = "unknown_stage".into();
        assert!(validate(d.clone(), &state, "teach resilient", false).is_err());
        d.stage = "explain".into();
        d.confidence = f32::NAN;
        assert!(validate(d, &state, "teach resilient", false).is_err());
        let mut d = decision("conversation", "converse");
        d.context = ContextRequest::FreshCamera;
        assert!(validate(d, &state, "", true).is_err());
        let mut d = decision("conversation", "converse");
        d.context = ContextRequest::CurrentMaterial;
        assert!(validate(d, &state, "read it", false).is_err());
        assert!(parse("{\"mode\":\"conversation\"}", &state, "hello", false).is_err());
    }
    #[test]
    fn help_on_a_kept_task_survives_neutral_turns() {
        let mut profile = Profile::default();
        let now = Utc::now();
        let target = Target {
            key: "resilient".into(),
            kind: "vocabulary".into(),
            meaning: "able to recover".into(),
        };
        let state = Session {
            mode: "vocabulary".into(),
            stage: "retrieve".into(),
            targets: vec![target],
            items: vec![Item {
                key: "resilient".into(),
                ..Default::default()
            }],
            task: Some(Task {
                id: "original".into(),
                prompt: "Define resilient".into(),
                target_keys: vec!["resilient".into()],
                kind: "meaning_recall".into(),
                support: Support::None,
            }),
            ..Default::default()
        };
        profile
            .agent_states
            .entry("amigo".into())
            .or_default()
            .learning = state;
        for (revision, support) in [(0, Support::Hint), (1, Support::None)] {
            let mut d = decision("vocabulary", "retrieve");
            d.support_provided = support;
            assert!(commit(
                &mut profile,
                Commit {
                    agent_id: "amigo",
                    execution: &Execution {
                        base_revision: revision,
                        decision: Some(d),
                        status: "planned".into()
                    },
                    analysis: &Analysis::default(),
                    text: "Can I have a hint?",
                    event: Uuid::new_v4(),
                    visual: None,
                    now
                }
            ));
            assert_eq!(
                session(&profile, "amigo").task.unwrap().support,
                Support::Hint
            );
        }
        let mut d = decision("vocabulary", "retrieve");
        d.assess_user = true;
        let analysis = Analysis {
            learning_evidence: vec![EvidenceProposal {
                task_id: "current".into(),
                target_key: "resilient".into(),
                user_span: "able to recover".into(),
                kind: EvidenceKind::IndependentUse,
                success: true,
            }],
            ..Default::default()
        };
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &Execution {
                    base_revision: 2,
                    decision: Some(d),
                    status: "planned".into()
                },
                analysis: &analysis,
                text: "It means able to recover",
                event: Uuid::new_v4(),
                visual: None,
                now
            }
        ));
        let state = session(&profile, "amigo");
        assert_eq!(state.items[0].independent, 0);
        assert_eq!(state.items[0].assisted, 1);
        assert_eq!(state.evidence[0].task_id, "original");
        assert_eq!(state.evidence[0].kind, EvidenceKind::CuedRecall);
    }
    #[test]
    fn exposure_imitation_and_independent_use_are_different_evidence() {
        let mut profile = Profile::default();
        let now = Utc::now();
        // A reply can briefly introduce a word and enter guided practice in
        // the same turn; exposure follows what was actually said, not the label.
        let mut d = decision("vocabulary", "guided_practice");
        d.targets = vec![Target {
            key: "resilient".into(),
            kind: "vocabulary".into(),
            meaning: "able to recover".into(),
        }];
        d.task_policy = TaskPolicy::Replace;
        d.task_spec = Some(TaskSpec {
            target_keys: vec!["resilient".into()],
            kind: "sentence_use".into(),
            support: Support::None,
            objective: "Use the word in an original sentence".into(),
        });
        let first = Execution {
            base_revision: 0,
            decision: Some(d),
            status: "planned".into(),
        };
        let mut analysis=Analysis {reply_en:"Resilient means able to recover. The tree is resilient. Use resilient in a sentence.".into(),next_task:Some(PresentedTask{prompt:"Use resilient in a sentence.".into(),target_keys:vec!["resilient".into()],kind:"sentence_use".into(),support:Support::None}),..Default::default()};
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &first,
                analysis: &analysis,
                text: "Teach me resilient",
                event: Uuid::new_v4(),
                visual: None,
                now
            }
        ));
        let state = session(&profile, "amigo");
        assert_eq!(state.items[0].exposures, 1);
        assert_eq!(state.items[0].independent, 0);
        assert!(profile.weaknesses.is_empty());
        let task = state.task.clone().unwrap();
        assert_eq!(task.kind, "sentence_use");
        profile.history.push(Dialogue {
            role: "assistant".into(),
            text: analysis.reply_en.clone(),
            agent_id: "amigo".into(),
            ..Default::default()
        });
        let mut d = decision("vocabulary", "guided_practice");
        d.assess_user = true;
        let second = Execution {
            base_revision: 1,
            decision: Some(d.clone()),
            status: "planned".into(),
        };
        analysis.reply_en = "Try another example in your own context.".into();
        analysis.next_task = None;
        analysis.learning_evidence = vec![EvidenceProposal {
            task_id: task.id,
            target_key: "resilient".into(),
            user_span: "The tree is resilient.".into(),
            kind: EvidenceKind::IndependentUse,
            success: true,
        }];
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &second,
                analysis: &analysis,
                text: "The tree is resilient.",
                event: Uuid::new_v4(),
                visual: None,
                now
            }
        ));
        let state = session(&profile, "amigo");
        assert_eq!(state.items[0].imitations, 1);
        assert_eq!(state.items[0].independent, 0);
        let mut state = state;
        state.task = Some(Task {
            id: "independent-task".into(),
            prompt: "Describe your town".into(),
            target_keys: vec!["resilient".into()],
            kind: "transfer".into(),
            support: Support::None,
        });
        profile.agent_states.get_mut("amigo").unwrap().learning = state;
        let third = Execution {
            base_revision: 2,
            decision: Some(d),
            status: "planned".into(),
        };
        analysis.learning_evidence = vec![EvidenceProposal {
            task_id: "current".into(),
            target_key: "resilient".into(),
            user_span: "Our town is resilient after the storm.".into(),
            kind: EvidenceKind::Transfer,
            success: true,
        }];
        let event = Uuid::new_v4();
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &third,
                analysis: &analysis,
                text: "Our town is resilient after the storm.",
                event,
                visual: None,
                now
            }
        ));
        let state = session(&profile, "amigo");
        assert_eq!(state.items[0].independent, 1);
        assert_eq!(state.evidence.last().unwrap().task_id, "independent-task");
        assert!(state.task.is_none());
        assert!(state.items[0].next_review_at.is_some());
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &third,
                analysis: &analysis,
                text: "Our town is resilient after the storm.",
                event,
                visual: None,
                now
            }
        ));
        assert_eq!(session(&profile, "amigo").items[0].independent, 1);
    }
    #[test]
    fn stale_revision_and_unspoken_task_cannot_advance_learning() {
        let mut profile = Profile::default();
        let now = Utc::now();
        let mut d = decision("conversation", "converse");
        d.task_policy = TaskPolicy::Replace;
        d.task_spec = Some(TaskSpec {
            target_keys: vec![],
            kind: "communication".into(),
            support: Support::None,
            objective: "A question never spoken".into(),
        });
        let execution = Execution {
            base_revision: 0,
            decision: Some(d),
            status: "planned".into(),
        };
        let analysis = Analysis {
            reply_en: "Hello!".into(),
            next_task: Some(PresentedTask {
                prompt: "A question never spoken?".into(),
                target_keys: vec![],
                kind: "communication".into(),
                support: Support::None,
            }),
            ..Default::default()
        };
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &execution,
                analysis: &analysis,
                text: "hello",
                event: Uuid::new_v4(),
                visual: None,
                now
            }
        ));
        assert!(session(&profile, "amigo").task.is_none());
        assert!(!commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &execution,
                analysis: &analysis,
                text: "hello",
                event: Uuid::new_v4(),
                visual: None,
                now
            }
        ));
        assert_eq!(session(&profile, "amigo").revision, 1);
    }
    #[test]
    fn roleplay_and_planner_failure_cannot_create_real_identity_memories() {
        let execution = Execution {
            base_revision: 0,
            decision: Some(decision("roleplay", "in_scene")),
            status: "planned".into(),
        };
        let analysis = Analysis {
            reply_en: "Welcome, doctor.".into(),
            memories: vec![super::super::model::MemoryEvidence {
                evidence: "I am a doctor".into(),
                subject: "occupation".into(),
                summary: "User is a doctor".into(),
            }],
            ..Default::default()
        };
        assert!(
            filter_analysis(&analysis, &execution, "I am a doctor")
                .memories
                .is_empty()
        );
        let mut profile = Profile::default();
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &execution,
                analysis: &analysis,
                text: "I am a doctor",
                event: Uuid::new_v4(),
                visual: None,
                now: Utc::now()
            }
        ));
        profile.learn_for_agent(
            "I am a doctor",
            &filter_analysis(&analysis, &execution, "I am a doctor"),
            &[],
            Utc::now(),
            "amigo",
            ResponseAct::Respond,
        );
        assert!(profile.memories.is_empty());
        assert!(profile.agent_states["amigo"].memories.is_empty());
        assert!(
            profile
                .history
                .iter()
                .all(|line| line.fictional && line.learning_mode == "roleplay")
        );
        let real = Execution {
            base_revision: 0,
            decision: Some(decision("conversation", "converse")),
            status: "planned".into(),
        };
        assert_eq!(
            filter_analysis(&analysis, &real, "I am a doctor")
                .memories
                .len(),
            1
        );
        assert!(
            filter_analysis(&analysis, &real, "Let's end the scene")
                .memories
                .is_empty()
        );
        let failed = fallback(&session(&profile, "amigo"));
        assert!(
            filter_analysis(&analysis, &failed, "hello")
                .memories
                .is_empty()
        );
        assert!(commit(
            &mut profile,
            Commit {
                agent_id: "amigo",
                execution: &failed,
                analysis: &analysis,
                text: "hello",
                event: Uuid::new_v4(),
                visual: None,
                now: Utc::now()
            }
        ));
        assert_eq!(session(&profile, "amigo").revision, 1);
    }
}
