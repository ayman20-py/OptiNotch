use skia_safe::{FontMgr, FontStyle, Typeface};
use std::sync::OnceLock;

/// Shared Font and Typeface manager to avoid recreating FontMgr and resolving font families on every frame.
pub struct FontCache {
    pub regular: Typeface,
    pub bold: Typeface,
    pub clock: Typeface,
}

static FONT_CACHE: OnceLock<FontCache> = OnceLock::new();

impl FontCache {
    pub fn get() -> &'static FontCache {
        FONT_CACHE.get_or_init(|| {
            let font_mgr = FontMgr::new();

            let regular = font_mgr
                .match_family_style("Google Sans", FontStyle::normal())
                .or_else(|| font_mgr.match_family_style("Google Sans Display", FontStyle::normal()))
                .or_else(|| font_mgr.match_family_style("Product Sans", FontStyle::normal()))
                .or_else(|| font_mgr.match_family_style("Segoe UI Variable Display", FontStyle::normal()))
                .or_else(|| font_mgr.match_family_style("Segoe UI Variable Text", FontStyle::normal()))
                .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::normal()))
                .or_else(|| font_mgr.match_family_style("Inter", FontStyle::normal()))
                .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
                .expect("Failed to load regular typeface");

            let bold = font_mgr
                .match_family_style("Google Sans", FontStyle::bold())
                .or_else(|| font_mgr.match_family_style("Google Sans Display", FontStyle::bold()))
                .or_else(|| font_mgr.match_family_style("Product Sans", FontStyle::bold()))
                .or_else(|| font_mgr.match_family_style("Segoe UI Variable Display", FontStyle::bold()))
                .or_else(|| font_mgr.match_family_style("Segoe UI Variable Text", FontStyle::bold()))
                .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::bold()))
                .or_else(|| font_mgr.match_family_style("Inter", FontStyle::bold()))
                .unwrap_or_else(|| regular.clone());

            let clock = font_mgr
                .match_family_style("Lilita One", FontStyle::normal())
                .or_else(|| font_mgr.match_family_style("Google Sans", FontStyle::bold()))
                .or_else(|| font_mgr.match_family_style("Segoe UI", FontStyle::bold()))
                .or_else(|| font_mgr.legacy_make_typeface(None, FontStyle::normal()))
                .expect("Failed to load clock typeface");

            FontCache {
                regular,
                bold,
                clock,
            }
        })
    }
}
