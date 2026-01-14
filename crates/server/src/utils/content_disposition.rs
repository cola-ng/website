use std::borrow::Cow;

use salvo::http::headers::ContentDisposition;

/// as defined by MSC2702
const ALLOWED_INLINE_CONTENT_TYPES: [&str; 26] = [
    // keep sorted
    "application/json",
    "application/ld+json",
    "audio/aac",
    "audio/flac",
    "audio/mp4",
    "audio/mpeg",
    "audio/ogg",
    "audio/wav",
    "audio/wave",
    "audio/webm",
    "audio/x-flac",
    "audio/x-pn-wav",
    "audio/x-wav",
    "image/apng",
    "image/avif",
    "image/gif",
    "image/jpeg",
    "image/png",
    "image/webp",
    "text/css",
    "text/csv",
    "text/plain",
    "video/mp4",
    "video/ogg",
    "video/quicktime",
    "video/webm",
];

/// sanitises the file name for the Content-Disposition using
/// `sanitize_filename` crate
#[tracing::instrument(level = "debug")]
pub fn sanitise_filename(filename: &str) -> String {
    sanitize_filename::sanitize_with_options(
        filename,
        sanitize_filename::Options {
            truncate: false,
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn string_sanitisation() {
        const SAMPLE: &str = "🏳️‍⚧️this\\r\\n įs \r\\n ä \\r\nstrïng 🥴that\n\r \
		                      ../../../../../../../may be\r\n malicious🏳️‍⚧️";
        const SANITISED: &str = "🏳️‍⚧️thisrn įs n ä rstrïng 🥴that ..............may be malicious🏳️‍⚧️";

        let options = sanitize_filename::Options {
            windows: true,
            truncate: true,
            replacement: "",
        };

        // cargo test -- --nocapture
        println!("{SAMPLE}");
        println!(
            "{}",
            sanitize_filename::sanitize_with_options(SAMPLE, options.clone())
        );
        println!("{SAMPLE:?}");
        println!(
            "{:?}",
            sanitize_filename::sanitize_with_options(SAMPLE, options.clone())
        );

        assert_eq!(
            SANITISED,
            sanitize_filename::sanitize_with_options(SAMPLE, options.clone())
        );
    }

    // #[test]
    // fn empty_sanitisation() {
    //     use crate::EMPTY;

    //     let result = sanitize_filename::sanitize_with_options(
    //         EMPTY,
    //         sanitize_filename::Options {
    //             windows: true,
    //             truncate: true,
    //             replacement: "",
    //         },
    //     );

    //     assert_eq!(EMPTY, result);
    // }
}
