//! A two-sided extrude's second side has an extent type of its own: `extent.through_all2` (the
//! second side goes through every target body, the way the first side's `through_all` does) and
//! `extent.to2` with `extent.to2_offset` (the second side ends at a face, vertex or point), both
//! on the side `direction` leaves free. Older designs need no transformation: a second side
//! without them is a distance, as before. The last release asks for an update instead of
//! opening a design that carries this change.
