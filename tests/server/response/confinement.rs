//! RFC 063: a resolved file is served only if it stays within the
//! directory it was resolved against, at all four `FileResponse`
//! construction sites.

#[path = "confinement/dyn_route.rs"]
mod dyn_route;
#[path = "confinement/middleware.rs"]
mod middleware;
#[path = "confinement/respond_file_path.rs"]
mod respond_file_path;
