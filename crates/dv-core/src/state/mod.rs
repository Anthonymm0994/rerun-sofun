use std::sync::Arc;
use parking_lot::RwLock;
use crate::navigation::NavigationEngine;
use crate::sync::SyncManager;
use crate::events::EventBus;

/// Space view identifier type
pub type SpaceViewId = uuid::Uuid;

/// The main application state
pub struct AppState {
    /// The navigation engine
    pub navigation: Arc<NavigationEngine>,
    
    /// The synchronization manager
    pub sync_manager: Arc<SyncManager>,
    
    /// The event bus
    pub event_bus: Arc<EventBus>,
    
    /// The currently loaded data source
    pub data_source: Arc<RwLock<Option<Arc<dyn crate::data::DataSource>>>>,
    
    /// The current template
    pub template: Arc<RwLock<Option<crate::templates::Template>>>,
    
    /// Application settings
    pub settings: Arc<RwLock<AppSettings>>,
}

/// Application settings
#[derive(Debug, Clone)]
pub struct AppSettings {
    /// Whether to show the navigation bar
    pub show_navigation_bar: bool,
    
    /// Whether to show statistics panel
    pub show_stats_panel: bool,
    
    /// Theme settings
    pub theme: ThemeSettings,
    
    /// Performance settings
    pub performance: PerformanceSettings,
}

/// Theme settings
#[derive(Debug, Clone)]
pub struct ThemeSettings {
    /// UI scale factor
    pub scale_factor: f32,
    
    /// Whether to use dark mode
    pub dark_mode: bool,
}

/// Performance settings
#[derive(Debug, Clone)]
pub struct PerformanceSettings {
    /// Maximum rows to load in memory
    pub max_rows_in_memory: usize,
    
    /// Cache size in MB
    pub cache_size_mb: usize,
    
    /// Number of worker threads
    pub worker_threads: usize,
}

/// Hovered data point information
#[derive(Debug, Clone, Default)]
pub struct HoveredData {
    pub x: f64,
    pub y: f64,
    pub column: Option<String>,
    pub view_id: Option<SpaceViewId>,
    pub point_index: Option<usize>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            show_navigation_bar: true,
            show_stats_panel: true,
            theme: ThemeSettings {
                scale_factor: 1.0,
                dark_mode: true,
            },
            performance: PerformanceSettings {
                max_rows_in_memory: 1_000_000,
                cache_size_mb: 512,
                worker_threads: num_cpus::get(),
            },
        }
    }
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        // Start with sequential navigation as default
        let navigation = NavigationEngine::new(
            crate::navigation::NavigationMode::Sequential
        );
        
        Self {
            navigation: Arc::new(navigation),
            sync_manager: Arc::new(SyncManager::new()),
            event_bus: Arc::new(EventBus::new()),
            data_source: Arc::new(RwLock::new(None)),
            template: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(AppSettings::default())),
        }
    }
    
    /// Load a data source
    pub async fn load_data_source(&self, source: Arc<dyn crate::data::DataSource>) -> anyhow::Result<()> {
        // Get schema for event
        let schema = source.schema().await;
        let row_count = source.row_count().await.unwrap_or(0);
        let column_count = schema.fields().len();
        
        // Get navigation spec from the data source
        let nav_spec = source.navigation_spec().await?;
        
        // Update navigation engine
        self.navigation.update_spec(nav_spec);
        
        // Store the data source
        *self.data_source.write() = Some(source.clone());
        
        // Publish event
        self.event_bus.publish(crate::events::events::DataSourceLoaded {
            source_name: source.source_name().to_string(),
            row_count,
            column_count,
        });
        
        Ok(())
    }
    
    /// Clear the current data source
    pub fn clear_data_source(&self) {
        *self.data_source.write() = None;
    }
    
    /// Set the current template
    pub fn set_template(&self, template: crate::templates::Template) {
        let template_id = template.id.clone();
        let template_name = template.name.clone();
        
        *self.template.write() = Some(template);
        
        // Publish event
        self.event_bus.publish(crate::events::events::TemplateChanged {
            template_id,
            template_name,
        });
    }
} 