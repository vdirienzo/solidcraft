//! A sweep saves `path_body` and `path_edges` when its path is a chain of a body's edges (the
//! user picked the edges of the model, not the curves of a sketch); absent, the path is the
//! sketch's curves it always was, and a design from before this change keeps them. The last
//! release refuses a design with this change (the newer format number).
