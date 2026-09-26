use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::{DataReader, InputStreamOptions};

#[derive(Clone, Default, Debug)]
pub struct MediaInfo {
    pub has_media: bool,
    pub title: String,
    pub artist: String,
    pub is_playing: bool,
    pub artwork_bytes: Option<Vec<u8>>,
    pub position_secs: f32,
    pub duration_secs: f32,
}

impl MediaInfo {
    fn read_thumbnail(session: &GlobalSystemMediaTransportControlsSession) -> Option<Vec<u8>> {
        let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let thumb_stream_ref = props.Thumbnail().ok()?;

        let read_stream = thumb_stream_ref.OpenReadAsync().ok()?.get().ok()?;
        let size = read_stream.Size().ok()? as usize;

        if size == 0 || size > 10 * 1024 * 1024 {
            return None;
        }

        let reader = DataReader::CreateDataReader(&read_stream).ok()?;
        reader
            .SetInputStreamOptions(InputStreamOptions::None)
            .ok()?;
        reader.LoadAsync(size as u32).ok()?.get().ok()?;

        let mut buffer = vec![0u8; size];
        reader.ReadBytes(&mut buffer).ok()?;
        Some(buffer)
    }

    pub fn query_current_media() -> Result<MediaInfo, windows::core::Error> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;

        let session = match manager.GetCurrentSession() {
            Ok(s) => s,
            Err(_) => return Ok(MediaInfo::default()),
        };

        // Fetch track metadata (Title & Artist)
        let media_props = match session.TryGetMediaPropertiesAsync() {
            Ok(op) => match op.get() {
                Ok(p) => p,
                Err(_) => return Ok(MediaInfo::default()),
            },
            Err(_) => return Ok(MediaInfo::default()),
        };

        let title = media_props.Title().unwrap_or_default().to_string();
        let artist = media_props.Artist().unwrap_or_default().to_string();

        if title.is_empty() && artist.is_empty() {
            return Ok(MediaInfo::default());
        }

        // Check whether media is playing or paused
        let is_playing = match session.GetPlaybackInfo() {
            Ok(info) => match info.PlaybackStatus() {
                Ok(status) => {
                    status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                }
                Err(_) => false,
            },
            Err(_) => false,
        };

        // Fetch timeline (position & duration in 100-nanosecond units)
        let (position_secs, duration_secs) = match session.GetTimelineProperties() {
            Ok(timeline) => {
                let pos = timeline.Position().unwrap_or_default().Duration as f32 / 10_000_000.0;
                let end = timeline.EndTime().unwrap_or_default().Duration as f32 / 10_000_000.0;
                (pos, end)
            }
            Err(_) => (0.0, 0.0),
        };

        let artwork_bytes = Self::read_thumbnail(&session);

        Ok(MediaInfo {
            has_media: true,
            title,
            artist,
            is_playing,
            artwork_bytes,
            position_secs,
            duration_secs,
        })
    }
}
