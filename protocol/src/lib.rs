//! Everything the client and server must agree on.
//!
//! Split in two on purpose:
//!   - `shared` is the functional core. Pure functions, no ECS, no I/O. The
//!     whole rulebook of the game lives there and can be tested with `assert!`.
//!   - `protocol` is the imperative shell. Component registration, replication
//!     config, and the thin systems that feed queries through `shared`.
//!
//! If a rule cannot be written as a pure function, it belongs in `shared`
//! anyway and the impurity belongs in `protocol`. Keeping the boundary honest
//! is the entire point of the project.

pub mod protocol;
pub mod shared;
