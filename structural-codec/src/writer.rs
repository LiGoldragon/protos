//! The canonical Block→text writer, owned by structural-codec's encode path.
//! Encoding selects the one canonical structural form, so its final text policy
//! belongs beside that operation; raw-discovery discovers raw structure and does not
//! define canonical textual output. Expressed as a local extension trait so the
//! writing logic still lives on a data-bearing type (`Block`), never a free function.

use raw_discovery::Block;

/// Render a discovered block back to its canonical NOTA text.
pub trait CanonicalText {
    fn canonical_text(&self) -> String;
}

impl CanonicalText for Block {
    fn canonical_text(&self) -> String {
        match self {
            Block::Atom(atom) => atom.text().to_owned(),
            Block::PipeText(pipe) => {
                let escaped = pipe.text().replace('\\', "\\\\").replace("|)", "\\|)");
                format!("(|{escaped}|)")
            }
            Block::Application { head, payload } => {
                format!("{}.{}", head.canonical_text(), payload.canonical_text())
            }
            Block::Delimited {
                delimiter,
                root_objects,
            } => delimiter.wrap(root_objects.iter().map(CanonicalText::canonical_text)),
        }
    }
}
