use chrono::Utc;
use diesel::prelude::*;
use salvo::oapi::ToSchema;
use salvo::prelude::*;
use serde::Serialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::achievement::*;
use crate::{AppResult, DepotExt, JsonResult, hoops, json_ok};

pub fn router() -> Router {
    Router::with_path("achievements")
        .push(Router::with_path("definitions").get(list_achievement_definitions))
        .push(Router::with_path("ranks").get(list_rank_definitions))
        .push(
            Router::new()
                .hoop(hoops::require_auth)
                .push(Router::with_path("profile").get(get_user_profile_summary))
                .push(Router::with_path("my").get(list_user_achievements))
                .push(Router::with_path("xp-history").get(list_xp_history)),
        )
}

// ============================================================================
// Public Endpoints (no auth required)
// ============================================================================

/// List all active achievement definitions
#[handler]
pub async fn list_achievement_definitions(res: &mut Response) -> AppResult<()> {
    let achievements: Vec<AchievementDefinition> = with_conn(move |conn| {
        archive_achievement_definitions::table
            .filter(archive_achievement_definitions::is_active.eq(true))
            .filter(archive_achievement_definitions::is_hidden.eq(false).or(archive_achievement_definitions::is_hidden.is_null()))
            .order(archive_achievement_definitions::sort_order.asc())
            .load::<AchievementDefinition>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list achievements"))?;

    res.render(Json(achievements));
    Ok(())
}

/// List all rank definitions
#[handler]
pub async fn list_rank_definitions(res: &mut Response) -> AppResult<()> {
    let ranks: Vec<RankDefinition> = with_conn(move |conn| {
        archive_rank_definitions::table
            .order(archive_rank_definitions::level.asc())
            .load::<RankDefinition>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list ranks"))?;

    res.render(Json(ranks));
    Ok(())
}

// ============================================================================
// User-specific Endpoints (auth required)
// ============================================================================

/// Get user profile summary with achievements for header/dropdown
#[handler]
pub async fn get_user_profile_summary(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let summary = with_conn(move |conn| {
        // Get or create user profile
        let profile = get_or_create_profile(conn, user_id)?;

        // Get all ranks
        let ranks: Vec<RankDefinition> = archive_rank_definitions::table
            .order(archive_rank_definitions::level.asc())
            .load::<RankDefinition>(conn)?;

        // Find current rank and next rank
        let current_rank = ranks
            .iter()
            .filter(|r| r.min_xp <= profile.total_xp)
            .last()
            .cloned();

        let next_rank = current_rank.as_ref().and_then(|cr| {
            ranks.iter().find(|r| r.level == cr.level + 1).cloned()
        });

        let xp_to_next_rank = next_rank
            .as_ref()
            .map(|nr| nr.min_xp - profile.total_xp)
            .unwrap_or(0)
            .max(0);

        // Get recent completed achievements (last 5)
        // Step 1: Get user's recent completed achievement records
        let user_achievements: Vec<UserAchievementRecord> = archive_user_achievements::table
            .filter(archive_user_achievements::user_id.eq(user_id))
            .filter(archive_user_achievements::is_completed.eq(true))
            .order(archive_user_achievements::completed_at.desc())
            .limit(5)
            .load::<UserAchievementRecord>(conn)?;

        // Step 2: Get achievement definitions for those IDs
        let achievement_ids: Vec<i64> = user_achievements.iter().map(|a| a.achievement_id).collect();
        let definitions: Vec<AchievementDefinition> = archive_achievement_definitions::table
            .filter(archive_achievement_definitions::id.eq_any(&achievement_ids))
            .load::<AchievementDefinition>(conn)?;

        // Step 3: Combine the results, preserving the order from user_achievements
        let recent_achievements: Vec<AchievementBadge> = user_achievements
            .into_iter()
            .filter_map(|ua| {
                definitions.iter().find(|d| d.id == ua.achievement_id).map(|def| {
                    AchievementBadge {
                        code: def.code.clone(),
                        name_en: def.name_en.clone(),
                        name_zh: def.name_zh.clone(),
                        icon: def.icon.clone(),
                        rarity: def.rarity.clone(),
                        completed_at: ua.completed_at,
                    }
                })
            })
            .collect();

        // Count total and completed achievements
        let total_achievements: i64 = archive_achievement_definitions::table
            .filter(archive_achievement_definitions::is_active.eq(true))
            .count()
            .get_result(conn)?;

        let completed_achievements: i64 = archive_user_achievements::table
            .filter(archive_user_achievements::user_id.eq(user_id))
            .filter(archive_user_achievements::is_completed.eq(true))
            .count()
            .get_result(conn)?;

        Ok::<_, diesel::result::Error>(UserProfileSummary {
            total_xp: profile.total_xp,
            current_streak_days: profile.current_streak_days,
            rank: current_rank.map(RankInfo::from),
            next_rank: next_rank.map(RankInfo::from),
            xp_to_next_rank,
            recent_achievements,
            total_achievements: total_achievements as i32,
            completed_achievements: completed_achievements as i32,
        })
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to get profile summary"))?;

    res.render(Json(summary));
    Ok(())
}

/// List all achievements with user progress
#[handler]
pub async fn list_user_achievements(depot: &mut Depot, res: &mut Response) -> AppResult<()> {
    let user_id = depot.user_id()?;

    let achievements: Vec<AchievementWithProgress> = with_conn(move |conn| {
        // Get all active achievement definitions
        let definitions: Vec<AchievementDefinition> = archive_achievement_definitions::table
            .filter(archive_achievement_definitions::is_active.eq(true))
            .order(archive_achievement_definitions::sort_order.asc())
            .load::<AchievementDefinition>(conn)?;

        // Get user's achievement progress
        let user_progress: Vec<UserAchievementRecord> = archive_user_achievements::table
            .filter(archive_user_achievements::user_id.eq(user_id))
            .load::<UserAchievementRecord>(conn)?;

        // Combine definitions with user progress
        let result: Vec<AchievementWithProgress> = definitions
            .into_iter()
            .filter(|d| d.is_hidden != Some(true))
            .map(|def| {
                let progress = user_progress
                    .iter()
                    .find(|p| p.achievement_id == def.id);

                AchievementWithProgress {
                    id: def.id,
                    code: def.code,
                    name_en: def.name_en,
                    name_zh: def.name_zh,
                    description_en: def.description_en,
                    description_zh: def.description_zh,
                    icon: def.icon,
                    category: def.category,
                    rarity: def.rarity,
                    xp_reward: def.xp_reward,
                    requirement_value: def.requirement_value,
                    progress: progress.map(|p| p.progress).unwrap_or(0),
                    is_completed: progress.map(|p| p.is_completed).unwrap_or(false),
                    completed_at: progress.and_then(|p| p.completed_at),
                }
            })
            .collect();

        Ok::<_, diesel::result::Error>(result)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list user achievements"))?;

    res.render(Json(achievements));
    Ok(())
}

/// List user's XP history
#[handler]
pub async fn list_xp_history(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> AppResult<()> {
    let user_id = depot.user_id()?;
    let limit = req.query::<i64>("limit").unwrap_or(50).clamp(1, 200);

    let history: Vec<UserXpHistory> = with_conn(move |conn| {
        archive_user_xp_history::table
            .filter(archive_user_xp_history::user_id.eq(user_id))
            .order(archive_user_xp_history::created_at.desc())
            .limit(limit)
            .load::<UserXpHistory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list xp history"))?;

    res.render(Json(history));
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get or create user profile
fn get_or_create_profile(
    conn: &mut diesel::PgConnection,
    user_id: i64,
) -> Result<UserProfile, diesel::result::Error> {
    // Try to find existing profile
    let existing = archive_user_profiles::table
        .filter(archive_user_profiles::user_id.eq(user_id))
        .first::<UserProfile>(conn)
        .optional()?;

    if let Some(profile) = existing {
        return Ok(profile);
    }

    // Create new profile
    let new_profile = NewUserProfile { user_id };
    diesel::insert_into(archive_user_profiles::table)
        .values(&new_profile)
        .get_result::<UserProfile>(conn)
}
