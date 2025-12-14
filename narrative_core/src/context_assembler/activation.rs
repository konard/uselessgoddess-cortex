//! Activation state for the spreading activation algorithm.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::knowledge_base::Tag;

/// Tracks activation energy for tags during spreading.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivationState {
    energies: HashMap<Tag, f32>,
}

impl ActivationState {
    /// Create a new empty activation state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add energy to a tag (accumulates with existing energy).
    pub fn add_energy(&mut self, tag: Tag, energy: f32) {
        let current = self.energies.entry(tag).or_insert(0.0);
        *current += energy;
    }

    /// Set the energy of a tag to a specific value.
    pub fn set_energy(&mut self, tag: Tag, energy: f32) {
        self.energies.insert(tag, energy);
    }

    /// Get the energy of a tag.
    pub fn get_energy(&self, tag: &Tag) -> f32 {
        self.energies.get(tag).copied().unwrap_or(0.0)
    }

    /// Check if a tag has any energy.
    pub fn is_active(&self, tag: &Tag) -> bool {
        self.get_energy(tag) > 0.0
    }

    /// Get all tags with energy above the threshold, sorted by energy (descending).
    pub fn hot_tags(&self, threshold: f32) -> Vec<(&Tag, f32)> {
        let mut tags: Vec<_> = self
            .energies
            .iter()
            .filter(|(_, energy)| **energy >= threshold)
            .map(|(tag, energy)| (tag, *energy))
            .collect();

        tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        tags
    }

    /// Get the tag with the highest energy.
    pub fn hottest_tag(&self) -> Option<(&Tag, f32)> {
        self.energies
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(tag, energy)| (tag, *energy))
    }

    /// Get the total energy in the system.
    pub fn total_energy(&self) -> f32 {
        self.energies.values().sum()
    }

    /// Get the number of active tags.
    pub fn active_count(&self) -> usize {
        self.energies.len()
    }

    /// Iterate over all tag energies.
    pub fn iter_energies(&self) -> impl Iterator<Item = (&Tag, &f32)> {
        self.energies.iter()
    }

    /// Apply decay to all energies.
    pub fn apply_decay(&mut self, decay_rate: f32) {
        for energy in self.energies.values_mut() {
            *energy *= decay_rate;
        }
    }

    /// Remove tags with energy below threshold.
    pub fn prune(&mut self, threshold: f32) {
        self.energies.retain(|_, energy| *energy >= threshold);
    }

    /// Normalize energies so the maximum is 1.0.
    pub fn normalize(&mut self) {
        if let Some((_, max_energy)) = self.hottest_tag() {
            if max_energy > 0.0 {
                for energy in self.energies.values_mut() {
                    *energy /= max_energy;
                }
            }
        }
    }

    /// Merge another activation state into this one (adding energies).
    pub fn merge(&mut self, other: &ActivationState) {
        for (tag, energy) in &other.energies {
            self.add_energy(tag.clone(), *energy);
        }
    }

    /// Clear all activation energies.
    pub fn clear(&mut self) {
        self.energies.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_state_basic() {
        let mut state = ActivationState::new();

        let tag = Tag::concept("Test");
        state.add_energy(tag.clone(), 0.5);

        assert!((state.get_energy(&tag) - 0.5).abs() < 0.001);
        assert!(state.is_active(&tag));
    }

    #[test]
    fn test_energy_accumulation() {
        let mut state = ActivationState::new();

        let tag = Tag::concept("Test");
        state.add_energy(tag.clone(), 0.3);
        state.add_energy(tag.clone(), 0.4);

        assert!((state.get_energy(&tag) - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_hot_tags() {
        let mut state = ActivationState::new();

        state.add_energy(Tag::concept("High"), 0.9);
        state.add_energy(Tag::concept("Medium"), 0.5);
        state.add_energy(Tag::concept("Low"), 0.1);

        let hot = state.hot_tags(0.4);
        assert_eq!(hot.len(), 2);
        assert_eq!(hot[0].0, &Tag::concept("High"));
        assert_eq!(hot[1].0, &Tag::concept("Medium"));
    }

    #[test]
    fn test_hottest_tag() {
        let mut state = ActivationState::new();

        state.add_energy(Tag::concept("A"), 0.3);
        state.add_energy(Tag::concept("B"), 0.9);
        state.add_energy(Tag::concept("C"), 0.5);

        let hottest = state.hottest_tag();
        assert!(hottest.is_some());
        assert_eq!(hottest.unwrap().0, &Tag::concept("B"));
    }

    #[test]
    fn test_decay() {
        let mut state = ActivationState::new();

        state.add_energy(Tag::concept("Test"), 1.0);
        state.apply_decay(0.5);

        assert!((state.get_energy(&Tag::concept("Test")) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_prune() {
        let mut state = ActivationState::new();

        state.add_energy(Tag::concept("High"), 0.9);
        state.add_energy(Tag::concept("Low"), 0.1);

        state.prune(0.5);

        assert!(state.is_active(&Tag::concept("High")));
        assert!(!state.is_active(&Tag::concept("Low")));
        assert_eq!(state.active_count(), 1);
    }

    #[test]
    fn test_normalize() {
        let mut state = ActivationState::new();

        state.add_energy(Tag::concept("A"), 0.5);
        state.add_energy(Tag::concept("B"), 1.0);

        state.normalize();

        assert!((state.get_energy(&Tag::concept("B")) - 1.0).abs() < 0.001);
        assert!((state.get_energy(&Tag::concept("A")) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_merge() {
        let mut state1 = ActivationState::new();
        let mut state2 = ActivationState::new();

        state1.add_energy(Tag::concept("A"), 0.5);
        state2.add_energy(Tag::concept("A"), 0.3);
        state2.add_energy(Tag::concept("B"), 0.7);

        state1.merge(&state2);

        assert!((state1.get_energy(&Tag::concept("A")) - 0.8).abs() < 0.001);
        assert!((state1.get_energy(&Tag::concept("B")) - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_total_energy() {
        let mut state = ActivationState::new();

        state.add_energy(Tag::concept("A"), 0.3);
        state.add_energy(Tag::concept("B"), 0.4);
        state.add_energy(Tag::concept("C"), 0.3);

        assert!((state.total_energy() - 1.0).abs() < 0.001);
    }
}
