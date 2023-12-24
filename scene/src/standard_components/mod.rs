// * TODO: entity to stop the scene
// * TODO: logging entity

// TODO: entity to shut down other entities
// TODO: scripting entity
// TODO: HTTP server entity
// TODO: JSON streaming entity
// TODO: error reporting entity
// TODO: progress reporting entity
// TODO: named pipe entity (+ entity to introduce the contents of a named pipe as entities)

pub use self::empty::*;
pub use self::entity_ids::*;
pub use self::entity_registry::*;
pub use self::example::*;
#[cfg(feature = "properties")]
pub use self::floating_binding::*;
pub use self::heartbeat::*;
pub use self::logging::*;
#[cfg(feature = "properties")]
pub use self::properties::*;
pub use self::scene_control::*;
pub use self::timer::*;

mod entity_ids;
mod example;
mod entity_registry;
mod heartbeat;
mod scene_control;
mod timer;
mod empty;
mod logging;

#[cfg(feature = "properties")]
mod properties;
#[cfg(feature = "properties")]
mod floating_binding;

