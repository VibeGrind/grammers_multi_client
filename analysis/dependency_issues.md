# Dependency Issues

## Critical Issues

### 1. Missing Cargo.lock
**Location**: Root directory
**Severity**: HIGH
**Description**: Cargo.lock не существует в репозитории
**Impact**: Несогласованные версии зависимостей между сборками
**Fix**: Добавить Cargo.lock в git

### 2. Local Path Dependencies
**Location**: `Cargo.toml:19-21`
**Severity**: HIGH
**Description**: Зависимости указывают на локальные пути
```toml
grammers-client = { path = "../grammers-master/grammers-client" }
```
**Impact**: Код не переносим, не собирается на других машинах
**Fix**: Использовать git dependencies или опубликовать в crates.io

### 3. No Version Pinning
**Location**: `Cargo.toml`
**Severity**: MEDIUM
**Description**: Некоторые зависимости без точных версий
**Impact**: Breaking changes могут сломать сборку
**Fix**: Pin major versions

## Medium Issues

### 4. Very Early Phoenix Version
**Location**: `Cargo.toml:34`
**Severity**: MEDIUM
**Description**: phoenix_channels_client = "0.1" - очень ранняя версия
**Impact**: Возможно отсутствие важных фич и bugfixes
**Fix**: Проверить последнюю стабильную версию и обновить

### 5. Basic Logger
**Location**: `Cargo.toml:28`
**Severity**: MEDIUM
**Description**: simple_logger не имеет rotation, structured logging
**Impact**: Логи растут бесконечно, сложно парсить
**Fix**: Использовать tracing + tracing-subscriber

### 6. Old SQLite Library
**Location**: `Cargo.toml:31`
**Severity**: LOW
**Description**: sqlite = "0.37" - старая версия, не async
**Impact**: Blocking I/O, возможные баги
**Fix**: Использовать sqlx или tokio-rusqlite

### 7. Minimal Tokio Features
**Location**: `Cargo.toml:24`
**Severity**: LOW
**Description**: Минимальный набор features для Tokio
**Impact**: Может не хватить для некоторых операций
**Fix**: Добавить fs, process если нужны

### 8. No Error Handling Library
**Location**: Missing dependency
**Severity**: MEDIUM
**Description**: Нет thiserror или anyhow
**Impact**: Плохая обработка ошибок
**Fix**: Добавить thiserror для library errors, anyhow для application

### 9. No Async Trait Support
**Location**: Missing dependency
**Severity**: LOW
**Description**: Нет async-trait crate
**Impact**: Сложно создавать async traits
**Fix**: Добавить async-trait если нужны async traits

### 10. No Metrics/Telemetry
**Location**: Missing dependencies
**Severity**: LOW
**Description**: Нет метрик (prometheus, metrics crate)
**Impact**: Невозможно мониторить производительность
**Fix**: Добавить metrics crate

### 11. No Validation Library
**Location**: Missing dependency
**Severity**: LOW
**Description**: Нет validator или garde для валидации
**Impact**: Ручная валидация везде
**Fix**: Добавить validation library

**Total Issues**: 11 (2 High, 3 Medium, 6 Low)
