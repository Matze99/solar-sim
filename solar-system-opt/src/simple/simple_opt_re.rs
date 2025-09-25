use ems_model::building::electricity::ElectricityRate;
use good_lp::variables;
use good_lp::{Expression, Solution, SolverModel, constraint, variable};

use crate::general::electricity_demand::create_scaled_load_curve_from_csv;
use crate::simple::plot::{plot_hourly_averages, plot_hourly_averages_with_title};
use crate::simple::solar_system_utils::{
    HeatingType, InsulationLevel, OptimizationConfig, SimpleOptimizationResults,
    calculate_heat_demand, calculate_heat_demand_with_insulation,
    calculate_heat_pump_electricity_consumption, load_demand_from_csv,
    load_solar_radiance_from_csv,
};
use ems_model::building::insulation::{BuildingTypeEnum, YearCategoryESEnum};

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

pub fn run_simple_opt(
    config: OptimizationConfig,
    pv_cap_w_max: f64,
    solar_irradiance: Vec<f64>,
    electricity_demand: Vec<f64>,
    electricity_rate: ElectricityRate,
) -> Result<SimpleOptimizationResults, Box<dyn std::error::Error>> {
    // Use monthly demand to generate scaled load curve if available, otherwise use provided electricity_demand
    let scaled_electricity_demand: Vec<f64> =
        if let Some(ref monthly_demand) = config.monthly_demand {
            // Generate scaled load curve using monthly demand and base CSV data
            create_scaled_load_curve_from_csv(monthly_demand, "data/demand.csv")?
                .iter()
                .map(|&demand| demand * 1000.0) // Convert from kWh to Wh to match existing scaling
                .collect()
        } else {
            // Use the provided electricity_demand and scale by desired annual usage
            electricity_demand
                .iter()
                .map(|&demand| demand * (config.electricity_usage / 4173440.0))
                .collect()
        };
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
            cap_heat_pump;
    }

    // energy usage of own production
    let mut e_pv: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    // energy usage of grid
    let mut e_grid: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    // energy overproduction
    let mut e_o: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS); // overproduction
    // battery storage variables
    let mut est_battery: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    let mut est_in_battery: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    let mut est_out_battery: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    // electric car charging variables
    let mut e_car_charge: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);
    // heat pump variables
    let mut e_heat_pump: Vec<good_lp::Variable> = Vec::with_capacity(NUM_HOURS);

    // Load heat pump data if enabled
    let heat_demand = if config.heat_pump_enabled {
        // Use insulation-based calculation with when2heat data
        match calculate_heat_demand_with_insulation(
            config.house_square_meters,
            config.building_type,
            config.construction_period,
            config.insulation_standard,
        ) {
            Ok(insulation_heat_demand) => {
                println!("Using insulation-based heating demand calculation");
                insulation_heat_demand
                    .iter()
                    .map(|&heat| heat * 10.0)
                    .collect() // TODO: remove this. This is a hack to make it work for now
            }
            Err(e) => {
                println!(
                    "Warning: Failed to load insulation-based heat demand: {}. Falling back to temperature-based calculation.",
                    e
                );
                calculate_heat_demand(
                    config.house_square_meters,
                    &config.insulation_level,
                    &config.monthly_temperatures,
                )
            }
        }
    } else {
        vec![0.0; NUM_HOURS]
    };

    // Calculate heat pump electricity consumption using COP values if heat pump is enabled
    let heat_pump_electricity_consumption = if config.heat_pump_enabled {
        match calculate_heat_pump_electricity_consumption(&heat_demand, &config.heating_type) {
            Ok(electricity) => {
                println!("Using COP-based heat pump electricity consumption calculation");
                electricity
            }
            Err(e) => {
                println!(
                    "Warning: Failed to calculate COP-based electricity consumption: {}. Using default COP of 3.0.",
                    e
                );
                heat_demand.iter().map(|&heat| heat / 3.0).collect()
            }
        }
    } else {
        vec![0.0; NUM_HOURS]
    };

    // Create variables for each hour
    for _t in 0..NUM_HOURS {
        e_pv.push(vars.add(variable().min(0.0))); // PV energy (non-negative)
        e_grid.push(vars.add(variable().min(0.0))); // Grid energy (can be negative for feed-in)
        e_o.push(vars.add(variable().min(0.0))); // Overproduction (non-negative)
        est_battery.push(vars.add(variable().min(0.0))); // Battery storage level (non-negative)
        est_in_battery.push(vars.add(variable().min(0.0))); // Battery input energy (non-negative)
        est_out_battery.push(vars.add(variable().min(0.0))); // Battery output energy (non-negative)
        e_car_charge.push(vars.add(variable().min(0.0))); // Electric car charging energy (non-negative)
        e_heat_pump.push(vars.add(variable().min(0.0))); // Heat pump energy consumption (non-negative)
    }

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
    objective += cst_battery / 1000.0 * config.inv_bat * config.annuity;
    objective += cap_heat_pump / 1000.0 * config.inv_heat_pump * config.annuity;

    // Operating costs and revenues (time-dependent)
    for t in 0..NUM_HOURS {
        objective += e_grid[t] / 1000.0 * electricity_rate_hourly[t]; // Cost of grid electricity
        objective -= e_o[t] / 1000.0 * config.feed_in_tariff; // Revenue from feed-in
    }

    // Create model
    let mut model = vars.minimise(objective).using(good_lp::clarabel);

    // Fixed capacity constraints
    if config.pv_fixed {
        model = model.with(constraint!(cap_pv == pv_cap_w_max));
    } else {
        model = model.with(constraint!(cap_pv >= 0.0));
        model = model.with(constraint!(cap_pv <= pv_cap_w_max));
    }
    // Battery capacity constraints
    if config.bat_fixed {
        model = model.with(constraint!(cst_battery == config.bat_value));
    } else {
        model = model.with(constraint!(cst_battery >= 0.0));
        model = model.with(constraint!(cst_battery <= config.bat_value));
    }
    model = model.with(constraint!(cap_heat_pump >= 0.0));

    // Battery initialization constraint
    model = model.with(constraint!(est_battery[0] == 0.0));

    // Calculate electric car parameters if enabled
    let car_daily_energy_required = if config.electric_car_enabled {
        (config.car_daily_km * config.car_efficiency_kwh_per_km * 1000.0) // Convert to Wh
            .min(config.car_battery_size_kwh * 1000.0) // Take minimum with battery capacity
    } else {
        0.0
    };

    // Time-dependent constraints
    for t in 0..NUM_HOURS {
        let solar_t = solar_irradiance[t];
        let elec_demand_t = scaled_electricity_demand[t];

        // Energy balance: PV + Grid + Battery Out = Demand + Battery In + Car Charging + Heat Pump
        model = model.with(constraint!(
            e_pv[t] + e_grid[t] - elec_demand_t - est_in_battery[t] + est_out_battery[t]
                - e_car_charge[t]
                - e_heat_pump[t]
                == 0.0
        ));

        // Overproduction constraint: overproduction = potential PV - actual PV
        model = model.with(constraint!(e_o[t] - cap_pv * solar_t + e_pv[t] == 0.0));

        // PV capacity limit: actual PV <= potential PV
        model = model.with(constraint!(cap_pv * solar_t - e_pv[t] >= 0.0));

        // Grid capacity limit
        model = model.with(constraint!(cap_grid - e_grid[t] >= 0.0));

        // Battery capacity limit
        model = model.with(constraint!(cst_battery - est_battery[t] >= 0.0));

        // Heat pump constraints
        if config.heat_pump_enabled {
            // Heat pump capacity limit
            model = model.with(constraint!(cap_heat_pump - e_heat_pump[t] >= 0.0));

            // Use pre-calculated electricity consumption based on COP
            model = model.with(constraint!(
                e_heat_pump[t] == heat_pump_electricity_consumption[t]
            ));
        } else {
            // If heat pump is disabled, set consumption to zero
            model = model.with(constraint!(e_heat_pump[t] == 0.0));
        }

        // C-rate constraints
        model = model.with(constraint!(
            config.c_rate_limit * cst_battery - est_in_battery[t] >= 0.0
        ));
        model = model.with(constraint!(
            config.c_rate_limit * cst_battery - est_out_battery[t] >= 0.0
        ));

        // Storage balance constraints (t >= 1)
        if t > 0 {
            model = model.with(constraint!(
                est_battery[t]
                    - est_battery[t - 1] * storage_retention_bat
                    - eta_in_bat * est_in_battery[t]
                    + est_out_battery[t] * eta_out_bat_inv
                    == 0.0
            ));
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

    // Electric car total energy constraint
    if config.electric_car_enabled {
        // Sum of all charging must equal required daily energy * 365 days
        let total_car_charging: Expression =
            e_car_charge.iter().map(|&var| Expression::from(var)).sum();
        model = model.with(constraint!(
            total_car_charging == car_daily_energy_required * 365.0
        ));
    }

    // Solve the optimization
    match model.solve() {
        Ok(solution) => {
            // Calculate and print results
            let pv_sum: f64 = e_pv.iter().map(|&var| solution.value(var)).sum();
            let grid_sum: f64 = e_grid.iter().map(|&var| solution.value(var)).sum();
            let overproduction: f64 = e_o.iter().map(|&var| solution.value(var)).sum();
            let total_demand: f64 = scaled_electricity_demand.iter().sum();
            let battery_in_sum: f64 = est_in_battery.iter().map(|&var| solution.value(var)).sum();
            let battery_out_sum: f64 = est_out_battery.iter().map(|&var| solution.value(var)).sum();
            let car_charging_sum: f64 = e_car_charge.iter().map(|&var| solution.value(var)).sum();
            let heat_pump_sum: f64 = e_heat_pump.iter().map(|&var| solution.value(var)).sum();
            let heat_demand_sum: f64 = heat_demand.iter().sum();

            // Collect hourly data for struct
            let pv_production: Vec<f64> = e_pv.iter().map(|&var| solution.value(var)).collect();
            let overproduction_hourly: Vec<f64> =
                e_o.iter().map(|&var| solution.value(var)).collect();
            let grid_consumption: Vec<f64> =
                e_grid.iter().map(|&var| solution.value(var)).collect();
            let battery_storage: Vec<f64> =
                est_battery.iter().map(|&var| solution.value(var)).collect();
            let car_charging_hourly: Vec<f64> = e_car_charge
                .iter()
                .map(|&var| solution.value(var))
                .collect();
            let heat_pump_consumption_hourly: Vec<f64> =
                e_heat_pump.iter().map(|&var| solution.value(var)).collect();

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

            // Calculate heat pump capacity from maximum electricity consumption
            let heat_pump_capacity_kw = if config.heat_pump_enabled {
                heat_pump_electricity_consumption
                    .iter()
                    .fold(0.0_f64, |max, &val| max.max(val))
                    / 1000.0
            } else {
                0.0
            };

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

            Ok(SimpleOptimizationResults {
                pv_capacity_kw: solution.value(cap_pv) / 1000.0,
                grid_capacity_kw: solution.value(cap_grid) / 1000.0,
                battery_capacity_kwh: solution.value(cst_battery) / 1000.0,
                heat_pump_capacity_kw,
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
                annual_heat_pump_energy_kwh: heat_pump_sum / 1000.0,
                annual_heat_demand_kwh: heat_demand_sum / 1000.0,
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
                hourly_electricity_demand_base: scaled_electricity_demand,
                hourly_heat_pump_consumption: heat_pump_consumption_hourly,
                hourly_heat_demand: heat_demand,
                config: config.clone(),
            })
        }
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
    )?;

    // Print results
    println!("=== SIMPLE OPTIMIZATION RESULTS ===");
    println!("Config: {:?}", results.config);
    println!("PV Capacity: {:.2} kW", results.pv_capacity_kw);
    println!("Grid Capacity: {:.2} kW", results.grid_capacity_kw);
    println!("Battery Capacity: {:.2} kWh", results.battery_capacity_kwh);
    if results.config.heat_pump_enabled {
        println!(
            "Heat Pump Capacity: {:.2} kW",
            results.heat_pump_capacity_kw
        );
    }
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
    if results.config.heat_pump_enabled {
        println!(
            "Annual Heat Pump Energy: {:.2} kWh",
            results.annual_heat_pump_energy_kwh
        );
        println!(
            "Annual Heat Demand: {:.2} kWh",
            results.annual_heat_demand_kwh
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

pub fn run_simple_opt_loop(
    mut config: OptimizationConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let solar_irradiance = load_solar_radiance_from_csv();
    let (_hot_water_demand, electricity_demand) = load_demand_from_csv();

    config.feed_in_tariff = 0.0;
    config.fc_grid = 0.30; // Realistic grid electricity cost (30 cents per kWh)
    config.bat_value = 100000.0;
    config.bat_fixed = false;
    config.pv_capacity_max = 100000.0;
    config.pv_fixed = false;

    // Enable electric car with example parameters
    config.electric_car_enabled = false;
    config.car_daily_km = 50.0;
    config.car_efficiency_kwh_per_km = 15.0;
    config.car_battery_size_kwh = 20.0;
    config.car_charge_during_day = true;
    config.electricity_usage = 5500000.0;

    // Enable heat pump with example parameters
    config.heat_pump_enabled = false;
    config.house_square_meters = 120.0;
    config.insulation_level = InsulationLevel::Moderate;
    config.monthly_temperatures = [
        20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0,
    ];
    config.inv_heat_pump = 1500.0; // Investment cost per kW

    // Set building configuration parameters
    config.building_type = BuildingTypeEnum::SingleFamily;
    config.construction_period = YearCategoryESEnum::Between1980and2006;
    config.insulation_standard = InsulationLevel::Moderate;
    config.optimize_for_autonomy = false;

    run_simple_opt_with_output(
        config.clone(),
        config.pv_capacity_max,
        solar_irradiance,
        electricity_demand,
        None,
    )?;

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
