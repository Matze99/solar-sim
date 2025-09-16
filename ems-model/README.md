# EMS Model Library

A Rust library providing data models and structures for Energy Management
Systems (EMS), with a focus on building energy modeling and factory production
optimization. This library serves as the foundation for the `solar-system-opt`
library and other energy management applications.

## Overview

The `ems-model` library provides comprehensive data structures for modeling:

- **Building Energy Systems** - Building types, insulation levels, and heating
  requirements
- **Factory Production** - Production lines, machines, workers, and scheduling
- **Geographic Information** - Location data and coordinates for energy
  calculations

## Features

- 🏢 **Building Energy Modeling** - Spanish building energy standards and
  heating requirements
- 🏭 **Factory Production Models** - Production lines with dependency graphs and
  worker scheduling
- 🌍 **Geographic Support** - Multi-country location and coordinate systems
- 🔄 **Serialization** - Full serde support for JSON/YAML serialization
- 📊 **API Documentation** - Auto-generated OpenAPI schemas with utoipa
- 🌐 **TypeScript Bindings** - Automatic TypeScript type generation with ts-rs

## Modules

### Building (`building`)

Models building energy characteristics and insulation requirements:

- **Building Types**: Single family, terraced, multi-family, and apartment
  buildings
- **Year Categories**: Spanish building construction periods (pre-1900 to
  post-2007)
- **Heating Requirements**: Energy consumption standards (kWh/m²/year) for
  different building types and construction periods
- **Insulation Levels**: National minimum, improved, and ambitious standards

### Factory (`factory`)

Models industrial production systems and workforce management:

- **Production Lines**: Directed acyclic graph (DAG) based production workflows
- **Machines**: Equipment with power consumption, runtime, and control
  requirements
- **Workers**: Human resources with specializations and work schedules
- **Dependencies**: Step-by-step production dependencies with cycle detection

### General (`general`)

Provides common utilities and geographic information:

- **Location**: Multi-country address and coordinate systems
- **Countries**: Supported countries (Germany, Spain, Portugal) with ISO codes
- **Coordinates**: Geographic coordinate validation and utilities

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ems-model = { path = "../ems-model" }
```

### Building Energy Modeling

```rust
use ems_model::building::insulation::{
    BuildingTypeEnum, YearCategoryESEnum, YearCategoryESMapping
};

// Get heating requirements for a Spanish apartment built after 2007
let mapping = YearCategoryESMapping::default();
let heating_need = mapping
    .get(YearCategoryESEnum::After2007)
    .and_then(|building_mapping| {
        building_mapping.get(BuildingTypeEnum::Apartment)
    });

if let Some(heating) = heating_need {
    println!("National minimum: {} kWh/m²/year", 
             heating.national_minimum_requirement);
    println!("Improved standard: {} kWh/m²/year", 
             heating.improved_standard);
    println!("Ambitious standard: {} kWh/m²/year", 
             heating.ambitious_standard);
}
```

### Factory Production Lines

```rust
use ems_model::factory::{Factory, Line, machine::{Step, StepType, MachineControl}};

// Create a production line
let mut line = Line::new("Assembly Line".to_string(), "line1".to_string());

// Add production steps
line.add_step("step1".to_string(), "Cutting".to_string(), "step1".to_string());
line.add_step("step2".to_string(), "Assembly".to_string(), "step2".to_string());

// Define dependencies
line.add_dependency("step1".to_string(), "step2".to_string()).unwrap();

// Get execution order
let execution_order = line.topological_sort().unwrap();
println!("Execution order: {:?}", execution_order);
```

### Location and Geographic Data

```rust
use ems_model::general::location::{Location, Country, Address, Coordinates};

// Create a location
let coordinates = Coordinates::new(40.4168, -3.7038).unwrap(); // Madrid
let address = Address::new(
    "Plaza Mayor 1".to_string(),
    "Madrid".to_string(),
    Some("Madrid".to_string()),
    "28012".to_string(),
    None
);

let location = Location::new(
    "Madrid Office".to_string(),
    Country::Spain,
    address,
    coordinates
);

println!("Location: {}", location.display());
```

## Integration with Solar System Optimization

This library is specifically designed to support the `solar-system-opt` library,
providing:

- **Building Energy Data**: Spanish building standards for accurate energy
  demand calculations
- **Geographic Context**: Location-based energy modeling and optimization
- **Type Safety**: Strong typing for energy system parameters and configurations

The library enables precise modeling of building energy requirements based on
construction period and building type, which is essential for solar system
sizing and optimization.

## Dependencies

- `serde` - Serialization and deserialization
- `utoipa` - OpenAPI schema generation
- `ts-rs` - TypeScript type generation

## API Documentation

The library generates comprehensive API documentation and TypeScript bindings:

- **OpenAPI Schemas**: Auto-generated with utoipa for API documentation
- **TypeScript Types**: Exported to `./bindings/` directory for frontend
  integration
- **Rust Docs**: Standard rustdoc documentation

## Contributing

This library is part of the EverBlue P2P EMS project. When contributing:

1. Maintain backward compatibility for existing solar-system-opt integration
2. Follow the established patterns for serialization and TypeScript generation
3. Add comprehensive tests for new functionality
4. Update documentation for any API changes

## License

Part of the EverBlue P2P EMS project. See the main project repository for
licensing information.
