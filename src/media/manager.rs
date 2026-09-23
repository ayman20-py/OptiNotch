use skia_safe::{Data, Image};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;

use crate::media::control::MediaInfo;
use std::time::Instant;

struct InternalMediaState {
    info: MediaInfo,
    optimistic_until: Option<Instant>,
}

pub struct MediaManager {
    state: Arc<Mutex<InternalMediaState>>,
    cached_artwork_bytes: Option<Vec<u8>>,
    cached_image: Option<Image>,
}

impl MediaManager {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(InternalMediaState {
            info: MediaInfo::default(),
            optimistic_until: None,
        }));
        let state_clone = Arc::clone(&state);

        thread::spawn(move || {
            loop {
                if let Ok(mut info) = MediaInfo::query_current_media() {
                    if let Ok(mut lock) = state_clone.lock() {
                        if let Some(deadline) = lock.optimistic_until {
                            if Instant::now() < deadline {
                                // If the newly polled state has caught up, clear the guard early
                                if info.is_playing == lock.info.is_playing {
                                    lock.optimistic_until = None;
                                } else {
                                    // Otherwise preserve the optimistic state to prevent clobbering
                                    info.is_playing = lock.info.is_playing;
                                }
                            } else {
                                lock.optimistic_until = None;
                            }
                        }
                        lock.info = info;
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
        let info = self
            .state
            .lock()
            .map(|l| l.info.clone())
            .unwrap_or_default();

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

    pub fn toggle_play_pause(&mut self) {
        if let Ok(mut lock) = self.state.lock() {
            lock.info.is_playing = !lock.info.is_playing;
            lock.optimistic_until = Some(Instant::now() + Duration::from_millis(1500));
        }
        thread::spawn(|| {
            if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = m.get() {
                    if let Ok(s) = mgr.GetCurrentSession() {
                        if let Ok(op) = s.TryTogglePlayPauseAsync() {
                            let _ = op.get();
                        }
                    }
                }
            }
        });
    }

    pub fn skip_next(&self) {
        thread::spawn(|| {
            if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = m.get() {
                    if let Ok(s) = mgr.GetCurrentSession() {
                        if let Ok(op) = s.TrySkipNextAsync() {
                            let _ = op.get();
                        }
                    }
                }
            }
        });
    }

    pub fn skip_previous(&self) {
        thread::spawn(|| {
            if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = m.get() {
                    if let Ok(s) = mgr.GetCurrentSession() {
                        if let Ok(op) = s.TrySkipPreviousAsync() {
                            let _ = op.get();
                        }
                    }
                }
            }
        });
    }
}
