//! Markdown pipeline.
//!
//! `parse` turns source markdown into a typed [`parse::Block`] tree (CommonMark + GFM),
//! `layout` will turn that tree × a width into wrapped [`Line`]s, and `highlight` provides the
//! instant code tokenizer. Parsing sanitizes hostile escape sequences at the boundary (§4.5).

pub mod parse;
