// Parallel-iterator prelude.
//
// Re-exports `rayon::prelude`, so `.par_iter()`, `.into_par_iter()` and
// friends resolve through one path. Bring it in with
// `use crate::par::prelude::*;`.

pub mod prelude {
    pub use rayon::prelude::*;
}

pub use rayon::join;
