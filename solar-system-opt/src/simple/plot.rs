use plotters::prelude::*;
use std::collections::HashMap;

use crate::simple::solar_system_utils::OptimizationResults;

// Equivalent to plot_data1 function
pub fn plot_data1(
    data: &[f64],
    title: &str,
    x_axis: &str,
    y_axis: &str,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0f64..data.len() as f64,
            data.iter().fold(f64::INFINITY, |a, &b| a.min(b))
                ..data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
        )?;

    chart
        .configure_mesh()
        .x_desc(x_axis)
        .y_desc(y_axis)
        .draw()?;

    chart
        .draw_series(LineSeries::new(
            data.iter().enumerate().map(|(i, &y)| (i as f64, y)),
            &RED,
        ))?
        .label("Data")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &RED));

    chart.draw_series(PointSeries::of_element(
        data.iter().enumerate().map(|(i, &y)| (i as f64, y)),
        5,
        &RED,
        &|c, s, st| {
            return EmptyElement::at(c) + Circle::new((0, 0), s, st.filled());
        },
    ))?;

    chart.configure_series_labels().draw()?;
    root.present()?;
    println!("Plot saved as {}", filename);
    Ok(())
}

// Equivalent to plot_data2 function
pub fn plot_data2(
    dem_elec: &[f64],
    dem_charge: &[f64],
    sup_grid: &[f64],
    sup_pv: &[f64],
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (800, 1000)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));
    let upper = &areas[0];
    let lower = &areas[1];

    // First subplot: demand comparison
    let mut chart1 = ChartBuilder::on(&upper)
        .caption("Demand Comparison", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0f64..dem_elec.len() as f64,
            0f64..dem_elec
                .iter()
                .chain(dem_charge.iter())
                .fold(0f64, |a, &b| a.max(b)),
        )?;

    chart1
        .configure_mesh()
        .x_desc("Index")
        .y_desc("Value")
        .draw()?;

    chart1
        .draw_series(dem_elec.iter().enumerate().map(|(i, &y)| {
            Rectangle::new([(i as f64 - 0.2, 0.0), (i as f64 + 0.2, y)], BLUE.filled())
        }))?
        .label("dem_elec")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));

    chart1
        .draw_series(dem_charge.iter().enumerate().map(|(i, &y)| {
            Rectangle::new([(i as f64 + 0.2, 0.0), (i as f64 + 0.6, y)], GREEN.filled())
        }))?
        .label("dem_charge")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &GREEN));

    chart1.configure_series_labels().draw()?;

    // Second subplot: supply comparison
    let mut chart2 = ChartBuilder::on(&lower)
        .caption("Supply Comparison", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0f64..sup_grid.len() as f64,
            0f64..sup_grid
                .iter()
                .chain(sup_pv.iter())
                .fold(0f64, |a, &b| a.max(b)),
        )?;

    chart2
        .configure_mesh()
        .x_desc("Index")
        .y_desc("Value")
        .draw()?;

    chart2
        .draw_series(sup_grid.iter().enumerate().map(|(i, &y)| {
            Rectangle::new([(i as f64 - 0.2, 0.0), (i as f64 + 0.2, y)], RED.filled())
        }))?
        .label("GRID")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &RED));

    chart2
        .draw_series(sup_pv.iter().enumerate().map(|(i, &y)| {
            Rectangle::new(
                [(i as f64 + 0.2, 0.0), (i as f64 + 0.6, y)],
                MAGENTA.filled(),
            )
        }))?
        .label("PV")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &MAGENTA));

    chart2.configure_series_labels().draw()?;

    root.present()?;
    println!("Plot saved as {}", filename);
    Ok(())
}

// Equivalent to plot_result1 function
pub fn plot_result1(
    results: &HashMap<String, Vec<f64>>,
    pv_capacity: &[f64],
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (800, 1200)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((3, 1));
    let upper = &areas[0];
    let middle = &areas[1];
    let lower = &areas[2];

    // Plot 1: PV and GRID production
    let mut chart1 = ChartBuilder::on(&upper)
        .caption("Energy vs PV-Capacity", ("sans-serif", 25))
        .margin(15)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            pv_capacity[0]..pv_capacity[pv_capacity.len() - 1],
            results["PV"]
                .iter()
                .chain(results["GRID"].iter())
                .fold(f64::INFINITY, |a, &b| a.min(b))
                ..results["PV"]
                    .iter()
                    .chain(results["GRID"].iter())
                    .fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
        )?;

    chart1
        .configure_mesh()
        .x_desc("PV-Capacity [kW]")
        .y_desc("Energy [kWh]")
        .draw()?;

    chart1
        .draw_series(LineSeries::new(
            pv_capacity
                .iter()
                .zip(results["PV"].iter())
                .map(|(&x, &y)| (x, y)),
            &BLUE,
        ))?
        .label("PV_PROD")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));

    chart1
        .draw_series(LineSeries::new(
            pv_capacity
                .iter()
                .zip(results["GRID"].iter())
                .map(|(&x, &y)| (x, y)),
            &RED,
        ))?
        .label("GRID_PROD")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &RED));

    chart1.configure_series_labels().draw()?;

    // Plot 2: Overproduction
    let mut chart2 = ChartBuilder::on(&middle)
        .caption("Overproduction vs PV-Capacity", ("sans-serif", 25))
        .margin(15)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            pv_capacity[0]..pv_capacity[pv_capacity.len() - 1],
            0f64..results["OP"].iter().fold(0f64, |a, &b| a.max(b)),
        )?;

    chart2
        .configure_mesh()
        .x_desc("PV-Capacity [kW]")
        .y_desc("Overproduction [kWh]")
        .draw()?;

    chart2
        .draw_series(LineSeries::new(
            pv_capacity
                .iter()
                .zip(results["OP"].iter())
                .map(|(&x, &y)| (x, y)),
            &RGBColor(255, 165, 0), // Orange
        ))?
        .label("OverProd")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &RGBColor(255, 165, 0)));

    chart2.configure_series_labels().draw()?;

    // Plot 3: Cost
    let mut chart3 = ChartBuilder::on(&lower)
        .caption("Cost vs PV-Capacity", ("sans-serif", 25))
        .margin(15)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            pv_capacity[0]..pv_capacity[pv_capacity.len() - 1],
            results["OBJEC"]
                .iter()
                .fold(f64::INFINITY, |a, &b| a.min(b))
                ..results["OBJEC"]
                    .iter()
                    .fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
        )?;

    chart3
        .configure_mesh()
        .x_desc("PV-Capacity [kW]")
        .y_desc("Cost [€]")
        .draw()?;

    chart3
        .draw_series(LineSeries::new(
            pv_capacity
                .iter()
                .zip(results["OBJEC"].iter())
                .map(|(&x, &y)| (x, y)),
            &GREEN,
        ))?
        .label("ObRes")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &GREEN));

    chart3.configure_series_labels().draw()?;

    root.present()?;
    println!("Plot saved as {}", filename);
    Ok(())
}

// Function to generate plots with actual optimization results
pub fn generate_optimization_plots(
    results: &OptimizationResults,
) -> Result<(), Box<dyn std::error::Error>> {
    // Plot 1: Energy flows over first 168 hours (one week)
    let hours_to_plot = 168.min(results.pv_energy.len());

    plot_data1(
        &results.pv_energy[..hours_to_plot],
        "PV Energy Production (First Week)",
        "Time (hours)",
        "Energy (kWh)",
        "pv_energy_week.png",
    )?;

    plot_data1(
        &results.battery_storage[..hours_to_plot],
        "Battery Storage Level (First Week)",
        "Time (hours)",
        "Storage (kWh)",
        "battery_storage_week.png",
    )?;

    plot_data1(
        &results.energy_overproduction[..hours_to_plot],
        "Energy Overproduction (First Week)",
        "Time (hours)",
        "Energy (kWh)",
        "overproduction_week.png",
    )?;

    // Plot 2: Compare demand vs supply for first 24 hours
    let daily_hours = 24.min(results.pv_energy.len());
    let electricity_demand = vec![2.0; daily_hours];

    plot_data2(
        &electricity_demand,
        &results.charging_energy[..daily_hours],
        &results.grid_energy[..daily_hours],
        &results.pv_energy[..daily_hours],
        "daily_energy_balance.png",
    )?;

    // Plot 3: Create a summary result plot showing the optimization outcome
    let mut optimization_summary = HashMap::new();

    // Aggregate annual data
    let annual_pv: f64 = results.pv_energy.iter().sum();
    let annual_grid: f64 = results.grid_energy.iter().sum();
    let annual_overproduction: f64 = results.energy_overproduction.iter().sum();

    optimization_summary.insert("PV".to_string(), vec![annual_pv]);
    optimization_summary.insert("GRID".to_string(), vec![annual_grid]);
    optimization_summary.insert("OP".to_string(), vec![annual_overproduction]);
    optimization_summary.insert("OBJEC".to_string(), vec![results.total_cost]);

    let pv_cap_vec = vec![results.pv_capacity];

    plot_result1(
        &optimization_summary,
        &pv_cap_vec,
        "optimization_summary.png",
    )?;

    println!("Optimization plots generated successfully!");
    Ok(())
}

// Function to print optimization summary
pub fn print_optimization_summary(results: &OptimizationResults) {
    println!("\n=== OPTIMIZATION RESULTS SUMMARY ===");
    println!("Total Cost: €{:.2}", results.total_cost);
    println!("PV Capacity: {:.2} kW", results.pv_capacity);
    println!("Battery Capacity: {:.2} kWh", results.battery_capacity);
    println!(
        "Hot Water Storage Capacity: {:.2} kWh",
        results.hot_water_capacity
    );
    println!("Grid Capacity: {:.2} kW", results.grid_capacity);

    let annual_pv: f64 = results.pv_energy.iter().sum();
    let annual_grid: f64 = results.grid_energy.iter().sum();
    let annual_overproduction: f64 = results.energy_overproduction.iter().sum();
    let annual_charging: f64 = results.charging_energy.iter().sum();

    println!("\nAnnual Energy Summary:");
    println!("PV Production: {:.2} kWh", annual_pv);
    println!("Grid Energy: {:.2} kWh", annual_grid);
    println!("Overproduction: {:.2} kWh", annual_overproduction);
    println!("EV Charging: {:.2} kWh", annual_charging);

    let max_battery_level = results
        .battery_storage
        .iter()
        .fold(0.0f64, |a, &b| a.max(b));
    let max_hot_water_level = results
        .hot_water_storage
        .iter()
        .fold(0.0f64, |a, &b| a.max(b));

    println!("\nStorage Utilization:");
    println!(
        "Max Battery Level: {:.2} kWh ({:.1}% of capacity)",
        max_battery_level,
        (max_battery_level / results.battery_capacity.max(1e-6)) * 100.0
    );
    println!(
        "Max Hot Water Level: {:.2} kWh ({:.1}% of capacity)",
        max_hot_water_level,
        (max_hot_water_level / results.hot_water_capacity.max(1e-6)) * 100.0
    );
    println!("=====================================\n");
}

/// Plot hourly averages for electricity demand, PV production, and grid consumption
/// Can handle both full year data (8760 hours) for averaging, or single day data (24 hours)
pub fn plot_hourly_averages(
    electricity_demand: &[f64],
    pv_production: &[f64],
    grid_consumption: &[f64],
    battery_storage: &[f64],
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    plot_hourly_averages_with_title(
        electricity_demand,
        pv_production,
        grid_consumption,
        battery_storage,
        filename,
        None,
    )
}

/// Plot hourly data with optional custom title
pub fn plot_hourly_averages_with_title(
    electricity_demand: &[f64],
    pv_production: &[f64],
    grid_consumption: &[f64],
    battery_storage: &[f64],
    filename: &str,
    custom_title: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let data_len = electricity_demand.len();

    // Determine if this is single day data or full year data
    let (hourly_demand, hourly_pv, hourly_grid, hourly_battery, title) = if data_len == 24 {
        // Single day data - use as is
        (
            electricity_demand.to_vec(),
            pv_production.to_vec(),
            grid_consumption.to_vec(),
            battery_storage.to_vec(),
            custom_title
                .unwrap_or("Single Day Energy Profile")
                .to_string(),
        )
    } else {
        // Full year or partial year data - calculate hourly averages
        const NUM_HOURS: usize = 8760;
        let mut hourly_demand = vec![0.0; 24];
        let mut hourly_pv = vec![0.0; 24];
        let mut hourly_grid = vec![0.0; 24];
        let mut hourly_battery = vec![0.0; 24];
        let mut hour_counts = vec![0; 24];

        // Sum values for each hour of the day
        for hour in 0..NUM_HOURS.min(data_len) {
            let hour_of_day = hour % 24;
            hourly_demand[hour_of_day] += electricity_demand[hour];
            hourly_pv[hour_of_day] += pv_production[hour];
            hourly_grid[hour_of_day] += grid_consumption[hour];
            hourly_battery[hour_of_day] += battery_storage[hour];
            hour_counts[hour_of_day] += 1;
        }

        // Calculate averages
        for hour in 0..24 {
            if hour_counts[hour] > 0 {
                hourly_demand[hour] /= hour_counts[hour] as f64;
                hourly_pv[hour] /= hour_counts[hour] as f64;
                hourly_grid[hour] /= hour_counts[hour] as f64;
                hourly_battery[hour] /= hour_counts[hour] as f64;
            }
        }

        (
            hourly_demand,
            hourly_pv,
            hourly_grid,
            hourly_battery,
            custom_title
                .unwrap_or("Hourly Energy Profile Averages")
                .to_string(),
        )
    };

    // Create the plot
    let root = BitMapBackend::new(filename, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find the range for y-axis
    let min_val = hourly_demand
        .iter()
        .chain(hourly_pv.iter())
        .chain(hourly_grid.iter())
        .chain(hourly_battery.iter())
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = hourly_demand
        .iter()
        .chain(hourly_pv.iter())
        .chain(hourly_grid.iter())
        .chain(hourly_battery.iter())
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    let mut chart = ChartBuilder::on(&root)
        .caption(&title, ("sans-serif", 40))
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(80)
        .build_cartesian_2d(0f64..23f64, (min_val * 0.9)..(max_val * 1.1))?;

    chart
        .configure_mesh()
        .x_desc("Hour of Day")
        .y_desc("Energy (kWh)")
        .x_label_formatter(&|x| format!("{:.0}", x))
        .draw()?;

    // Draw electricity demand line
    chart
        .draw_series(LineSeries::new(
            hourly_demand
                .iter()
                .enumerate()
                .map(|(i, &y)| (i as f64, y)),
            RED.stroke_width(3),
        ))?
        .label("Electricity Demand")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 15, y)], RED.stroke_width(3)));

    // Draw PV production line
    chart
        .draw_series(LineSeries::new(
            hourly_pv.iter().enumerate().map(|(i, &y)| (i as f64, y)),
            BLUE.stroke_width(3),
        ))?
        .label("PV Production")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 15, y)], BLUE.stroke_width(3)));

    // Draw grid consumption line
    chart
        .draw_series(LineSeries::new(
            hourly_grid.iter().enumerate().map(|(i, &y)| (i as f64, y)),
            GREEN.stroke_width(3),
        ))?
        .label("Grid Consumption (+) / Feed-in (-)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 15, y)], GREEN.stroke_width(3)));

    // Draw battery storage line
    chart
        .draw_series(LineSeries::new(
            hourly_battery
                .iter()
                .enumerate()
                .map(|(i, &y)| (i as f64, y)),
            MAGENTA.stroke_width(3),
        ))?
        .label("Battery Storage Level")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 15, y)], MAGENTA.stroke_width(3)));

    // Add point markers for better visibility
    chart.draw_series(
        hourly_demand
            .iter()
            .enumerate()
            .map(|(i, &y)| Circle::new((i as f64, y), 3, RED.filled())),
    )?;

    chart.draw_series(
        hourly_pv
            .iter()
            .enumerate()
            .map(|(i, &y)| Circle::new((i as f64, y), 3, BLUE.filled())),
    )?;

    chart.draw_series(
        hourly_grid
            .iter()
            .enumerate()
            .map(|(i, &y)| Circle::new((i as f64, y), 3, GREEN.filled())),
    )?;

    chart.draw_series(
        hourly_battery
            .iter()
            .enumerate()
            .map(|(i, &y)| Circle::new((i as f64, y), 3, MAGENTA.filled())),
    )?;

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    println!("Hourly averages plot saved as {}", filename);
    Ok(())
}
