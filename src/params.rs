pub use crate::context::ShowContext; // re-export
pub use crate::files::RecurseHaystacks; // re-export
pub use crate::matching::{AcceptSet, InterfaceMode, MatchMode}; // re-export
pub use crate::needle::Needle; // re-export
pub use crate::output::OutputStyle; // re-export

#[derive(Debug)]
pub struct Parameters {
    // Matching Control:
    pub accept: AcceptSet,
    pub interface_mode: InterfaceMode,
    pub match_mode: MatchMode,
    // General Output Control:
    pub output_style: OutputStyle,
    // Output Line Prefix Control:
    pub hide_filename: bool,
    pub show_lineno: bool,
    // Context Line Control:
    pub show_context: ShowContext,
    // File and Directory Selection:
    pub recursive: RecurseHaystacks,
    // Other Options:
    pub line_buffered: bool,
    // Positional arguments:
    pub needles: Vec<Needle>,
    pub haystack_filenames: Vec<String>,
}
