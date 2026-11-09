use salvo::prelude::*;

use crate::hoops;

mod context;
pub mod reading;
mod script;
mod stage;
mod taxonomy;

pub fn router() -> Router {
    Router::with_path("asset")
        .hoop(require_taxonomy_admin)
        // Taxonomy Domains
        .push(
            Router::with_path("domains")
                .get(taxonomy::list_domains)
                .post(taxonomy::create_domain),
        )
        .push(
            Router::with_path("domains/{id}")
                .get(taxonomy::get_domain)
                .put(taxonomy::update_domain)
                .delete(taxonomy::delete_domain),
        )
        .push(
            Router::with_path("domains/{domain_id}/categories")
                .get(taxonomy::get_categories_by_domain),
        )
        // Taxonomy Categories
        .push(
            Router::with_path("categories")
                .get(taxonomy::list_categories)
                .post(taxonomy::create_category),
        )
        .push(
            Router::with_path("categories/{id}")
                .get(taxonomy::get_category)
                .put(taxonomy::update_category)
                .delete(taxonomy::delete_category),
        )
        // Contexts
        .push(
            Router::with_path("contexts")
                .get(context::list_contexts)
                .push(Router::with_path("{id}").get(context::get_context)),
        )
        // Stages
        .push(
            Router::with_path("stages")
                .get(stage::list_stages)
                .push(Router::with_path("{id}").get(stage::get_stage))
                .push(Router::with_path("{id}/scripts").get(stage::get_scripts_by_stage)),
        )
        // Scripts (dialogues)
        .push(
            Router::with_path("scripts")
                .get(script::list_scripts)
                .push(Router::with_path("{script_id}/turns").get(script::get_script_turns))
                .push(Router::with_path("turns/{id}/audio").get(script::serve_turn_audio)),
        )
        // Reading subjects and sentences
        .push(
            Router::with_path("read")
                .push(
                    Router::with_path("subjects")
                        .get(reading::list_read_subjects)
                        .push(Router::with_path("{id}/sentences").get(reading::get_read_sentences)),
                )
                .push(
                    Router::with_path("sentences")
                        .get(reading::list_read_sentences)
                        .push(Router::with_path("{id}/audio").get(reading::serve_sentence_audio)),
                )
                // Authenticated reading practice endpoints
                .push(
                    Router::with_path("recommended")
                        .hoop(hoops::require_auth)
                        .get(reading::get_recommended_sentences),
                )
                .push(
                    Router::with_path("practice")
                        .hoop(hoops::require_auth)
                        .post(reading::submit_read_practice),
                )
                .push(
                    Router::with_path("reset-level")
                        .hoop(hoops::require_auth)
                        .post(reading::reset_pronunciation_level),
                )
                .push(
                    Router::with_path("advance-level")
                        .hoop(hoops::require_auth)
                        .post(reading::advance_pronunciation_level),
                ),
        )
}

#[handler]
async fn require_taxonomy_admin(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    let path = req.uri().path();
    let taxonomy = ["/api/asset/domains", "/api/asset/categories"]
        .iter()
        .any(|base| {
            path == *base
                || path
                    .strip_prefix(base)
                    .is_some_and(|tail| tail.starts_with('/'))
        });
    if taxonomy
        && !matches!(
            *req.method(),
            salvo::http::Method::GET | salvo::http::Method::HEAD | salvo::http::Method::OPTIONS
        )
    {
        crate::hoops::require_admin
            .handle(req, depot, res, ctrl)
            .await;
    }
}
