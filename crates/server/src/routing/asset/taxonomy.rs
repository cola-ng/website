use diesel::prelude::*;
use salvo::prelude::*;
use serde::Deserialize;

use crate::db::schema::*;
use crate::db::with_conn;
use crate::models::asset::*;
use crate::{JsonResult, json_ok};

// ============================================================================
// Domain Handlers
// ============================================================================

#[handler]
pub async fn list_domains() -> JsonResult<Vec<TaxonDomain>> {
    let domains: Vec<TaxonDomain> =
        with_conn(move |conn| taxon_domains::table.order(taxon_domains::id.asc()).load::<TaxonDomain>(conn))
            .await
            .map_err(|_| StatusError::internal_server_error().brief("failed to list domains"))?;

    json_ok(domains)
}

#[handler]
pub async fn get_domain(req: &mut Request) -> JsonResult<TaxonDomain> {
    let domain_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let domain: TaxonDomain = with_conn(move |conn| {
        taxon_domains::table
            .filter(taxon_domains::id.eq(domain_id))
            .first::<TaxonDomain>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("domain not found"))?;

    json_ok(domain)
}

#[derive(Deserialize)]
pub struct CreateDomainRequest {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
}

#[handler]
pub async fn create_domain(req: &mut Request) -> JsonResult<TaxonDomain> {
    let input: CreateDomainRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.code.trim().is_empty() || input.name_en.trim().is_empty() || input.name_zh.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("code, name_en, and name_zh are required")
            .into());
    }

    let created: TaxonDomain = with_conn(move |conn| {
        diesel::insert_into(taxon_domains::table)
            .values(&NewTaxonDomain {
                code: input.code.trim().to_string(),
                name_en: input.name_en.trim().to_string(),
                name_zh: input.name_zh.trim().to_string(),
            })
            .get_result::<TaxonDomain>(conn)
    })
    .await
    .map_err(|e| {
        if e.contains("unique") || e.contains("duplicate") {
            StatusError::conflict().brief("domain code or name already exists")
        } else {
            StatusError::internal_server_error().brief("failed to create domain")
        }
    })?;

    json_ok(created)
}

#[derive(Deserialize)]
pub struct UpdateDomainRequest {
    pub code: Option<String>,
    pub name_en: Option<String>,
    pub name_zh: Option<String>,
}

#[handler]
pub async fn update_domain(req: &mut Request) -> JsonResult<TaxonDomain> {
    let domain_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let input: UpdateDomainRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    // Validate non-empty values if provided
    if let Some(ref code) = input.code {
        if code.trim().is_empty() {
            return Err(StatusError::bad_request()
                .brief("code cannot be empty")
                .into());
        }
    }

    let changes = UpdateTaxonDomain {
        code: input.code.map(|c| c.trim().to_string()),
        name_en: input.name_en.map(|n| n.trim().to_string()),
        name_zh: input.name_zh.map(|n| n.trim().to_string()),
    };

    let updated: TaxonDomain = with_conn(move |conn| {
        diesel::update(taxon_domains::table.filter(taxon_domains::id.eq(domain_id)))
            .set(changes)
            .get_result::<TaxonDomain>(conn)
    })
    .await
    .map_err(|e| {
        if e.contains("unique") || e.contains("duplicate") {
            StatusError::conflict().brief("domain code or name already exists")
        } else {
            StatusError::internal_server_error().brief("failed to update domain")
        }
    })?;

    json_ok(updated)
}

#[handler]
pub async fn delete_domain(req: &mut Request) -> JsonResult<serde_json::Value> {
    let domain_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    // Check if domain has categories
    let has_categories: bool = with_conn(move |conn| {
        use diesel::dsl::exists;
        diesel::select(exists(
            taxon_categories::table.filter(taxon_categories::domain_id.eq(domain_id)),
        ))
        .get_result(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check categories"))?;

    if has_categories {
        return Err(StatusError::conflict()
            .brief("cannot delete domain with existing categories")
            .into());
    }

    with_conn(move |conn| {
        diesel::delete(taxon_domains::table.filter(taxon_domains::id.eq(domain_id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete domain"))?;

    json_ok(serde_json::json!({ "deleted": true }))
}

// ============================================================================
// Category Handlers
// ============================================================================

#[handler]
pub async fn list_categories(req: &mut Request) -> JsonResult<Vec<TaxonCategory>> {
    let domain_id = req.query::<i64>("domain_id");
    let parent_id = req.query::<i64>("parent_id");
    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 500);

    let categories: Vec<TaxonCategory> = with_conn(move |conn| {
        let mut query = taxon_categories::table
            .order(taxon_categories::id.asc())
            .limit(limit)
            .into_boxed();

        if let Some(did) = domain_id {
            query = query.filter(taxon_categories::domain_id.eq(did));
        }

        if let Some(pid) = parent_id {
            query = query.filter(taxon_categories::parent_id.eq(pid));
        }

        query.load::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list categories"))?;

    json_ok(categories)
}

#[handler]
pub async fn get_category(req: &mut Request) -> JsonResult<TaxonCategory> {
    let category_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let category: TaxonCategory = with_conn(move |conn| {
        taxon_categories::table
            .filter(taxon_categories::id.eq(category_id))
            .first::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::not_found().brief("category not found"))?;

    json_ok(category)
}

#[handler]
pub async fn get_categories_by_domain(req: &mut Request) -> JsonResult<Vec<TaxonCategory>> {
    let domain_id: i64 = req
        .param::<i64>("domain_id")
        .ok_or_else(|| StatusError::bad_request().brief("missing domain_id"))?;

    let categories: Vec<TaxonCategory> = with_conn(move |conn| {
        taxon_categories::table
            .filter(taxon_categories::domain_id.eq(domain_id))
            .order(taxon_categories::id.asc())
            .load::<TaxonCategory>(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to list categories"))?;

    json_ok(categories)
}

#[derive(Deserialize)]
pub struct CreateCategoryRequest {
    pub code: String,
    pub name_en: String,
    pub name_zh: String,
    pub domain_id: i64,
    pub parent_id: Option<i64>,
}

#[handler]
pub async fn create_category(req: &mut Request) -> JsonResult<TaxonCategory> {
    let input: CreateCategoryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    if input.code.trim().is_empty() || input.name_en.trim().is_empty() || input.name_zh.trim().is_empty() {
        return Err(StatusError::bad_request()
            .brief("code, name_en, and name_zh are required")
            .into());
    }

    // Verify domain exists
    let domain_exists: bool = with_conn({
        let domain_id = input.domain_id;
        move |conn| {
            use diesel::dsl::exists;
            diesel::select(exists(
                taxon_domains::table.filter(taxon_domains::id.eq(domain_id)),
            ))
            .get_result(conn)
        }
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check domain"))?;

    if !domain_exists {
        return Err(StatusError::bad_request()
            .brief("domain not found")
            .into());
    }

    // Verify parent exists and domain matches if provided
    if let Some(pid) = input.parent_id {
        let parent_category: TaxonCategory = with_conn(move |conn| {
            taxon_categories::table
                .filter(taxon_categories::id.eq(pid))
                .first::<TaxonCategory>(conn)
        })
        .await
        .map_err(|_| StatusError::bad_request().brief("parent category not found"))?;

        // Validate domain consistency
        if parent_category.domain_id != input.domain_id {
            return Err(StatusError::bad_request()
                .brief("category domain must match parent's domain")
                .into());
        }
    }

    let created: TaxonCategory = with_conn(move |conn| {
        diesel::insert_into(taxon_categories::table)
            .values(&NewTaxonCategory {
                code: input.code.trim().to_string(),
                name_en: input.name_en.trim().to_string(),
                name_zh: input.name_zh.trim().to_string(),
                domain_id: input.domain_id,
                parent_id: input.parent_id,
            })
            .get_result::<TaxonCategory>(conn)
    })
    .await
    .map_err(|e| {
        if e.contains("unique") || e.contains("duplicate") {
            StatusError::conflict().brief("category name already exists in this domain")
        } else {
            StatusError::internal_server_error().brief("failed to create category")
        }
    })?;

    json_ok(created)
}

#[derive(Deserialize)]
pub struct UpdateCategoryRequest {
    pub code: Option<String>,
    pub name_en: Option<String>,
    pub name_zh: Option<String>,
    pub domain_id: Option<i64>,
    pub parent_id: Option<i64>,
}

#[handler]
pub async fn update_category(req: &mut Request) -> JsonResult<TaxonCategory> {
    let category_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    let input: UpdateCategoryRequest = req
        .parse_json()
        .await
        .map_err(|_| StatusError::bad_request().brief("invalid json"))?;

    // Validate non-empty values if provided
    if let Some(ref code) = input.code {
        if code.trim().is_empty() {
            return Err(StatusError::bad_request()
                .brief("code cannot be empty")
                .into());
        }
    }

    // Prevent self-reference
    if let Some(pid) = input.parent_id {
        if pid == category_id {
            return Err(StatusError::bad_request()
                .brief("category cannot be its own parent")
                .into());
        }
    }

    // If parent_id is specified, validate domain consistency
    if let Some(pid) = input.parent_id {
        // Get the current category to know its domain_id
        let current_category: TaxonCategory = with_conn(move |conn| {
            taxon_categories::table
                .filter(taxon_categories::id.eq(category_id))
                .first::<TaxonCategory>(conn)
        })
        .await
        .map_err(|_| StatusError::not_found().brief("category not found"))?;

        // Determine the effective domain_id (from input or current)
        let effective_domain_id = input.domain_id.unwrap_or(current_category.domain_id);

        // Fetch parent and check domain consistency
        let parent_category: TaxonCategory = with_conn(move |conn| {
            taxon_categories::table
                .filter(taxon_categories::id.eq(pid))
                .first::<TaxonCategory>(conn)
        })
        .await
        .map_err(|_| StatusError::bad_request().brief("parent category not found"))?;

        if parent_category.domain_id != effective_domain_id {
            return Err(StatusError::bad_request()
                .brief("category domain must match parent's domain")
                .into());
        }
    }

    let changes = UpdateTaxonCategory {
        code: input.code.map(|c| c.trim().to_string()),
        name_en: input.name_en.map(|n| n.trim().to_string()),
        name_zh: input.name_zh.map(|n| n.trim().to_string()),
        domain_id: input.domain_id,
        parent_id: input.parent_id,
    };

    let updated: TaxonCategory = with_conn(move |conn| {
        diesel::update(taxon_categories::table.filter(taxon_categories::id.eq(category_id)))
            .set(changes)
            .get_result::<TaxonCategory>(conn)
    })
    .await
    .map_err(|e| {
        if e.contains("unique") || e.contains("duplicate") {
            StatusError::conflict().brief("category name already exists in this domain")
        } else {
            StatusError::internal_server_error().brief("failed to update category")
        }
    })?;

    json_ok(updated)
}

#[handler]
pub async fn delete_category(req: &mut Request) -> JsonResult<serde_json::Value> {
    let category_id: i64 = req
        .param::<i64>("id")
        .ok_or_else(|| StatusError::bad_request().brief("missing id"))?;

    // Check if category has children
    let has_children: bool = with_conn(move |conn| {
        use diesel::dsl::exists;
        diesel::select(exists(
            taxon_categories::table.filter(taxon_categories::parent_id.eq(category_id)),
        ))
        .get_result(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check children"))?;

    if has_children {
        return Err(StatusError::conflict()
            .brief("cannot delete category with children")
            .into());
    }

    // Check if category is used by contexts
    let has_contexts: bool = with_conn(move |conn| {
        use diesel::dsl::exists;
        diesel::select(exists(
            asset_context_categories::table.filter(asset_context_categories::category_id.eq(category_id)),
        ))
        .get_result(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to check contexts"))?;

    if has_contexts {
        return Err(StatusError::conflict()
            .brief("cannot delete category used by contexts")
            .into());
    }

    with_conn(move |conn| {
        diesel::delete(taxon_categories::table.filter(taxon_categories::id.eq(category_id))).execute(conn)
    })
    .await
    .map_err(|_| StatusError::internal_server_error().brief("failed to delete category"))?;

    json_ok(serde_json::json!({ "deleted": true }))
}
