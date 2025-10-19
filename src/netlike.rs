use std::cmp::min;

use memchr::memchr;

#[derive(Copy, Clone, PartialEq)]
enum NetLikeRestriction {
    IpsAndCidrs,
    AlsoOldNets,
}

pub struct NetLikeScanner<'a> {
    buf: &'a [u8],
    pos: usize,
    restrict: NetLikeRestriction,
}

const IPV46_START: &[u8; 23] = b"0123456789abcdefABCDEF:";

impl<'a> NetLikeScanner<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            restrict: NetLikeRestriction::IpsAndCidrs,
        }
    }

    /// Enables recognition of "old style" masks like 128.128.0.0/255.255.0.0.
    pub fn with_oldnet(self) -> Self {
        Self {
            restrict: NetLikeRestriction::AlsoOldNets,
            ..self
        }
    }

    #[inline]
    fn next_impl(&mut self) -> Option<(usize, usize)> {
        let bytes = self.buf;
        let len = bytes.len(); // self.pos can be beyond

        // Shortest IPv4 is 7 ("1.1.1.1").
        let leftover = len.saturating_sub(self.pos);
        if leftover < 7 {
            while self.pos < len {
                if matches!(
                    bytes[self.pos],
                    b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':')
                {
                    return self.try_ipv6();
                }
                self.pos += 1;
            }
            return None;
        }

        // Leftover is >=7
        let len_minus_6 = len - 6;
        let mut delim_is_colon = if self.pos > 0 {
            // memchr(bytes[self.pos - 1], IPV46_START).is_some()
            bytes[self.pos - 1] == b':'
        } else {
            false
        };

        while self.pos < len_minus_6 {
            let b = bytes[self.pos];

            if !delim_is_colon {
                if let Some(idx) = memchr(b, IPV46_START) {
                    if idx < 10 {
                        // 0..9
                        if let Some(res) = self.try_ipv4_or_ipv6() {
                            return Some(res);
                        }
                        // Retry loop now that self.pos is increased.
                        delim_is_colon = bytes[self.pos - 1] == b':';
                        continue;
                    } else if idx < 22 || bytes[self.pos + 1] == b':' {
                        // a..fA..F || (':' && nextchar is ':')
                        if let Some(res) = self.try_ipv6() {
                            return Some(res);
                        }
                        // Retry loop now that self.pos is increased.
                        delim_is_colon = bytes[self.pos - 1] == b':';
                        continue;
                    }
                }
            } else if b.is_ascii_digit() {
                // 0..9
                if let Some(res) = self.try_ipv4() {
                    return Some(res);
                }
                // Retry loop now that self.pos is increased.
                delim_is_colon = bytes[self.pos - 1] == b':';
                continue;
            } else {
                delim_is_colon = false;
            }

            self.pos += 1;
        }

        // Leftover is <7
        while self.pos < len {
            if matches!(
                bytes[self.pos],
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':')
            {
                if let Some(res) = self.try_ipv6() {
                    return Some(res);
                }
            }
            self.pos += 1;
        }

        None
    }

    #[inline]
    fn seek_to_non_digit(&mut self, start: usize) -> Option<(usize, usize)> {
        let bytes = self.buf;
        let len = bytes.len();
        let mut i = start;

        while i < len {
            if !bytes[i].is_ascii_digit() {
                break;
            }
            i += 1;
        }
        self.pos = min(len, i + 1);
        None
    }

    #[inline]
    fn seek_to_non_digit_period(
        &mut self,
        start: usize,
    ) -> Option<(usize, usize)> {
        let bytes = self.buf;
        let len = bytes.len();
        let mut i = start;

        while i < len {
            if !matches!(bytes[i], b'0'..=b'9' | b'.') {
                break;
            }
            i += 1;
        }
        self.pos = min(len, i + 1);
        None
    }

    #[inline]
    fn seek_to_non_letter(&mut self, start: usize) -> Option<(usize, usize)> {
        let bytes = self.buf;
        let len = bytes.len();
        let mut i = start;

        while i < len {
            if !bytes[i].is_ascii_alphabetic() {
                break;
            }
            i += 1;
        }
        self.pos = min(len, i + 1);
        None
    }

    #[inline]
    fn maybe_netmask(
        &mut self,
        end: usize,
        restrict: NetLikeRestriction,
    ) -> Option<(usize, usize)> {
        let bytes = self.buf;
        let len = bytes.len();
        let start = self.pos;
        let mut i = end;

        let mut numdigits = 0;

        while i < len {
            if !bytes[i].is_ascii_digit() {
                break;
            }
            numdigits += 1;
            i += 1;
            if numdigits > 3 {
                break;
            }
        }

        // 0 digits or more than 3; then this is not a netmask.
        if numdigits == 0 || numdigits > 3 {
            self.pos = end;
            return Some((start, end - 1));
        }

        // 1..3 digits en then period and then a number? could be old
        // style netmask.
        if i + 1 < len && bytes[i] == b'.' && bytes[i + 1].is_ascii_digit() {
            match restrict {
                // Don't accept old style? Go back and return the IP.
                NetLikeRestriction::IpsAndCidrs => {
                    self.pos = end;
                    return Some((start, end - 1));
                }
                // Slurp all the digits and dots and then return it if
                // it's a valid IP. Always put pos beyond that.
                NetLikeRestriction::AlsoOldNets => {
                    let mut valid = false;
                    let mut dots = 1;
                    numdigits = 0;
                    i += 1;
                    while i < len {
                        match bytes[i] {
                            b'0'..=b'9' => {
                                numdigits += 1;
                                if dots == 3 {
                                    valid = true;
                                }
                            }
                            b'.' => {
                                if numdigits > 3 {
                                    valid = false;
                                } else if numdigits == 0 {
                                    break;
                                } else {
                                    numdigits = 0;
                                    dots += 1;
                                }
                            }
                            _ => {
                                break;
                            }
                        }
                        i += 1;
                    }
                    // Update pos in either case.
                    self.pos = i + 1;
                    if valid {
                        return Some((start, i));
                    } else {
                        return Some((start, end - 1));
                    }
                }
            }
        }

        self.pos = i + 1;
        Some((start, i))
    }

    #[inline]
    fn try_ipv4(&mut self) -> Option<(usize, usize)> {
        // We have at least 7 chars and the first token is 0..9.
        let bytes = self.buf;
        let len = bytes.len();
        let start = self.pos;

        let mut dots = 0;
        let mut numsize = 1; // starting with digit
        let mut end = start + 1;

        while end < len {
            match bytes[end] {
                b'0'..=b'9' => {
                    numsize += 1;
                    if numsize > 3 {
                        return self.seek_to_non_digit(end + 1);
                    }
                }
                b'.' => {
                    if numsize == 0 {
                        self.pos = end;
                        return None;
                    }
                    numsize = 0;
                    dots += 1;
                    if dots == 3 {
                        end += 1;
                        break;
                    }
                }
                _ => {
                    self.pos = end;
                    return None;
                }
            }
            end += 1;
        }

        // Fourth digit.
        while end < len {
            if !bytes[end].is_ascii_digit() {
                break;
            }
            numsize += 1;
            if numsize > 3 {
                return self.seek_to_non_digit(end + 1);
            }
            end += 1;
        }

        // Easy case.
        if numsize == 0 {
            self.pos = min(len, end + 1);
            return None;
        }
        if end == len {
            self.pos = end;
            return Some((start, end));
        }

        // Check for legal endings.
        match bytes[end] {
            b'/' => {
                return self.maybe_netmask(end + 1, self.restrict);
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                // Reject all the matches.
                return self.seek_to_non_letter(end + 1);
            }
            b'.' => {
                // Address stops being legal if we get another octet
                // after the fourth.
                // ["1.2.3.4".] <- legal
                // ["1.2.3.4.5"] <- illegal
                if end + 1 < len && bytes[end + 1].is_ascii_digit() {
                    return self.seek_to_non_digit_period(end + 1);
                }
            }
            _ => {}
        }

        self.pos = end + 1;
        Some((start, end))
    }

    #[inline]
    fn try_ipv6(&mut self) -> Option<(usize, usize)> {
        // Unsure how many characters we have, but ldelim is not ':'.
        let bytes = self.buf;
        let len = bytes.len();
        let start = self.pos;
        let mut end = start;
        let mut colons = 0;

        while end < len {
            match bytes[end] {
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {}
                b':' => {
                    colons += 1;
                }
                b'.' => {
                    if start + 7 < len
                        && !(bytes[start] == b':'
                            && bytes[start + 1] == b':'
                            && matches!(bytes[start + 2], b'f' | b'F')
                            && matches!(bytes[start + 3], b'f' | b'F')
                            && matches!(bytes[start + 4], b'f' | b'F')
                            && matches!(bytes[start + 5], b'f' | b'F')
                            && bytes[start + 6] == b':')
                    {
                        self.pos = end + 1;
                        return Some((start, end));
                    }
                }
                b'/' => {
                    if colons >= 2 {
                        return self.maybe_netmask(
                            end + 1,
                            NetLikeRestriction::IpsAndCidrs,
                        );
                    } else {
                        self.pos = end + 1;
                        return None;
                    }
                }
                b'g'..=b'z' | b'G'..=b'Z' => {
                    colons = 0; // make the match invalid
                    break;
                }
                _ => {
                    break;
                }
            }
            end += 1;
        }

        self.pos = end + 1;

        if colons >= 2 {
            Some((start, end))
        } else {
            None
        }
    }

    #[inline]
    fn try_ipv4_or_ipv6(&mut self) -> Option<(usize, usize)> {
        // We have at least 7 chars and the first token is 0..9.
        let bytes = self.buf;
        //
        // Second digit.
        match bytes[self.pos + 1] {
            b'0'..=b'9' => {
                // undecided
            }
            b'a'..=b'f' | b'A'..=b'F' | b':' => {
                return self.try_ipv6();
            }
            b'.' => {
                return self.try_ipv4();
            }
            _ => {
                self.pos += 2;
                return None;
            }
        }
        // Third digit.
        match bytes[self.pos + 2] {
            b'0'..=b'9' => {
                // undecided
            }
            b'a'..=b'f' | b'A'..=b'F' | b':' => {
                return self.try_ipv6();
            }
            b'.' => {
                return self.try_ipv4();
            }
            _ => {
                self.pos += 3;
                return None;
            }
        }
        // Fourth digit.
        match bytes[self.pos + 3] {
            b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b':' => self.try_ipv6(),
            b'.' => self.try_ipv4(),
            _ => {
                self.pos += 4;
                None
            }
        }
    }
}

impl Iterator for NetLikeScanner<'_> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl()
    }
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
            (
                b"multiple colons 1.1.1.1:2.2.2.2:3.3.3.3 end",
                &["1.1.1.1", "2.2.2.2", "3.3.3.3"][..],
                &["1.1.1.1", "2.2.2.2", "3.3.3.3"][..],
            ),
            (
                b"ports 1.2.3.4:17772 and 10.20.30.40:22",
                &["1.2.3.4", "10.20.30.40"][..],
                &["1.2.3.4", "10.20.30.40"][..],
            ),
            (
                b"ipv6 unaffected ::1:80 still valid",
                &["::1:80"][..],
                &["::1:80"][..],
            ),
            (
                b"100.200.300.400:80, 40.30.20.10:443",
                &["100.200.300.400", "40.30.20.10"][..],
                &["100.200.300.400", "40.30.20.10"][..],
            ),
            (
                b"range like: 192.168.0.0..192.168.2.255",
                &["192.168.0.0", "192.168.2.255"][..],
                &["192.168.0.0", "192.168.2.255"][..],
            ),
            (
                b"::Ffff:123.45.67.89/1",
                &["::Ffff:123.45.67.89/1"][..],
                &["::Ffff:123.45.67.89/1"][..],
            ),
            (
                b"/::fFfF:123.45.67.89/1.2.3.4.5/::/",
                &["::fFfF:123.45.67.89", "::"][..],
                &["::fFfF:123.45.67.89", "::"][..],
            ),
            (
                b"..1.2.3.4..5.6.7.8..",
                &["1.2.3.4", "5.6.7.8"][..],
                &["1.2.3.4", "5.6.7.8"][..],
            ),
            (
                b"1.2.3.4z",
                &[][..],
                &[][..],
            ),
            (
                b"199.8.7.166.5/199.8.7.166/199.8.177/199.8/199",
                &["199.8.7.166"][..],
                &["199.8.7.166"][..],
            ),
            (
                // FIXME: That 'd' could be part of the IPv6, so I don't
                // like that this matches. It should match "this::" though.
                b"colons at the end::",
                &["::"][..],
                &["::"][..],
            ),
            (
                b"::/::",
                &["::", "::"][..],
                &["::", "::"][..],
            ),
            (
                b"0.0.0.0/::/::",
                &["0.0.0.0", "::", "::"][..],
                &["0.0.0.0", "::", "::"][..],
            ),
            (
                b"[::255:255.0.0.4]",
                &["::255:255"][..],
                &["::255:255"][..],
            ),
            (
                b"11.22.33.",
                &[][..],
                &[][..],
            ),
            (
                // FIXME: Not sure if we want to match "fe0c::fee" here.
                b":255.255.0.0/24::fec0::fee",
                &["255.255.0.0/24", "fec0::fee"][..],
                &["255.255.0.0/24", "fec0::fee"][..],
            ),
            (
                // FIXME: Not sure if we want to match "fe0c::fee" here.
                b":255.255.0.0::fec0::fee",
                &["255.255.0.0", "fec0::fee"][..],
                &["255.255.0.0", "fec0::fee"][..],
            ),
        ];

        for (input, expected, expected_with_oldnet) in cases {
            let got: Vec<_> = NetLikeScanner::new(input)
                .map(|(s, e)| str::from_utf8(&input[s..e]).unwrap().to_string())
                .collect();
            let got_with_oldnet: Vec<_> = NetLikeScanner::new(input)
                .with_oldnet()
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
                "for (with oldnet) input {:?}, expected {:?}, got {:?}",
                std::str::from_utf8(input).unwrap(),
                expected_with_oldnet,
                got_with_oldnet
            );
        }
    }
}
