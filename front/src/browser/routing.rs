use super::*;

thread_local! {
    static LOCATION: RefCell<Option<Cage<String>>> = const { RefCell::new(None) };
}

fn current_location() -> String {
    let location = window().location();
    format!(
        "{}{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default(),
        location.hash().unwrap_or_default()
    )
}

pub(super) fn internal_location(path: &str) -> Option<String> {
    let base = url::Url::parse(&window().location().href().ok()?).ok()?;
    let url = base.join(path).ok()?;
    if url.origin() != base.origin() || url.path() == "/api" || url.path().starts_with("/api/") {
        return None;
    }
    Some(format!(
        "{}{}{}",
        url.path(),
        url.query().map(|q| format!("?{q}")).unwrap_or_default(),
        url.fragment().map(|h| format!("#{h}")).unwrap_or_default()
    ))
}

pub(super) fn go(path: &str) {
    if let Some(next) = internal_location(path) {
        let navigated = LOCATION.with(|location| {
            let Some(location) = *location.borrow() else {
                return false;
            };
            if !location.is_alive() {
                return false;
            }
            if next == current_location() {
                return true;
            }
            let Ok(history) = window().history() else {
                return false;
            };
            if history
                .push_state_with_url(&JsValue::NULL, "", Some(&next))
                .is_err()
            {
                return false;
            }
            location.revise(|value| *value = next);
            window().scroll_to_with_x_and_y(0.0, 0.0);
            true
        });
        if navigated {
            return;
        }
    }
    // API/OAuth and external protocols must leave the application normally.
    let _ = window().location().set_href(path);
}

pub(super) fn install(scope: &Scope) -> Cage<String> {
    let location = scope.cage(current_location());
    LOCATION.with(|value| *value.borrow_mut() = Some(location));
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if location.is_alive() {
            let next = current_location();
            if location.get_untracked() != next {
                location.revise(|value| *value = next);
            }
        }
    });
    for event in ["popstate", "hashchange"] {
        window()
            .add_event_listener_with_callback(event, listener.as_ref().unchecked_ref())
            .expect("browser navigation listener");
    }
    let _ = scope.on_cleanup(move || {
        for event in ["popstate", "hashchange"] {
            let _ = window()
                .remove_event_listener_with_callback(event, listener.as_ref().unchecked_ref());
        }
        LOCATION.with(|value| *value.borrow_mut() = None);
        drop(listener);
    });
    location
}
