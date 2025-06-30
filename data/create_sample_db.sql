-- Create sample database tables for demo

-- Sensor telemetry data
CREATE TABLE IF NOT EXISTS sensor_telemetry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    sensor_id VARCHAR(20),
    temperature REAL,
    humidity REAL,
    pressure REAL,
    vibration_x REAL,
    vibration_y REAL,
    vibration_z REAL,
    power_consumption REAL,
    status VARCHAR(20)
);

-- Financial transactions
CREATE TABLE IF NOT EXISTS transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    transaction_date DATE,
    account_id VARCHAR(20),
    merchant VARCHAR(100),
    category VARCHAR(50),
    amount DECIMAL(10,2),
    balance DECIMAL(10,2),
    transaction_type VARCHAR(20),
    location VARCHAR(100)
);

-- Production metrics
CREATE TABLE IF NOT EXISTS production_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME,
    line_id VARCHAR(20),
    products_made INTEGER,
    defects INTEGER,
    efficiency REAL,
    downtime_minutes INTEGER,
    shift VARCHAR(10),
    operator_id VARCHAR(20)
);

-- Generate sample sensor data
WITH RECURSIVE generate_series(value) AS (
    SELECT 0
    UNION ALL
    SELECT value + 1 FROM generate_series WHERE value < 10000
)
INSERT INTO sensor_telemetry (timestamp, sensor_id, temperature, humidity, pressure, 
                            vibration_x, vibration_y, vibration_z, power_consumption, status)
SELECT 
    datetime('2024-01-01 00:00:00', '+' || value || ' seconds'),
    'SENSOR_' || (value % 5 + 1),
    20.0 + 5.0 * sin(value * 0.01) + (random() % 100) * 0.02,
    40.0 + 10.0 * cos(value * 0.01) + (random() % 100) * 0.05,
    1013.25 + 2.0 * sin(value * 0.001) + (random() % 100) * 0.01,
    sin(value * 0.1) * (1 + (random() % 100) * 0.001),
    cos(value * 0.1) * (1 + (random() % 100) * 0.001),
    sin(value * 0.05) * cos(value * 0.05) * (1 + (random() % 100) * 0.001),
    100.0 + 20.0 * sin(value * 0.001) + (random() % 100) * 0.1,
    CASE 
        WHEN (random() % 100) < 95 THEN 'OK'
        WHEN (random() % 100) < 98 THEN 'WARNING'
        ELSE 'ERROR'
    END
FROM generate_series;

-- Generate sample transaction data
WITH RECURSIVE generate_dates(value) AS (
    SELECT 0
    UNION ALL
    SELECT value + 1 FROM generate_dates WHERE value < 5000
)
INSERT INTO transactions (transaction_date, account_id, merchant, category, 
                        amount, balance, transaction_type, location)
SELECT 
    date('2024-01-01', '+' || value || ' hours'),
    'ACC_' || (abs(random()) % 100 + 1),
    CASE (abs(random()) % 10)
        WHEN 0 THEN 'Amazon'
        WHEN 1 THEN 'Walmart'
        WHEN 2 THEN 'Starbucks'
        WHEN 3 THEN 'Shell Gas'
        WHEN 4 THEN 'Target'
        WHEN 5 THEN 'Apple Store'
        WHEN 6 THEN 'Netflix'
        WHEN 7 THEN 'Uber'
        WHEN 8 THEN 'Whole Foods'
        ELSE 'Local Store'
    END,
    CASE (abs(random()) % 8)
        WHEN 0 THEN 'Groceries'
        WHEN 1 THEN 'Entertainment'
        WHEN 2 THEN 'Transport'
        WHEN 3 THEN 'Shopping'
        WHEN 4 THEN 'Dining'
        WHEN 5 THEN 'Bills'
        WHEN 6 THEN 'Healthcare'
        ELSE 'Other'
    END,
    round((abs(random()) % 50000) / 100.0, 2),
    round(10000.0 + (value * 10) + (random() % 10000) / 100.0, 2),
    CASE (abs(random()) % 3)
        WHEN 0 THEN 'Purchase'
        WHEN 1 THEN 'Withdrawal'
        ELSE 'Deposit'
    END,
    CASE (abs(random()) % 5)
        WHEN 0 THEN 'New York'
        WHEN 1 THEN 'Los Angeles'
        WHEN 2 THEN 'Chicago'
        WHEN 3 THEN 'Houston'
        ELSE 'Online'
    END
FROM generate_dates;

-- Generate production metrics
WITH RECURSIVE generate_hours(value) AS (
    SELECT 0
    UNION ALL
    SELECT value + 1 FROM generate_hours WHERE value < 2000
)
INSERT INTO production_metrics (timestamp, line_id, products_made, defects, 
                              efficiency, downtime_minutes, shift, operator_id)
SELECT 
    datetime('2024-01-01 00:00:00', '+' || value || ' hours'),
    'LINE_' || (value % 4 + 1),
    100 + (abs(random()) % 50),
    abs(random()) % 10,
    70.0 + 20.0 * sin(value * 0.1) + (random() % 100) * 0.1,
    CASE WHEN (random() % 100) < 90 THEN 0 ELSE (abs(random()) % 60) END,
    CASE ((value / 8) % 3)
        WHEN 0 THEN 'Morning'
        WHEN 1 THEN 'Evening'
        ELSE 'Night'
    END,
    'OP_' || (abs(random()) % 20 + 1)
FROM generate_hours;

-- Create indexes for better performance
CREATE INDEX idx_sensor_timestamp ON sensor_telemetry(timestamp);
CREATE INDEX idx_sensor_id ON sensor_telemetry(sensor_id);
CREATE INDEX idx_transaction_date ON transactions(transaction_date);
CREATE INDEX idx_account_id ON transactions(account_id);
CREATE INDEX idx_production_timestamp ON production_metrics(timestamp);
CREATE INDEX idx_line_id ON production_metrics(line_id); 