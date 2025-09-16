# Solar Energy System Optimization

A high-performance Rust implementation for optimizing solar energy systems using
linear programming. This tool helps design and optimize residential and
commercial solar energy systems with battery storage, heat pumps, electric
vehicle charging, and grid integration.

## Features

### Core Optimization Capabilities

- **Photovoltaic (PV) Systems** - Solar panel capacity optimization
- **Battery Storage** - Energy storage with configurable efficiency and losses
- **Grid Integration** - Electricity import/export with feed-in tariffs
- **Heat Pump Systems** - Space heating optimization with COP calculations
- **Electric Vehicle Charging** - Flexible and fixed charging schedules
- **Hot Water Storage** - Thermal energy storage optimization

### Advanced Features

- **Insulation-based Heating** - Uses building characteristics for accurate heat
  demand
- **Monthly Demand Scaling** - Customize electricity demand patterns
- **Multiple Optimization Modes** - Cost minimization or autonomy maximization
- **Comprehensive Plotting** - Visual analysis of optimization results
- **Caching System** - Efficient data loading and processing

## Quick Start

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd p2p-ems/solar-system-opt

# Build the project
cargo build --release

# Run with default configuration
cargo run

# Run with individual day plots
cargo run days
```

### Basic Usage

```rust
use solar_system_opt::run_simple_opt;
use solar_system_opt::simple::solar_system_utils::OptimizationConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create default configuration
    let config = OptimizationConfig::default();
    
    // Load data
    let solar_irradiance = load_solar_radiance_from_csv();
    let electricity_demand = load_demand_from_csv();
    
    // Run optimization
    let results = run_simple_opt(
        config,
        8000.0,  // Max PV capacity in W
        solar_irradiance,
        electricity_demand,
    )?;
    
    println!("Optimization completed!");
    println!("Total cost: {:.2} €", results.total_cost);
    println!("PV capacity: {:.2} W", results.pv_capacity);
    println!("Battery capacity: {:.2} Wh", results.battery_capacity);
    
    Ok(())
}
```

## Core Function: `run_simple_opt`

The main optimization function that performs linear programming optimization of
solar energy systems.

### Function Signature

```rust
pub fn run_simple_opt(
    config: OptimizationConfig,
    pv_cap_w_max: f64,
    solar_irradiance: Vec<f64>,
    electricity_demand: Vec<f64>,
) -> Result<SimpleOptimizationResults, Box<dyn std::error::Error>>
```

### Parameters

- **`config`**: `OptimizationConfig` - Complete system configuration (see
  Configuration section)
- **`pv_cap_w_max`**: `f64` - Maximum PV capacity in watts
- **`solar_irradiance`**: `Vec<f64>` - Hourly solar irradiance values (8760
  hours, 0-1 scale)
- **`electricity_demand`**: `Vec<f64>` - Hourly electricity demand in Wh (8760
  hours)

### Returns

- **`SimpleOptimizationResults`**: Struct containing optimization results
  including:
  - System capacities (PV, battery, grid, heat pump)
  - Energy flows (hourly PV production, grid consumption, battery storage)
  - Economic metrics (total cost, investment costs, operating costs)
  - Performance indicators (autonomy, self-consumption, etc.)

## Detailed Usage Examples

### Example 1: Basic Residential System

```rust
use solar_system_opt::run_simple_opt;
use solar_system_opt::simple::solar_system_utils::{OptimizationConfig, load_solar_radiance_from_csv, load_demand_from_csv};

fn optimize_residential_system() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration for a typical residential system
    let mut config = OptimizationConfig::default();
    
    // Customize for residential use
    config.electricity_usage = 4000.0 * 1000.0; // 4000 kWh annually in Wh
    config.inv_pv = 800.0;  // €800/kW PV cost
    config.inv_bat = 300.0; // €300/kWh battery cost
    config.fc_grid = 0.25;  // €0.25/kWh grid electricity
    config.feed_in_tariff = 0.08; // €0.08/kWh feed-in tariff
    
    // Load time series data
    let solar_irradiance = load_solar_radiance_from_csv();
    let electricity_demand = load_demand_from_csv();
    
    // Run optimization for 8 kW max PV system
    let results = run_simple_opt(
        config,
        8000.0, // 8 kW in watts
        solar_irradiance,
        electricity_demand,
    )?;
    
    // Print results
    println!("=== Residential Solar System Optimization ===");
    println!("Optimal PV capacity: {:.2} kW", results.pv_capacity / 1000.0);
    println!("Optimal battery capacity: {:.2} kWh", results.battery_capacity / 1000.0);
    println!("Total system cost: {:.2} €", results.total_cost);
    println!("Annual PV production: {:.2} kWh", results.total_pv_production / 1000.0);
    println!("Annual grid consumption: {:.2} kWh", results.total_grid_consumption / 1000.0);
    println!("Self-consumption rate: {:.1}%", results.self_consumption_rate);
    
    Ok(())
}
```

### Example 2: System with Heat Pump and Electric Vehicle

```rust
use solar_system_opt::run_simple_opt;
use solar_system_opt::simple::solar_system_utils::{
    OptimizationConfig, HeatingType, InsulationLevel, BuildingTypeEnum, 
    YearCategoryESEnum, MonthlyDemand, load_solar_radiance_from_csv, load_demand_from_csv
};

fn optimize_advanced_system() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = OptimizationConfig::default();
    
    // Enable heat pump system
    config.heat_pump_enabled = true;
    config.house_square_meters = 150.0; // 150 m² house
    config.building_type = BuildingTypeEnum::SingleFamily;
    config.construction_period = YearCategoryESEnum::Between1977and1995;
    config.insulation_standard = InsulationLevel::Good;
    config.heating_type = HeatingType::Floor; // Underfloor heating
    config.inv_heat_pump = 200.0; // €200/kW heat pump cost
    
    // Enable electric vehicle
    config.electric_car_enabled = true;
    config.car_daily_km = 60.0; // 60 km per day
    config.car_efficiency_kwh_per_km = 0.18; // 0.18 kWh/km
    config.car_battery_size_kwh = 60.0; // 60 kWh battery
    config.car_charge_during_day = true; // Charge during daytime
    
    // Set monthly demand for more accurate optimization
    config.monthly_demand = Some(MonthlyDemand {
        january: 450.0,
        february: 420.0,
        march: 380.0,
        april: 350.0,
        may: 320.0,
        june: 300.0,
        july: 290.0,
        august: 300.0,
        september: 330.0,
        october: 380.0,
        november: 420.0,
        december: 450.0,
    });
    
    // Load data
    let solar_irradiance = load_solar_radiance_from_csv();
    let electricity_demand = load_demand_from_csv();
    
    // Run optimization
    let results = run_simple_opt(
        config,
        12000.0, // 12 kW max PV
        solar_irradiance,
        electricity_demand,
    )?;
    
    println!("=== Advanced System with Heat Pump and EV ===");
    println!("PV capacity: {:.2} kW", results.pv_capacity / 1000.0);
    println!("Battery capacity: {:.2} kWh", results.battery_capacity / 1000.0);
    println!("Heat pump capacity: {:.2} kW", results.heat_pump_capacity / 1000.0);
    println!("Total cost: {:.2} €", results.total_cost);
    println!("Heat pump consumption: {:.2} kWh", results.total_heat_pump_consumption / 1000.0);
    println!("EV charging: {:.2} kWh", results.total_car_charging / 1000.0);
    
    Ok(())
}
```

### Example 3: Optimization for Maximum Autonomy

```rust
use solar_system_opt::run_simple_opt;
use solar_system_opt::simple::solar_system_utils::{OptimizationConfig, load_solar_radiance_from_csv, load_demand_from_csv};

fn optimize_for_autonomy() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = OptimizationConfig::default();
    
    // Optimize for maximum autonomy instead of minimum cost
    config.optimize_for_autonomy = true;
    
    // Set high feed-in tariff to encourage overproduction
    config.feed_in_tariff = 0.15; // €0.15/kWh
    
    // Increase battery capacity limit
    config.bat_value = 50000.0; // 50 kWh max battery
    
    // Load data
    let solar_irradiance = load_solar_radiance_from_csv();
    let electricity_demand = load_demand_from_csv();
    
    // Run optimization
    let results = run_simple_opt(
        config,
        15000.0, // 15 kW max PV
        solar_irradiance,
        electricity_demand,
    )?;
    
    println!("=== Autonomy Optimization ===");
    println!("PV capacity: {:.2} kW", results.pv_capacity / 1000.0);
    println!("Battery capacity: {:.2} kWh", results.battery_capacity / 1000.0);
    println!("Grid autonomy: {:.1}%", results.grid_autonomy);
    println!("Self-consumption: {:.1}%", results.self_consumption_rate);
    
    Ok(())
}
```

## Configuration Reference

The `OptimizationConfig` struct contains all system parameters:

### Investment Costs

```rust
pub inv_pv: f64,        // PV investment cost (€/kW)
pub inv_bat: f64,       // Battery investment cost (€/kWh)
pub inv_hwat: f64,      // Hot water storage cost (€/kWh)
pub inv_grid: f64,      // Grid connection cost (€/kW)
pub inv_heat_pump: f64, // Heat pump cost (€/kW)
```

### Economic Parameters

```rust
pub annuity: f64,        // Annuity factor for investments
pub fc_grid: f64,        // Grid electricity cost (€/kWh)
pub feed_in_tariff: f64, // Feed-in tariff (€/kWh)
```

### System Parameters

```rust
pub storage_loss_bat: f64,  // Battery hourly loss rate
pub eta_in_bat: f64,        // Battery charging efficiency
pub eta_out_bat: f64,       // Battery discharging efficiency
pub c_rate_limit: f64,      // C-rate limit (fraction of capacity per hour)
pub electricity_usage: f64, // Annual electricity usage (Wh)
```

### Electric Vehicle Parameters

```rust
pub electric_car_enabled: bool,     // Enable/disable EV charging
pub car_daily_km: f64,              // Daily kilometers driven
pub car_efficiency_kwh_per_km: f64, // Car efficiency (kWh/km)
pub car_battery_size_kwh: f64,      // Car battery size (kWh)
pub car_charge_during_day: bool,    // Day/night charging preference
```

### Heat Pump Parameters

```rust
pub heat_pump_enabled: bool,           // Enable/disable heat pump
pub house_square_meters: f64,          // House size (m²)
pub building_type: BuildingTypeEnum,   // Building type
pub construction_period: YearCategoryESEnum, // Construction period
pub insulation_standard: InsulationLevel,    // Insulation quality
pub heating_type: HeatingType,         // Floor or radiator heating
```

## Data Requirements

### Input Data Files

The system expects CSV files in the `data/` directory:

- **`ts_res.csv`** - Solar irradiance time series (8760 hours, 0-1 scale)
- **`demand.csv`** - Base electricity demand pattern (8760 hours, kWh)
- **`when2heat_processed_2022.csv`** - Heat demand data for insulation
  calculations

### Data Format

All time series must contain exactly 8760 hourly values representing a full
year.

## Command Line Usage

```bash
# Run optimization with default settings
cargo run

# Run with individual day plots (shows detailed daily profiles)
cargo run days

# Build release version for better performance
cargo build --release
./target/release/solar-system-opt

# Run with custom data directory
RUST_LOG=info cargo run
```

## Output and Visualization

The system generates several types of output:

### Console Output

- Optimization progress and results
- System performance metrics
- Economic analysis

### Plot Files

- Hourly energy profiles
- Seasonal analysis
- Individual day plots (when using `cargo run days`)

### Results Structure

```rust
pub struct SimpleOptimizationResults {
    pub pv_capacity: f64,                    // Optimal PV capacity (W)
    pub battery_capacity: f64,               // Optimal battery capacity (Wh)
    pub grid_capacity: f64,                  // Required grid capacity (W)
    pub heat_pump_capacity: f64,             // Heat pump capacity (W)
    pub total_cost: f64,                     // Total system cost (€)
    pub total_pv_production: f64,            // Annual PV production (Wh)
    pub total_grid_consumption: f64,         // Annual grid consumption (Wh)
    pub self_consumption_rate: f64,          // Self-consumption rate (%)
    pub grid_autonomy: f64,                  // Grid autonomy (%)
    // ... hourly data vectors for detailed analysis
}
```

## Performance

- **Optimization Time**: Typically 1-5 seconds for residential systems
- **Memory Usage**: ~50-100 MB for full year optimization
- **Solver**: Uses Clarabel solver via `good_lp` crate
- **Caching**: Automatic caching of loaded data for repeated runs

## Dependencies

- **`good_lp`** - Linear programming solver
- **`calamine`** - Excel file reading
- **`plotters`** - Plotting and visualization
- **`ems-model`** - Building energy model integration
- **`serde`** - Serialization support

## Error Handling

The system provides comprehensive error handling:

- **Data Loading Errors** - Invalid CSV files or missing data
- **Optimization Errors** - Solver failures or infeasible problems
- **Configuration Errors** - Invalid parameter values
- **File I/O Errors** - Missing or corrupted data files

## Contributing

1. Follow Rust conventions and use `cargo fmt`
2. Add tests for new functionality
3. Update documentation for API changes
4. Ensure all tests pass with `cargo test`

## License

This project is part of the EverBlue P2P Energy Management System.
