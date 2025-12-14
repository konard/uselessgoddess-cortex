//! Knowledge Base module - Long-term memory using an associative knowledge graph.
//!
//! The knowledge graph consists of:
//! - **Tags**: Nodes representing concepts, entities, locations, or themes
//! - **Facts**: Data entries associated with one or more tags
//! - **Associations**: Weighted edges between tags

mod fact;
mod graph;
mod tag;

pub use fact::*;
pub use graph::*;
pub use tag::*;
