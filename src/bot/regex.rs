use once_cell::sync::Lazy;
use regex::Regex;
use tracing::debug;

const YOUTUBE_REGEX: &'static str =
    r"(?:https?://)?(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/)([a-zA-Z0-9_-]{11})";

const SOUNDCLOUD_REGEX: &'static str =
    r"(?:https?:\/\/)?(?:www\.)?soundcloud\.com\/([\w-]+)\/([\w-]+)";

pub struct Regexp {
    youtube: Regex,
    soundcloud: Regex,
}

// Regexp singleton to use globally and not have to carry a reference around
static BOT_REGEX: Lazy<Regexp> = Lazy::new(|| Regexp {
    youtube: Regex::new(YOUTUBE_REGEX).expect("Youtube regex failed to compile"),
    soundcloud: Regex::new(SOUNDCLOUD_REGEX).expect("Soundcloud regex failed to compile"),
});

impl Regexp {
    pub fn is_supported_link(input: &str) -> bool {
        let youtube = BOT_REGEX.youtube.is_match(input);
        let soundcloud = BOT_REGEX.soundcloud.is_match(input);

        debug!("Regexp for {input}\n youtube: {youtube}\n soundcloud: {soundcloud}\n");

        youtube || soundcloud
    }
}
