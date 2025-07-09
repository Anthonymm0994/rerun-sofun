# Advanced Rust Patterns in F.R.O.G.

This guide explains advanced Rust patterns and idioms found throughout the F.R.O.G. codebase. These patterns are common in production Rust but can be challenging for developers coming from other languages.

## 1. Arc<RwLock<T>> Pattern

**What it is**: Atomic Reference Counted Read-Write Lock - enables shared mutable state across threads.

**Where it's used**: ViewerContext, data sources, time control

```rust
pub struct ViewerContext {
    pub data_sources: Arc<RwLock<HashMap<String, Arc<dyn DataSource>>>>,
    pub time_control: Arc<RwLock<TimeControl>>,
}

// Reading
let sources = viewer_context.data_sources.read();
let source = sources.get(&id)?;

// Writing
let mut sources = viewer_context.data_sources.write();
sources.insert(id, Arc::new(source));
```

**Why**: Allows multiple readers OR one writer. Essential for GUI apps where many components need to read state, but updates must be synchronized.

## 2. Trait Objects with dyn

**What it is**: Dynamic dispatch for polymorphism at runtime.

**Where it's used**: DataSource, SpaceView traits

```rust
// Trait definition
pub trait DataSource: Send + Sync {
    async fn query_at(&self, position: &NavigationPosition) -> Result<RecordBatch>;
}

// Storing trait objects
pub type DataSourceMap = HashMap<String, Arc<dyn DataSource>>;

// Using trait objects
let source: Arc<dyn DataSource> = Arc::new(CsvSource::new(path));
```

**Why**: Allows storing different implementations (CSV, SQLite, etc.) in the same collection.

## 3. async/await with block_on

**What it is**: Asynchronous code running in synchronous context.

**Where it's used**: Data loading in UI callbacks

```rust
// In synchronous egui callback
let batch = runtime.block_on(async {
    source.query_at(&position).await
})?;

// Better pattern - spawn task
let handle = runtime.spawn(async move {
    source.query_at(&position).await
});
```

**Why**: egui is synchronous but data loading should be async. `block_on` bridges the gap but should be used carefully to avoid blocking UI.

## 4. Phantom Types and Zero-Sized Types

**What it is**: Types that exist at compile time but have no runtime cost.

**Where it's used**: Type-safe IDs

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpaceViewId(pub(crate) u64);

// Zero runtime cost, but type-safe
let view_id = SpaceViewId(123);
let entity_id = EntityId(123);
// view_id != entity_id  // Compile error!
```

**Why**: Prevents mixing up different ID types without runtime overhead.

## 5. Builder Pattern

**What it is**: Fluent API for constructing complex objects.

**Where it's used**: View creation, configuration

```rust
impl ViewBuilder {
    pub fn new() -> Self { /* ... */ }
    
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    pub fn with_config(mut self, config: ViewConfig) -> Self {
        self.config = config;
        self
    }
    
    pub fn build(self) -> Result<Box<dyn SpaceView>> {
        // Construct the view
    }
}

// Usage
let view = ViewBuilder::new()
    .with_title("My Chart")
    .with_config(config)
    .build()?;
```

**Why**: Makes optional parameters ergonomic and API evolution easier.

## 6. Type State Pattern

**What it is**: Encoding state in the type system.

**Where it's used**: Navigation states

```rust
pub enum NavigationPosition {
    Sequential(usize),
    Temporal(i64),
    Categorical(String),
}

// Different behavior based on type state
match position {
    NavigationPosition::Sequential(idx) => query_by_index(idx),
    NavigationPosition::Temporal(time) => query_by_time(time),
    NavigationPosition::Categorical(cat) => query_by_category(cat),
}
```

**Why**: Makes invalid states unrepresentable at compile time.

## 7. Interior Mutability with RefCell/Cell

**What it is**: Mutation through shared references.

**Where it's used**: Caching, lazy initialization

```rust
struct View {
    cached_data: RefCell<Option<RecordBatch>>,
}

impl View {
    fn get_data(&self) -> Ref<RecordBatch> {
        if self.cached_data.borrow().is_none() {
            let data = self.compute_data();
            *self.cached_data.borrow_mut() = Some(data);
        }
        Ref::map(self.cached_data.borrow(), |opt| opt.as_ref().unwrap())
    }
}
```

**Why**: Allows mutation in contexts that only have `&self`.

## 8. Closure Captures and move

**What it is**: Controlling how closures capture variables.

**Where it's used**: Event handlers, async tasks

```rust
// Clone before move
let source_id = source_id.clone();
let runtime = runtime.clone();

// Move ownership into closure
ui.button("Load").clicked().then(move || {
    runtime.spawn(async move {
        load_data(source_id).await
    });
});
```

**Why**: Rust's ownership rules require explicit handling of captured variables.

## 9. Match Guards and if let

**What it is**: Pattern matching with additional conditions.

**Where it's used**: Complex event handling

```rust
// Match with guard
match event {
    Event::KeyPressed { key, modifiers } if modifiers.ctrl => {
        handle_ctrl_key(key)
    }
    Event::KeyPressed { key: Key::Space, .. } => {
        toggle_playback()
    }
    _ => {}
}

// if let for single pattern
if let Some(ref mut dialog) = self.file_dialog {
    dialog.show(ui);
}
```

**Why**: More expressive than nested if statements.

## 10. Deref Coercion and Smart Pointers

**What it is**: Automatic dereferencing for ergonomics.

**Where it's used**: Arc, Box, custom smart pointers

```rust
let data: Arc<RecordBatch> = Arc::new(batch);

// Deref coercion allows calling RecordBatch methods directly
let num_rows = data.num_rows();  // No need for (*data).num_rows()

// Custom smart pointer
impl Deref for SpaceViewId {
    type Target = u64;
    fn deref(&self) -> &Self::Target { &self.0 }
}
```

**Why**: Makes smart pointers feel like regular references.

## 11. Trait Bounds and where Clauses

**What it is**: Constraining generic types.

**Where it's used**: Generic functions and data structures

```rust
// Simple bounds
fn process_data<T: DataSource + Send + Sync>(source: T) { }

// Complex bounds with where clause
fn create_view<T, U>(source: T, config: U) -> Result<Box<dyn SpaceView>>
where
    T: DataSource + 'static,
    U: Into<ViewConfig>,
{
    // Implementation
}
```

**Why**: Ensures generic code can only be used with appropriate types.

## 12. Error Handling Patterns

**What it is**: Idiomatic error propagation.

**Where it's used**: Throughout for robust error handling

```rust
// Result type alias
type Result<T> = std::result::Result<T, anyhow::Error>;

// ? operator for propagation
fn load_data(&self) -> Result<RecordBatch> {
    let file = std::fs::File::open(&self.path)?;
    let reader = csv::Reader::from_reader(file);
    let batch = self.parse_csv(reader)?;
    Ok(batch)
}

// Converting errors with context
let data = load_file(&path)
    .context("Failed to load data file")?;
```

**Why**: Makes error handling concise while preserving error context.

## 13. Lifetime Elision and Explicit Lifetimes

**What it is**: Rust's system for tracking reference validity.

**Where it's used**: String slices, borrowed data

```rust
// Lifetime elision - compiler infers 'a
fn get_title(&self) -> &str {
    &self.title
}

// Explicit lifetime needed
struct DataView<'a> {
    data: &'a RecordBatch,
    filter: Option<&'a str>,
}

// Multiple lifetimes
fn combine<'a, 'b>(first: &'a str, second: &'b str) -> String {
    format!("{} {}", first, second)
}
```

**Why**: Prevents use-after-free and ensures memory safety.

## 14. From/Into Trait Implementations

**What it is**: Type conversion traits.

**Where it's used**: Config types, error conversion

```rust
impl From<CsvConfig> for FileConfig {
    fn from(csv: CsvConfig) -> Self {
        FileConfig::Csv(csv)
    }
}

// Enables ergonomic conversions
let config: FileConfig = csv_config.into();

// Also works with ?
fn process() -> Result<()> {
    let data = load_data()?;  // Error automatically converted
    Ok(())
}
```

**Why**: Makes type conversions explicit but convenient.

## Tips for Learning These Patterns

1. **Start Simple**: Understand `Box<T>` before `Arc<RwLock<T>>`
2. **Read Error Messages**: Rust's compiler teaches you
3. **Use rust-analyzer**: Hover for types, inline hints
4. **Experiment**: Try removing `clone()` calls to understand ownership
5. **Read the Book**: "The Rust Programming Language" covers fundamentals

## Common Gotchas

1. **Deadlocks**: Taking multiple locks in different orders
2. **Async in Sync**: Don't `block_on` in async context
3. **Lifetime Confusion**: Start with owned data, optimize later
4. **Over-Engineering**: Not every problem needs advanced patterns

Remember: These patterns exist to solve real problems. Learn them as you encounter those problems, not all at once! 