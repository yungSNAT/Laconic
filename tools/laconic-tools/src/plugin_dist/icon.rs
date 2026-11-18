// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The listing icon rule of the plugin directory build: the icon must be a
//! complete, square PNG within the directory's limits.

use super::ICON;

/// The directory's icon limits: square, 512 to 2048 px a side, under 2 MB.
const ICON_SIDES: std::ops::RangeInclusive<u32> = 512..=2048;
const ICON_MAX_BYTES: usize = 2_000_000;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The CRC-32 a PNG chunk carries (ISO 3309, as in the PNG specification).
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// Refuse a listing icon the directory would not take: anything but a
/// complete PNG, square and of `ICON_SIDES`, under `ICON_MAX_BYTES`. Complete
/// means every chunk is whole with a matching CRC, one header chunk comes
/// first, image data follows, and an empty `IEND` ends the file. This checks
/// the chunk structure only: the image data is not decompressed.
pub(super) fn check(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() >= ICON_MAX_BYTES {
        return Err(format!(
            "{ICON} has {} bytes; the directory needs under {ICON_MAX_BYTES}",
            bytes.len()
        ));
    }
    let incomplete = || format!("{ICON} is not a complete PNG");
    let mut rest = bytes
        .strip_prefix(PNG_SIGNATURE)
        .ok_or_else(|| format!("{ICON} is not a PNG"))?;
    let mut sides = None;
    let mut image = false;
    let (width, height) = loop {
        let length = rest
            .get(..4)
            .and_then(|length| usize::try_from(be32(length)).ok())
            .ok_or_else(incomplete)?;
        let end = length
            .checked_add(12)
            .filter(|end| *end <= rest.len())
            .ok_or_else(incomplete)?;
        let (chunk, after) = rest.split_at(end);
        let (typed, crc) = chunk[4..].split_at(length + 4);
        if crc32(typed) != be32(crc) {
            return Err(format!("{ICON} has a damaged chunk"));
        }
        let (kind, data) = typed.split_at(4);
        match (kind, sides) {
            (b"IHDR", None) if length == 13 => sides = Some((be32(data), be32(&data[4..]))),
            (_, None) => return Err(format!("{ICON} does not start with a PNG header")),
            (b"IHDR", _) => return Err(format!("{ICON} has a second PNG header")),
            (b"IDAT", _) => image = true,
            (b"IEND", Some(found)) if length == 0 && after.is_empty() => break found,
            (b"IEND", _) => return Err(format!("{ICON} does not end with an empty IEND")),
            _ => {}
        }
        rest = after;
    };
    if !image {
        return Err(format!("{ICON} has no image data"));
    }
    if width != height || !ICON_SIDES.contains(&width) {
        return Err(format!(
            "{ICON} is {width} x {height} px; the directory needs a square of 512 to 2048 px"
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A PNG made of these chunks, each with its length and CRC.
    fn chunks(list: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut out = PNG_SIGNATURE.to_vec();
        for (kind, data) in list {
            let typed = [kind.as_slice(), data].concat();
            out.extend(u32::try_from(data.len()).unwrap().to_be_bytes());
            out.extend(&typed);
            out.extend(crc32(&typed).to_be_bytes());
        }
        out
    }

    fn header(width: u32, height: u32) -> Vec<u8> {
        [
            &width.to_be_bytes()[..],
            &height.to_be_bytes(),
            &[8, 2, 0, 0, 0],
        ]
        .concat()
    }

    /// A whole PNG of this size, also for the build's own tests.
    pub(crate) fn png(width: u32, height: u32) -> Vec<u8> {
        chunks(&[
            (b"IHDR", &header(width, height)),
            (b"IDAT", b"pixels"),
            (b"IEND", b""),
        ])
    }

    #[test]
    fn the_crc_is_the_png_one() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn the_icon_must_be_a_complete_square_png_of_the_allowed_size() {
        let head = header(1024, 1024);
        for side in [512, 2048] {
            assert_eq!(check(&png(side, side)), Ok(()), "{side}");
        }
        for (width, height) in [(511, 511), (2049, 2049), (1024, 1000)] {
            let error = check(&png(width, height)).unwrap_err();
            assert!(error.contains("square of 512 to 2048"), "{error}");
        }
        // Ancillary chunks and image data split over several chunks are valid.
        let ancillary = chunks(&[
            (b"IHDR", &head),
            (b"sRGB", &[0]),
            (b"pHYs", &[0; 9]),
            (b"IDAT", b"pix"),
            (b"IDAT", b"els"),
            (b"tEXt", b"k\0v"),
            (b"IEND", b""),
        ]);
        assert_eq!(check(&ancillary), Ok(()));
        // Signature, IHDR and IEND take 57 bytes; IDAT fills the rest.
        let sized = |total: usize| {
            chunks(&[
                (b"IHDR", &head),
                (b"IDAT", &vec![0; total - 57]),
                (b"IEND", b""),
            ])
        };
        assert_eq!(check(&sized(ICON_MAX_BYTES - 1)), Ok(()));
        let whole = png(1024, 1024);
        let mut damaged = whole.clone();
        let at = whole.windows(6).position(|w| w == b"pixels").unwrap();
        damaged[at] = b'q';
        let cases: [(Vec<u8>, &str); 11] = [
            (b"GIF89a".to_vec(), "not a PNG"),
            (whole[..whole.len() - 1].to_vec(), "not a complete PNG"),
            (
                whole[..PNG_SIGNATURE.len() + 2].to_vec(),
                "not a complete PNG",
            ),
            ([&whole[..], b"x"].concat(), "empty IEND"),
            (damaged, "damaged chunk"),
            (
                chunks(&[(b"IDAT", b"x"), (b"IHDR", &head)]),
                "start with a PNG header",
            ),
            (chunks(&[(b"IHDR", &head), (b"IEND", b"")]), "no image data"),
            (
                chunks(&[(b"IHDR", b"short"), (b"IEND", b"")]),
                "start with a PNG header",
            ),
            (
                chunks(&[(b"IHDR", &head), (b"IHDR", &head), (b"IEND", b"")]),
                "second PNG header",
            ),
            (
                chunks(&[(b"IHDR", &head), (b"IDAT", b"x"), (b"IEND", b"x")]),
                "empty IEND",
            ),
            (sized(ICON_MAX_BYTES), "needs under 2000000"),
        ];
        for (bytes, expected) in cases {
            let error = check(&bytes).unwrap_err();
            assert!(error.contains(expected), "{expected}: {error}");
        }
    }
}
