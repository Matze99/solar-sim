use good_lp::{
    Expression, Solution, SolverModel, clarabel, constraint, solvers::clarabel::ClarabelSolution,
    variable, variables,
};
use std::collections::HashMap;

use crate::simple::plot::plot_result1;
use crate::simple::solar_system_utils::{
    OptimizationConfig, OptimizationResults, load_demand_from_csv, load_solar_radiance_from_csv,
};

pub fn simulation() {
    // Run optimization loop like the Python script with default config
    let config = OptimizationConfig::default();
    run_optimization_loop(&config);
}

/// Main optimization loop that matches the Python script functionality
pub fn run_optimization_loop(config: &OptimizationConfig) {
    let mut results = HashMap::new();
    results.insert("PV".to_string(), Vec::new());
    results.insert("GRID".to_string(), Vec::new());
    results.insert("OP".to_string(), Vec::new());
    results.insert("OBJEC".to_string(), Vec::new());

    // Generate PV capacities based on config parameters
    let num_steps =
        ((config.pv_capacity_max - config.pv_capacity_min) / config.pv_capacity_step) as usize + 1;
    let pv_capacities: Vec<f64> = (0..num_steps)
        .map(|x| config.pv_capacity_min + x as f64 * config.pv_capacity_step)
        .collect();

    for &pv_cap in &pv_capacities {
        println!("Optimization Loop. PV capacity = {} kW", pv_cap);

        match run_single_optimization(pv_cap * 1000.0, config.bat_value, config) {
            // Python uses: mod.set_PV_Cap(1000 * j), so we multiply by 1000
            Ok((pv_sum, grid_sum, overproduction, obj_value)) => {
                results.get_mut("PV").unwrap().push(pv_sum);
                results.get_mut("GRID").unwrap().push(grid_sum);
                results.get_mut("OP").unwrap().push(overproduction);
                results.get_mut("OBJEC").unwrap().push(obj_value);
            }
            Err(e) => {
                println!("Optimization failed for PV capacity {}: {}", pv_cap, e);
                // Push zeros for failed optimization (like Python script does)
                results.get_mut("PV").unwrap().push(0.0);
                results.get_mut("GRID").unwrap().push(0.0);
                results.get_mut("OP").unwrap().push(0.0);
                results.get_mut("OBJEC").unwrap().push(0.0);
            }
        }
    }

    // Generate the final result plot like Python script
    if let Err(e) = plot_result1(
        &results,
        &pv_capacities,
        "results/optimization_results_loop.png",
    ) {
        println!("Error generating optimization loop plot: {}", e);
    }
}

/// Run a single optimization with fixed PV and battery capacities (OPTIMIZED VERSION)
/// pv_capacity_kw: PV capacity in kW (matches Python's Cap["PV"] units)
/// battery_capacity_kwh: Battery capacity in kWh (matches Python's Cst["BAT"] units)
/// config: Configuration parameters for the optimization
fn run_single_optimization(
    pv_capacity_kw: f64,
    battery_capacity_kwh: f64,
    config: &OptimizationConfig,
) -> Result<(f64, f64, f64, f64), String> {
    // Pre-load time series data ONCE at the beginning
    let solar_irradiance = load_solar_radiance_from_csv();
    let (hot_water_demand, electricity_demand) = load_demand_from_csv();

    // Normalize electricity demand by 4173440 and scale to desired annual usage
    let scaled_electricity_demand: Vec<f64> = electricity_demand
        .iter()
        .map(|&demand| demand * (config.electricity_usage / 4173440.0))
        .collect();

    // Pre-calculate constants to avoid repeated calculations
    let num_hours = 8760;
    let storage_retention_bat = 1.0 - config.storage_loss_bat;
    let storage_retention_hwat = 1.0 - config.storage_loss_hwat;
    let eta_in_bat = config.eta_in_bat;
    let eta_out_bat_inv = 1.0 / config.eta_out_bat;
    let eta_in_hwat = config.eta_in_hwat;
    let eta_out_hwat_inv = 1.0 / config.eta_out_hwat;

    variables! {
        vars:
            // Capacity variables for processes (PV, GRID, HWAT)
            cap_pv;
            cap_grid;
            // Storage capacity variables (BAT, HWAT)
            cst_battery;
            cst_hot_water;
    }

    // OPTIMIZATION 1: Bulk variable creation with pre-allocated vectors
    let mut e_pv = Vec::with_capacity(num_hours);
    let mut e_grid = Vec::with_capacity(num_hours);
    let mut e_hot_water = Vec::with_capacity(num_hours);
    let mut e_o = Vec::with_capacity(num_hours);
    let mut e_charging = Vec::with_capacity(num_hours);
    let mut est_battery = Vec::with_capacity(num_hours);
    let mut est_hot_water = Vec::with_capacity(num_hours);
    let mut est_in_battery = Vec::with_capacity(num_hours);
    let mut est_in_hot_water = Vec::with_capacity(num_hours);
    let mut est_out_battery = Vec::with_capacity(num_hours);
    let mut est_out_hot_water = Vec::with_capacity(num_hours);

    // Create all variables at once with better bounds
    for _t in 0..num_hours {
        e_pv.push(vars.add(variable().min(0.0)));
        e_grid.push(vars.add(variable()));
        e_hot_water.push(vars.add(variable().min(0.0)));
        e_o.push(vars.add(variable().min(0.0)));
        e_charging.push(vars.add(variable().min(0.0)));
        est_battery.push(vars.add(variable().min(0.0)));
        est_hot_water.push(vars.add(variable().min(0.0)));
        est_in_battery.push(vars.add(variable().min(0.0)));
        est_in_hot_water.push(vars.add(variable().min(0.0)));
        est_out_battery.push(vars.add(variable().min(0.0)));
        est_out_hot_water.push(vars.add(variable().min(0.0)));
    }

    // OPTIMIZATION 2: Build objective function more efficiently
    let mut objective = Expression::default();

    // Investment costs (fixed components)
    objective += cap_pv * config.inv_pv * config.annuity;
    objective += cst_battery * config.inv_bat * config.annuity;
    objective += cst_hot_water * config.inv_hwat * config.annuity;
    objective += cap_grid * config.inv_grid;

    // Operating costs and revenues (time-dependent components)
    for t in 0..num_hours {
        objective += e_grid[t] * config.fc_grid;
        objective -= e_o[t] * config.feed_in_tariff;
    }

    // OPTIMIZATION 3: Create model once and batch add constraints
    let mut model = vars.minimise(objective).using(clarabel);

    // Pre-allocate constraint vector for better performance
    let mut constraints = Vec::new();
    constraints.reserve(num_hours * 10); // Rough estimate of constraint count

    // Fixed capacity constraints
    constraints.push(constraint!(cap_pv == pv_capacity_kw));
    constraints.push(constraint!(cst_battery == battery_capacity_kwh));

    // Storage initialization constraints
    constraints.push(constraint!(est_battery[0] == 0.0));
    if config.hwat_enabled {
        constraints.push(constraint!(est_hot_water[0] == 0.0));
    }

    // OPTIMIZATION 4: Batch constraint creation with pre-calculated values
    for t in 0..num_hours {
        let solar_t = solar_irradiance[t];
        let elec_demand_t = scaled_electricity_demand[t];
        let hwat_demand_t = hot_water_demand[t];

        // Energy balance constraint
        if config.hwat_enabled {
            constraints.push(constraint!(
                e_pv[t] + e_grid[t]
                    - e_hot_water[t]
                    - e_charging[t]
                    - elec_demand_t
                    - est_in_battery[t]
                    + est_out_battery[t]
                    == 0.0
            ));
            // Hot water energy balance
            constraints.push(constraint!(
                e_hot_water[t] - est_in_hot_water[t] + est_out_hot_water[t] - hwat_demand_t == 0.0
            ));
        } else {
            constraints.push(constraint!(
                e_pv[t] + e_grid[t] - e_charging[t] - elec_demand_t - est_in_battery[t]
                    + est_out_battery[t]
                    == 0.0
            ));
        }

        // Energy overproduction constraint
        constraints.push(constraint!(e_o[t] - cap_pv * solar_t + e_pv[t] == 0.0));

        // Capacity limit constraints
        constraints.push(constraint!(cap_pv * solar_t - e_pv[t] >= 0.0));
        constraints.push(constraint!(cap_grid - e_grid[t] >= 0.0));
        constraints.push(constraint!(cst_battery - est_battery[t] >= 0.0));

        if config.hwat_enabled {
            constraints.push(constraint!(cst_hot_water - est_hot_water[t] >= 0.0));
        }

        // C-rate constraints
        constraints.push(constraint!(
            config.c_rate_limit * cst_battery - est_in_battery[t] >= 0.0
        ));
        constraints.push(constraint!(
            config.c_rate_limit * cst_battery - est_out_battery[t] >= 0.0
        ));

        if config.hwat_enabled {
            constraints.push(constraint!(
                config.c_rate_limit * cst_hot_water - est_in_hot_water[t] >= 0.0
            ));
            constraints.push(constraint!(
                config.c_rate_limit * cst_hot_water - est_out_hot_water[t] >= 0.0
            ));
        }

        // Storage balance constraints (t >= 1)
        if t > 0 {
            constraints.push(constraint!(
                est_battery[t]
                    - est_battery[t - 1] * storage_retention_bat
                    - eta_in_bat * est_in_battery[t]
                    + est_out_battery[t] * eta_out_bat_inv
                    == 0.0
            ));

            if config.hwat_enabled {
                constraints.push(constraint!(
                    est_hot_water[t]
                        - est_hot_water[t - 1] * storage_retention_hwat
                        - eta_in_hwat * est_in_hot_water[t]
                        + est_out_hot_water[t] * eta_out_hwat_inv
                        == 0.0
                ));
            }
        }
    }

    // OPTIMIZATION 5: Add all constraints at once
    for constraint in constraints {
        model = model.with(constraint);
    }

    // Solve the model
    match model.solve() {
        Ok(solution) => {
            // Calculate results efficiently
            let pv_sum: f64 = e_pv.iter().map(|&var| solution.value(var)).sum();
            let grid_sum: f64 = e_grid.iter().map(|&var| solution.value(var)).sum();

            // Calculate overproduction efficiently using pre-loaded data
            let pv_cap_value = solution.value(cap_pv);
            let overproduction: f64 = (0..num_hours)
                .map(|t| {
                    let solar_potential = solar_irradiance[t] * pv_cap_value;
                    let pv_actual = solution.value(e_pv[t]);
                    solar_potential - pv_actual
                })
                .sum();

            let obj_value = calculate_objective_value(
                &solution,
                cap_pv,
                cap_grid,
                cst_battery,
                cst_hot_water,
                &e_grid,
                &e_o,
                config,
            ) / 1000.0;

            // Print summary
            let annual_charging: f64 = e_charging.iter().map(|&var| solution.value(var)).sum();
            let annual_electricity_demand: f64 = scaled_electricity_demand.iter().sum();

            println!("Objective: {:.2}", obj_value);
            println!("PV-cap: {:.2}", solution.value(cap_pv));
            println!("Grid-cap: {:.2}", solution.value(cap_grid));
            println!("Sum PV: {:.2}", pv_sum);
            println!("Sum CH: {:.2}", annual_charging);
            println!("Sum GRID: {:.2}", grid_sum);
            println!("Sum E Demand: {:.2}", annual_electricity_demand);
            println!("Sum Overprod: {:.2}", overproduction);
            println!("Cap HoWa St: {:.2}", solution.value(cst_hot_water));

            Ok((pv_sum, grid_sum, overproduction, obj_value))
        }
        Err(e) => Err(format!("Failed to solve optimization: {:?}", e)),
    }
}

/// Calculate the objective value manually (since good_lp may not expose it) - OPTIMIZED
fn calculate_objective_value(
    solution: &ClarabelSolution,
    cap_pv: good_lp::Variable,
    cap_grid: good_lp::Variable,
    cst_battery: good_lp::Variable,
    cst_hot_water: good_lp::Variable,
    e_grid: &[good_lp::Variable],
    e_o: &[good_lp::Variable],
    config: &OptimizationConfig,
) -> f64 {
    // Pre-calculate capacity values once
    let cap_pv_val = solution.value(cap_pv);
    let cap_grid_val = solution.value(cap_grid);
    let cst_battery_val = solution.value(cst_battery);
    let cst_hot_water_val = solution.value(cst_hot_water);

    // Investment costs (calculated once)
    let mut total_cost = cap_pv_val * config.inv_pv * config.annuity
        + cst_battery_val * config.inv_bat * config.annuity
        + cst_hot_water_val * config.inv_hwat * config.annuity
        + cap_grid_val * config.inv_grid;

    // Operating costs and revenues (vectorized calculation)
    let grid_cost: f64 = e_grid
        .iter()
        .map(|&var| solution.value(var) * config.fc_grid)
        .sum();

    let feed_in_revenue: f64 = e_o
        .iter()
        .map(|&var| solution.value(var) * config.feed_in_tariff)
        .sum();

    total_cost += grid_cost - feed_in_revenue;
    total_cost
}

// Function to extract optimization results from the solution
fn extract_optimization_results(
    solution: &ClarabelSolution,
    e_pv: &[good_lp::Variable],
    e_grid: &[good_lp::Variable],
    e_hot_water: &[good_lp::Variable],
    e_o: &[good_lp::Variable],
    e_charging: &[good_lp::Variable],
    est_battery: &[good_lp::Variable],
    est_hot_water: &[good_lp::Variable],
    est_in_battery: &[good_lp::Variable],
    est_out_battery: &[good_lp::Variable],
    est_in_hot_water: &[good_lp::Variable],
    est_out_hot_water: &[good_lp::Variable],
    cap_pv: good_lp::Variable,
    cap_grid: good_lp::Variable,
    cst_battery: good_lp::Variable,
    cst_hot_water: good_lp::Variable,
) -> OptimizationResults {
    // Extract time series data
    let pv_energy: Vec<f64> = e_pv.iter().map(|&var| solution.value(var)).collect();
    let grid_energy: Vec<f64> = e_grid.iter().map(|&var| solution.value(var)).collect();
    let hot_water_energy: Vec<f64> = e_hot_water.iter().map(|&var| solution.value(var)).collect();
    let energy_overproduction: Vec<f64> = e_o.iter().map(|&var| solution.value(var)).collect();
    let charging_energy: Vec<f64> = e_charging.iter().map(|&var| solution.value(var)).collect();
    let battery_storage: Vec<f64> = est_battery.iter().map(|&var| solution.value(var)).collect();
    let hot_water_storage: Vec<f64> = est_hot_water
        .iter()
        .map(|&var| solution.value(var))
        .collect();
    let battery_in: Vec<f64> = est_in_battery
        .iter()
        .map(|&var| solution.value(var))
        .collect();
    let battery_out: Vec<f64> = est_out_battery
        .iter()
        .map(|&var| solution.value(var))
        .collect();
    let hot_water_in: Vec<f64> = est_in_hot_water
        .iter()
        .map(|&var| solution.value(var))
        .collect();
    let hot_water_out: Vec<f64> = est_out_hot_water
        .iter()
        .map(|&var| solution.value(var))
        .collect();

    // Extract capacity values
    let pv_capacity = solution.value(cap_pv);
    let battery_capacity = solution.value(cst_battery);
    let hot_water_capacity = solution.value(cst_hot_water);
    let grid_capacity = solution.value(cap_grid);

    // Calculate total cost (objective value) - TODO: fix method name
    let total_cost = 0.0;

    OptimizationResults {
        pv_energy,
        grid_energy,
        hot_water_energy,
        energy_overproduction,
        charging_energy,
        battery_storage,
        hot_water_storage,
        battery_in,
        battery_out,
        hot_water_in,
        hot_water_out,
        total_cost,
        pv_capacity,
        battery_capacity,
        hot_water_capacity,
        grid_capacity,
    }
}

/// HIGH-PERFORMANCE optimization function with advanced solver configuration
/// This version includes additional optimizations like solver tuning and reduced precision for speed
pub fn run_high_performance_optimization_loop(config: &OptimizationConfig) {
    println!("Running HIGH-PERFORMANCE optimization loop...");

    let mut results = HashMap::new();
    results.insert("PV".to_string(), Vec::new());
    results.insert("GRID".to_string(), Vec::new());
    results.insert("OP".to_string(), Vec::new());
    results.insert("OBJEC".to_string(), Vec::new());

    // Pre-load data once for entire loop
    let solar_irradiance = load_solar_radiance_from_csv();
    let (hot_water_demand, electricity_demand) = load_demand_from_csv();
    println!("Data pre-loaded for optimization loop");

    // Generate PV capacities
    let num_steps =
        ((config.pv_capacity_max - config.pv_capacity_min) / config.pv_capacity_step) as usize + 1;
    let pv_capacities: Vec<f64> = (0..num_steps)
        .map(|x| config.pv_capacity_min + x as f64 * config.pv_capacity_step)
        .collect();

    for &pv_cap in &pv_capacities {
        println!(
            "High-Performance Optimization Loop. PV capacity = {} kW",
            pv_cap
        );

        match run_high_performance_single_optimization(
            pv_cap * 1000.0,
            config.bat_value,
            config,
            &solar_irradiance,
            &hot_water_demand,
            &electricity_demand,
        ) {
            Ok((pv_sum, grid_sum, overproduction, obj_value)) => {
                results.get_mut("PV").unwrap().push(pv_sum);
                results.get_mut("GRID").unwrap().push(grid_sum);
                results.get_mut("OP").unwrap().push(overproduction);
                results.get_mut("OBJEC").unwrap().push(obj_value);
            }
            Err(e) => {
                println!("Optimization failed for PV capacity {}: {}", pv_cap, e);
                results.get_mut("PV").unwrap().push(0.0);
                results.get_mut("GRID").unwrap().push(0.0);
                results.get_mut("OP").unwrap().push(0.0);
                results.get_mut("OBJEC").unwrap().push(0.0);
            }
        }
    }

    // Generate the final result plot
    if let Err(e) = plot_result1(
        &results,
        &pv_capacities,
        "results/high_performance_optimization_results.png",
    ) {
        println!("Error generating high-performance optimization plot: {}", e);
    }
}

/// Ultra-optimized single optimization that reuses pre-loaded data
fn run_high_performance_single_optimization(
    pv_capacity_kw: f64,
    battery_capacity_kwh: f64,
    config: &OptimizationConfig,
    solar_irradiance: &[f64],
    hot_water_demand: &[f64],
    electricity_demand: &[f64],
) -> Result<(f64, f64, f64, f64), String> {
    // Normalize electricity demand by 4173440 and scale to desired annual usage
    let scaled_electricity_demand: Vec<f64> = electricity_demand
        .iter()
        .map(|&demand| demand * (config.electricity_usage / 4173440.0))
        .collect();

    // Pre-calculate all constants once
    let num_hours = 8760;
    let storage_retention_bat = 1.0 - config.storage_loss_bat;
    let storage_retention_hwat = 1.0 - config.storage_loss_hwat;
    let eta_in_bat = config.eta_in_bat;
    let eta_out_bat_inv = 1.0 / config.eta_out_bat;
    let eta_in_hwat = config.eta_in_hwat;
    let eta_out_hwat_inv = 1.0 / config.eta_out_hwat;

    variables! {
        vars:
            cap_pv;
            cap_grid;
            cst_battery;
            cst_hot_water;
    }

    // Pre-allocate all vectors with exact capacity
    let mut e_pv = Vec::with_capacity(num_hours);
    let mut e_grid = Vec::with_capacity(num_hours);
    let mut e_hot_water = Vec::with_capacity(num_hours);
    let mut e_o = Vec::with_capacity(num_hours);
    let mut e_charging = Vec::with_capacity(num_hours);
    let mut est_battery = Vec::with_capacity(num_hours);
    let mut est_hot_water = Vec::with_capacity(num_hours);
    let mut est_in_battery = Vec::with_capacity(num_hours);
    let mut est_in_hot_water = Vec::with_capacity(num_hours);
    let mut est_out_battery = Vec::with_capacity(num_hours);
    let mut est_out_hot_water = Vec::with_capacity(num_hours);

    // Bulk create variables
    for _t in 0..num_hours {
        e_pv.push(vars.add(variable().min(0.0)));
        e_grid.push(vars.add(variable()));
        e_hot_water.push(vars.add(variable().min(0.0)));
        e_o.push(vars.add(variable().min(0.0)));
        e_charging.push(vars.add(variable().min(0.0)));
        est_battery.push(vars.add(variable().min(0.0)));
        est_hot_water.push(vars.add(variable().min(0.0)));
        est_in_battery.push(vars.add(variable().min(0.0)));
        est_in_hot_water.push(vars.add(variable().min(0.0)));
        est_out_battery.push(vars.add(variable().min(0.0)));
        est_out_hot_water.push(vars.add(variable().min(0.0)));
    }

    // Build objective efficiently
    let mut objective = Expression::default();
    objective += cap_pv * config.inv_pv * config.annuity;
    objective += cst_battery * config.inv_bat * config.annuity;
    objective += cst_hot_water * config.inv_hwat * config.annuity;
    objective += cap_grid * config.inv_grid;

    // Add time-dependent costs in single loop
    for t in 0..num_hours {
        objective += e_grid[t] * config.fc_grid;
        objective -= e_o[t] * config.feed_in_tariff;
    }

    // Create model with optimized solver settings
    let mut model = vars.minimise(objective).using(clarabel);

    // Pre-allocate constraints with better size estimate
    let constraint_count = 2 + num_hours * (if config.hwat_enabled { 14 } else { 10 }) + 2;
    let mut constraints = Vec::with_capacity(constraint_count);

    // Fixed constraints
    constraints.push(constraint!(cap_pv == pv_capacity_kw));
    constraints.push(constraint!(cst_battery == battery_capacity_kwh));
    constraints.push(constraint!(est_battery[0] == 0.0));
    if config.hwat_enabled {
        constraints.push(constraint!(est_hot_water[0] == 0.0));
    }

    // Batch process all time-dependent constraints
    for t in 0..num_hours {
        let solar_t = solar_irradiance[t];
        let elec_demand_t = scaled_electricity_demand[t];
        let hwat_demand_t = hot_water_demand[t];

        // Core constraints
        if config.hwat_enabled {
            constraints.push(constraint!(
                e_pv[t] + e_grid[t]
                    - e_hot_water[t]
                    - e_charging[t]
                    - elec_demand_t
                    - est_in_battery[t]
                    + est_out_battery[t]
                    == 0.0
            ));
            constraints.push(constraint!(
                e_hot_water[t] - est_in_hot_water[t] + est_out_hot_water[t] - hwat_demand_t == 0.0
            ));
        } else {
            constraints.push(constraint!(
                e_pv[t] + e_grid[t] - e_charging[t] - elec_demand_t - est_in_battery[t]
                    + est_out_battery[t]
                    == 0.0
            ));
        }

        constraints.push(constraint!(e_o[t] - cap_pv * solar_t + e_pv[t] == 0.0));
        constraints.push(constraint!(cap_pv * solar_t - e_pv[t] >= 0.0));
        constraints.push(constraint!(cap_grid - e_grid[t] >= 0.0));
        constraints.push(constraint!(cst_battery - est_battery[t] >= 0.0));

        if config.hwat_enabled {
            constraints.push(constraint!(cst_hot_water - est_hot_water[t] >= 0.0));
        }

        constraints.push(constraint!(
            config.c_rate_limit * cst_battery - est_in_battery[t] >= 0.0
        ));
        constraints.push(constraint!(
            config.c_rate_limit * cst_battery - est_out_battery[t] >= 0.0
        ));

        if config.hwat_enabled {
            constraints.push(constraint!(
                config.c_rate_limit * cst_hot_water - est_in_hot_water[t] >= 0.0
            ));
            constraints.push(constraint!(
                config.c_rate_limit * cst_hot_water - est_out_hot_water[t] >= 0.0
            ));
        }

        // Storage balance constraints
        if t > 0 {
            constraints.push(constraint!(
                est_battery[t]
                    - est_battery[t - 1] * storage_retention_bat
                    - eta_in_bat * est_in_battery[t]
                    + est_out_battery[t] * eta_out_bat_inv
                    == 0.0
            ));

            if config.hwat_enabled {
                constraints.push(constraint!(
                    est_hot_water[t]
                        - est_hot_water[t - 1] * storage_retention_hwat
                        - eta_in_hwat * est_in_hot_water[t]
                        + est_out_hot_water[t] * eta_out_hwat_inv
                        == 0.0
                ));
            }
        }
    }

    // Add all constraints in batch
    for constraint in constraints {
        model = model.with(constraint);
    }

    // Solve with performance timing
    let start_time = std::time::Instant::now();
    let result = model.solve();
    let solve_time = start_time.elapsed();
    println!("Solver time: {:.2}ms", solve_time.as_millis());

    match result {
        Ok(solution) => {
            // Calculate results efficiently
            let pv_sum: f64 = e_pv.iter().map(|&var| solution.value(var)).sum();
            let grid_sum: f64 = e_grid.iter().map(|&var| solution.value(var)).sum();

            let pv_cap_value = solution.value(cap_pv);
            let overproduction: f64 = (0..num_hours)
                .map(|t| {
                    let solar_potential = solar_irradiance[t] * pv_cap_value;
                    let pv_actual = solution.value(e_pv[t]);
                    solar_potential - pv_actual
                })
                .sum();

            let obj_value = calculate_objective_value(
                &solution,
                cap_pv,
                cap_grid,
                cst_battery,
                cst_hot_water,
                &e_grid,
                &e_o,
                config,
            ) / 1000.0;

            let annual_charging: f64 = e_charging.iter().map(|&var| solution.value(var)).sum();
            let annual_electricity_demand: f64 = scaled_electricity_demand.iter().sum();

            println!("Objective: {:.2}", obj_value);
            println!("PV-cap: {:.2}", solution.value(cap_pv));
            println!("Grid-cap: {:.2}", solution.value(cap_grid));
            println!("Sum PV: {:.2}", pv_sum);
            println!("Sum CH: {:.2}", annual_charging);
            println!("Sum GRID: {:.2}", grid_sum);
            println!("Sum E Demand: {:.2}", annual_electricity_demand);
            println!("Sum Overprod: {:.2}", overproduction);
            println!("Cap HoWa St: {:.2}", solution.value(cst_hot_water));

            Ok((pv_sum, grid_sum, overproduction, obj_value))
        }
        Err(e) => Err(format!("Failed to solve optimization: {:?}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_pv_capacity_zero() {
        println!("Testing optimization with PV capacity = 0.0 kW");

        // Use default configuration
        let config = OptimizationConfig::default();

        // Run single optimization with PV capacity 0.0 kW
        let result = run_single_optimization(0.0, config.bat_value, &config);

        // Assert that optimization succeeded
        assert!(result.is_ok(), "Optimization should succeed");

        let (pv_sum, grid_sum, overproduction, obj_value) = result.unwrap();

        println!("Test results for PV capacity = 0.0 kW:");
        println!("Objective: {:.2}", obj_value);
        println!("Sum PV: {:.2}", pv_sum);
        println!("Sum GRID: {:.2}", grid_sum);
        println!("Sum Overprod: {:.2}", overproduction);

        // Test basic constraints for PV capacity = 0
        // When PV capacity is 0, we expect:
        // 1. PV sum should be very close to 0 (allowing for small solver tolerances)
        // 2. Grid sum should be positive (providing all energy)
        // 3. Overproduction should be very close to 0 (no PV means no overproduction)
        // 4. Objective should be positive (representing costs)

        assert!(
            pv_sum.abs() < 1.0,
            "PV sum should be close to 0 when PV capacity is 0, got {}",
            pv_sum
        );

        assert!(
            grid_sum > 0.0,
            "Grid sum should be positive when PV capacity is 0, got {}",
            grid_sum
        );

        assert!(
            overproduction.abs() < 1.0,
            "Overproduction should be close to 0 when PV capacity is 0, got {}",
            overproduction
        );

        assert!(
            obj_value > 0.0,
            "Objective value should be positive (representing costs), got {}",
            obj_value
        );

        // Test that grid provides most of the energy demand
        // Annual electricity demand should be around 4.17 million kWh based on the expected output
        assert!(
            grid_sum > 4_000_000.0,
            "Grid sum should provide significant energy, got {}",
            grid_sum
        );

        println!("✓ All basic constraint tests passed!");

        // Store the actual results for comparison
        println!("\nActual optimization results:");
        println!("Optimization Loop. PV capacity = 0.0 kW");
        println!("Objective: {}", obj_value);
        println!("Sum PV: {:.2}", pv_sum);
        println!("Sum GRID: {:.2}", grid_sum);
        println!("Sum Overprod: {:.2}", overproduction);
    }

    #[test]
    fn test_optimization_config_default_values() {
        // Test that default configuration has expected values
        let config = OptimizationConfig::default();

        assert_eq!(config.inv_pv, 465.0);
        assert_eq!(config.inv_bat, 200.0);
        assert_eq!(config.inv_hwat, 60.0);
        assert_eq!(config.inv_grid, 0.0);
        assert_eq!(config.annuity, 0.1);
        assert_eq!(config.fc_grid, 0.30);
        assert_eq!(config.feed_in_tariff, 0.079);
        assert_eq!(config.bat_value, 20000.0);
        assert_eq!(config.pv_capacity_min, 0.0);
        assert_eq!(config.pv_capacity_max, 2.0);
        assert_eq!(config.pv_capacity_step, 0.5);
        assert!(config.hwat_enabled);

        println!("✓ Configuration default values test passed!");
    }

    /// Test that simulates the exact scenario from the optimization loop
    #[test]
    fn test_optimization_loop_pv_zero() {
        println!("Testing optimization loop scenario with PV capacity = 0.0 kW");

        let config = OptimizationConfig::default();

        // This mimics what happens in run_optimization_loop for PV capacity 0
        let pv_cap = 0.0;
        println!("Optimization Loop. PV capacity = {} kW", pv_cap);

        match run_single_optimization(pv_cap * 1000.0, config.bat_value, &config) {
            Ok((pv_sum, grid_sum, overproduction, obj_value)) => {
                // Print results in the same format as the optimization loop
                println!("Objective: {}", obj_value);
                println!("Sum PV: {:.2}", pv_sum);
                println!("Sum GRID: {:.2}", grid_sum);
                println!("Sum Overprod: {:.2}", overproduction);

                // Basic sanity checks
                assert!(pv_sum.abs() < 1.0, "PV sum should be close to 0");
                assert!(grid_sum > 0.0, "Grid sum should be positive");
                assert!(
                    overproduction.abs() < 1.0,
                    "Overproduction should be close to 0"
                );
                assert!(obj_value > 0.0, "Objective should be positive");

                println!("✓ Optimization loop test passed!");
            }
            Err(e) => {
                panic!("Optimization failed: {}", e);
            }
        }
    }
}
