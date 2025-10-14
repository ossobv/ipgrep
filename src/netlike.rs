//!! NOTE: This file contains AI-generated code that has not been scrutinized.

pub struct NetLikeScanner<'a> {
    buf: &'a [u8],
    pos: usize,
    oldnet: bool,
}

impl<'a> NetLikeScanner<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            oldnet: false,
        }
    }

    /// Enables recognition of "old style" masks like 128.128.0.0/255.255.0.0.
    pub fn with_oldnet(mut self, oldnet: bool) -> Self {
        self.oldnet = oldnet;
        self
    }
}

impl Iterator for NetLikeScanner<'_> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.buf;
        let len = bytes.len();
        let mut i = self.pos;

        while i < len {
            let b = bytes[i];
            let start_candidate =
                matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':');

            if !start_candidate {
                i += 1;
                continue;
            }
            if i > 0 && is_ascii_alnum(bytes[i - 1]) {
                i += 1;
                continue;
            }

            let start = i;
            i += 1;

            // pre-slash stats
            let mut pre_dots: u8 = if b == b'.' { 1 } else { 0 };
            let mut pre_colons: u8 = if b == b':' { 1 } else { 0 };
            let mut pre_has_hex = matches!(b, b'a'..=b'f' | b'A'..=b'F');

            // post-slash stats (only used after we see a slash)
            let mut post_dots: usize = 0;
            let mut post_colons: usize = 0;
            let mut post_has_hex = false;

            let mut seen_slash = false;
            let mut slash_pos: Option<usize> = None;

            while i < len {
                match bytes[i] {
                    b'0'..=b'9' => {
                        if seen_slash {
                            // post
                        } else { /* pre */
                        }
                        i += 1;
                    }
                    b'a'..=b'f' | b'A'..=b'F' => {
                        if seen_slash {
                            post_has_hex = true;
                        } else {
                            pre_has_hex = true;
                        }
                        i += 1;
                    }
                    b'.' => {
                        if seen_slash {
                            post_dots += 1;
                        } else {
                            pre_dots += 1;
                        }
                        i += 1;
                    }
                    b':' => {
                        if seen_slash {
                            post_colons += 1;
                        } else {
                            pre_colons += 1;
                        }
                        i += 1;
                    }
                    b'/' => {
                        if !seen_slash {
                            seen_slash = true;
                            slash_pos = Some(i);
                        } else {
                            // second slash -- keep consuming but treat
                            // as part of post
                        }
                        i += 1;
                    }
                    _ => break,
                }
            }

            let mut end = i;

            // Embedded-in-word guard: if char after run is alnum,
            // reject (we're inside a word)
            if end < len && is_ascii_alnum(bytes[end]) {
                self.pos = end;
                continue;
            }

            // Trim trailing '.' or ':' characters IF they are followed
            // by a delimiter.
            while end > start {
                let last = bytes[end - 1];
                if last == b'.' || last == b':' {
                    let next_after_last = bytes.get(end);
                    if next_after_last.is_none_or(|nb| is_delim_punct(*nb)) {
                        end -= 1;
                        // adjust pre/post counts when trimming
                        if let Some(slash_idx) = slash_pos {
                            if end - 1 < slash_idx {
                                // trimming happened in pre
                                if last == b'.' {
                                    pre_dots = pre_dots.saturating_sub(1);
                                }
                                if last == b':' {
                                    pre_colons = pre_colons.saturating_sub(1);
                                }
                            } else {
                                // trimming happened in post
                                if last == b'.' {
                                    post_dots = post_dots.saturating_sub(1);
                                }
                                if last == b':' {
                                    post_colons = post_colons.saturating_sub(1);
                                }
                            }
                        } else {
                            // no slash seen: trimming in pre
                            if last == b'.' {
                                pre_dots = pre_dots.saturating_sub(1);
                            }
                            if last == b':' {
                                pre_colons = pre_colons.saturating_sub(1);
                            }
                        }
                        continue;
                    }
                }
                break;
            }

            // advance scanner position to after the maximal run we
            // consumed; final decisions below
            self.pos = i;

            // minimal length guard
            if end - start < 2 {
                continue;
            }

            // Split the logic into two cases: no slash seen, or slash seen.
            if !seen_slash {
                // No slash: previous logic (per-part)
                // reject runs that mix hex letters and dots without any
                // colon (can't be IPv4 or IPv6)
                if pre_has_hex && pre_dots > 0 && pre_colons == 0 {
                    continue;
                }
                // IPv4-like: more than 3 dots is invalid
                if pre_colons == 0 && pre_dots > 3 {
                    continue;
                }
                // Reject tokens ending with a single '.' or ':' that we
                // didn't trim (defensive)
                if end > start {
                    let last = bytes[end - 1];
                    if last == b'.' || last == b':' {
                        continue;
                    }
                }
                return Some((start, end));
            } else {
                // Slash was present; decide based on oldnet flag and
                // per-part stats
                let slash_idx = slash_pos.unwrap();

                // compute slices for pre and post as seen inside the run
                // pre: start..slash_idx
                // post: (slash_idx+1)..end
                let post_len = end.saturating_sub(slash_idx + 1);

                // If post is empty-ish, reject (something like "1.2./")
                if post_len == 0 {
                    continue;
                }

                // Helper checks:
                let post_starts_digit = bytes[slash_idx + 1].is_ascii_digit();
                let post_has_dot = post_dots > 0;
                let post_has_colon = post_colons > 0;
                let post_has_hex = post_has_hex;

                let pre_valid_for_ip = {
                    // pre part must look like either an IPv4-ish or
                    // IPv6-ish fragment:
                    if pre_colons == 0 {
                        // IPv4-ish: dots <= 3 and doesn't mix hex+dot
                        !(pre_has_hex && pre_dots > 0) && pre_dots <= 3
                    } else {
                        // IPv6-ish: allow anything with colons (defer
                        // deeper validation)
                        true
                    }
                };

                // Case A: oldnet enabled -> allow dotted-mask after
                // slash
                if self.oldnet {
                    let post_looks_like_dotted_ipv4 = !post_has_hex
                        && post_has_dot
                        && post_dots == 3
                        && post_colons == 0;
                    let post_looks_like_ip =
                        post_has_dot || post_has_colon || post_has_hex;
                    let post_looks_like_number = {
                        // all digits?
                        let mut all_digits = true;
                        for b in bytes.iter().take(end).skip(slash_idx + 1) {
                            if !b.is_ascii_digit() {
                                all_digits = false;
                                break;
                            }
                        }
                        all_digits
                    };

                    if pre_valid_for_ip
                        && (post_looks_like_dotted_ipv4
                            || post_looks_like_ip
                            || post_looks_like_number)
                    {
                        // Accept entire run as one token (oldnet or
                        // CIDR or weird but plausible); classifier will
                        // validate fully.
                        return Some((start, end));
                    } else {
                        // can't interpret post as something plausible:
                        // fallthrough to rejecting
                        continue;
                    }
                } else {
                    // oldnet disabled
                    // If post looks like a numeric CIDR (all digits),
                    // keep the whole token (CIDR)
                    let post_all_digits = {
                        let mut all_digits = true;
                        for b in bytes.iter().take(end).skip(slash_idx + 1) {
                            if !b.is_ascii_digit() {
                                all_digits = false;
                                break;
                            }
                        }
                        all_digits
                    };

                    if post_all_digits {
                        // Keep whole CIDR token: "1.2.3.0/24"
                        // but minimal guard: pre must be a plausible ip-ish
                        if pre_valid_for_ip {
                            return Some((start, end));
                        } else {
                            continue;
                        }
                    }

                    // If post starts with digit and looks like an IP
                    // (contains '.' or ':'), split: return pre only
                    if post_starts_digit
                        && (post_has_dot || post_has_colon)
                        && pre_valid_for_ip
                    {
                        // return prefix (start..slash_idx) and leave
                        // scanner.pos set to slash+1 so next() will see
                        // the post token
                        self.pos = slash_idx + 1;
                        return Some((start, slash_idx));
                    }

                    // Otherwise keep whole run if pre_valid_for_ip
                    // (e.g. weird cases with letters -- defer to
                    // classifier)
                    if pre_valid_for_ip {
                        return Some((start, end));
                    }

                    continue;
                }
            }
        }

        self.pos = len;
        None
    }
}

fn is_ascii_alnum(b: u8) -> bool {
    b.is_ascii_digit() || b.is_ascii_lowercase() || b.is_ascii_uppercase()
}

fn is_ascii_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0B | 0x0C)
}

/// ASCII punctuation we consider safe delimiters (trim trailing '.' /
/// ':' when followed by these)
fn is_delim_punct(b: u8) -> bool {
    // typical punctuation that can follow an IP in text: space, comma,
    // semicolon, parentheses, etc.
    is_ascii_whitespace(b)
        || matches!(
            b,
            b',' | b';'
                | b')'
                | b'('
                | b']'
                | b'['
                | b'<'
                | b'>'
                | b'\"'
                | b'\''
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    #[test]
    fn scan_ip_like_runs() {
        #[rustfmt::skip]
        let cases = [
            (
                // First one needs the "as &[u8]" to define the type.
                b"plain 1.2.3.4 end" as &[u8],  
                &["1.2.3.4"][..],
                &["1.2.3.4"][..],
            ),
            (
                b"0.0.0.0",
                &["0.0.0.0"][..],
                &["0.0.0.0"][..],
            ),
            (
                // Beware, these are illegal.
                b" 255.255.255.256. 312.34.56.781.",
                &["255.255.255.256", "312.34.56.781"][..],
                &["255.255.255.256", "312.34.56.781"][..],
            ),
            (
                b"cidr 1.2.3.0/24 mask 255.255.255.0 oldnet",
                &["1.2.3.0/24", "255.255.255.0"][..],
                &["1.2.3.0/24", "255.255.255.0"][..],
            ),
            (
                b"v6 ::1, ::ffff:10.0.0.1/127 and fd4e:3732:3033::1/64",
                &["::1", "::ffff:10.0.0.1/127", "fd4e:3732:3033::1/64"][..],
                &["::1", "::ffff:10.0.0.1/127", "fd4e:3732:3033::1/64"][..],
            ),
            (
                b"no match: 1.2.3.4.5 and garbage 12a3::zz",
                &[][..],
                &[][..],
            ),
            (
                b"do match: 1.2.3.4. 5 and no garbage 12a3::def. zz",
                &["1.2.3.4", "12a3::def"][..],
                &["1.2.3.4", "12a3::def"][..],
            ),
            (
                b"bordering punctuation: [1.2.3.4],(::1)",
                &["1.2.3.4", "::1"][..],
                &["1.2.3.4", "::1"][..],
            ),
            (
                b"128.128.0.0/255.255.0.0 is old style netmask notation",
                &["128.128.0.0", "255.255.0.0"][..],
                &["128.128.0.0/255.255.0.0"][..],
            ),
            (
                b" ipv4.address1: \"10.20.30.123/24,10.20.30.1\"",
                &["10.20.30.123/24", "10.20.30.1"][..],
                &["10.20.30.123/24", "10.20.30.1"][..],
            ),
            (
                b"last on the line 100.200.300.400.",
                &["100.200.300.400"][..],
                &["100.200.300.400"][..],
            ),
        ];

        for (input, expected, expected_with_oldnet) in cases {
            let got: Vec<_> = NetLikeScanner::new(input)
                .map(|(s, e)| str::from_utf8(&input[s..e]).unwrap().to_string())
                .collect();
            let got_with_oldnet: Vec<_> = NetLikeScanner::new(input)
                .with_oldnet(true)
                .map(|(s, e)| str::from_utf8(&input[s..e]).unwrap().to_string())
                .collect();

            assert_eq!(
                got,
                expected,
                "for input {:?}, expected {:?}, got {:?}",
                std::str::from_utf8(input).unwrap(),
                expected,
                got
            );
            assert_eq!(
                got_with_oldnet,
                expected_with_oldnet,
                "for input {:?}, expected {:?}, got {:?}",
                std::str::from_utf8(input).unwrap(),
                expected_with_oldnet,
                got_with_oldnet
            );
        }
    }
}
