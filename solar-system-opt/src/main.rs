use solar_system_opt::simple::simple_opt_re::{run_simple_opt_loop, run_simple_opt_with_day_plots};
use solar_system_opt::simple::solar_system_utils::OptimizationConfig;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("days") => {
            println!("Running simple optimization with individual day plots...");
            let config = OptimizationConfig::default();
            // Plot first day of each season: Winter solstice (day 0), Spring equinox (~day 80),
            // Summer solstice (~day 172), Fall equinox (~day 266)
            let days_to_plot = vec![0, 80, 172, 266, 100, 200, 300];
            if let Err(e) = run_simple_opt_with_day_plots(config, &days_to_plot) {
                eprintln!("Error running simple optimization with day plots: {}", e);
            }
        }
        _ => {
            println!("Running simple optimization with hourly averages plot...");
            let config = OptimizationConfig::default();
            if let Err(e) = run_simple_opt_loop(config) {
                eprintln!("Error running simple optimization: {}", e);
            }
        }
    }

    println!("Optimization complete!");
}
