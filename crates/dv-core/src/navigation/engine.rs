//! Navigation engine implementation

use super::{NavigationMode, NavigationPosition, NavigationRange, NavigationSpec, NavigationContext, NavigationSubscriber};
use std::sync::{Arc, Weak};
use parking_lot::RwLock;

/// Navigation state stored internally
#[derive(Debug, Clone)]
struct NavigationState {
    mode: NavigationMode,
    position: NavigationPosition,
    selection_range: Option<NavigationRange>,
    total_rows: usize,
}

/// The main navigation engine
pub struct NavigationEngine {
    state: Arc<RwLock<NavigationState>>,
    subscribers: Arc<RwLock<Vec<Weak<dyn NavigationSubscriber>>>>,
}

impl NavigationEngine {
    /// Create a new navigation engine
    pub fn new(mode: NavigationMode) -> Self {
        let state = NavigationState {
            mode,
            position: NavigationPosition::Sequential(0),
            selection_range: None,
            total_rows: 0,
        };
        
        Self {
            state: Arc::new(RwLock::new(state)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Update the navigation specification (e.g., when data source changes)
    pub fn update_spec(&self, spec: NavigationSpec) {
        let mut state = self.state.write();
        state.mode = spec.mode;
        state.total_rows = spec.total_rows;
        
        // Reset position to beginning
        state.position = match &state.mode {
            NavigationMode::Temporal => NavigationPosition::Temporal(0),
            NavigationMode::Sequential => NavigationPosition::Sequential(0),
            NavigationMode::Categorical { categories } => {
                if !categories.is_empty() {
                    NavigationPosition::Categorical(categories[0].clone())
                } else {
                    NavigationPosition::Sequential(0)
                }
            }
        };
        
        drop(state);
        self.notify_subscribers();
    }
    
    /// Navigate to a specific position
    pub fn seek_to(&self, position: NavigationPosition) -> Result<(), String> {
        let mut state = self.state.write();
        
        // Validate position matches current mode
        match (&state.mode, &position) {
            (NavigationMode::Temporal, NavigationPosition::Temporal(_)) => {
                state.position = position;
            }
            (NavigationMode::Sequential, NavigationPosition::Sequential(idx)) => {
                if *idx < state.total_rows {
                    state.position = position;
                } else {
                    return Err(format!("Position {} out of bounds (max: {})", idx, state.total_rows - 1));
                }
            }
            (NavigationMode::Categorical { categories }, NavigationPosition::Categorical(cat)) => {
                if categories.contains(cat) {
                    state.position = position;
                } else {
                    return Err(format!("Category '{}' not found", cat));
                }
            }
            _ => return Err("Position type doesn't match navigation mode".to_string()),
        }
        
        drop(state);
        self.notify_subscribers();
        Ok(())
    }
    
    /// Navigate forward by one step
    pub fn next(&self) -> Result<(), String> {
        let mut state = self.state.write();
        
        match &state.position {
            NavigationPosition::Sequential(idx) => {
                if *idx + 1 < state.total_rows {
                    state.position = NavigationPosition::Sequential(idx + 1);
                } else {
                    return Err("Already at end".to_string());
                }
            }
            NavigationPosition::Temporal(time) => {
                state.position = NavigationPosition::Temporal(time + 1);
            }
            NavigationPosition::Categorical(current) => {
                if let NavigationMode::Categorical { categories } = &state.mode {
                    if let Some(current_idx) = categories.iter().position(|c| c == current) {
                        if current_idx + 1 < categories.len() {
                            state.position = NavigationPosition::Categorical(categories[current_idx + 1].clone());
                        } else {
                            return Err("Already at last category".to_string());
                        }
                    }
                }
            }
        }
        
        drop(state);
        self.notify_subscribers();
        Ok(())
    }
    
    /// Navigate backward by one step  
    pub fn previous(&self) -> Result<(), String> {
        let mut state = self.state.write();
        
        match &state.position {
            NavigationPosition::Sequential(idx) => {
                if *idx > 0 {
                    state.position = NavigationPosition::Sequential(idx - 1);
                } else {
                    return Err("Already at beginning".to_string());
                }
            }
            NavigationPosition::Temporal(time) => {
                if *time > 0 {
                    state.position = NavigationPosition::Temporal(time - 1);
                } else {
                    return Err("Already at beginning".to_string());
                }
            }
            NavigationPosition::Categorical(current) => {
                if let NavigationMode::Categorical { categories } = &state.mode {
                    if let Some(current_idx) = categories.iter().position(|c| c == current) {
                        if current_idx > 0 {
                            state.position = NavigationPosition::Categorical(categories[current_idx - 1].clone());
                        } else {
                            return Err("Already at first category".to_string());
                        }
                    }
                }
            }
        }
        
        drop(state);
        self.notify_subscribers();
        Ok(())
    }
    
    /// Advance by multiple steps (for playback)
    pub fn advance(&self, steps: usize) {
        let mut state = self.state.write();
        
        match &state.position {
            NavigationPosition::Sequential(idx) => {
                let new_idx = (*idx + steps).min(state.total_rows.saturating_sub(1));
                state.position = NavigationPosition::Sequential(new_idx);
            }
            NavigationPosition::Temporal(time) => {
                state.position = NavigationPosition::Temporal(time + steps as i64);
            }
            NavigationPosition::Categorical(current) => {
                if let NavigationMode::Categorical { categories } = &state.mode {
                    if let Some(current_idx) = categories.iter().position(|c| c == current) {
                        let new_idx = (current_idx + steps).min(categories.len().saturating_sub(1));
                        state.position = NavigationPosition::Categorical(categories[new_idx].clone());
                    }
                }
            }
        }
        
        drop(state);
        self.notify_subscribers();
    }
    
    /// Set selection range
    pub fn set_range(&self, range: Option<NavigationRange>) {
        let mut state = self.state.write();
        state.selection_range = range;
        drop(state);
        self.notify_subscribers();
    }
    
    /// Get current navigation context
    pub fn get_context(&self) -> NavigationContext {
        let state = self.state.read();
        NavigationContext {
            mode: state.mode.clone(),
            position: state.position.clone(),
            selection_range: state.selection_range.clone(),
            total_rows: state.total_rows,
        }
    }
    
    /// Add a subscriber
    pub fn add_subscriber(&self, subscriber: Arc<dyn NavigationSubscriber>) {
        let mut subscribers = self.subscribers.write();
        subscribers.push(Arc::downgrade(&subscriber));
    }
    
    /// Notify all subscribers of navigation change
    fn notify_subscribers(&self) {
        let context = self.get_context();
        let mut subscribers = self.subscribers.write();
        
        // Remove any dead weak references
        subscribers.retain(|weak| weak.strong_count() > 0);
        
        // Notify live subscribers
        for weak in subscribers.iter() {
            if let Some(subscriber) = weak.upgrade() {
                subscriber.on_navigation_change(&context);
            }
        }
    }
} 