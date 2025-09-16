# solar-sim

Rust packages for simulating and optimizing solar power plants

## Introduction

This repository contains tools designed to optimize solar energy systems through
a dual approach:

1. **System Dimensioning**: Calculate optimal solar system dimensions (panel
   capacity, battery storage, etc.) based on consumption profiles to ensure the
   system is properly sized for actual energy needs.

2. **Consumption Optimization**: Develop strategies to optimize energy
   consumption patterns to better align with solar panel production cycles,
   maximizing the utilization of renewable energy and reducing dependency on
   grid power.

The goal is to create a comprehensive optimization framework that considers both
the technical sizing of solar installations and the behavioral aspects of energy
consumption, ultimately leading to more efficient and cost-effective solar
energy systems.

## Packages

### ems-model

A foundational Rust library providing comprehensive data models for Energy
Management Systems (EMS), with specialized focus on building energy modeling and
factory production optimization.

**Key Features:**

- **Building Energy Systems** - Spanish building energy standards, insulation
  levels, and heating requirements based on construction periods and building
  types
- **Factory Production Models** - Production lines with dependency graphs,
  machine scheduling, and worker management
- **Geographic Support** - Multi-country location data and coordinate systems
  for accurate energy calculations
- **Type Safety** - Strong typing with full serde serialization support and
  automatic TypeScript bindings

This library serves as the foundation for energy system modeling, providing the
building blocks for accurate consumption profile calculations and system
dimensioning.

### solar-system-opt

A high-performance Rust implementation for optimizing solar energy systems using
linear programming. This is the main optimization engine that takes consumption
profiles and calculates optimal system dimensions.

**Key Features:**

- **Multi-Component Optimization** - Photovoltaic systems, battery storage, heat
  pumps, electric vehicle charging, and grid integration
- **Building-Aware Modeling** - Uses building characteristics from `ems-model`
  for accurate heat demand calculations
- **Flexible Optimization Modes** - Cost minimization or autonomy maximization
  strategies
- **Advanced Features** - Monthly demand scaling, insulation-based heating
  calculations, and comprehensive visualization tools

**Core Function:** The `run_simple_opt` function performs linear programming
optimization to determine optimal system capacities based on consumption
profiles, solar irradiance data, and economic parameters.

This package implements the dual optimization approach by taking consumption
profiles (potentially optimized through behavioral strategies) and calculating
the most cost-effective solar system dimensions to meet those consumption
patterns.
