use std::sync::Arc;
use parking_lot::Mutex;
use ahash::AHashMap;

/// System-wide event bus
pub struct EventBus {
    handlers: Arc<Mutex<AHashMap<std::any::TypeId, Vec<Box<dyn EventHandler>>>>>,
}

/// Event trait that all events must implement
pub trait Event: Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Handler trait for event handlers
pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &dyn Event);
}

/// Common system events
pub mod events {
    use super::Event;
    
    /// Data source loaded event
    #[derive(Debug, Clone)]
    pub struct DataSourceLoaded {
        pub source_name: String,
        pub row_count: usize,
        pub column_count: usize,
    }
    
    /// Data source error event
    #[derive(Debug, Clone)]
    pub struct DataSourceError {
        pub source_name: String,
        pub error: String,
    }
    
    /// View created event
    #[derive(Debug, Clone)]
    pub struct ViewCreated {
        pub view_id: String,
        pub view_type: String,
    }
    
    /// View closed event
    #[derive(Debug, Clone)]
    pub struct ViewClosed {
        pub view_id: String,
    }
    
    /// Template changed event
    #[derive(Debug, Clone)]
    pub struct TemplateChanged {
        pub template_id: String,
        pub template_name: String,
    }
    
    // Implement Event trait for all event types
    macro_rules! impl_event {
        ($($t:ty),*) => {
            $(
                impl Event for $t {
                    fn as_any(&self) -> &dyn std::any::Any {
                        self
                    }
                }
            )*
        }
    }
    
    impl_event!(
        DataSourceLoaded,
        DataSourceError,
        ViewCreated,
        ViewClosed,
        TemplateChanged
    );
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(AHashMap::new())),
        }
    }
    
    /// Subscribe to events of a specific type
    pub fn subscribe<E: Event>(&self, handler: Box<dyn EventHandler>) {
        let type_id = std::any::TypeId::of::<E>();
        let mut handlers = self.handlers.lock();
        handlers.entry(type_id).or_insert_with(Vec::new).push(handler);
    }
    
    /// Publish an event
    pub fn publish<E: Event>(&self, event: E) {
        let type_id = std::any::TypeId::of::<E>();
        let mut handlers = self.handlers.lock();
        
        if let Some(event_handlers) = handlers.get_mut(&type_id) {
            for handler in event_handlers.iter_mut() {
                handler.handle(&event);
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct for creating event handlers from closures
pub struct ClosureEventHandler<F> {
    handler: F,
    _phantom: std::marker::PhantomData<()>,
}

impl<F> EventHandler for ClosureEventHandler<F>
where
    F: FnMut(&dyn Event) + Send + Sync,
{
    fn handle(&mut self, event: &dyn Event) {
        (self.handler)(event);
    }
}

/// Create an event handler from a closure
pub fn handler_from_fn<F>(f: F) -> Box<dyn EventHandler>
where
    F: FnMut(&dyn Event) + Send + Sync + 'static,
{
    Box::new(ClosureEventHandler { 
        handler: f,
        _phantom: std::marker::PhantomData,
    })
} 