//! Create sample SQLite database with demo data

use rusqlite::{Connection, Result};

/// Create and populate sample database
pub fn create_sample_database() -> Result<()> {
    let conn = Connection::open("data/sample_analytics.db")?;
    
    // Create tables
    conn.execute_batch(
        "
        -- Sensor telemetry data
        CREATE TABLE IF NOT EXISTS sensor_telemetry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            sensor_id TEXT,
            temperature REAL,
            humidity REAL,
            pressure REAL,
            vibration_x REAL,
            vibration_y REAL,
            vibration_z REAL,
            power_consumption REAL,
            status TEXT
        );
        
        -- Financial transactions
        CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            transaction_date TEXT,
            account_id TEXT,
            merchant TEXT,
            category TEXT,
            amount REAL,
            balance REAL,
            transaction_type TEXT,
            location TEXT
        );
        
        -- Production metrics
        CREATE TABLE IF NOT EXISTS production_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT,
            line_id TEXT,
            products_made INTEGER,
            defects INTEGER,
            efficiency REAL,
            downtime_minutes INTEGER,
            shift TEXT,
            operator_id TEXT
        );
        "
    )?;
    
    // Generate and insert sensor data
    println!("Generating sensor telemetry data...");
    generate_sensor_data(&conn)?;
    
    // Generate and insert transaction data
    println!("Generating transaction data...");
    generate_transaction_data(&conn)?;
    
    // Generate and insert production data
    println!("Generating production metrics...");
    generate_production_data(&conn)?;
    
    // Create indexes
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_sensor_timestamp ON sensor_telemetry(timestamp);
        CREATE INDEX IF NOT EXISTS idx_sensor_id ON sensor_telemetry(sensor_id);
        CREATE INDEX IF NOT EXISTS idx_transaction_date ON transactions(transaction_date);
        CREATE INDEX IF NOT EXISTS idx_account_id ON transactions(account_id);
        CREATE INDEX IF NOT EXISTS idx_production_timestamp ON production_metrics(timestamp);
        CREATE INDEX IF NOT EXISTS idx_line_id ON production_metrics(line_id);
        "
    )?;
    
    println!("Sample database created successfully!");
    Ok(())
}

fn generate_sensor_data(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO sensor_telemetry (timestamp, sensor_id, temperature, humidity, pressure, 
                                     vibration_x, vibration_y, vibration_z, power_consumption, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    )?;
    
    let mut rng = 42u32;
    let base_time = chrono::NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    
    for i in 0..10000 {
        let timestamp = base_time + chrono::Duration::seconds(i);
        let sensor_id = format!("SENSOR_{}", (i % 5) + 1);
        
        // Generate sensor values
        let t = i as f64 * 0.01;
        let temperature = 20.0 + 5.0 * (t).sin() + random_float(&mut rng) * 0.2;
        let humidity = 40.0 + 10.0 * (t).cos() + random_float(&mut rng) * 0.5;
        let pressure = 1013.25 + 2.0 * (t * 0.1).sin() + random_float(&mut rng) * 0.1;
        let vibration_x = (t * 10.0).sin() * (1.0 + random_float(&mut rng) * 0.01);
        let vibration_y = (t * 10.0).cos() * (1.0 + random_float(&mut rng) * 0.01);
        let vibration_z = (t * 5.0).sin() * (t * 5.0).cos() * (1.0 + random_float(&mut rng) * 0.01);
        let power_consumption = 100.0 + 20.0 * (t * 0.1).sin() + random_float(&mut rng);
        
        let status = if random_float(&mut rng) < 0.95 {
            "OK"
        } else if random_float(&mut rng) < 0.7 {
            "WARNING"
        } else {
            "ERROR"
        };
        
        stmt.execute((
            timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            sensor_id,
            temperature,
            humidity,
            pressure,
            vibration_x,
            vibration_y,
            vibration_z,
            power_consumption,
            status,
        ))?;
        
        if i % 1000 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }
    
    println!();
    Ok(())
}

fn generate_transaction_data(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO transactions (transaction_date, account_id, merchant, category, 
                                 amount, balance, transaction_type, location)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    )?;
    
    let mut rng = 12345u32;
    let base_date = chrono::NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
    
    let merchants = vec![
        "Amazon", "Walmart", "Starbucks", "Shell Gas", "Target",
        "Apple Store", "Netflix", "Uber", "Whole Foods", "Local Store"
    ];
    
    let categories = vec![
        "Groceries", "Entertainment", "Transport", "Shopping",
        "Dining", "Bills", "Healthcare", "Other"
    ];
    
    let locations = vec!["New York", "Los Angeles", "Chicago", "Houston", "Online"];
    
    let mut balances = vec![10000.0; 100]; // Track balance per account
    
    for i in 0..5000 {
        let date = base_date + chrono::Duration::hours(i);
        let account_idx = (random_int(&mut rng) % 100) as usize;
        let account_id = format!("ACC_{}", account_idx + 1);
        
        let merchant = merchants[random_int(&mut rng) as usize % merchants.len()];
        let category = categories[random_int(&mut rng) as usize % categories.len()];
        let location = locations[random_int(&mut rng) as usize % locations.len()];
        
        let transaction_type = match random_int(&mut rng) % 3 {
            0 => "Purchase",
            1 => "Withdrawal",
            _ => "Deposit",
        };
        
        let amount = (random_int(&mut rng) % 50000) as f64 / 100.0;
        
        // Update balance
        if transaction_type == "Deposit" {
            balances[account_idx] += amount;
        } else {
            balances[account_idx] -= amount;
        }
        
        stmt.execute((
            date.format("%Y-%m-%d").to_string(),
            account_id,
            merchant,
            category,
            amount,
            balances[account_idx],
            transaction_type,
            location,
        ))?;
        
        if i % 500 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }
    
    println!();
    Ok(())
}

fn generate_production_data(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "INSERT INTO production_metrics (timestamp, line_id, products_made, defects, 
                                       efficiency, downtime_minutes, shift, operator_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    )?;
    
    let mut rng = 98765u32;
    let base_time = chrono::NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    
    for i in 0..2000 {
        let timestamp = base_time + chrono::Duration::hours(i);
        let line_id = format!("LINE_{}", (i % 4) + 1);
        
        let t = i as f64 * 0.1;
        let products_made = 100 + (random_int(&mut rng) % 50) as i32;
        let defects = (random_int(&mut rng) % 10) as i32;
        let efficiency = 70.0 + 20.0 * (t).sin() + random_float(&mut rng);
        let downtime_minutes = if random_float(&mut rng) < 0.9 {
            0
        } else {
            (random_int(&mut rng) % 60) as i32
        };
        
        let shift = match (i / 8) % 3 {
            0 => "Morning",
            1 => "Evening",
            _ => "Night",
        };
        
        let operator_id = format!("OP_{}", (random_int(&mut rng) % 20) + 1);
        
        stmt.execute((
            timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            line_id,
            products_made,
            defects,
            efficiency,
            downtime_minutes,
            shift,
            operator_id,
        ))?;
        
        if i % 200 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
    }
    
    println!();
    Ok(())
}

fn random_float(seed: &mut u32) -> f64 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed as f64) / (u32::MAX as f64)
}

fn random_int(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed
} 