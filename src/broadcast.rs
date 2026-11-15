//! DVB event information carried by MPEG transport stream recordings (EN 300 468).
use crate::Player;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub(crate) fn subtitle_search(p: &Player) -> String {
    let base = "https://www.opensubtitles.org/en/search2/sublanguageid-all";
    let Some((item, network)) = p.current.and_then(|i| p.queue.get(i)) else {
        return base.into();
    };
    let name = item.to_string_lossy();
    let path = if *network {
        name.split(['?', '#']).next().unwrap_or("")
    } else {
        &name
    };
    let title = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let encoded: String = title
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("{base}/moviename-{encoded}")
}

pub(crate) fn guide(p: &Player) -> String {
    let Some((item, network)) = p.current.and_then(|i| p.queue.get(i)) else {
        return "Open a broadcast recording to see its programme guide.".into();
    };
    if *network {
        return "The bundled playback engine does not expose a live programme guide for this stream.".into();
    }
    let name = item.to_string_lossy();
    let mut bytes = Vec::new();
    let Ok(file) = File::open(&*name) else {
        return "Unable to open the recording.".into();
    };
    if file.take(8 * 1024 * 1024).read_to_end(&mut bytes).is_err() {
        return "Unable to read the programme guide.".into();
    }
    let events = programmes(&bytes);
    if events.is_empty() {
        return "No DVB programme listings found in the first 8 MiB of this recording.".into();
    }
    events.into_iter().collect::<Vec<_>>().join("\n")
}

pub(crate) fn programme_options(p: &Player) -> Vec<(String, String)> {
    let Some((item, false)) = p.current.and_then(|i| p.queue.get(i)) else {
        return Vec::new();
    };
    let Ok(file) = File::open(&*item.to_string_lossy()) else {
        return Vec::new();
    };
    // Reading a bounded prefix keeps opening the programme menu inexpensive.
    let mut bytes = Vec::new();
    if file.take(256 * 1024).read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let mut ids = BTreeSet::new();
    scan_sections(&bytes, 0, |table| {
        if table[0] != 0 || table.len() < 12 || table[5] & 1 == 0 {
            return;
        }
        for entry in table[8..table.len() - 4].chunks_exact(4) {
            let id = u16::from_be_bytes([entry[0], entry[1]]);
            if id != 0 {
                ids.insert(id);
            }
        }
    });
    ids.into_iter()
        .map(|id| (id.to_string(), format!("Programme {id}")))
        .collect()
}

fn programmes(data: &[u8]) -> BTreeSet<String> {
    let mut events = BTreeSet::new();
    scan_sections(data, 0x12, |buf| parse_events(buf, &mut events));
    events
}

fn scan_sections(data: &[u8], pid: u16, mut visit: impl FnMut(&[u8])) {
    let Some((start, stride)) = [188, 192, 204].into_iter().find_map(|stride| {
        (0..stride)
            .find(|&i| (0..3).all(|n| data.get(i + n * stride) == Some(&0x47)))
            .map(|i| (i, stride))
    }) else {
        return;
    };
    let mut section = Vec::new();
    let mut previous = None;
    for packet in data[start..].chunks(stride).filter_map(|p| p.get(..188)) {
        if packet[0] != 0x47
            || packet[1] & 0x80 != 0
            || u16::from_be_bytes([packet[1] & 0x1f, packet[2]]) != pid
            || packet[3] & 0x10 == 0
        {
            continue;
        }
        let counter = packet[3] & 15;
        if previous == Some(counter) {
            continue;
        }
        if previous.is_some_and(|v| (v + 1) & 15 != counter) {
            section.clear();
        }
        previous = Some(counter);
        let offset = if packet[3] & 0x20 != 0 {
            5 + usize::from(packet[4])
        } else {
            4
        };
        let Some(mut payload) = packet.get(offset..) else {
            section.clear();
            continue;
        };
        if packet[1] & 0x40 != 0 {
            let Some((&pointer, rest)) = payload.split_first() else {
                continue;
            };
            let Some((tail, next)) = rest.split_at_checked(pointer as usize) else {
                section.clear();
                continue;
            };
            if !section.is_empty() {
                section.extend_from_slice(tail);
                parse_sections(&mut section, &mut visit);
            }
            section.clear();
            payload = next;
        } else if section.is_empty() {
            continue;
        }
        section.extend_from_slice(payload);
        parse_sections(&mut section, &mut visit);
    }
}

fn parse_sections(buf: &mut Vec<u8>, visit: &mut impl FnMut(&[u8])) {
    while buf.len() >= 3 {
        if buf[0] == 0xff {
            buf.clear();
            return;
        }
        let size = 3 + usize::from(u16::from_be_bytes([buf[1] & 15, buf[2]]));
        if buf.len() < size {
            return;
        }
        visit(&buf[..size]);
        buf.drain(..size);
    }
}

fn parse_events(buf: &[u8], events: &mut BTreeSet<String>) {
    let size = buf.len();
    if (0x4e..=0x6f).contains(&buf[0]) && size >= 18 && buf[5] & 1 != 0 {
        let service = u16::from_be_bytes([buf[3], buf[4]]);
        let mut entries = &buf[14..size - 4];
        while let Some(header) = entries.get(..12) {
            let n = usize::from(u16::from_be_bytes([header[10] & 15, header[11]]));
            let Some((mut descriptors, rest)) = entries[12..].split_at_checked(n) else {
                break;
            };
            entries = rest;
            while descriptors.len() >= 2 {
                let tag = descriptors[0];
                let Some((value, rest)) =
                    descriptors[2..].split_at_checked(descriptors[1] as usize)
                else {
                    break;
                };
                descriptors = rest;
                if tag != 0x4d || value.len() < 5 {
                    continue;
                }
                let Some(title) = value[4..].get(..value[3] as usize) else {
                    continue;
                };
                let title = if title.first() == Some(&0x15) {
                    &title[1..]
                } else {
                    title
                };
                let title = String::from_utf8_lossy(title);
                let day = u16::from_be_bytes([header[2], header[3]]);
                let (y, m, d) = date(day);
                events.insert(format!(
                    "{y:04}-{m:02}-{d:02} {:02x}:{:02x} UTC   {title}   (service {service})",
                    header[4], header[5]
                ));
            }
        }
    }
}

// Convert the broadcast's modified Julian day to a Gregorian date.
fn date(mjd: u16) -> (i32, i32, i32) {
    let j = i32::from(mjd) + 2_400_001 + 32044;
    let century = (4 * j + 3) / 146097;
    let days = j - (146097 * century) / 4;
    let year = (4 * days + 3) / 1461;
    let doy = days - 1461 * year / 4;
    let month = (5 * doy + 2) / 153;
    (
        100 * century + year - 4800 + month / 10,
        month + 3 - 12 * (month / 10),
        doy - (153 * month + 2) / 5 + 1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guide_reads_programme_title_and_utc_date() {
        let mut section = vec![0x4e, 0xb0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 0, 0x4e];
        section.extend([0, 1, 0x9e, 0x8b, 0x12, 0x30, 0, 0, 0x30, 0, 0, 11]);
        section.extend([0x4d, 9, b'e', b'n', b'g', 4, b'N', b'e', b'w', b's', 0]);
        section.extend([0; 4]);
        section[2] = (section.len() - 3) as u8;
        let mut packets = vec![0xff; 188 * 3];
        packets[..5].copy_from_slice(&[0x47, 0x40, 0x12, 0x10, 0]);
        packets[5..5 + section.len()].copy_from_slice(&section);
        packets[188] = 0x47;
        packets[376] = 0x47;
        assert_eq!(
            programmes(&packets).first().unwrap(),
            "1970-01-01 12:30 UTC   News   (service 1)"
        );
    }
}
