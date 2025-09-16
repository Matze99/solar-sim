use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use ts_rs::TS;
use utoipa::ToSchema;

/// Represents a node in the production line graph.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./line.ts")]
pub struct LineNode {
    /// The id of the node.
    pub id: String,
    /// The name of the node.
    pub name: String,
    /// The step ID.
    pub step_id: String,
    /// The steps that this step depends on (prerequisites).
    pub dependencies: Vec<String>,
    /// The steps that depend on this step.
    pub dependents: Vec<String>,
}

impl LineNode {
    pub fn new(id: String, name: String, step_id: String) -> Self {
        Self {
            id,
            name,
            step_id,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }

    pub fn step_id(&self) -> &str {
        &self.step_id
    }

    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    pub fn dependents(&self) -> &[String] {
        &self.dependents
    }
}

/// A production line that models dependencies between steps as a directed acyclic graph (DAG).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, export_to = "./line.ts")]
pub struct Line {
    /// The name of the line.
    pub name: String,
    /// The id of the line.
    pub id: String,
    /// The dependency graph nodes.
    pub nodes: HashMap<String, LineNode>,
}

impl Line {
    /// Creates a new production line.
    pub fn new(name: String, id: String) -> Self {
        Self {
            name,
            id,
            nodes: HashMap::new(),
        }
    }

    /// Adds a step to the production line.
    pub fn add_step(&mut self, id: String, name: String, step_id: String) {
        self.nodes.insert(
            id.clone(),
            LineNode::new(id.clone(), name.clone(), step_id.clone()),
        );
    }

    /// Adds a dependency between two steps.
    /// Returns an error if the dependency would create a cycle.
    pub fn add_dependency(
        &mut self,
        prerequisite_id: String,
        dependent_id: String,
    ) -> Result<(), String> {
        // Check if both steps exist
        if !self.nodes.contains_key(&prerequisite_id) {
            return Err(format!(
                "Prerequisite step '{}' does not exist",
                prerequisite_id
            ));
        }
        if !self.nodes.contains_key(&dependent_id) {
            return Err(format!("Dependent step '{}' does not exist", dependent_id));
        }

        // Check if this dependency would create a cycle
        if self.would_create_cycle(&prerequisite_id, &dependent_id) {
            return Err(format!(
                "Adding dependency from '{}' to '{}' would create a cycle",
                prerequisite_id, dependent_id
            ));
        }

        // Update the nodes
        if let Some(prerequisite_node) = self.nodes.get_mut(&prerequisite_id) {
            prerequisite_node.dependents.push(dependent_id.clone());
        }
        if let Some(dependent_node) = self.nodes.get_mut(&dependent_id) {
            dependent_node.dependencies.push(prerequisite_id);
        }

        Ok(())
    }

    /// Checks if adding a dependency would create a cycle in the graph.
    fn would_create_cycle(&self, from: &str, to: &str) -> bool {
        // Use DFS to check if there's already a path from 'to' to 'from'
        let mut visited = HashSet::new();
        self.has_path_dfs(to, from, &mut visited)
    }

    /// Performs a depth-first search to check if there's a path from start to target.
    fn has_path_dfs(&self, start: &str, target: &str, visited: &mut HashSet<String>) -> bool {
        if start == target {
            return true;
        }

        if visited.contains(start) {
            return false;
        }

        visited.insert(start.to_string());

        if let Some(node) = self.nodes.get(start) {
            for dependent in &node.dependents {
                if self.has_path_dfs(dependent, target, visited) {
                    return true;
                }
            }
        }

        false
    }

    /// Returns the steps in topological order (dependency order).
    /// Returns None if the graph contains cycles.
    pub fn topological_sort(&self) -> Option<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Initialize in-degree for all nodes
        for step_id in self.nodes.keys() {
            in_degree.insert(step_id.clone(), 0);
        }

        // Calculate in-degrees
        for node in self.nodes.values() {
            for dependent in &node.dependents {
                *in_degree.get_mut(dependent).unwrap() += 1;
            }
        }

        // Find all nodes with in-degree 0
        for (step_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(step_id.clone());
            }
        }

        // Process nodes
        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            // Reduce in-degree of dependent nodes
            if let Some(node) = self.nodes.get(&current) {
                for dependent in &node.dependents {
                    let degree = in_degree.get_mut(dependent).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        // Check if all nodes were processed (no cycles)
        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // Cycle detected
        }
    }

    /// Gets all immediate prerequisites for a given step.
    pub fn get_prerequisites(&self, step_id: &str) -> Option<&[String]> {
        self.nodes.get(step_id).map(|node| node.dependencies())
    }

    /// Gets all immediate dependents for a given step.
    pub fn get_dependents(&self, step_id: &str) -> Option<&[String]> {
        self.nodes.get(step_id).map(|node| node.dependents())
    }

    /// Gets all steps that can be started (have no unfinished prerequisites).
    pub fn get_ready_steps(&self, completed_steps: &HashSet<String>) -> Vec<String> {
        let mut ready_steps = Vec::new();

        for (step_id, node) in &self.nodes {
            if completed_steps.contains(step_id) {
                continue; // Already completed
            }

            // Check if all prerequisites are completed
            let all_prerequisites_completed = node
                .dependencies
                .iter()
                .all(|prereq| completed_steps.contains(prereq));

            if all_prerequisites_completed {
                ready_steps.push(step_id.clone());
            }
        }

        ready_steps
    }

    /// Validates the production line graph.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check for cycles
        if self.topological_sort().is_none() {
            errors.push("The production line contains cycles".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Returns the line name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the line ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a reference to all nodes.
    pub fn nodes(&self) -> &HashMap<String, LineNode> {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::machine::{MachineControl, Step, StepType};

    fn create_test_step(id: &str, name: &str) -> Step {
        Step {
            id: id.to_string(),
            step_type: StepType::Machine,
            name: name.to_string(),
            power_consumption: 100.0,
            runtime_minutes: 60.0,
            control: MachineControl::Computer,
            required_specialization: None,
        }
    }

    #[test]
    fn test_add_steps_and_dependencies() {
        let mut line = Line::new("Test Line".to_string(), "line1".to_string());

        line.add_step(
            "step1".to_string(),
            "Step 1".to_string(),
            "step1".to_string(),
        );
        line.add_step(
            "step2".to_string(),
            "Step 2".to_string(),
            "step2".to_string(),
        );
        line.add_step(
            "step3".to_string(),
            "Step 3".to_string(),
            "step3".to_string(),
        );

        // Add dependencies: step1 -> step2 -> step3
        assert!(line
            .add_dependency("step1".to_string(), "step2".to_string())
            .is_ok());
        assert!(line
            .add_dependency("step2".to_string(), "step3".to_string())
            .is_ok());

        let sorted = line.topological_sort().unwrap();
        assert_eq!(sorted, vec!["step1", "step2", "step3"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut line = Line::new("Test Line".to_string(), "line1".to_string());

        line.add_step(
            "step1".to_string(),
            "Step 1".to_string(),
            "step1".to_string(),
        );
        line.add_step(
            "step2".to_string(),
            "Step 2".to_string(),
            "step2".to_string(),
        );

        // Add dependencies that would create a cycle
        assert!(line
            .add_dependency("step1".to_string(), "step2".to_string())
            .is_ok());
        assert!(line
            .add_dependency("step2".to_string(), "step1".to_string())
            .is_err());
    }

    #[test]
    fn test_get_ready_steps() {
        let mut line = Line::new("Test Line".to_string(), "line1".to_string());

        line.add_step(
            "step1".to_string(),
            "Step 1".to_string(),
            "step1".to_string(),
        );
        line.add_step(
            "step2".to_string(),
            "Step 2".to_string(),
            "step2".to_string(),
        );
        line.add_step(
            "step3".to_string(),
            "Step 3".to_string(),
            "step3".to_string(),
        );

        line.add_dependency("step1".to_string(), "step2".to_string())
            .unwrap();
        line.add_dependency("step2".to_string(), "step3".to_string())
            .unwrap();

        let mut completed = HashSet::new();

        // Initially, only step1 should be ready
        let ready = line.get_ready_steps(&completed);
        assert_eq!(ready, vec!["step1"]);

        // After completing step1, step2 should be ready
        completed.insert("step1".to_string());
        let ready = line.get_ready_steps(&completed);
        assert_eq!(ready, vec!["step2"]);

        // After completing step2, step3 should be ready
        completed.insert("step2".to_string());
        let ready = line.get_ready_steps(&completed);
        assert_eq!(ready, vec!["step3"]);
    }
}
