# Architecture Issues

## Critical Issues

### 1. Monolithic run() Function
**Location**: `src/main.rs:90-336`
**Severity**: HIGH
**Description**: 246 строк кода в одной функции, делает все
**Impact**: Сложно тестировать, поддерживать и расширять
**Fix**: Разбить на отдельные модули и слои

### 2. No Separation of Concerns
**Location**: Throughout codebase
**Severity**: HIGH
**Description**: Business logic, I/O, logging смешаны вместе
**Impact**: Высокая связанность, низкая когезия
**Fix**: Разделить на presentation, business, data layers

### 3. Tight Coupling
**Location**: `src/main.rs` and `src/phoenix_bridge.rs`
**Severity**: MEDIUM
**Description**: Прямая зависимость между модулями
**Impact**: Сложно тестировать и изменять
**Fix**: Использовать traits и dependency injection

### 4. Hardcoded Constants
**Location**: `src/main.rs:10, 14-16`
**Severity**: MEDIUM
**Description**: Константы захардкожены в коде
```rust
const SESSION_FILE: &str = "session/my.session";
const DEFAULT_PHOENIX_URL: &str = "ws://localhost:4000/socket";
```
**Impact**: Невозможно изменить без перекомпиляции
**Fix**: Вынести в конфигурацию

### 5. No Configuration Layer
**Location**: Throughout
**Severity**: HIGH
**Description**: Конфигурация разбросана по коду
**Impact**: Сложно управлять настройками
**Fix**: Создать централизованный Config struct

## Medium Issues

### 6. Global Logger State
**Location**: `src/main.rs:92-96`
**Severity**: MEDIUM
**Description**: Глобальная инициализация логгера
**Impact**: Невозможно настроить или протестировать
**Fix**: Dependency injection для logger

### 7. No Abstraction for File System
**Location**: Session loading
**Severity**: MEDIUM
**Description**: Прямая работа с файловой системой
**Impact**: Сложно тестировать
**Fix**: Создать Storage trait

### 8. Mixed Responsibilities in load_session_data
**Location**: `src/main.rs:29-71`
**Severity**: MEDIUM
**Description**: Функция и читает файл и парсит данные
**Impact**: Нарушение SRP
**Fix**: Разделить на read и parse

### 9. No Dependency Injection
**Location**: Throughout
**Severity**: MEDIUM
**Description**: Зависимости создаются внутри функций
**Impact**: Сложно тестировать и менять реализации
**Fix**: Использовать DI pattern

### 10. No Repository Pattern
**Location**: Session data access
**Severity**: MEDIUM
**Description**: Прямой доступ к SQLite из business logic
**Impact**: Tight coupling с БД
**Fix**: Создать SessionRepository trait

## Low Issues

### 11. No Domain Models
**Location**: Throughout
**Severity**: MEDIUM
**Description**: Используются примитивные типы вместо domain models
**Impact**: Слабая типизация, ошибки во время выполнения
**Fix**: Создать NewType wrappers и domain structs

### 12. No Service Layer
**Location**: N/A
**Severity**: MEDIUM
**Description**: Отсутствует слой бизнес-логики
**Impact**: Логика размазана по коду
**Fix**: Создать service layer

### 13. No Event Bus
**Location**: Update processing
**Severity**: LOW
**Description**: Прямая передача обновлений
**Impact**: Сложно добавить новые обработчики
**Fix**: Использовать event bus pattern

### 14. No Adapter Pattern
**Location**: Phoenix integration
**Severity**: LOW
**Description**: Прямое использование phoenix_channels_client
**Impact**: Сложно заменить реализацию
**Fix**: Создать NotificationAdapter trait

### 15. No Factory Pattern
**Location**: Client creation
**Severity**: LOW
**Description**: Клиент создается прямо в коде
**Impact**: Сложная инициализация
**Fix**: Создать ClientFactory

### 16. Lack of Modularity
**Location**: Project structure
**Severity**: MEDIUM
**Description**: Только 2 файла для всего кода
**Impact**: Плохая организация кода
**Fix**: Разбить на модули: config, domain, infrastructure, application

### 17. No Command Pattern
**Location**: Update handling
**Severity**: LOW
**Description**: Обработка обновлений inline
**Impact**: Сложно добавить middleware или logging
**Fix**: Использовать command pattern

### 18. No Strategy Pattern
**Location**: Update serialization
**Severity**: LOW
**Description**: Один способ обработки всех обновлений
**Impact**: Сложно добавить разные стратегии
**Fix**: Использовать strategy pattern для разных типов обновлений

### 19. No Builder Pattern
**Location**: ConnectionParams, Config
**Severity**: LOW
**Description**: Сложная инициализация структур
**Impact**: Много параметров, легко ошибиться
**Fix**: Использовать builder pattern

### 20. No Observer Pattern
**Location**: Update notifications
**Severity**: LOW
**Description**: Один способ получения обновлений
**Impact**: Сложно добавить несколько подписчиков
**Fix**: Реализовать observer pattern

### 21. No Facade Pattern
**Location**: Client API
**Severity**: LOW
**Description**: Прямое использование низкоуровневых API
**Impact**: Сложный интерфейс для пользователя
**Fix**: Создать high-level facade

### 22. Missing Documentation
**Location**: Throughout
**Severity**: LOW
**Description**: Нет rustdoc комментариев
**Impact**: Сложно понять API
**Fix**: Добавить документацию

### 23. No Health Checks
**Location**: N/A
**Severity**: LOW
**Description**: Нет способа проверить здоровье сервиса
**Impact**: Сложно мониторить
**Fix**: Добавить health check endpoint/function

### 24. No Graceful Shutdown Handler
**Location**: `src/main.rs:237-240`
**Severity**: MEDIUM
**Description**: Только Ctrl+C обработка
**Impact**: Может потерять данные при shutdown
**Fix**: Добавить полноценный shutdown manager

**Total Issues**: 24 (3 High, 12 Medium, 9 Low)
