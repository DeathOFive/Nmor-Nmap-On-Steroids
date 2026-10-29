use std::collections::BTreeSet;

/// Expand comma-separated ports and inclusive ranges into a sorted,
/// deduplicated list. `-` means all TCP ports, including -p- shorthand.
pub fn parse_ports(spec: &str) -> Result<Vec<u16>, String> {
    if spec.trim().is_empty() {
        return Err("port specification cannot be empty".into());
    }

    let mut ports = BTreeSet::new();
    for raw in spec.split(',') {
        let part = raw.trim();
        if part.is_empty() {
            return Err("empty item in port list".into());
        }

        if let Some((start, end)) = part.split_once('-') {
            // Allow - (all), -1024 (from 1), and 1024- (through 65535).
            let start = if start.is_empty() {
                1
            } else {
                parse_port(start)?
            };
            let end = if end.is_empty() {
                u16::MAX
            } else {
                parse_port(end)?
            };
            if start > end {
                return Err(format!("invalid range '{part}': start is larger than end"));
            }
            ports.extend(start..=end);
        } else {
            ports.insert(parse_port(part)?);
        }
    }
    Ok(ports.into_iter().collect())
}

fn parse_port(text: &str) -> Result<u16, String> {
    let port = text
        .parse::<u16>()
        .map_err(|_| format!("invalid TCP port '{text}': expected 1-65535"))?;
    if port == 0 {
        return Err("port 0 is not a valid remote TCP service port".into());
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_ports_and_ranges() {
        assert_eq!(
            parse_ports("22,80,443,8000-8002").unwrap(),
            vec![22, 80, 443, 8000, 8001, 8002]
        );
    }

    #[test]
    fn sorts_and_deduplicates() {
        assert_eq!(
            parse_ports("443,80,80,79-81").unwrap(),
            vec![79, 80, 81, 443]
        );
    }

    #[test]
    fn supports_open_ended_ranges() {
        assert_eq!(parse_ports("-3").unwrap(), vec![1, 2, 3]);
        let upper = parse_ports("65534-").unwrap();
        assert_eq!(upper, vec![65534, 65535]);
        assert_eq!(parse_ports("-").unwrap().len(), 65535);
    }

    #[test]
    fn rejects_bad_ports() {
        for input in ["", "0", "65536", "1-0", "100-50", "80,,443", "abc", "5--7"] {
            assert!(parse_ports(input).is_err(), "should reject '{input}'");
        }
    }
}
