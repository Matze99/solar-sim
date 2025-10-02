use ems_model::building::electricity::ElectricityRate;
use good_lp::{Expression, SolverModel, constraint, variable};
use good_lp::{Solver, variables};

use crate::general::electricity_demand::{MonthlyDemand, create_scaled_load_curve_from_csv};
use crate::simple::plot::{plot_hourly_averages, plot_hourly_averages_with_title};
use crate::simple::solar_system_utils::{
    HeatingType, InsulationLevel, OptimizationConfig, SimpleOptimizationResults,
    load_demand_from_csv, load_solar_radiance_from_csv,
};

const NUM_HOURS: usize = 8760;

/// Helper function to convert day number to a readable date string
fn get_date_string(day: usize) -> String {
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut remaining_days = day;
    let mut month = 0;

    for (i, &days) in days_in_month.iter().enumerate() {
        if remaining_days < days {
            month = i;
            break;
        }
        remaining_days -= days;
    }

    format!("{} {}", months[month], remaining_days + 1)
}

pub fn get_scaled_electricity_demand(
    monthly_demand: Option<MonthlyDemand>,
    electricity_usage: f64,
    electricity_demand: Vec<f64>,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let scaled_electricity_demand = if let Some(ref monthly_demand) = monthly_demand {
        // Generate scaled load curve using monthly demand and base CSV data
        create_scaled_load_curve_from_csv(monthly_demand, "data/demand.csv")?
            .iter()
            .map(|&demand| demand * 1000.0) // Convert from kWh to Wh to match existing scaling
            .collect()
    } else {
        // Use the provided electricity_demand and scale by desired annual usage
        electricity_demand
            .iter()
            .map(|&demand| demand * (electricity_usage / 4173440.0))
            .collect()
    };
    Ok(scaled_electricity_demand)
}

fn generate_objective(
    config: &OptimizationConfig,
    e_grid: &Vec<good_lp::Variable>,
    cap_pv: good_lp::Variable,
    cap_grid: good_lp::Variable,
    cst_battery: good_lp::Variable,
    electricity_rate_hourly: &Vec<f64>,
    e_o: &Vec<good_lp::Variable>,
) -> (
    Expression,
    good_lp::Variable,
    good_lp::Variable,
    good_lp::Variable,
) {
    // Build objective function
    let mut objective = Expression::default();

    if config.optimize_for_autonomy {
        // Optimize for maximum autonomy: minimize grid consumption
        for t in 0..NUM_HOURS {
            objective += e_grid[t] / 1000.0; // Minimize grid consumption
        }
    }
    // Optimize for minimum cost: include investment costs and operating costs
    // Investment costs
    objective += cap_pv / 1000.0 * config.inv_pv * config.annuity;
    objective += cap_grid / 1000.0 * config.inv_grid;
    if config.bat_value > 0.0 {
        objective += cst_battery / 1000.0 * config.inv_bat * config.annuity;
    }

    // Operating costs and revenues (time-dependent)
    for t in 0..NUM_HOURS {
        objective += e_grid[t] / 1000.0 * electricity_rate_hourly[t]; // Cost of grid electricity
        objective -= e_o[t] / 1000.0 * config.feed_in_tariff; // Revenue from feed-in
    }

    (objective, cap_pv, cap_grid, cst_battery)
}

/// Adds all fixed constraints that are not time dependent
fn add_fixed_constraints<M>(
    mut model: M,
    config: &OptimizationConfig,
    pv_cap_w_max: f64,
    cap_pv: good_lp::Variable,
    cst_battery: good_lp::Variable,
    est_battery: &Option<Vec<good_lp::Variable>>,
    e_car_charge: &[good_lp::Variable],
    car_daily_energy_required: f64,
) -> M
where
    M: good_lp::SolverModel,
{
    // Fixed capacity constraints
    if config.pv_fixed {
        model = model.with(constraint!(cap_pv == pv_cap_w_max));
    } else {
        model = model.with(constraint!(cap_pv >= 0.0));
        model = model.with(constraint!(cap_pv <= pv_cap_w_max));
    }

    // Battery capacity constraints (only if bat_value > 0)
    if config.bat_value > 0.0 {
        if config.bat_fixed {
            model = model.with(constraint!(cst_battery == config.bat_value));
        } else {
            model = model.with(constraint!(cst_battery >= 0.0));
            model = model.with(constraint!(cst_battery <= config.bat_value));
        }

        // Battery initialization constraint
        if let Some(battery_vars) = est_battery {
            model = model.with(constraint!(battery_vars[0] == 0.0));
        }
    }

    // Electric car total energy constraint
    if config.electric_car_enabled {
        // Sum of all charging must equal required daily energy * 365 days
        let total_car_charging: Expression =
            e_car_charge.iter().map(|&var| Expression::from(var)).sum();
        model = model.with(constraint!(
            total_car_charging == car_daily_energy_required * 365.0
        ));
    }

    model
}

/// Generates time-dependent constraints for the optimization model
fn add_time_dependent_constraints<M>(
    mut model: M,
    config: &OptimizationConfig,
    solar_irradiance: &[f64],
    scaled_electricity_demand: &[f64],
    e_pv: &[good_lp::Variable],
    e_grid: &[good_lp::Variable],
    e_o: &[good_lp::Variable],
    est_battery: &Option<Vec<good_lp::Variable>>,
    est_in_battery: &Option<Vec<good_lp::Variable>>,
    est_out_battery: &Option<Vec<good_lp::Variable>>,
    e_car_charge: &[good_lp::Variable],
    cap_pv: good_lp::Variable,
    cap_grid: good_lp::Variable,
    cst_battery: good_lp::Variable,
    storage_retention_bat: f64,
    eta_in_bat: f64,
    eta_out_bat_inv: f64,
) -> M
where
    M: good_lp::SolverModel,
{
    for t in 0..NUM_HOURS {
        let solar_t = solar_irradiance[t];
        let elec_demand_t = scaled_electricity_demand[t];

        // Energy balance: PV + Grid + Battery Out = Demand + Battery In + Car Charging + Heat Pump
        if let (Some(battery_in), Some(battery_out)) = (est_in_battery, est_out_battery) {
            model = model.with(constraint!(
                e_pv[t] + e_grid[t] - elec_demand_t - battery_in[t] + battery_out[t]
                    - e_car_charge[t]
                    == 0.0
            ));
        } else {
            // No battery: PV + Grid = Demand + Car Charging
            model = model.with(constraint!(
                e_pv[t] + e_grid[t] - elec_demand_t - e_car_charge[t] == 0.0
            ));
        }

        // Overproduction constraint: overproduction = potential PV - actual PV
        model = model.with(constraint!(e_o[t] - cap_pv * solar_t + e_pv[t] == 0.0));

        // PV capacity limit: actual PV <= potential PV
        model = model.with(constraint!(cap_pv * solar_t - e_pv[t] >= 0.0));

        // Grid capacity limit
        model = model.with(constraint!(cap_grid - e_grid[t] >= 0.0));

        // Battery constraints
        if config.bat_value > 0.0 {
            if let (Some(battery_storage), Some(battery_in), Some(battery_out)) =
                (est_battery, est_in_battery, est_out_battery)
            {
                // Battery capacity limit
                model = model.with(constraint!(cst_battery - battery_storage[t] >= 0.0));

                // C-rate constraints
                model = model.with(constraint!(
                    config.c_rate_limit * cst_battery - battery_in[t] >= 0.0
                ));
                model = model.with(constraint!(
                    config.c_rate_limit * cst_battery - battery_out[t] >= 0.0
                ));

                // Storage balance constraints (t >= 1)
                if t > 0 {
                    model = model.with(constraint!(
                        battery_storage[t]
                            - battery_storage[t - 1] * storage_retention_bat
                            - eta_in_bat * battery_in[t]
                            + battery_out[t] * eta_out_bat_inv
                            == 0.0
                    ));
                }
            }
        }

        // Electric car charging constraints
        if config.electric_car_enabled {
            // Determine if this is a charging hour (simplified: day = 6-18, night = 18-6)
            let hour_of_day = t % 24;
            let is_day_hour = hour_of_day >= 6 && hour_of_day < 18;
            let can_charge = if config.car_charge_during_day {
                is_day_hour
            } else {
                !is_day_hour
            };

            // If car cannot charge during this hour, set charging to zero
            if !can_charge {
                model = model.with(constraint!(e_car_charge[t] == 0.0));
            }
        } else {
            // If electric car is disabled, set all charging to zero
            model = model.with(constraint!(e_car_charge[t] == 0.0));
        }
    }

    model
}

/// Formats the optimization solution into a SimpleOptimizationResults struct
fn format_solution_results(
    solution: &dyn good_lp::Solution,
    config: &OptimizationConfig,
    e_pv: &[good_lp::Variable],
    e_grid: &[good_lp::Variable],
    e_o: &[good_lp::Variable],
    est_battery: &Option<Vec<good_lp::Variable>>,
    est_in_battery: &Option<Vec<good_lp::Variable>>,
    est_out_battery: &Option<Vec<good_lp::Variable>>,
    e_car_charge: &[good_lp::Variable],
    scaled_electricity_demand: &[f64],
    cap_pv: good_lp::Variable,
    cap_grid: good_lp::Variable,
    cst_battery: good_lp::Variable,
    car_daily_energy_required: f64,
    optimization_duration: std::time::Duration,
) -> SimpleOptimizationResults {
    // Calculate and print results
    let pv_sum: f64 = e_pv.iter().map(|&var| solution.value(var)).sum();
    let grid_sum: f64 = e_grid.iter().map(|&var| solution.value(var)).sum();
    let overproduction: f64 = e_o.iter().map(|&var| solution.value(var)).sum();
    let total_demand: f64 = scaled_electricity_demand.iter().sum();
    let battery_in_sum: f64 = if let Some(battery_in) = est_in_battery {
        battery_in.iter().map(|&var| solution.value(var)).sum()
    } else {
        0.0
    };
    let battery_out_sum: f64 = if let Some(battery_out) = est_out_battery {
        battery_out.iter().map(|&var| solution.value(var)).sum()
    } else {
        0.0
    };
    let car_charging_sum: f64 = e_car_charge.iter().map(|&var| solution.value(var)).sum();

    // Collect hourly data for struct
    let pv_production: Vec<f64> = e_pv.iter().map(|&var| solution.value(var)).collect();
    let overproduction_hourly: Vec<f64> = e_o.iter().map(|&var| solution.value(var)).collect();
    let grid_consumption: Vec<f64> = e_grid.iter().map(|&var| solution.value(var)).collect();
    let battery_storage: Vec<f64> = if let Some(battery_storage_vars) = est_battery {
        battery_storage_vars
            .iter()
            .map(|&var| solution.value(var))
            .collect()
    } else {
        vec![0.0; NUM_HOURS]
    };
    let car_charging_hourly: Vec<f64> = e_car_charge
        .iter()
        .map(|&var| solution.value(var))
        .collect();

    // Calculate total PV production (consumed + overproduction)
    let total_pv_production: Vec<f64> = pv_production
        .iter()
        .zip(overproduction_hourly.iter())
        .map(|(&consumed, &over)| consumed + over)
        .collect();

    // Combine electricity demand with car charging consumption
    let total_electricity_demand: Vec<f64> = scaled_electricity_demand
        .iter()
        .zip(car_charging_hourly.iter())
        .map(|(&demand, &charging)| demand + charging)
        .collect();

    // Calculate autarky without battery by checking when user consumes directly from PV
    // and summing that up, then dividing by total demand
    let mut direct_pv_consumption = 0.0;
    let mut total_demand_without_battery = 0.0;

    for t in 0..NUM_HOURS {
        let pv_prod_t = total_pv_production[t];
        let demand_t = total_electricity_demand[t];

        // Direct consumption is the minimum of PV production and demand
        let direct_consumption = pv_prod_t.min(demand_t);
        direct_pv_consumption += direct_consumption;
        total_demand_without_battery += demand_t;
    }

    let autarky_without_battery = if total_demand_without_battery > 0.0 {
        (direct_pv_consumption / total_demand_without_battery) * 100.0
    } else {
        0.0
    };

    SimpleOptimizationResults {
        pv_capacity_kw: solution.value(cap_pv) / 1000.0,
        grid_capacity_kw: solution.value(cap_grid) / 1000.0,
        battery_capacity_kwh: solution.value(cst_battery) / 1000.0,
        annual_pv_production_kwh: (pv_sum + overproduction) / 1000.0,
        annual_grid_energy_kwh: grid_sum / 1000.0,
        annual_battery_in_kwh: battery_in_sum / 1000.0,
        annual_battery_out_kwh: battery_out_sum / 1000.0,
        annual_car_charging_kwh: car_charging_sum / 1000.0,
        annual_overproduction_kwh: overproduction / 1000.0,
        annual_electricity_demand_kwh: total_demand / 1000.0,
        required_car_energy_kwh: if config.electric_car_enabled {
            car_daily_energy_required * 365.0 / 1000.0
        } else {
            0.0
        },
        pv_coverage_percent: (pv_sum / total_demand) * 100.0,
        autarky: (1.0 - grid_sum / total_demand) * 100.0,
        autarky_without_battery,
        hourly_pv_production: pv_production,
        hourly_overproduction: overproduction_hourly,
        hourly_grid_consumption: grid_consumption,
        hourly_battery_storage: battery_storage,
        hourly_car_charging: car_charging_hourly,
        hourly_total_pv_production: total_pv_production,
        hourly_total_electricity_demand: total_electricity_demand,
        hourly_electricity_demand_base: scaled_electricity_demand.to_vec(),
        config: config.clone(),
        optimization_duration_ms: optimization_duration.as_millis(),
    }
}

pub fn run_simple_opt<S: Solver>(
    config: OptimizationConfig,
    pv_cap_w_max: f64,
    solar_irradiance: Vec<f64>,
    electricity_demand: Vec<f64>,
    electricity_rate: ElectricityRate,
    solver: S,
) -> Result<SimpleOptimizationResults, Box<dyn std::error::Error>> {
    // Use monthly demand to generate scaled load curve if available, otherwise use provided electricity_demand
    let scaled_electricity_demand = get_scaled_electricity_demand(
        config.monthly_demand.clone(),
        config.electricity_usage.clone(),
        electricity_demand,
    )?;

    let electricity_rate_hourly = electricity_rate.to_yearly_hourly_rates();
    // Pre-calculate battery constants
    let storage_retention_bat = 1.0 - config.storage_loss_bat;
    let eta_in_bat = config.eta_in_bat;
    let eta_out_bat_inv = 1.0 / config.eta_out_bat;

    variables! {
        vars:
            cap_pv;
            cap_grid;
            cst_battery;
    }

    // energy usage of own production
    let mut e_pv: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    // energy usage of grid
    let mut e_grid: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    // energy overproduction
    let mut e_o: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS); // overproduction
    // battery storage variables (only created if bat_value > 0)
    let mut est_battery: Option<Vec<good_lp::Variable>> = if config.bat_value > 0.0 {
        Some(Vec::with_capacity(NUM_HOURS))
    } else {
        None
    };
    let mut est_in_battery: Option<Vec<good_lp::Variable>> = if config.bat_value > 0.0 {
        Some(Vec::with_capacity(NUM_HOURS))
    } else {
        None
    };
    let mut est_out_battery: Option<Vec<good_lp::Variable>> = if config.bat_value > 0.0 {
        Some(Vec::with_capacity(NUM_HOURS))
    } else {
        None
    };
    // electric car charging variables
    let mut e_car_charge: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);

    // Create variables for each hour
    for _t in 0..NUM_HOURS {
        e_pv.push(vars.add(variable().min(0.0))); // PV energy (non-negative)
        e_grid.push(vars.add(variable().min(0.0))); // Grid energy (can be negative for feed-in)
        e_o.push(vars.add(variable().min(0.0))); // Overproduction (non-negative)

        // Only create battery variables if bat_value > 0
        if config.bat_value > 0.0 {
            est_battery
                .as_mut()
                .unwrap()
                .push(vars.add(variable().min(0.0))); // Battery storage level (non-negative)
            est_in_battery
                .as_mut()
                .unwrap()
                .push(vars.add(variable().min(0.0))); // Battery input energy (non-negative)
            est_out_battery
                .as_mut()
                .unwrap()
                .push(vars.add(variable().min(0.0))); // Battery output energy (non-negative)
        }

        e_car_charge.push(vars.add(variable().min(0.0))); // Electric car charging energy (non-negative)
    }

    // Build objective function
    let (objective, cap_pv, cap_grid, cst_battery) = generate_objective(
        &config,
        &e_grid,
        cap_pv,
        cap_grid,
        cst_battery,
        &electricity_rate_hourly,
        &e_o,
    );
    // Create model
    let mut model = vars.minimise(objective).using(solver);

    // Calculate electric car parameters if enabled
    let car_daily_energy_required = if config.electric_car_enabled {
        (config.car_daily_km * config.car_efficiency_kwh_per_km * 1000.0) // Convert to Wh
            .min(config.car_battery_size_kwh * 1000.0) // Take minimum with battery capacity
    } else {
        0.0
    };

    // Add fixed constraints (non-time dependent)
    model = add_fixed_constraints(
        model,
        &config,
        pv_cap_w_max,
        cap_pv,
        cst_battery,
        &est_battery,
        &e_car_charge,
        car_daily_energy_required,
    );

    // Add time-dependent constraints
    model = add_time_dependent_constraints(
        model,
        &config,
        &solar_irradiance,
        &scaled_electricity_demand,
        &e_pv,
        &e_grid,
        &e_o,
        &est_battery,
        &est_in_battery,
        &est_out_battery,
        &e_car_charge,
        cap_pv,
        cap_grid,
        cst_battery,
        storage_retention_bat,
        eta_in_bat,
        eta_out_bat_inv,
    );

    // Time the optimization
    let start_time = std::time::Instant::now();
    let opt_result = model.solve();
    let optimization_duration = start_time.elapsed();

    // Solve the optimization
    match opt_result {
        Ok(solution) => Ok(format_solution_results(
            &solution,
            &config,
            &e_pv,
            &e_grid,
            &e_o,
            &est_battery,
            &est_in_battery,
            &est_out_battery,
            &e_car_charge,
            &scaled_electricity_demand,
            cap_pv,
            cap_grid,
            cst_battery,
            car_daily_energy_required,
            optimization_duration,
        )),
        Err(e) => Err(format!("Optimization failed: {:?}", e).into()),
    }
}

/// Run simple optimization with printing and plotting
pub fn run_simple_opt_with_output(
    config: OptimizationConfig,
    pv_cap_w_max: f64,
    solar_irradiance: Vec<f64>,
    electricity_demand: Vec<f64>,
    days_to_plot: Option<&[usize]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = run_simple_opt(
        config.clone(),
        pv_cap_w_max,
        solar_irradiance,
        electricity_demand,
        ElectricityRate::fixed(config.fc_grid),
        good_lp::clarabel,
    )?;

    // Print results
    println!("=== SIMPLE OPTIMIZATION RESULTS ===");
    println!("Config: {:?}", results.config);
    println!("PV Capacity: {:.2} kW", results.pv_capacity_kw);
    println!("Grid Capacity: {:.2} kW", results.grid_capacity_kw);
    println!("Battery Capacity: {:.2} kWh", results.battery_capacity_kwh);
    println!(
        "Annual PV Production: {:.2} kWh",
        results.annual_pv_production_kwh
    );
    println!(
        "Annual Grid Energy: {:.2} kWh",
        results.annual_grid_energy_kwh
    );
    println!(
        "Annual Battery In: {:.2} kWh",
        results.annual_battery_in_kwh
    );
    println!(
        "Annual Battery Out: {:.2} kWh",
        results.annual_battery_out_kwh
    );
    if results.config.electric_car_enabled {
        println!(
            "Annual Car Charging: {:.2} kWh",
            results.annual_car_charging_kwh
        );
        println!(
            "Required Car Energy: {:.2} kWh",
            results.required_car_energy_kwh
        );
    }
    println!(
        "Annual Overproduction: {:.2} kWh",
        results.annual_overproduction_kwh
    );
    println!(
        "Annual Electricity Demand: {:.2} kWh",
        results.annual_electricity_demand_kwh
    );
    println!("PV Coverage: {:.1}%", results.pv_coverage_percent);
    println!("Autarky: {:.1}%", results.autarky);
    println!(
        "Autarky without Battery: {:.1}%",
        results.autarky_without_battery
    );
    println!(
        "Optimization Duration: {} ms",
        results.optimization_duration_ms
    );
    println!("===================================");

    // Create the hourly averages plot
    if let Err(e) = plot_hourly_averages(
        &results.hourly_total_electricity_demand,
        &results.hourly_total_pv_production,
        &results.hourly_grid_consumption,
        &results.hourly_battery_storage,
        "results/hourly_energy_profile.png",
    ) {
        println!("Warning: Failed to create plot: {}", e);
    }

    // Plot individual days if requested
    if let Some(days) = days_to_plot {
        if !days.is_empty() {
            // Create results directory for individual day plots
            if let Err(e) = std::fs::create_dir_all("results/individual_days") {
                println!("Warning: Failed to create individual_days directory: {}", e);
            } else {
                const HOURS_PER_DAY: usize = 24;

                for &day in days {
                    if day >= 365 {
                        println!("Warning: Day {} is out of range (0-364), skipping.", day);
                        continue;
                    }

                    let start_hour = day * HOURS_PER_DAY;
                    let end_hour = (start_hour + HOURS_PER_DAY)
                        .min(results.hourly_total_electricity_demand.len());

                    if start_hour >= results.hourly_total_electricity_demand.len() {
                        println!("Warning: Day {} is out of data range, skipping.", day);
                        continue;
                    }

                    // Extract data for this specific day
                    let day_demand = &results.hourly_total_electricity_demand[start_hour..end_hour];
                    let day_pv = &results.hourly_total_pv_production[start_hour..end_hour];
                    let day_grid = &results.hourly_grid_consumption[start_hour..end_hour];
                    let day_battery = &results.hourly_battery_storage[start_hour..end_hour];

                    // Create filename for this day
                    let filename =
                        format!("results/individual_days/day_{:03}_energy_profile.png", day);

                    // Create custom title for this day
                    let title = format!("Energy Profile - Day {} ({})", day, get_date_string(day));

                    // Call the existing plot function with this day's data and custom title
                    if let Err(e) = plot_hourly_averages_with_title(
                        day_demand,
                        day_pv,
                        day_grid,
                        day_battery,
                        &filename,
                        Some(&title),
                    ) {
                        println!("Warning: Failed to create plot for day {}: {}", day, e);
                    } else {
                        println!("Day {} energy profile plot saved as {}", day, filename);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run simple optimization with specific days to plot
///
/// # Arguments
/// * `config` - Optimization configuration
/// * `days_to_plot` - List of day numbers (0-364) to create individual plots for
///
/// # Example
/// ```rust
/// use solar_system_opt::simple::simple_opt_re::run_simple_opt_with_day_plots;
/// use solar_system_opt::simple::solar_system_utils::OptimizationConfig;
///
/// let config = OptimizationConfig::default();
/// let days = vec![0, 100, 200, 300]; // Plot first day of each season
/// run_simple_opt_with_day_plots(config, &days).unwrap();
/// ```
pub fn run_simple_opt_with_day_plots(
    mut config: OptimizationConfig,
    days_to_plot: &[usize],
) -> Result<(), Box<dyn std::error::Error>> {
    let solar_irradiance = load_solar_radiance_from_csv();
    let (_hot_water_demand, electricity_demand) = load_demand_from_csv();

    config.feed_in_tariff = 0.1;

    // Enable electric car with example parameters
    config.electric_car_enabled = true;
    config.car_daily_km = 10.0;
    config.car_efficiency_kwh_per_km = 0.18;
    config.car_battery_size_kwh = 20.0;
    config.car_charge_during_day = true;
    config.electricity_usage = 5000000.0;

    // Enable heat pump with example parameters
    config.heat_pump_enabled = true;
    config.house_square_meters = 120.0;
    config.insulation_level = InsulationLevel::Moderate;
    config.heating_type = HeatingType::Floor;
    config.monthly_temperatures = [
        20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0,
    ];
    config.inv_heat_pump = 1500.0; // Investment cost per kW

    run_simple_opt_with_output(
        config,
        4.0 * 1000.0,
        solar_irradiance,
        electricity_demand,
        Some(days_to_plot),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_simple_opt() {
        let solar_irradiance = load_solar_radiance_from_csv();
        let electricity_demand = load_demand_from_csv();
        let mut config = OptimizationConfig::default();
        config.feed_in_tariff = 0.0;
        config.fc_grid = 0.15;
        config.electricity_usage = 8000000.0;
        config.bat_value = 100000.0;

        let results = run_simple_opt(
            config.clone(),
            100000.0,
            solar_irradiance,
            electricity_demand.1,
            ElectricityRate::fixed(0.1),
            good_lp::highs,
        )
        .unwrap();
        println!(
            "Simple optimization took: {} ms",
            results.optimization_duration_ms
        );
        println!("Results: {:?}", results.annual_overproduction_kwh);
        assert!(
            results.annual_grid_energy_kwh + results.annual_pv_production_kwh
                - results.annual_overproduction_kwh
                - config.electricity_usage
                < 100.0
        );
        assert_eq!(results.pv_capacity_kw, 1.8513584545416046);
        assert_eq!(results.battery_capacity_kwh, 0.0);
    }
}
