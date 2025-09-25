pub mod general;
pub mod simple;

// Re-export commonly used items for convenience
pub use general::finance::calculate_optimized_roi;
pub use simple::simple_opt_re::run_simple_opt;
