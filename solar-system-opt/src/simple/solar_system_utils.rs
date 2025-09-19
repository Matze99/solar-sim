use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use ems_model::building::insulation::{
    BuildingTypeEnum, YearCategoryESEnum, YearCategoryESMapping,
};

use crate::general::electricity_demand::MonthlyDemand;

/// Configuration struct holding all optimization parameters
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    // Investment costs
    pub inv_pv: f64,        // Investment cost for PV per kW
    pub inv_bat: f64,       // Investment cost for battery per kWh
    pub inv_hwat: f64,      // Investment cost for hot water storage per kWh
    pub inv_grid: f64,      // Investment cost for grid connection per kW
    pub inv_heat_pump: f64, // Investment cost for heat pump per kW

    // Economic parameters
    pub annuity: f64,        // Annuity factor
    pub fc_grid: f64,        // Fuel cost for grid electricity per kWh
    pub feed_in_tariff: f64, // Feed-in tariff per kWh

    // System parameters
    pub hwat_enabled: bool,                    // Flag for hot water system
    pub storage_loss_bat: f64,                 // Battery hourly loss rate
    pub storage_loss_hwat: f64,                // Hot water storage hourly loss rate
    pub eta_in_bat: f64,                       // Battery charging efficiency
    pub eta_out_bat: f64,                      // Battery discharging efficiency
    pub eta_in_hwat: f64,                      // Hot water storage efficiency
    pub eta_out_hwat: f64,                     // Hot water discharge efficiency
    pub c_rate_limit: f64, // C-rate limit for battery (fraction of capacity per hour)
    pub electricity_usage: f64, // Annual electricity usage in kWh (normalizes timeseries to this total)
    pub monthly_demand: Option<MonthlyDemand>, // Monthly demand in kWh

    // Electric car parameters
    pub electric_car_enabled: bool,     // Flag for electric car
    pub car_daily_km: f64,              // Daily kilometers driven
    pub car_efficiency_kwh_per_km: f64, // Car efficiency in kWh per km
    pub car_battery_size_kwh: f64,      // Car battery size in kWh
    pub car_charge_during_day: bool,    // Whether car charges during day (true) or night (false)

    // Heat pump parameters
    pub heat_pump_enabled: bool,           // Flag for heat pump system
    pub house_square_meters: f64,          // House size in square meters
    pub insulation_level: InsulationLevel, // Insulation quality
    pub heating_type: HeatingType,         // Floor or radiator heating
    pub monthly_temperatures: [f64; 12],   // Desired temperature for each month (°C)

    // Building configuration parameters
    pub building_type: BuildingTypeEnum, // Building type (SingleFamily, Terraced, MultiFamily, Apartment)
    pub construction_period: YearCategoryESEnum, // Construction period (Before1900, Between1901and1936, etc.)
    pub insulation_standard: InsulationLevel,    // Insulation standard (Poor, Moderate, Good)

    // Optimization loop parameters
    pub bat_value: f64,        // Fixed battery capacity for optimization loop
    pub pv_capacity_min: f64,  // Minimum PV capacity to test
    pub pv_capacity_max: f64,  // Maximum PV capacity to test
    pub pv_capacity_step: f64, // Step size for PV capacity testing
    pub pv_fixed: bool,        // if true, pv capacity cannot be changed by optimization
    pub bat_fixed: bool,       // if true, battery capacity cannot be changed by optimization
    pub electricity_price_increase: f64, // Electricity price increase per year

    // Optimization mode
    pub optimize_for_autonomy: bool, // if true, optimize for maximum autonomy instead of minimum cost
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            inv_pv: 465.0,
            inv_bat: 200.0,
            inv_hwat: 60.0,
            inv_grid: 0.0,
            inv_heat_pump: 0.0,

            // Economic parameters
            annuity: 0.1,
            fc_grid: 0.30,
            feed_in_tariff: 0.079,

            // System parameters
            hwat_enabled: true,
            storage_loss_bat: 0.001,
            storage_loss_hwat: 0.01,
            eta_in_bat: 0.95,
            eta_out_bat: 0.95,
            eta_in_hwat: 0.90,
            eta_out_hwat: 0.90,
            c_rate_limit: 0.3,
            electricity_usage: 4173440.0, // Default: normalized annual electricity usage in Wh
            monthly_demand: None,

            // Electric car parameters
            electric_car_enabled: false,
            car_daily_km: 50.0,             // 50 km per day default
            car_efficiency_kwh_per_km: 0.2, // 0.2 kWh per km default
            car_battery_size_kwh: 50.0,     // 50 kWh battery default
            car_charge_during_day: true,    // Default to daytime charging

            // Heat pump parameters
            heat_pump_enabled: false,
            house_square_meters: 100.0,
            insulation_level: InsulationLevel::Moderate,
            heating_type: HeatingType::Floor,
            monthly_temperatures: [20.0; 12],

            // Building configuration parameters
            building_type: BuildingTypeEnum::SingleFamily,
            construction_period: YearCategoryESEnum::Before1900,
            insulation_standard: InsulationLevel::Moderate,

            // Optimization loop parameters
            bat_value: 20000.0,
            pv_capacity_min: 0.0,
            pv_capacity_max: 2.0,
            pv_capacity_step: 0.5,
            pv_fixed: false,
            bat_fixed: false,
            electricity_price_increase: 0.0,

            // Optimization mode
            optimize_for_autonomy: false,
        }
    }
}

// PERFORMANCE OPTIMIZATION: Cache loaded data to avoid repeated file I/O
static SOLAR_DATA_CACHE: LazyLock<Mutex<Option<Vec<f64>>>> = LazyLock::new(|| Mutex::new(None));
static DEMAND_DATA_CACHE: LazyLock<Mutex<Option<(Vec<f64>, Vec<f64>)>>> =
    LazyLock::new(|| Mutex::new(None));
static COP_DATA_CACHE: LazyLock<Mutex<Option<Vec<f64>>>> = LazyLock::new(|| Mutex::new(None));

/// Load solar radiance time series from CSV file with caching
/// Returns a vector of 8760 hourly solar radiance values
/// Falls back to default values if file cannot be read
pub fn load_solar_radiance_from_csv() -> Vec<f64> {
    // Check cache first
    {
        let cache = SOLAR_DATA_CACHE.lock().unwrap();
        if let Some(ref cached_data) = *cache {
            return cached_data.clone();
        }
    }

    // Load from file if not cached
    let csv_path = "data/ts_res.csv";
    let data = match load_csv_data(csv_path) {
        Ok(data) => {
            if data.len() >= 8760 {
                println!(
                    "Successfully loaded {} solar radiance values from {}",
                    data.len(),
                    csv_path
                );
                data[..8760].to_vec() // Take first 8760 hours for annual simulation
            } else {
                println!(
                    "Warning: CSV file has only {} values, expected 8760. Using default values.",
                    data.len()
                );
                get_default_solar_radiance()
            }
        }
        Err(e) => {
            println!(
                "Warning: Could not load solar radiance from {}: {}. Using default values.",
                csv_path, e
            );
            get_default_solar_radiance()
        }
    };

    // Cache the data
    {
        let mut cache = SOLAR_DATA_CACHE.lock().unwrap();
        *cache = Some(data.clone());
    }

    data
}

/// Load solar radiance data from CSV file
pub fn load_csv_data(file_path: &str) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut solar_data = Vec::new();

    // Skip header line and read data
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;

        // Skip header line
        if line_num == 0 {
            continue;
        }

        // Parse CSV line: "Time,Solar"
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 2 {
            if let Ok(solar_value) = parts[1].trim().parse::<f64>() {
                solar_data.push(solar_value);
            } else {
                return Err(format!(
                    "Could not parse solar value on line {}: '{}'",
                    line_num + 1,
                    parts[1]
                )
                .into());
            }
        } else {
            return Err(format!("Invalid CSV format on line {}: '{}'", line_num + 1, line).into());
        }
    }

    Ok(solar_data)
}

/// Get default solar radiance values (fallback)
pub fn get_default_solar_radiance() -> Vec<f64> {
    vec![0.5; 8760] // Normalized solar irradiance for each hour
}

/// Load demand data from CSV file with caching
/// Returns a tuple of (hot_water_demand, electricity_demand) vectors
/// Falls back to default values if file cannot be read
pub fn load_demand_from_csv() -> (Vec<f64>, Vec<f64>) {
    // Check cache first
    {
        let cache = DEMAND_DATA_CACHE.lock().unwrap();
        if let Some(ref cached_data) = *cache {
            return cached_data.clone();
        }
    }

    // Load from file if not cached
    let csv_path = "data/demand.csv";
    let data = match load_demand_csv_data(csv_path) {
        Ok((hot_water, electricity)) => {
            if hot_water.len() >= 8760 && electricity.len() >= 8760 {
                println!(
                    "Successfully loaded {} demand values from {}",
                    hot_water.len(),
                    csv_path
                );
                (hot_water[..8760].to_vec(), electricity[..8760].to_vec()) // Take first 8760 hours for annual simulation
            } else {
                println!(
                    "Warning: CSV file has only {} values, expected 8760. Using default values.",
                    hot_water.len().min(electricity.len())
                );
                get_default_demand()
            }
        }
        Err(e) => {
            println!(
                "Warning: Could not load demand data from {}: {}. Using default values.",
                csv_path, e
            );
            get_default_demand()
        }
    };

    // Cache the data
    {
        let mut cache = DEMAND_DATA_CACHE.lock().unwrap();
        *cache = Some(data.clone());
    }

    data
}

/// Load demand data from CSV file
/// Expected format: Time,Hot Water,Space Heat,Electricity,Charge
pub fn load_demand_csv_data(
    file_path: &str,
) -> Result<(Vec<f64>, Vec<f64>), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut hot_water_data = Vec::new();
    let mut electricity_data = Vec::new();

    // Skip header line and read data
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;

        // Skip header line
        if line_num == 0 {
            continue;
        }

        // Parse CSV line: "Time,Hot Water,Space Heat,Electricity,Charge"
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 5 {
            // Parse Hot Water (column 1)
            let hot_water_value = parts[1].trim().parse::<f64>().map_err(|_| {
                format!(
                    "Could not parse hot water value on line {}: '{}'",
                    line_num + 1,
                    parts[1]
                )
            })?;

            // Parse Electricity (column 3)
            let electricity_value = parts[3].trim().parse::<f64>().map_err(|_| {
                format!(
                    "Could not parse electricity value on line {}: '{}'",
                    line_num + 1,
                    parts[3]
                )
            })?; // Convert to kWh

            hot_water_data.push(hot_water_value);
            electricity_data.push(electricity_value);
        } else {
            return Err(format!(
                "Invalid CSV format on line {}: '{}'. Expected 5 columns.",
                line_num + 1,
                line
            )
            .into());
        }
    }

    Ok((hot_water_data, electricity_data))
}

/// Get default demand values (fallback)
fn get_default_demand() -> (Vec<f64>, Vec<f64>) {
    (vec![1.0; 8760], vec![2.0; 8760]) // (hot_water_demand, electricity_demand)
}

// Struct to hold optimization results for plotting
#[derive(Debug, Clone)]
pub struct OptimizationResults {
    pub pv_energy: Vec<f64>,
    pub grid_energy: Vec<f64>,
    pub hot_water_energy: Vec<f64>,
    pub energy_overproduction: Vec<f64>,
    pub charging_energy: Vec<f64>,
    pub battery_storage: Vec<f64>,
    pub hot_water_storage: Vec<f64>,
    pub battery_in: Vec<f64>,
    pub battery_out: Vec<f64>,
    pub hot_water_in: Vec<f64>,
    pub hot_water_out: Vec<f64>,
    pub total_cost: f64,
    pub pv_capacity: f64,
    pub battery_capacity: f64,
    pub hot_water_capacity: f64,
    pub grid_capacity: f64,
}

/// Struct to hold simple optimization results for printing and plotting
#[derive(Debug, Clone, Default)]
pub struct SimpleOptimizationResults {
    // Capacities
    pub pv_capacity_kw: f64,
    pub grid_capacity_kw: f64,
    pub battery_capacity_kwh: f64,
    pub heat_pump_capacity_kw: f64,

    // Annual totals
    pub annual_pv_production_kwh: f64,
    pub annual_grid_energy_kwh: f64,
    pub annual_battery_in_kwh: f64,
    pub annual_battery_out_kwh: f64,
    pub annual_car_charging_kwh: f64,
    pub annual_overproduction_kwh: f64,
    pub annual_electricity_demand_kwh: f64,
    pub required_car_energy_kwh: f64,
    pub annual_heat_pump_energy_kwh: f64,
    pub annual_heat_demand_kwh: f64,

    // Coverage metrics
    pub pv_coverage_percent: f64,
    pub autarky: f64,
    pub autarky_without_battery: f64,

    // Hourly data for plotting
    pub hourly_pv_production: Vec<f64>,
    pub hourly_overproduction: Vec<f64>,
    pub hourly_grid_consumption: Vec<f64>,
    pub hourly_battery_storage: Vec<f64>,
    pub hourly_car_charging: Vec<f64>,
    pub hourly_total_pv_production: Vec<f64>,
    pub hourly_total_electricity_demand: Vec<f64>,
    pub hourly_electricity_demand_base: Vec<f64>,
    pub hourly_heat_pump_consumption: Vec<f64>,
    pub hourly_heat_demand: Vec<f64>,

    // Configuration used
    pub config: OptimizationConfig,
}

#[derive(Debug, Clone, Copy)]
pub enum InsulationLevel {
    Poor,
    Moderate,
    Good,
}

#[derive(Debug, Clone, Default)]
pub enum HeatingType {
    Floor,
    #[default]
    Radiator,
}

/// Load COP data from when2heat_processed_2022.csv file
pub fn load_cop_data_from_csv(
    heating_type: &HeatingType,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    // Check cache first
    {
        let cache = COP_DATA_CACHE.lock().unwrap();
        if let Some(cached_data) = cache.as_ref() {
            return Ok(cached_data.clone());
        }
    }

    let file_path = Path::new("data/when2heat_processed_2022.csv");
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Read header to find the correct column
    let header = lines.next().ok_or("Empty file")??;
    let columns: Vec<&str> = header.split(',').collect();

    let cop_column = match heating_type {
        HeatingType::Floor => columns.iter().position(|&col| col == "ES_COP_ASHP_floor"),
        HeatingType::Radiator => columns
            .iter()
            .position(|&col| col == "ES_COP_ASHP_radiator"),
    }
    .ok_or("COP column not found")?;

    let mut cop_data = Vec::new();

    for line in lines {
        let line = line?;
        let values: Vec<&str> = line.split(',').collect();

        if values.len() > cop_column {
            // Handle comma-separated decimal values (e.g., "3,67" -> 3.67)
            let cop_str = values[cop_column].trim_matches('"');
            let cop_value = cop_str.replace(',', ".").parse::<f64>()?;
            cop_data.push(cop_value);
        }
    }

    // Cache the result
    {
        let mut cache = COP_DATA_CACHE.lock().unwrap();
        *cache = Some(cop_data.clone());
    }

    Ok(cop_data)
}

/// Load heat demand profile from when2heat_processed_2022.csv file
pub fn load_heat_demand_profile_from_csv(
    building_type: &str,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let file_path = Path::new("data/when2heat_processed_2022.csv");
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Read header to find the correct column
    let header = lines.next().ok_or("Empty file")??;
    let columns: Vec<&str> = header.split(',').collect();

    let heat_demand_column = match building_type {
        "SFH" => columns.iter().position(|&col| col == "ES_heat_demand_space_SFH"),
        "MFH" => columns.iter().position(|&col| col == "ES_heat_demand_space_MFH"),
        _ => return Err("Invalid building type. Use 'SFH' for single family homes or 'MFH' for multi family homes.".into()),
    }
    .ok_or("Heat demand column not found")?;

    let mut heat_demand_data = Vec::new();

    for line in lines {
        let line = line?;
        let values: Vec<&str> = line.split(',').collect();

        if values.len() > heat_demand_column {
            let heat_demand_str = values[heat_demand_column].trim_matches('"');
            let heat_demand_value = heat_demand_str.replace(',', ".").parse::<f64>()?;
            heat_demand_data.push(heat_demand_value);
        }
    }

    Ok(heat_demand_data)
}

/// Get annual heating demand per m² based on building characteristics
pub fn get_annual_heating_demand_per_m2(
    building_type: BuildingTypeEnum,
    construction_period: YearCategoryESEnum,
    insulation_standard: InsulationLevel,
) -> Result<f64, Box<dyn std::error::Error>> {
    // Get the default mapping with all the insulation data
    let year_category_mapping = YearCategoryESMapping::default();

    // Get the building type mapping for the given construction period
    let building_type_mapping = year_category_mapping
        .get(construction_period)
        .ok_or("Construction period not found in mapping")?;

    // Get the heating need for the given building type
    let heating_need = building_type_mapping
        .get(building_type)
        .ok_or("Building type not found in mapping")?;

    // Return the appropriate heating need based on insulation standard
    let annual_demand = match insulation_standard {
        InsulationLevel::Poor => heating_need.national_minimum_requirement,
        InsulationLevel::Moderate => heating_need.improved_standard,
        InsulationLevel::Good => heating_need.ambitious_standard,
    };

    Ok(annual_demand)
}

/// Calculate hourly heat demand using insulation values and when2heat data
pub fn calculate_heat_demand_with_insulation(
    house_square_meters: f64,
    building_type: BuildingTypeEnum,
    construction_period: YearCategoryESEnum,
    insulation_standard: InsulationLevel,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    // Get annual heating demand per m²
    let annual_demand_per_m2 =
        get_annual_heating_demand_per_m2(building_type, construction_period, insulation_standard)?;

    // Calculate total annual demand for the house
    let total_annual_demand = annual_demand_per_m2 * house_square_meters * 1000.0; // Wh/year

    // Load hourly heat demand profile from when2heat data
    let heat_demand_column = match building_type {
        BuildingTypeEnum::SingleFamily | BuildingTypeEnum::Terraced => "SFH",
        BuildingTypeEnum::MultiFamily | BuildingTypeEnum::Apartment => "MFH",
        _ => return Err("Invalid building type".into()),
    };

    let hourly_profile = load_heat_demand_profile_from_csv(heat_demand_column)?;

    // Normalize the profile to sum to 1.0
    let profile_sum: f64 = hourly_profile.iter().sum();
    if profile_sum == 0.0 {
        return Err("Heat demand profile sum is zero".into());
    }

    let normalized_profile: Vec<f64> = hourly_profile
        .iter()
        .map(|&value| value / profile_sum)
        .collect();

    // Scale the normalized profile by the total annual demand
    let hourly_heat_demand: Vec<f64> = normalized_profile
        .iter()
        .map(|&value| value * total_annual_demand)
        .collect();

    Ok(hourly_heat_demand)
}

/// Calculate heat pump electricity consumption using COP values
pub fn calculate_heat_pump_electricity_consumption(
    heat_demand: &[f64],
    heating_type: &HeatingType,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    // Load COP data
    let cop_data = load_cop_data_from_csv(heating_type)?;

    if cop_data.len() != heat_demand.len() {
        return Err(format!(
            "COP data length ({}) doesn't match heat demand length ({})",
            cop_data.len(),
            heat_demand.len()
        )
        .into());
    }

    // Calculate electricity consumption: heat_demand / cop
    let electricity_consumption: Vec<f64> = heat_demand
        .iter()
        .zip(cop_data.iter())
        .map(|(&heat, &cop)| {
            if cop > 0.0 {
                heat / cop
            } else {
                0.0 // Avoid division by zero
            }
        })
        .collect();

    Ok(electricity_consumption)
}

/// Calculate hourly heat demand based on house characteristics and desired temperatures
pub fn calculate_heat_demand(
    house_square_meters: f64,
    insulation_level: &InsulationLevel,
    monthly_temperatures: &[f64; 12],
) -> Vec<f64> {
    // Base heat loss coefficient (W/m²K) based on insulation level
    let heat_loss_coefficient = match insulation_level {
        InsulationLevel::Poor => 2.5,     // Poor insulation
        InsulationLevel::Moderate => 1.8, // Moderate insulation
        InsulationLevel::Good => 1.2,     // Good insulation
    };

    // Outdoor temperature profile for Spain (simplified monthly averages)
    // These are approximate monthly average temperatures for Spain
    let outdoor_temperatures = [
        8.0,  // January
        9.0,  // February
        12.0, // March
        14.0, // April
        18.0, // May
        22.0, // June
        25.0, // July
        25.0, // August
        22.0, // September
        17.0, // October
        12.0, // November
        9.0,  // December
    ];

    // Hours per month (approximate)
    let hours_per_month = [
        744, // January (31 days)
        672, // February (28 days)
        744, // March (31 days)
        720, // April (30 days)
        744, // May (31 days)
        720, // June (30 days)
        744, // July (31 days)
        744, // August (31 days)
        720, // September (30 days)
        744, // October (31 days)
        720, // November (30 days)
        744, // December (31 days)
    ];

    let mut heat_demand = Vec::new();

    for month in 0..12 {
        let monthly_hours = hours_per_month[month];
        let outdoor_temp = outdoor_temperatures[month];
        let desired_temp = monthly_temperatures[month];

        // Calculate temperature difference
        let temp_diff = desired_temp - outdoor_temp;

        // Calculate heat demand for each hour in this month
        for _ in 0..monthly_hours {
            if temp_diff > 0.0 {
                // Heating needed
                let heat_power = heat_loss_coefficient * house_square_meters * temp_diff; // W
                let heat_energy = heat_power / 1000.0; // kWh
                heat_demand.push(heat_energy);
            } else {
                // No heating needed
                heat_demand.push(0.0);
            }
        }
    }

    heat_demand
}
