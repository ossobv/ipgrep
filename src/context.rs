//!! NOTE: This file contains AI-generated test cases that have not been
//! scrutinized.

use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct ShowContext {
    pub before: usize,
    pub after: usize,
}

#[derive(Debug)]
pub struct ContextBuffer {
    // Boolean so we can quickly check/skip all the logic if nobody
    // wanted context.
    is_used: bool,
    // Keep N lines before.
    before: usize,
    // Keep N lines after.
    after: usize,
    // The queue to keep the before lines in.
    queue: VecDeque<(usize, Vec<u8>)>,
    // We don't need to keep the after lines in a queue; we can print
    // them directly.
    after_remaining: usize,
    // Keep track of the last printed lineno, so we know whether to
    // print a context-dividing delimiter ("--").
    last_printed_lineno: usize,
}

impl ContextBuffer {
    pub fn from_show_context(show_context: &ShowContext) -> Self {
        Self {
            is_used: show_context.before > 0 || show_context.after > 0,
            before: show_context.before,
            after: show_context.after,
            queue: VecDeque::new(),
            after_remaining: 0,
            last_printed_lineno: 0,
        }
    }

    /// Cheap check, allowing us to skip logic if no context is needed.
    pub fn is_used(&self) -> bool {
        self.is_used
    }

    /// Call when a match is found. Returns true if a delimiter ("--")
    /// should be printed before showing the new context block.
    pub fn is_new_match_block(&self, lineno: usize) -> bool {
        if self.last_printed_lineno == 0 {
            return false;
        }
        let first_c_lineno =
            self.queue.front().map(|(n, _)| *n).unwrap_or(lineno);
        first_c_lineno > self.last_printed_lineno + 1
    }

    /// Get the before-lines.
    pub fn get_before_lines(&self) -> &VecDeque<(usize, Vec<u8>)> {
        &self.queue
    }

    /// Clear the before-lines, after you've printed them.
    pub fn clear_before_lines(&mut self) {
        self.queue.clear();
    }

    /// Call when no match is found and we're not printing after-lines.
    pub fn push_before_line(&mut self, lineno: usize, line: &[u8]) {
        if self.before == 0 {
            return;
        }
        self.queue.push_back((lineno, line.to_vec()));
        if self.queue.len() > self.before {
            self.queue.pop_front();
        }
    }

    /// Start counting after-lines to print. Do this after printing a match.
    pub fn request_after(&mut self) {
        self.after_remaining = self.after;
    }

    /// Should we print an after-line? Do this when there is no match.
    pub fn should_print_after_line(&mut self) -> bool {
        if self.after_remaining == 0 {
            return false;
        }
        self.after_remaining -= 1;
        true
    }

    /// Record what line we're at. Call this when printing lines or
    /// after-lines.
    pub fn update_last_printed(&mut self, lineno: usize) {
        self.last_printed_lineno = lineno;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> Vec<u8> {
        format!("line{}\n", n).into_bytes()
    }

    #[test]
    fn test_before_buffering() {
        let show = ShowContext {
            before: 2,
            after: 0,
        };
        let mut ctx = ContextBuffer::from_show_context(&show);

        ctx.push_before_line(1, &line(1));
        ctx.push_before_line(2, &line(2));
        ctx.push_before_line(3, &line(3)); // should evict line1

        let collected: Vec<_> =
            ctx.get_before_lines().iter().map(|(n, _)| *n).collect();
        assert_eq!(collected, vec![2, 3], "queue keeps last N lines");
    }

    #[test]
    fn test_after_countdown() {
        let show = ShowContext {
            before: 0,
            after: 2,
        };
        let mut ctx = ContextBuffer::from_show_context(&show);

        ctx.request_after();
        assert!(ctx.should_print_after_line(), "first after line allowed");
        assert!(ctx.should_print_after_line(), "second after line allowed");
        assert!(!ctx.should_print_after_line(), "then stop after lines");
    }

    #[test]
    fn test_delimiter_needed_between_blocks() {
        let show = ShowContext {
            before: 2,
            after: 0,
        };
        let mut ctx = ContextBuffer::from_show_context(&show);

        // First match: no delimiter.
        ctx.push_before_line(10, &line(10));
        ctx.update_last_printed(0);
        assert!(!ctx.is_new_match_block(10), "first block => no delimiter");

        // Simulate printing the first block and clearing the before-queue.
        ctx.clear_before_lines();
        ctx.update_last_printed(12);

        // Second block, nearby lines: still no delimiter.
        ctx.push_before_line(13, &line(13));
        assert!(
            !ctx.is_new_match_block(13),
            "adjacent block => no delimiter"
        );

        // Simulate printing/clearing the second block as well
        ctx.clear_before_lines();
        ctx.update_last_printed(13);

        // Now add a far-away line; should cause delimiter.
        ctx.push_before_line(30, &line(30));
        assert!(
            ctx.is_new_match_block(30),
            "non-contiguous block => delimiter needed"
        );
    }

    #[test]
    fn test_clear_before_lines() {
        let show = ShowContext {
            before: 2,
            after: 0,
        };
        let mut ctx = ContextBuffer::from_show_context(&show);
        ctx.push_before_line(1, &line(1));
        ctx.push_before_line(2, &line(2));
        assert_eq!(ctx.get_before_lines().len(), 2);
        ctx.clear_before_lines();
        assert!(ctx.get_before_lines().is_empty(), "queue cleared");
    }

    #[test]
    fn test_update_last_printed_and_used_flag() {
        let show = ShowContext {
            before: 1,
            after: 1,
        };
        let mut ctx = ContextBuffer::from_show_context(&show);
        assert!(ctx.is_used, "context should be marked used");
        ctx.update_last_printed(42);
        assert_eq!(ctx.last_printed_lineno, 42);
    }

    #[test]
    fn test_unused_context_does_nothing() {
        let show = ShowContext::default();
        let mut ctx = ContextBuffer::from_show_context(&show);

        assert!(!ctx.is_used, "unused context");
        ctx.push_before_line(1, &line(1));
        assert!(
            ctx.get_before_lines().is_empty(),
            "no lines should be queued"
        );
        ctx.request_after();
        assert!(!ctx.should_print_after_line(), "no after lines tracked");
        assert!(!ctx.is_new_match_block(10), "no delimiter logic active");
    }
}
