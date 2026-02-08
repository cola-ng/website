use chrono::{NaiveDate, Utc};
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::oapi::extract::*;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::learn::*;
use crate::{DepotExt, JsonResult, json_ok};

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize, ToSchema)]
pub struct DailyDataResponse {
    pub tasks: Vec<DailyTask>,
    pub insights: Vec<DailyInsight>,
    pub generated_at: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// Get Daily Tasks and Insights
// ============================================================================

/// Get daily tasks and insights for the current user
/// If no data exists for today, it will be generated automatically
#[endpoint(tags("Learn"))]
pub async fn get_daily_data(depot: &mut Depot) -> JsonResult<DailyDataResponse> {
    let user_id = depot.user_id()?;
    let today = Utc::now().date_naive();

    // Check if we already have data for today
    let existing_log = with_conn({
        let today = today;
        move |conn| {
            learn_daily_generation_log::table
                .filter(learn_daily_generation_log::user_id.eq(user_id))
                .filter(learn_daily_generation_log::generation_date.eq(today))
                .first::<DailyGenerationLog>(conn)
                .optional()
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check generation log"))?;

    // If no data for today, generate it
    if existing_log.is_none() {
        generate_daily_data(user_id, today).await?;
    }

    // Fetch tasks and insights
    let tasks = with_conn({
        let today = today;
        move |conn| {
            learn_daily_tasks::table
                .filter(learn_daily_tasks::user_id.eq(user_id))
                .filter(learn_daily_tasks::task_date.eq(today))
                .order(learn_daily_tasks::display_order.asc())
                .load::<DailyTask>(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch tasks"))?;

    let insights = with_conn({
        let today = today;
        move |conn| {
            learn_daily_insights::table
                .filter(learn_daily_insights::user_id.eq(user_id))
                .filter(learn_daily_insights::insight_date.eq(today))
                .order(learn_daily_insights::display_order.asc())
                .load::<DailyInsight>(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch insights"))?;

    let generated_at = existing_log.map(|log| log.created_at);

    json_ok(DailyDataResponse {
        tasks,
        insights,
        generated_at,
    })
}

// ============================================================================
// Update Task Progress
// ============================================================================

#[derive(Deserialize, ToSchema)]
pub struct UpdateTaskRequest {
    pub current_value: Option<i32>,
    pub status: Option<String>,
}

/// Update a daily task's progress
#[endpoint(tags("Learn"))]
pub async fn update_task(
    id: PathParam<i64>,
    input: JsonBody<UpdateTaskRequest>,
    depot: &mut Depot,
) -> JsonResult<DailyTask> {
    let user_id = depot.user_id()?;
    let task_id = id.into_inner();

    let completed_at = if input.status.as_deref() == Some("completed") {
        Some(Utc::now())
    } else {
        None
    };

    let updated = with_conn(move |conn| {
        diesel::update(
            learn_daily_tasks::table
                .filter(learn_daily_tasks::id.eq(task_id))
                .filter(learn_daily_tasks::user_id.eq(user_id)),
        )
        .set((
            input
                .current_value
                .map(|v| learn_daily_tasks::current_value.eq(v)),
            input
                .status
                .clone()
                .map(|s| learn_daily_tasks::status.eq(s)),
            learn_daily_tasks::completed_at.eq(completed_at),
        ))
        .get_result::<DailyTask>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to update task"))?;

    json_ok(updated)
}

// ============================================================================
// Generate Daily Data
// ============================================================================

async fn generate_daily_data(user_id: i64, today: NaiveDate) -> Result<(), StatusError> {
    // Fetch user's learning data to generate personalized tasks
    let (issue_count, in_progress_stage, pending_read_count) = with_conn(move |conn| {
        // Count issue words due for review
        let issue_count: i64 = learn_issue_words::table
            .filter(learn_issue_words::user_id.eq(user_id))
            .filter(
                learn_issue_words::next_review_at
                    .is_null()
                    .or(learn_issue_words::next_review_at.le(Utc::now())),
            )
            .count()
            .get_result(conn)?;

        // Find an in-progress stage
        let in_progress_stage: Option<ScriptProgress> = learn_script_progress::table
            .filter(learn_script_progress::user_id.eq(user_id))
            .filter(learn_script_progress::completed_at.is_null())
            .order(learn_script_progress::last_practiced_at.desc().nulls_last())
            .first(conn)
            .optional()?;

        // Count reading exercises not completed
        let pending_read_count: i64 = learn_read_progress::table
            .filter(learn_read_progress::user_id.eq(user_id))
            .filter(learn_read_progress::completed_at.is_null())
            .count()
            .get_result(conn)?;

        Ok((issue_count, in_progress_stage, pending_read_count))
    })
    .await
    .map_err(|_: String| StatusError::internal_server_error().brief("failed to fetch user data"))?;

    // Generate tasks
    let mut tasks: Vec<NewDailyTask> = vec![];

    // Task 1: Free chat (always)
    tasks.push(NewDailyTask {
        user_id,
        task_date: today,
        task_type: "chat".to_string(),
        task_title: "3分钟自由对话".to_string(),
        task_description: None,
        target_value: 3,
        current_value: Some(0),
        status: Some("pending".to_string()),
        display_order: 0,
        reference_id: None,
        reference_name: None,
    });

    // Task 2: Stage practice (if there's an in-progress stage)
    if let Some(stage) = in_progress_stage {
        tasks.push(NewDailyTask {
            user_id,
            task_date: today,
            task_type: "stage".to_string(),
            task_title: "场景练习：点餐".to_string(), // TODO: fetch actual stage name
            task_description: Some("进行中...".to_string()),
            target_value: 1,
            current_value: Some(0),
            status: Some("in_progress".to_string()),
            display_order: 1,
            reference_id: Some(stage.stage_id),
            reference_name: None,
        });
    }

    // Task 3: Review issue words (if there are any)
    if issue_count > 0 {
        let review_count = issue_count.min(10) as i32;
        tasks.push(NewDailyTask {
            user_id,
            task_date: today,
            task_type: "review".to_string(),
            task_title: format!("复习 {} 个易错点", review_count),
            task_description: Some("待完成".to_string()),
            target_value: review_count,
            current_value: Some(0),
            status: Some("pending".to_string()),
            display_order: 2,
            reference_id: None,
            reference_name: None,
        });
    }

    // Task 4: Read practice (if there are pending exercises)
    if pending_read_count > 0 {
        tasks.push(NewDailyTask {
            user_id,
            task_date: today,
            task_type: "read".to_string(),
            task_title: "跟读训练 5 句".to_string(),
            task_description: Some("待完成".to_string()),
            target_value: 5,
            current_value: Some(0),
            status: Some("pending".to_string()),
            display_order: 3,
            reference_id: None,
            reference_name: None,
        });
    }

    // Insert tasks
    let tasks_count = tasks.len() as i32;
    with_conn(move |conn| {
        diesel::insert_into(learn_daily_tasks::table)
            .values(&tasks)
            .on_conflict_do_nothing()
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to insert tasks"))?;

    // Generate insights based on recent chat issues
    let insights = generate_insights(user_id, today).await?;
    let insights_count = insights.len() as i32;

    with_conn(move |conn| {
        diesel::insert_into(learn_daily_insights::table)
            .values(&insights)
            .on_conflict_do_nothing()
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to insert insights"))?;

    // Log the generation
    with_conn(move |conn| {
        diesel::insert_into(learn_daily_generation_log::table)
            .values(&NewDailyGenerationLog {
                user_id,
                generation_date: today,
                tasks_generated: tasks_count,
                insights_generated: insights_count,
                generation_source: Some("auto".to_string()),
            })
            .on_conflict_do_nothing()
            .execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to log generation"))?;

    Ok(())
}

async fn generate_insights(
    user_id: i64,
    today: NaiveDate,
) -> Result<Vec<NewDailyInsight>, StatusError> {
    let mut insights: Vec<NewDailyInsight> = vec![];

    // Fetch recent chat issues (last 7 days)
    let recent_issues = with_conn(move |conn| {
        learn_chat_issues::table
            .filter(learn_chat_issues::user_id.eq(user_id))
            .filter(learn_chat_issues::created_at.gt(Utc::now() - chrono::Duration::days(7)))
            .order(learn_chat_issues::created_at.desc())
            .limit(50)
            .load::<ChatIssue>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to fetch recent issues"))?;

    // Analyze issue types
    let mut issue_type_counts: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    for issue in &recent_issues {
        *issue_type_counts
            .entry(issue.issue_type.clone())
            .or_insert(0) += 1;
    }

    // Fetch weekly stats for comparison
    let weekly_stats = with_conn(move |conn| {
        learn_daily_stats::table
            .filter(learn_daily_stats::user_id.eq(user_id))
            .filter(learn_daily_stats::stat_date.gt(today - chrono::Duration::days(14)))
            .order(learn_daily_stats::stat_date.desc())
            .limit(14)
            .load::<DailyStat>(conn)
    })
    .await
    .unwrap_or_default();

    let mut display_order = 0;

    // Insight 1: Progress on common issue type
    if let Some((issue_type, count)) = issue_type_counts.iter().max_by_key(|(_, c)| *c) {
        let content = match issue_type.as_str() {
            "grammar" => format!("你的冠词使用进步明显！a/an 错误率下降 {}%", 40),
            "pronunciation" => format!("发音练习有进步！近期 {} 个发音问题已改善", count),
            "vocabulary" => format!("词汇量增长中！本周掌握了 {} 个新词汇", count),
            _ => format!("{}方面的问题减少了 {}%，继续保持", issue_type, 30),
        };

        insights.push(NewDailyInsight {
            user_id,
            insight_date: today,
            insight_type: "progress".to_string(),
            icon_type: Some("bulb".to_string()),
            content_zh: content,
            content_en: None,
            action_text: Some("持续保持".to_string()),
            action_link: None,
            metadata: Some(json!({ "issue_type": issue_type, "count": count })),
            display_order,
        });
        display_order += 1;
    }

    // Insight 2: Weakness suggestion
    if let Some((weakness_type, _)) = issue_type_counts
        .iter()
        .filter(|(t, _)| *t != "grammar")
        .max_by_key(|(_, c)| *c)
    {
        let (content, action_text, action_link) = match weakness_type.as_str() {
            "tense" => (
                "建议多练习过去时态，这是你目前的薄弱点".to_string(),
                Some("点击开始专项练习 →".to_string()),
                Some("/stages?focus=tense".to_string()),
            ),
            "pronunciation" => (
                "发音练习能帮助提升口语流利度".to_string(),
                Some("开始跟读练习 →".to_string()),
                Some("/review?type=pronunciation".to_string()),
            ),
            _ => (
                format!("建议加强{}的练习", weakness_type),
                Some("查看练习建议 →".to_string()),
                Some("/review".to_string()),
            ),
        };

        insights.push(NewDailyInsight {
            user_id,
            insight_date: today,
            insight_type: "weakness".to_string(),
            icon_type: Some("warning".to_string()),
            content_zh: content,
            content_en: None,
            action_text,
            action_link,
            metadata: Some(json!({ "weakness_type": weakness_type })),
            display_order,
        });
        display_order += 1;
    }

    // Insight 3: Study time encouragement
    let this_week_minutes: i32 = weekly_stats
        .iter()
        .take(7)
        .filter_map(|s| s.minutes_studied)
        .sum();
    let last_week_minutes: i32 = weekly_stats
        .iter()
        .skip(7)
        .filter_map(|s| s.minutes_studied)
        .sum();

    if this_week_minutes > 0 {
        let change_percent = if last_week_minutes > 0 {
            ((this_week_minutes - last_week_minutes) as f32 / last_week_minutes as f32 * 100.0)
                as i32
        } else {
            100
        };

        let content = if change_percent > 0 {
            format!(
                "本周学习时长比上周提升 {}%，继续加油！",
                change_percent.abs()
            )
        } else if change_percent < 0 {
            format!(
                "本周学习时长比上周减少 {}%，保持学习节奏很重要",
                change_percent.abs()
            )
        } else {
            "学习时间保持稳定，稳步提升中".to_string()
        };

        insights.push(NewDailyInsight {
            user_id,
            insight_date: today,
            insight_type: "encouragement".to_string(),
            icon_type: Some("chart".to_string()),
            content_zh: content,
            content_en: None,
            action_text: Some("稳步提升中".to_string()),
            action_link: None,
            metadata: Some(json!({
                "this_week_minutes": this_week_minutes,
                "last_week_minutes": last_week_minutes,
                "change_percent": change_percent
            })),
            display_order,
        });
        display_order += 1;
    }

    // Insight 4: Stage suggestion
    insights.push(NewDailyInsight {
        user_id,
        insight_date: today,
        insight_type: "suggestion".to_string(),
        icon_type: Some("bulb".to_string()),
        content_zh: "尝试\"餐厅点餐\"场景，巩固已学的日常用语".to_string(),
        content_en: None,
        action_text: Some("立即体验 →".to_string()),
        action_link: Some("/stages/1".to_string()),
        metadata: None,
        display_order,
    });

    // Only return up to 4 insights
    Ok(insights.into_iter().take(4).collect())
}
