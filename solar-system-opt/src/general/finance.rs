use crate::simple::solar_system_utils::SimpleOptimizationResults;

#[derive(Debug)]
pub struct FinancialRentabilityResult {
    pub initial_investment: f64,
    pub annual_costs: f64,
    pub yearly_roi: f64,
}

#[derive(Debug)]
pub struct OptimizedROIResult {
    pub roi: f64,
    pub net_present_value: f64,
    pub payback_period: Option<f64>,
}

/// Calculate ROI using root-finding to solve the equation:
/// 0 = (sum_{i=0}^{N-1} (1+ROI)^i * s_i / I_0)^{1/N} - 1 - ROI
/// where s_i is the annual savings in year i, N is num_years, and I_0 is initial_investment
pub fn calculate_optimized_roi(
    simulation_results: SimpleOptimizationResults,
    num_years: usize,
) -> Result<OptimizedROIResult, Box<dyn std::error::Error>> {
    // Calculate initial investment (same as in calculate_financial_rentability)
    let initial_investment = simulation_results.pv_capacity_kw * simulation_results.config.inv_pv
        + simulation_results.grid_capacity_kw * simulation_results.config.inv_grid
        + simulation_results.battery_capacity_kwh * simulation_results.config.inv_bat;

    if initial_investment <= 0.0 {
        return Ok(OptimizedROIResult {
            roi: 0.0,
            net_present_value: 0.0,
            payback_period: None,
        });
    }

    // Calculate annual savings for each year
    let annual_costs_no_solar = (0..num_years)
        .map(|index| {
            let electricity_cost = simulation_results.config.fc_grid
                * simulation_results.config.electricity_usage
                * (1.0 + simulation_results.config.electricity_price_increase).powf(index as f64);
            electricity_cost
        })
        .collect::<Vec<f64>>();

    let annual_grid_costs_solar = (0..num_years)
        .map(|index| {
            let electricity_cost = simulation_results.config.fc_grid
                * simulation_results.annual_grid_energy_kwh
                * (1.0 + simulation_results.config.electricity_price_increase).powf(index as f64);
            electricity_cost
        })
        .collect::<Vec<f64>>();

    let annual_savings: Vec<f64> = annual_costs_no_solar
        .iter()
        .zip(annual_grid_costs_solar.iter())
        .map(|(cost_no_solar, cost_with_solar)| cost_no_solar - cost_with_solar)
        .collect();

    // Define the function to find the root of: f(ROI) = (sum / I_0)^{1/N} - 1 - ROI
    let equation_function = |roi: f64| -> f64 {
        let mut sum = 0.0;
        for i in 0..num_years {
            let savings_i = annual_savings[i];
            sum += (1.0 + roi).powf(i as f64) * savings_i;
        }

        let sum_normalized = sum / initial_investment;
        let nth_root = sum_normalized.powf(1.0 / (num_years as f64));

        nth_root - 1.0 - roi
    };

    // Use binary search to find the root within a reasonable range
    let mut low = -0.5; // -50% ROI
    let mut high = 2.0; // 200% ROI
    let tolerance = 1e-6;
    let max_iterations = 100;

    let mut roi_value = 0.0;
    let mut found_root = false;

    for _ in 0..max_iterations {
        let mid = (low + high) / 2.0;
        let f_mid = equation_function(mid);

        if f_mid.abs() < tolerance {
            roi_value = mid;
            found_root = true;
            break;
        }

        let f_low = equation_function(low);
        if f_low * f_mid < 0.0 {
            high = mid;
        } else {
            low = mid;
        }

        if (high - low).abs() < tolerance {
            roi_value = mid;
            found_root = true;
            break;
        }
    }

    if !found_root {
        // If binary search fails, try Newton's method as a fallback
        roi_value = newton_method_root_finding(&equation_function, 0.1, tolerance, max_iterations);
    }

    // Calculate actual NPV using the found ROI
    let mut npv = -initial_investment;
    for i in 0..num_years {
        let savings_i = annual_savings[i];
        npv += savings_i / (1.0 + roi_value).powf(i as f64);
    }

    // Calculate payback period
    let mut cumulative_savings = 0.0;
    let mut payback_period = None;
    for i in 0..num_years {
        cumulative_savings += annual_savings[i];
        if cumulative_savings >= initial_investment && payback_period.is_none() {
            payback_period = Some(
                i as f64
                    + (initial_investment - (cumulative_savings - annual_savings[i]))
                        / annual_savings[i],
            );
            break;
        }
    }

    Ok(OptimizedROIResult {
        roi: roi_value,
        net_present_value: npv,
        payback_period,
    })
}

/// Newton's method for root finding
fn newton_method_root_finding<F>(
    f: F,
    initial_guess: f64,
    tolerance: f64,
    max_iterations: usize,
) -> f64
where
    F: Fn(f64) -> f64,
{
    let mut x = initial_guess;
    let h = 1e-8; // Small step for numerical derivative

    for _ in 0..max_iterations {
        let fx = f(x);
        if fx.abs() < tolerance {
            return x;
        }

        // Numerical derivative: f'(x) ≈ (f(x+h) - f(x-h)) / (2h)
        let fx_plus_h = f(x + h);
        let fx_minus_h = f(x - h);
        let derivative = (fx_plus_h - fx_minus_h) / (2.0 * h);

        if derivative.abs() < 1e-12 {
            break; // Avoid division by zero
        }

        x = x - fx / derivative;
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_financial_rentability() {
        let mut simulation_results = SimpleOptimizationResults::default();
        let num_years = 25;
        simulation_results.config.electricity_usage = 9000000.0;
        simulation_results.battery_capacity_kwh = 0.0;
        simulation_results.grid_capacity_kw = simulation_results.config.electricity_usage * 0.57;
        simulation_results.pv_capacity_kw = simulation_results.config.electricity_usage * 0.43;
        let optimized_roi = calculate_optimized_roi(simulation_results, num_years).unwrap();
        println!("Optimized ROI: {:?}", optimized_roi);
        assert!(optimized_roi.roi == 0.23);
    }
}
