use once_cell::sync::Lazy;
use regex::Regex;

const YOUTUBE_REGEX: &'static str = r"^(()?\/\/)?((?:www|m)\.)?((?:youtube(?:-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$";

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
    pub fn is_supported_link(input: &String) -> bool {
        let youtube = BOT_REGEX.youtube.is_match(input.as_str());
        let soundcloud = BOT_REGEX.youtube.is_match(input.as_str());

        youtube || soundcloud
    }
}
