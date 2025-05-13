# Mongor 🚀

> A high-performance REST API for MongoDB, built with Rust and Actix-web

## Overview

Mongor provides a flexible REST API for MongoDB that allows you to interact with your MongoDB databases and collections through simple HTTP requests. It supports all standard CRUD operations and advanced MongoDB query features while delivering exceptional performance.

## Features

- **Complete CRUD Support**: Create, read, update and delete operations for MongoDB documents
- **Dynamic Collection Support**: Automatic endpoint generation for all collections
- **Query Features**: Full MongoDB query language support including projections, sorting, and aggregations
- **Authentication**: JWT-based authentication and role-based access control
- **WebSocket Support**: Real-time data subscriptions for collection changes
- **Swagger Documentation**: Interactive API documentation with OpenAPI/Swagger
- **High Performance**: Built with Rust for maximum efficiency and minimal resource usage

## Getting Started

### Prerequisites

- Rust 1.70+ (https://rustup.rs/)
- MongoDB 5.0+ instance
- (Optional) Docker for containerized deployment

### Installation

1. Clone the repository
```
git clone https://github.com/yourusername/mongor.git
cd mongor
```

2. Configure your MongoDB connection
```
cp .env.example .env
# Edit .env with your MongoDB connection details
```

3. Build and run
```
cargo build --release
./target/release/mongor
```

```

## API Usage Examples

### Basic CRUD Operations

**Create a document**
```
POST /api/v1/db/collection
Content-Type: application/json

{
  "name": "Example Item",
  "value": 42,
  "tags": ["sample", "demo"]
}
```

**Get all documents**
```
GET /api/v1/db/collection
```

**Get documents with filtering**
```
GET /api/v1/db/collection?query={"name":"Example Item"}
```

**Update a document**
```
PUT /api/v1/db/collection/document_id
Content-Type: application/json

{
  "value": 99,
  "updated": true
}
```

**Delete a document**
```
DELETE /api/v1/db/collection/document_id
```

### Advanced Features

**Projection (field selection)**
```
GET /api/v1/db/collection?fields={"name":1,"value":1}
```

**Sorting**
```
GET /api/v1/db/collection?sort={"value":-1}
```

**Pagination**
```
GET /api/v1/db/collection?limit=10&skip=20
```

**Aggregation Pipeline**
```
POST /api/v1/db/collection/aggregate
Content-Type: application/json

[
  { "$match": { "value": { "$gt": 10 } } },
  { "$group": { "_id": "$category", "total": { "$sum": "$value" } } }
]
```

## Configuration

Configuration is done through environment variables or a `.env` file:

| Variable | Description | Default |
|----------|-------------|---------|
| `MONGODB_URI` | MongoDB connection string | `mongodb://localhost:27017` |
| `API_PORT` | Port for the API server | `8080` |
| `LOG_LEVEL` | Logging level (error, warn, info, debug, trace) | `info` |
| `JWT_SECRET` | Secret for JWT token signing | (required) |
| `ENABLE_SWAGGER` | Enable Swagger documentation | `true` |

## Performance

Mongor delivers exceptional performance due to its Rust implementation and optimized database connection pooling:

- Handles 10,000+ requests/second on modest hardware
- Low memory footprint (typically
