//! Trace sink seam: the deterministic, timing-free commit trace — the observable
//! contract only (commit_begin / dirty / eval / publish / commit_end; floats render
//! shortest-roundtrip). Exact byte schema lands at the commit-engine increment.
//! Lands in increment 12.
