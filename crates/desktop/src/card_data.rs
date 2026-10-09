//! Card names and art in the windows (T-304, D-038). The `cards` crate
//! fetches and caches them from HearthstoneJSON; this file gives the pages
//! the names (a command) and the images (the `cardart` protocol, so an
//! `<img>` can show a cached JPEG). Anything missing shows the card id.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cards::net::ReqwestFetch;
use cards::{valid_id, CardStatus, CardStore};
use tauri::http::{header, Method, Request, Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, UriSchemeContext, UriSchemeResponder};

/// Pages load art from `http://cardart.localhost/<card id>` (Windows).
pub const SCHEME: &str = "cardart";
/// The names or the card data status changed: the pages ask again.
const CARDS_CHANGED: &str = "cards-changed";
/// How often the worker checks whether the card data is due a refresh
/// (the store itself makes at most one request an hour).
const CHECK_EVERY: Duration = Duration::from_secs(60);
/// A page never asks for more names than this at once.
const MAX_IDS: usize = 200;

#[derive(Default)]
pub struct Cards {
    store: OnceLock<Arc<CardStore>>,
    /// Why there are no names or art at all (the HTTPS client did not start).
    off: OnceLock<String>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Opens the cache in `<data folder>/card-cache` and starts the worker that
/// keeps the card data current.
pub fn start(app: &AppHandle, data_dir: &Path) {
    let cards = app.state::<Cards>();
    let fetch = match ReqwestFetch::new() {
        Ok(fetch) => fetch,
        Err(e) => {
            let _ = cards.off.set(format!(
                "Card names and art are off: the network client could not start ({e})."
            ));
            return;
        }
    };
    let store = Arc::new(CardStore::open(
        data_dir.join("card-cache"),
        Box::new(fetch),
    ));
    let _ = cards.store.set(store.clone());
    let app = app.clone();
    thread::spawn(move || {
        let mut shown = store.status();
        loop {
            let changed = store.refresh_if_due(now());
            let status = store.status();
            if changed || status != shown {
                shown = status;
                let _ = app.emit(CARDS_CHANGED, ());
            }
            thread::sleep(CHECK_EVERY);
        }
    });
}

/// Names for the ids asked, null for each one the card data does not know.
/// Ids that are not plain card ids are left out.
pub fn names_of(store: Option<&CardStore>, ids: &[String]) -> BTreeMap<String, Option<String>> {
    ids.iter()
        .filter(|id| valid_id(id))
        .take(MAX_IDS)
        .map(|id| (id.clone(), store.and_then(|s| s.name(id))))
        .collect()
}

#[tauri::command]
pub fn card_names(cards: State<'_, Cards>, ids: Vec<String>) -> BTreeMap<String, Option<String>> {
    names_of(cards.store.get().map(Arc::as_ref), &ids)
}

#[tauri::command]
pub fn card_data_status(cards: State<'_, Cards>) -> CardStatus {
    match cards.store.get() {
        Some(store) => store.status(),
        None => CardStatus {
            problem: Some(
                cards
                    .off
                    .get()
                    .cloned()
                    .unwrap_or_else(|| "Card names and art are starting.".into()),
            ),
            ..CardStatus::default()
        },
    }
}

/// `/BG_CFM_315` → `BG_CFM_315`; anything else is not a card.
pub fn id_from_path(path: &str) -> Option<&str> {
    path.strip_prefix('/').filter(|id| valid_id(id))
}

/// 200 with the JPEG, or an empty 404 (the page then shows no picture).
pub fn response(image: Option<Vec<u8>>) -> Response<Vec<u8>> {
    let built = match image {
        Some(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/jpeg")
            .header(header::CACHE_CONTROL, "max-age=86400")
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .body(bytes),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CACHE_CONTROL, "no-store")
            .body(Vec::new()),
    };
    built.unwrap_or_else(|_| Response::new(Vec::new()))
}

/// The `cardart` protocol. A fetch can take seconds, so it answers from
/// its own thread and never blocks the window.
pub fn protocol<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let id = (request.method() == Method::GET)
        .then(|| id_from_path(request.uri().path()))
        .flatten()
        .map(String::from);
    let store = ctx.app_handle().state::<Cards>().store.get().cloned();
    let (Some(id), Some(store)) = (id, store) else {
        return responder.respond(response(None));
    };
    thread::spawn(move || responder.respond(response(store.art(&id, now()))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_card_ids_are_served() {
        assert_eq!(id_from_path("/BG_CFM_315"), Some("BG_CFM_315"));
        for path in [
            "/",
            "",
            "BG_CFM_315",
            "/../catalog.json",
            "/a/b",
            "/A.jpg",
            "/a%2Fb",
        ] {
            assert_eq!(id_from_path(path), None, "{path}");
        }
    }

    #[test]
    fn an_image_is_a_jpeg_and_a_miss_is_an_empty_404() {
        let hit = response(Some(vec![0xFF, 0xD8, 0xFF, 0xE0]));
        assert_eq!(hit.status(), StatusCode::OK);
        assert_eq!(hit.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(hit.headers()[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        let miss = response(None);
        assert_eq!(miss.status(), StatusCode::NOT_FOUND);
        assert!(miss.body().is_empty());
    }

    #[test]
    fn without_card_data_every_name_is_unknown_and_bad_ids_are_dropped() {
        let ids: Vec<String> = ["CARD_1", "../x", "CARD_2"].map(String::from).to_vec();
        let names = names_of(None, &ids);
        assert_eq!(names.len(), 2);
        assert!(names.values().all(Option::is_none));
        let many: Vec<String> = (0..500).map(|i| format!("C_{i}")).collect();
        assert_eq!(names_of(None, &many).len(), MAX_IDS);
    }

    #[test]
    fn the_shared_script_stays_in_its_own_scope() {
        // Both pages declare `invoke` and `el` at the top level; a second
        // top-level declaration would stop the page.
        let script = include_str!("../ui/cards.js");
        assert!(script.contains("window.TLCards = (() => {"));
        assert!(script.trim_end().ends_with("})();"));
    }

    #[test]
    fn pages_may_load_images_only_from_the_app_and_the_card_art_protocol() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let csp = conf["app"]["security"]["csp"].as_str().unwrap();
        assert!(
            csp.contains("img-src 'self' data: http://cardart.localhost;"),
            "{csp}"
        );
        assert!(
            csp.contains("connect-src ipc: http://ipc.localhost;"),
            "no new network origin: {csp}"
        );
    }

    #[test]
    fn both_pages_load_the_card_script_before_their_own() {
        for (page, own) in [
            (include_str!("../ui/index.html"), "app.js"),
            (include_str!("../ui/overlay.html"), "overlay.js"),
        ] {
            let cards = page
                .find("<script src=\"cards.js\">")
                .expect("cards.js loaded");
            let own = page.find(&format!("<script src=\"{own}\">")).unwrap();
            assert!(cards < own);
        }
    }
}
