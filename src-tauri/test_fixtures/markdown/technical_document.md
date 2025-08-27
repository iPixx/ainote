# Technical Documentation Sample

## Overview

This document serves as a comprehensive test fixture for evaluating the text chunking system's ability to handle technical documentation with various markdown structures.

### Key Features

- **API endpoints** with detailed specifications
- Code blocks in multiple languages
- Complex nested lists
- Mathematical formulas and technical notation

## API Reference

### Authentication

Before making any API calls, you must authenticate using the following endpoint:

```http
POST /api/auth/login
Content-Type: application/json

{
  "username": "your_username",
  "password": "your_password"
}
```

The response will include an authentication token:

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 3600,
  "user_id": "12345"
}
```

### User Management

#### Create User

Creates a new user account with the specified parameters.

**Endpoint:** `POST /api/users`

**Request Body:**
```json
{
  "username": "string",
  "email": "string",
  "password": "string",
  "role": "user|admin",
  "profile": {
    "first_name": "string",
    "last_name": "string",
    "bio": "string"
  }
}
```

**Response:**
```json
{
  "id": "string",
  "username": "string",
  "email": "string",
  "role": "string",
  "created_at": "2023-01-01T00:00:00Z",
  "profile": {
    "first_name": "string",
    "last_name": "string",
    "bio": "string"
  }
}
```

#### Get User

Retrieves user information by ID.

**Endpoint:** `GET /api/users/{id}`

**Parameters:**
- `id` (path parameter): The unique identifier for the user

**Response:**
- `200 OK`: User found and returned
- `404 Not Found`: User does not exist
- `403 Forbidden`: Insufficient permissions

## Code Examples

### Python Implementation

```python
import requests
import json
from datetime import datetime

class APIClient:
    def __init__(self, base_url: str, token: str = None):
        self.base_url = base_url.rstrip('/')
        self.token = token
        self.session = requests.Session()
        
        if self.token:
            self.session.headers.update({
                'Authorization': f'Bearer {self.token}'
            })
    
    def authenticate(self, username: str, password: str) -> dict:
        """Authenticate with the API and store the token."""
        endpoint = f"{self.base_url}/api/auth/login"
        payload = {
            "username": username,
            "password": password
        }
        
        response = self.session.post(endpoint, json=payload)
        response.raise_for_status()
        
        auth_data = response.json()
        self.token = auth_data.get('token')
        
        # Update session headers
        self.session.headers.update({
            'Authorization': f'Bearer {self.token}'
        })
        
        return auth_data
    
    def create_user(self, user_data: dict) -> dict:
        """Create a new user."""
        endpoint = f"{self.base_url}/api/users"
        response = self.session.post(endpoint, json=user_data)
        response.raise_for_status()
        return response.json()
    
    def get_user(self, user_id: str) -> dict:
        """Retrieve user by ID."""
        endpoint = f"{self.base_url}/api/users/{user_id}"
        response = self.session.get(endpoint)
        response.raise_for_status()
        return response.json()

# Example usage
if __name__ == "__main__":
    client = APIClient("https://api.example.com")
    
    # Authenticate
    auth_result = client.authenticate("admin", "password123")
    print(f"Authenticated successfully. Token expires in {auth_result['expires_in']} seconds")
    
    # Create a new user
    new_user = {
        "username": "john_doe",
        "email": "john@example.com",
        "password": "secure_password",
        "role": "user",
        "profile": {
            "first_name": "John",
            "last_name": "Doe",
            "bio": "Software engineer with 5+ years of experience"
        }
    }
    
    created_user = client.create_user(new_user)
    print(f"Created user: {created_user['username']} (ID: {created_user['id']})")
```

### JavaScript Implementation

```javascript
class APIClient {
    constructor(baseUrl, token = null) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.token = token;
    }
    
    async authenticate(username, password) {
        const response = await fetch(`${this.baseUrl}/api/auth/login`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ username, password })
        });
        
        if (!response.ok) {
            throw new Error(`Authentication failed: ${response.statusText}`);
        }
        
        const authData = await response.json();
        this.token = authData.token;
        return authData;
    }
    
    async createUser(userData) {
        const response = await fetch(`${this.baseUrl}/api/users`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${this.token}`
            },
            body: JSON.stringify(userData)
        });
        
        if (!response.ok) {
            throw new Error(`Failed to create user: ${response.statusText}`);
        }
        
        return await response.json();
    }
    
    async getUser(userId) {
        const response = await fetch(`${this.baseUrl}/api/users/${userId}`, {
            headers: {
                'Authorization': `Bearer ${this.token}`
            }
        });
        
        if (!response.ok) {
            throw new Error(`Failed to get user: ${response.statusText}`);
        }
        
        return await response.json();
    }
}

// Usage example
async function main() {
    const client = new APIClient('https://api.example.com');
    
    try {
        // Authenticate
        const authResult = await client.authenticate('admin', 'password123');
        console.log(`Authenticated successfully. Token expires in ${authResult.expires_in} seconds`);
        
        // Create user
        const newUser = {
            username: 'jane_doe',
            email: 'jane@example.com',
            password: 'secure_password',
            role: 'user',
            profile: {
                first_name: 'Jane',
                last_name: 'Doe',
                bio: 'Product manager with expertise in user experience'
            }
        };
        
        const createdUser = await client.createUser(newUser);
        console.log(`Created user: ${createdUser.username} (ID: ${createdUser.id})`);
        
    } catch (error) {
        console.error('Error:', error.message);
    }
}

main();
```

## Configuration

### Environment Variables

The following environment variables are required for the application to function properly:

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `API_BASE_URL` | Base URL for the API server | `http://localhost:3000` | No |
| `DATABASE_URL` | PostgreSQL connection string | - | Yes |
| `JWT_SECRET` | Secret key for JWT token generation | - | Yes |
| `REDIS_URL` | Redis connection string for caching | `redis://localhost:6379` | No |
| `LOG_LEVEL` | Logging level (debug, info, warn, error) | `info` | No |

### Database Schema

The application uses the following database schema:

```sql
-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) DEFAULT 'user',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- User profiles table
CREATE TABLE user_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    bio TEXT,
    avatar_url VARCHAR(500),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_user_profiles_user_id ON user_profiles(user_id);
```

## Testing

### Unit Tests

Run the complete test suite with:

```bash
# Run all tests
npm test

# Run tests with coverage
npm run test:coverage

# Run tests in watch mode
npm run test:watch
```

### Integration Tests

Integration tests validate the entire API workflow:

```bash
# Run integration tests
npm run test:integration

# Run integration tests against specific environment
NODE_ENV=staging npm run test:integration
```

### Performance Tests

Load testing ensures the API can handle expected traffic:

```bash
# Run performance tests
npm run test:performance

# Run stress tests
npm run test:stress
```

## Deployment

### Docker Deployment

1. Build the Docker image:

```bash
docker build -t api-server:latest .
```

2. Run the container:

```bash
docker run -d \
  --name api-server \
  -p 3000:3000 \
  -e DATABASE_URL="postgresql://user:password@db:5432/myapp" \
  -e JWT_SECRET="your-super-secret-key" \
  api-server:latest
```

### Kubernetes Deployment

Deploy to Kubernetes using the provided manifests:

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secret.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/ingress.yaml
```

## Monitoring and Logging

### Health Checks

The application provides health check endpoints:

- **Liveness probe:** `GET /health/live`
- **Readiness probe:** `GET /health/ready`
- **Detailed health:** `GET /health/detailed`

### Metrics

Prometheus metrics are available at `/metrics` endpoint:

- Request count and duration
- Database connection pool stats
- Memory and CPU usage
- Custom business metrics

### Logging

Structured logging is implemented using JSON format:

```json
{
  "timestamp": "2023-01-01T12:00:00.000Z",
  "level": "info",
  "message": "User created successfully",
  "userId": "12345",
  "username": "john_doe",
  "requestId": "abc-123-def"
}
```

## Troubleshooting

### Common Issues

1. **Authentication failures**
   - Verify JWT secret is configured correctly
   - Check token expiration times
   - Ensure user credentials are valid

2. **Database connection errors**
   - Verify DATABASE_URL format
   - Check network connectivity to database
   - Validate database credentials and permissions

3. **Performance issues**
   - Monitor database query performance
   - Check Redis cache hit rates
   - Analyze request patterns and optimize endpoints

### Error Codes

| Code | Description | Resolution |
|------|-------------|------------|
| E001 | Invalid authentication token | Refresh token or re-authenticate |
| E002 | User not found | Verify user ID exists in database |
| E003 | Insufficient permissions | Check user role and permissions |
| E004 | Validation error | Review request payload format |
| E005 | Database connection failed | Check database connectivity |

This comprehensive documentation provides detailed information about the API, implementation examples, configuration options, and operational procedures. The content is structured to facilitate effective text chunking while preserving semantic meaning and context across different sections.