//! Navigation subscriber trait

use super::NavigationContext;

/// Trait for components that need to respond to navigation changes
pub trait NavigationSubscriber: Send + Sync {
    /// Called when navigation position or mode changes
    fn on_navigation_change(&self, context: &NavigationContext);
} 