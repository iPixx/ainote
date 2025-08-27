# Large Document for Performance Testing

## Introduction

This document is designed to test the performance of the text chunking system with large content. It contains multiple sections with substantial content to evaluate processing speed, memory usage, and chunk quality at scale.

## Section 1: Software Architecture Patterns

### Model-View-Controller (MVC) Pattern

The Model-View-Controller pattern is a software design pattern that separates an application into three interconnected components. This separation helps manage complexity in large applications by dividing concerns and responsibilities.

#### Model Component

The Model represents the data and business logic of the application. It is responsible for:

- Data retrieval and storage
- Business rule validation
- State management
- Notification of state changes to observers

In modern applications, the Model often includes:

1. **Data Access Layer**: Handles database interactions, API calls, and data persistence
2. **Business Logic Layer**: Implements domain-specific rules and operations
3. **Domain Objects**: Represent entities and value objects in the application domain
4. **Service Layer**: Provides high-level operations and workflows

Example implementation in Python:

```python
class UserModel:
    def __init__(self, database_connection):
        self.db = database_connection
        self.observers = []
    
    def create_user(self, user_data):
        # Validate user data
        if not self._validate_user_data(user_data):
            raise ValueError("Invalid user data")
        
        # Save to database
        user_id = self.db.insert_user(user_data)
        
        # Notify observers
        self._notify_observers('user_created', user_id)
        
        return user_id
    
    def get_user(self, user_id):
        return self.db.get_user_by_id(user_id)
    
    def update_user(self, user_id, updates):
        if self._validate_updates(updates):
            self.db.update_user(user_id, updates)
            self._notify_observers('user_updated', user_id)
    
    def _validate_user_data(self, data):
        required_fields = ['email', 'username', 'password']
        return all(field in data for field in required_fields)
    
    def _validate_updates(self, updates):
        # Implement validation logic
        return True
    
    def add_observer(self, observer):
        self.observers.append(observer)
    
    def _notify_observers(self, event, data):
        for observer in self.observers:
            observer.notify(event, data)
```

#### View Component

The View is responsible for presenting data to the user and handling user interface interactions. It should be:

- Passive and focused on presentation
- Independent of business logic
- Responsive to model changes
- Capable of multiple representations of the same data

Key responsibilities include:

1. **Data Presentation**: Formatting and displaying information from the Model
2. **User Input Handling**: Capturing user interactions and passing them to the Controller
3. **UI State Management**: Managing visual states like loading, error, and success states
4. **Accessibility**: Ensuring the interface is accessible to all users

Example View implementation:

```javascript
class UserView {
    constructor(container) {
        this.container = container;
        this.controller = null;
    }
    
    setController(controller) {
        this.controller = controller;
    }
    
    render(userData) {
        const userElement = document.createElement('div');
        userElement.className = 'user-profile';
        userElement.innerHTML = `
            <div class="user-header">
                <img src="${userData.avatar}" alt="${userData.username}'s avatar" />
                <h2>${userData.username}</h2>
                <p>${userData.email}</p>
            </div>
            <div class="user-details">
                <p><strong>Joined:</strong> ${this.formatDate(userData.created_at)}</p>
                <p><strong>Role:</strong> ${userData.role}</p>
                <p><strong>Status:</strong> ${userData.active ? 'Active' : 'Inactive'}</p>
            </div>
            <div class="user-actions">
                <button id="edit-user-btn" class="btn-primary">Edit Profile</button>
                <button id="delete-user-btn" class="btn-danger">Delete User</button>
            </div>
        `;
        
        this.container.appendChild(userElement);
        this.bindEvents();
    }
    
    bindEvents() {
        const editBtn = document.getElementById('edit-user-btn');
        const deleteBtn = document.getElementById('delete-user-btn');
        
        editBtn.addEventListener('click', () => {
            this.controller.editUser();
        });
        
        deleteBtn.addEventListener('click', () => {
            if (confirm('Are you sure you want to delete this user?')) {
                this.controller.deleteUser();
            }
        });
    }
    
    showLoading() {
        this.container.innerHTML = '<div class="loading">Loading...</div>';
    }
    
    showError(message) {
        this.container.innerHTML = `<div class="error">Error: ${message}</div>`;
    }
    
    formatDate(dateString) {
        return new Date(dateString).toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'long',
            day: 'numeric'
        });
    }
}
```

#### Controller Component

The Controller acts as an intermediary between the Model and View, handling:

- User input processing
- Model updates based on user actions
- View updates based on model changes
- Application flow control

Modern Controller responsibilities:

1. **Request Handling**: Processing incoming requests and routing them appropriately
2. **Input Validation**: Ensuring user input is valid before passing to the Model
3. **Error Handling**: Managing and responding to errors from the Model or View
4. **Session Management**: Handling user authentication and authorization

Example Controller implementation:

```python
class UserController:
    def __init__(self, model, view):
        self.model = model
        self.view = view
        self.view.set_controller(self)
        self.model.add_observer(self)
    
    def create_user(self, user_data):
        try:
            self.view.show_loading()
            user_id = self.model.create_user(user_data)
            user = self.model.get_user(user_id)
            self.view.render(user)
        except ValueError as e:
            self.view.show_error(str(e))
        except Exception as e:
            self.view.show_error("An unexpected error occurred")
            self._log_error(e)
    
    def edit_user(self):
        # Open edit form
        self.view.show_edit_form()
    
    def update_user(self, user_id, updates):
        try:
            self.model.update_user(user_id, updates)
            updated_user = self.model.get_user(user_id)
            self.view.render(updated_user)
        except Exception as e:
            self.view.show_error("Failed to update user")
            self._log_error(e)
    
    def delete_user(self, user_id):
        try:
            self.model.delete_user(user_id)
            self.view.show_success("User deleted successfully")
        except Exception as e:
            self.view.show_error("Failed to delete user")
            self._log_error(e)
    
    def notify(self, event, data):
        """Observer method to handle model notifications"""
        if event == 'user_created':
            self._on_user_created(data)
        elif event == 'user_updated':
            self._on_user_updated(data)
    
    def _on_user_created(self, user_id):
        # Handle user creation event
        pass
    
    def _on_user_updated(self, user_id):
        # Handle user update event
        pass
    
    def _log_error(self, error):
        # Log error for debugging
        print(f"Error: {error}")
```

### Observer Pattern

The Observer pattern defines a one-to-many dependency between objects so that when one object changes state, all dependent objects are notified automatically. This pattern is particularly useful in implementing distributed event handling systems.

#### Key Components

1. **Subject (Observable)**: The object being observed
2. **Observer**: Objects that want to be notified of changes
3. **ConcreteSubject**: Specific implementation of the subject
4. **ConcreteObserver**: Specific implementation of observers

#### Implementation Example

```python
from abc import ABC, abstractmethod
from typing import List

class Observer(ABC):
    @abstractmethod
    def update(self, subject):
        pass

class Subject(ABC):
    def __init__(self):
        self._observers: List[Observer] = []
    
    def attach(self, observer: Observer):
        if observer not in self._observers:
            self._observers.append(observer)
    
    def detach(self, observer: Observer):
        if observer in self._observers:
            self._observers.remove(observer)
    
    def notify(self):
        for observer in self._observers:
            observer.update(self)

class StockPrice(Subject):
    def __init__(self, symbol: str, price: float):
        super().__init__()
        self._symbol = symbol
        self._price = price
    
    @property
    def price(self):
        return self._price
    
    @price.setter
    def price(self, new_price: float):
        self._price = new_price
        self.notify()
    
    @property
    def symbol(self):
        return self._symbol

class StockDisplay(Observer):
    def update(self, subject: StockPrice):
        print(f"Stock {subject.symbol} is now ${subject.price:.2f}")

class StockAlert(Observer):
    def __init__(self, threshold: float):
        self.threshold = threshold
    
    def update(self, subject: StockPrice):
        if subject.price > self.threshold:
            print(f"ALERT: {subject.symbol} exceeded threshold! Price: ${subject.price:.2f}")

# Usage example
stock = StockPrice("AAPL", 150.00)
display = StockDisplay()
alert = StockAlert(160.00)

stock.attach(display)
stock.attach(alert)

stock.price = 155.00  # Both observers notified
stock.price = 165.00  # Alert triggered
```

### Strategy Pattern

The Strategy pattern defines a family of algorithms, encapsulates each one, and makes them interchangeable. This pattern lets the algorithm vary independently from clients that use it.

#### Benefits

- Eliminates conditional statements in client code
- Makes algorithms reusable
- Easy to add new algorithms
- Promotes composition over inheritance

#### Example: Payment Processing

```python
from abc import ABC, abstractmethod

class PaymentStrategy(ABC):
    @abstractmethod
    def pay(self, amount: float) -> bool:
        pass

class CreditCardPayment(PaymentStrategy):
    def __init__(self, card_number: str, cvv: str, expiry_date: str):
        self.card_number = card_number
        self.cvv = cvv
        self.expiry_date = expiry_date
    
    def pay(self, amount: float) -> bool:
        print(f"Paid ${amount:.2f} using Credit Card ending in {self.card_number[-4:]}")
        # Implement credit card payment logic
        return True

class PayPalPayment(PaymentStrategy):
    def __init__(self, email: str):
        self.email = email
    
    def pay(self, amount: float) -> bool:
        print(f"Paid ${amount:.2f} using PayPal account {self.email}")
        # Implement PayPal payment logic
        return True

class BankTransferPayment(PaymentStrategy):
    def __init__(self, account_number: str, routing_number: str):
        self.account_number = account_number
        self.routing_number = routing_number
    
    def pay(self, amount: float) -> bool:
        print(f"Paid ${amount:.2f} using Bank Transfer from account {self.account_number}")
        # Implement bank transfer logic
        return True

class PaymentProcessor:
    def __init__(self, strategy: PaymentStrategy):
        self._strategy = strategy
    
    def set_strategy(self, strategy: PaymentStrategy):
        self._strategy = strategy
    
    def process_payment(self, amount: float) -> bool:
        return self._strategy.pay(amount)

# Usage
credit_card = CreditCardPayment("1234567890123456", "123", "12/25")
paypal = PayPalPayment("user@example.com")
bank_transfer = BankTransferPayment("9876543210", "123456789")

processor = PaymentProcessor(credit_card)
processor.process_payment(99.99)

processor.set_strategy(paypal)
processor.process_payment(149.99)

processor.set_strategy(bank_transfer)
processor.process_payment(299.99)
```

## Section 2: Database Design Principles

### Normalization

Database normalization is the process of organizing data in a database to reduce redundancy and improve data integrity. It involves decomposing tables to eliminate data anomalies and ensure data dependencies make sense.

#### First Normal Form (1NF)

A table is in 1NF if:
- All entries in any column are of the same data type
- Each column contains atomic values (no repeating groups)
- Each row is unique
- Order of rows and columns doesn't matter

**Example Violation:**
```sql
-- NOT in 1NF - multiple values in skills column
CREATE TABLE employees_bad (
    id INT PRIMARY KEY,
    name VARCHAR(100),
    skills VARCHAR(255) -- "Python, JavaScript, SQL"
);
```

**Corrected to 1NF:**
```sql
-- In 1NF - atomic values only
CREATE TABLE employees (
    id INT PRIMARY KEY,
    name VARCHAR(100)
);

CREATE TABLE employee_skills (
    employee_id INT,
    skill VARCHAR(50),
    PRIMARY KEY (employee_id, skill),
    FOREIGN KEY (employee_id) REFERENCES employees(id)
);
```

#### Second Normal Form (2NF)

A table is in 2NF if:
- It's in 1NF
- All non-key attributes are fully functionally dependent on the primary key

**Example Violation:**
```sql
-- NOT in 2NF - order_date depends only on order_id, not the full key
CREATE TABLE order_items_bad (
    order_id INT,
    product_id INT,
    quantity INT,
    order_date DATE,
    customer_name VARCHAR(100),
    PRIMARY KEY (order_id, product_id)
);
```

**Corrected to 2NF:**
```sql
CREATE TABLE orders (
    order_id INT PRIMARY KEY,
    order_date DATE,
    customer_name VARCHAR(100)
);

CREATE TABLE order_items (
    order_id INT,
    product_id INT,
    quantity INT,
    PRIMARY KEY (order_id, product_id),
    FOREIGN KEY (order_id) REFERENCES orders(order_id)
);
```

#### Third Normal Form (3NF)

A table is in 3NF if:
- It's in 2NF
- No transitive dependencies exist (non-key attributes don't depend on other non-key attributes)

**Example:**
```sql
-- 3NF compliant design
CREATE TABLE customers (
    customer_id INT PRIMARY KEY,
    customer_name VARCHAR(100),
    city_id INT,
    FOREIGN KEY (city_id) REFERENCES cities(city_id)
);

CREATE TABLE cities (
    city_id INT PRIMARY KEY,
    city_name VARCHAR(100),
    state_id INT,
    FOREIGN KEY (state_id) REFERENCES states(state_id)
);

CREATE TABLE states (
    state_id INT PRIMARY KEY,
    state_name VARCHAR(100),
    country_id INT,
    FOREIGN KEY (country_id) REFERENCES countries(country_id)
);
```

### Indexing Strategies

Database indexes are data structures that improve query performance by providing faster access paths to data. Understanding when and how to use indexes is crucial for optimal database performance.

#### Types of Indexes

1. **Clustered Index**: Physically orders table data
2. **Non-Clustered Index**: Points to data locations
3. **Composite Index**: Covers multiple columns
4. **Unique Index**: Ensures uniqueness
5. **Partial Index**: Covers subset of rows

#### Index Design Examples

```sql
-- Primary key creates clustered index automatically
CREATE TABLE users (
    user_id INT IDENTITY(1,1) PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    created_at DATETIME DEFAULT GETDATE(),
    is_active BIT DEFAULT 1
);

-- Composite index for common query pattern
CREATE INDEX IX_users_name_active 
ON users (last_name, first_name) 
WHERE is_active = 1;

-- Covering index includes all needed columns
CREATE INDEX IX_users_email_covering
ON users (email)
INCLUDE (user_id, username, created_at);

-- Partial index for frequently queried subset
CREATE INDEX IX_users_active_recent
ON users (created_at)
WHERE is_active = 1 AND created_at > DATEADD(MONTH, -6, GETDATE());
```

#### Query Optimization with Indexes

```sql
-- Query that benefits from composite index
SELECT user_id, username, email 
FROM users 
WHERE last_name = 'Smith' 
  AND first_name = 'John' 
  AND is_active = 1;

-- Query using covering index
SELECT user_id, username, created_at
FROM users
WHERE email = 'john.smith@example.com';

-- Range query using partial index
SELECT COUNT(*)
FROM users
WHERE is_active = 1
  AND created_at BETWEEN '2024-01-01' AND '2024-03-31';
```

### Transaction Management

ACID (Atomicity, Consistency, Isolation, Durability) properties ensure reliable database transactions even in the face of errors, power failures, or other mishaps.

#### Transaction Isolation Levels

```sql
-- Read Uncommitted - lowest isolation, highest concurrency
SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED;
BEGIN TRANSACTION;
SELECT balance FROM accounts WHERE account_id = 123;
COMMIT;

-- Read Committed - prevents dirty reads
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
BEGIN TRANSACTION;
SELECT balance FROM accounts WHERE account_id = 123;
-- Other transactions can modify this row
SELECT balance FROM accounts WHERE account_id = 123; -- May return different value
COMMIT;

-- Repeatable Read - prevents dirty reads and non-repeatable reads
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
BEGIN TRANSACTION;
SELECT balance FROM accounts WHERE account_id = 123;
-- Row is locked, other transactions cannot modify
SELECT balance FROM accounts WHERE account_id = 123; -- Returns same value
COMMIT;

-- Serializable - highest isolation, prevents phantom reads
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
BEGIN TRANSACTION;
SELECT COUNT(*) FROM accounts WHERE balance > 1000;
-- Range is locked, no new qualifying rows can be inserted
SELECT COUNT(*) FROM accounts WHERE balance > 1000; -- Returns same count
COMMIT;
```

#### Practical Transaction Examples

```sql
-- Bank transfer with proper error handling
BEGIN TRY
    BEGIN TRANSACTION;
    
    DECLARE @source_balance DECIMAL(10,2);
    DECLARE @transfer_amount DECIMAL(10,2) = 500.00;
    
    -- Lock source account and check balance
    SELECT @source_balance = balance 
    FROM accounts WITH (UPDLOCK)
    WHERE account_id = 123;
    
    IF @source_balance < @transfer_amount
    BEGIN
        THROW 50001, 'Insufficient funds', 1;
    END
    
    -- Perform transfer
    UPDATE accounts 
    SET balance = balance - @transfer_amount,
        last_modified = GETDATE()
    WHERE account_id = 123;
    
    UPDATE accounts 
    SET balance = balance + @transfer_amount,
        last_modified = GETDATE()
    WHERE account_id = 456;
    
    -- Log transaction
    INSERT INTO transaction_log (from_account, to_account, amount, transaction_date)
    VALUES (123, 456, @transfer_amount, GETDATE());
    
    COMMIT TRANSACTION;
    PRINT 'Transfer completed successfully';
    
END TRY
BEGIN CATCH
    IF @@TRANCOUNT > 0
        ROLLBACK TRANSACTION;
    
    DECLARE @ErrorMessage NVARCHAR(4000) = ERROR_MESSAGE();
    DECLARE @ErrorSeverity INT = ERROR_SEVERITY();
    DECLARE @ErrorState INT = ERROR_STATE();
    
    RAISERROR(@ErrorMessage, @ErrorSeverity, @ErrorState);
END CATCH
```

## Section 3: Performance Optimization Techniques

### Algorithm Optimization

Choosing the right algorithm can dramatically impact performance. Understanding time and space complexity helps make informed decisions about algorithm selection.

#### Sorting Algorithms Comparison

```python
import time
import random
from typing import List

def bubble_sort(arr: List[int]) -> List[int]:
    """
    Bubble Sort - O(n²) time complexity
    Simple but inefficient for large datasets
    """
    n = len(arr)
    for i in range(n):
        for j in range(0, n - i - 1):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr

def merge_sort(arr: List[int]) -> List[int]:
    """
    Merge Sort - O(n log n) time complexity
    Divide and conquer approach, stable sort
    """
    if len(arr) <= 1:
        return arr
    
    mid = len(arr) // 2
    left = merge_sort(arr[:mid])
    right = merge_sort(arr[mid:])
    
    return merge(left, right)

def merge(left: List[int], right: List[int]) -> List[int]:
    result = []
    i = j = 0
    
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            result.append(left[i])
            i += 1
        else:
            result.append(right[j])
            j += 1
    
    result.extend(left[i:])
    result.extend(right[j:])
    return result

def quick_sort(arr: List[int]) -> List[int]:
    """
    Quick Sort - O(n log n) average, O(n²) worst case
    In-place sorting with good cache performance
    """
    if len(arr) <= 1:
        return arr
    
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    
    return quick_sort(left) + middle + quick_sort(right)

def heap_sort(arr: List[int]) -> List[int]:
    """
    Heap Sort - O(n log n) time complexity
    Not stable but guarantees O(n log n) worst case
    """
    def heapify(arr, n, i):
        largest = i
        left = 2 * i + 1
        right = 2 * i + 2
        
        if left < n and arr[left] > arr[largest]:
            largest = left
        
        if right < n and arr[right] > arr[largest]:
            largest = right
        
        if largest != i:
            arr[i], arr[largest] = arr[largest], arr[i]
            heapify(arr, n, largest)
    
    n = len(arr)
    
    # Build max heap
    for i in range(n // 2 - 1, -1, -1):
        heapify(arr, n, i)
    
    # Extract elements from heap
    for i in range(n - 1, 0, -1):
        arr[0], arr[i] = arr[i], arr[0]
        heapify(arr, i, 0)
    
    return arr

# Performance testing
def benchmark_sorts(size: int = 1000):
    """Compare sorting algorithm performance"""
    # Generate test data
    original_data = [random.randint(1, 1000) for _ in range(size)]
    
    algorithms = [
        ("Bubble Sort", bubble_sort),
        ("Merge Sort", merge_sort),
        ("Quick Sort", quick_sort),
        ("Heap Sort", heap_sort),
        ("Built-in Sort", lambda x: sorted(x))
    ]
    
    print(f"Sorting {size} random integers:")
    print("-" * 50)
    
    for name, algorithm in algorithms:
        test_data = original_data.copy()
        start_time = time.time()
        
        if name == "Bubble Sort" and size > 10000:
            # Skip bubble sort for large datasets
            print(f"{name:15}: Skipped (too slow for large data)")
            continue
        
        sorted_data = algorithm(test_data)
        end_time = time.time()
        
        elapsed = (end_time - start_time) * 1000  # Convert to milliseconds
        print(f"{name:15}: {elapsed:.2f} ms")

# Example usage
if __name__ == "__main__":
    benchmark_sorts(1000)
    benchmark_sorts(10000)
```

#### Data Structure Selection

```python
import time
import sys
from collections import deque, defaultdict, Counter
from typing import Dict, List, Set

class PerformanceComparison:
    def __init__(self):
        self.results = {}
    
    def compare_list_vs_set_lookup(self, size: int = 10000):
        """Compare lookup performance: list vs set"""
        # Create test data
        data = list(range(size))
        data_set = set(data)
        search_items = [size // 4, size // 2, size - 1]  # Various positions
        
        # List lookup
        start_time = time.time()
        for item in search_items:
            item in data  # O(n) lookup
        list_time = time.time() - start_time
        
        # Set lookup
        start_time = time.time()
        for item in search_items:
            item in data_set  # O(1) average lookup
        set_time = time.time() - start_time
        
        print(f"Lookup in {size} items:")
        print(f"  List: {list_time * 1000:.4f} ms")
        print(f"  Set:  {set_time * 1000:.4f} ms")
        print(f"  Set is {list_time / set_time:.1f}x faster")
    
    def compare_string_concatenation(self, iterations: int = 10000):
        """Compare string concatenation methods"""
        test_string = "Hello World! "
        
        # Method 1: += operator
        start_time = time.time()
        result1 = ""
        for _ in range(iterations):
            result1 += test_string
        concat_time = time.time() - start_time
        
        # Method 2: join() method
        start_time = time.time()
        parts = []
        for _ in range(iterations):
            parts.append(test_string)
        result2 = "".join(parts)
        join_time = time.time() - start_time
        
        # Method 3: f-strings in loop (inefficient example)
        start_time = time.time()
        result3 = ""
        for _ in range(iterations):
            result3 = f"{result3}{test_string}"
        fstring_time = time.time() - start_time
        
        print(f"String concatenation ({iterations} iterations):")
        print(f"  += operator: {concat_time * 1000:.2f} ms")
        print(f"  join():      {join_time * 1000:.2f} ms")
        print(f"  f-strings:   {fstring_time * 1000:.2f} ms")
        
        assert result1 == result2  # Verify results are identical
    
    def compare_dictionary_methods(self, size: int = 100000):
        """Compare different dictionary usage patterns"""
        # Test data
        keys = [f"key_{i}" for i in range(size)]
        
        # Method 1: dict.get() with default
        test_dict = {}
        start_time = time.time()
        for key in keys:
            value = test_dict.get(key, 0)
            test_dict[key] = value + 1
        get_time = time.time() - start_time
        
        # Method 2: try/except KeyError
        test_dict = {}
        start_time = time.time()
        for key in keys:
            try:
                test_dict[key] += 1
            except KeyError:
                test_dict[key] = 1
        except_time = time.time() - start_time
        
        # Method 3: defaultdict
        test_dict = defaultdict(int)
        start_time = time.time()
        for key in keys:
            test_dict[key] += 1
        defaultdict_time = time.time() - start_time
        
        # Method 4: Counter
        start_time = time.time()
        counter = Counter(keys)
        counter_time = time.time() - start_time
        
        print(f"Dictionary operations ({size} items):")
        print(f"  dict.get():   {get_time * 1000:.2f} ms")
        print(f"  try/except:   {except_time * 1000:.2f} ms")
        print(f"  defaultdict:  {defaultdict_time * 1000:.2f} ms")
        print(f"  Counter:      {counter_time * 1000:.2f} ms")

# Memory usage comparison
def compare_memory_usage():
    """Compare memory usage of different data structures"""
    size = 100000
    
    # List of integers
    int_list = list(range(size))
    list_size = sys.getsizeof(int_list) + sum(sys.getsizeof(i) for i in int_list)
    
    # Set of integers
    int_set = set(range(size))
    set_size = sys.getsizeof(int_set) + sum(sys.getsizeof(i) for i in int_set)
    
    # Dictionary
    int_dict = {i: i for i in range(size)}
    dict_size = sys.getsizeof(int_dict)
    dict_size += sum(sys.getsizeof(k) + sys.getsizeof(v) for k, v in int_dict.items())
    
    print(f"Memory usage for {size} integers:")
    print(f"  List: {list_size / 1024 / 1024:.2f} MB")
    print(f"  Set:  {set_size / 1024 / 1024:.2f} MB")
    print(f"  Dict: {dict_size / 1024 / 1024:.2f} MB")

# Example usage
if __name__ == "__main__":
    perf = PerformanceComparison()
    
    print("Performance Comparison Results")
    print("=" * 50)
    
    perf.compare_list_vs_set_lookup()
    print()
    
    perf.compare_string_concatenation()
    print()
    
    perf.compare_dictionary_methods()
    print()
    
    compare_memory_usage()
```

This large document continues with extensive content designed to test the chunking system's performance with substantial text volumes while maintaining semantic coherence across different technical topics and code examples.