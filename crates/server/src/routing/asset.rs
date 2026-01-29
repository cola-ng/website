use salvo::prelude::*;

mod context;
mod reading;
mod script;
mod stage;
mod taxonomy;

pub fn router() -> Router {
    Router::with_path("asset")
        // Taxonomy
        .push(Router::with_path("domains").get(taxonomy::list_domains))
        .push(
            Router::with_path("categories")
                .get(taxonomy::list_categories)
                .push(Router::with_path("{domain_id}").get(taxonomy::get_categories_by_domain)),
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
                .push(Router::with_path("{script_id}/turns").get(script::get_script_turns)),
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
                .push(Router::with_path("evaluate").post(reading::evaluate_pronunciation)),
        )
}
