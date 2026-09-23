use skia_safe::{Data, Image};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;

use crate::media::control::MediaInfo;

pub struct MediaManager {
    state: Arc<Mutex<MediaInfo>>,
    cached_artwork_bytes: Option<Vec<u8>>,
    cached_image: Option<Image>,
}

impl MediaManager {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(MediaInfo::default()));
        let state_clone = Arc::clone(&state);

        thread::spawn(move || {
            loop {
                if let Ok(info) = MediaInfo::query_current_media() {
                    if let Ok(mut lock) = state_clone.lock() {
                        *lock = info;
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
        });

        Self {
            state,
            cached_artwork_bytes: None,
            cached_image: None,
        }
    }

    pub fn get_state(&mut self) -> (MediaInfo, Option<&Image>) {
        let info = self.state.lock().map(|l| l.clone()).unwrap_or_default();

        // Decode into skia image only when the thumbnail art bytes changes
        if info.artwork_bytes != self.cached_artwork_bytes {
            self.cached_artwork_bytes = info.artwork_bytes.clone();
            self.cached_image = info
                .artwork_bytes
                .as_ref()
                .and_then(|bytes| Image::from_encoded(Data::new_copy(bytes)));
        }

        let img = self.cached_image.as_ref();
        (info, img)
    }

    pub fn toggle_play_pause() {
        thread::spawn(|| {
            if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = m.get() {
                    if let Ok(s) = mgr.GetCurrentSession() {
                        let _ = s.TryTogglePlayPauseAsync();
                    }
                }
            }
        });
    }

    pub fn skip_next() {
        thread::spawn(|| {
            if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = m.get() {
                    if let Ok(s) = mgr.GetCurrentSession() {
                        let _ = s.TrySkipNextAsync();
                    }
                }
            }
        });
    }

    pub fn skip_previous() {
        thread::spawn(|| {
            if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = m.get() {
                    if let Ok(s) = mgr.GetCurrentSession() {
                        let _ = s.TrySkipPreviousAsync();
                    }
                }
            }
        });
    }
}
