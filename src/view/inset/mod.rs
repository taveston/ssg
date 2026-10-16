//! The inspection sidebar's contents (INS-01 to INS-09): the selected
//! body's details (`inspect.rs`), what they say (`content.rs`), its
//! placeholder images (`fidelity.rs`) and the "In orbit" outline of the
//! whole system (`outline.rs`). The sidebar's open state and slide are
//! shared view state, in `view/sidebar.rs`.

pub mod content;
pub mod fidelity;
pub mod inspect;
pub mod outline;
