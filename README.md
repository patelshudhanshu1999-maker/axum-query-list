# axum-query-list

A Rust crate that provides `QueryList` and `OptionalQueryList` extractors for [axum](https://github.com/tokio-rs/axum).

Deserialize repeated query parameters into `Vec<Enum>` — something axum's built-in `Query` extractor doesn't support.

## The Problem

axum's `Query<T>` works great for flat structs, but fails for `Vec<Enum>`:

```rust
// This works ✅
Query<Pagination>

// This fails ❌
Query<Vec<Filter>>
```

## Solution

```rust
use axum_query_list::QueryList;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Filter {
    Id(u32),
    Username(String),
}

// GET /search?id=123&username=alice&id=456
// Receives: [Id(123), Username("alice"), Id(456)]
async fn search(QueryList(filters): QueryList<Filter>) {
    for filter in filters {
        match filter {
            Filter::Id(id) => println!("Filter by ID: {}", id),
            Filter::Username(name) => println!("Filter by user: {}", name),
        }
    }
}
```

## Optional Cap

Limit total query parameters using const generics:

```rust
// Max 10 items allowed
async fn search(QueryList(filters): QueryList<Filter, 10>) {
    // more than 10 params → 400 Bad Request
}
```

## Installation

```toml
[dependencies]
axum-query-list = "0.1.0"
```

## Extractors

| Extractor | Empty query | Cap support |
|---|---|---|
| `QueryList<T>` | Error | Optional |
| `QueryList<T, MAX>` | Error | Yes |
| `OptionalQueryList<T>` | Empty Vec | Optional |
| `OptionalQueryList<T, MAX>` | Empty Vec | Yes |

## Background

This crate was suggested by [davidpdrsn](https://github.com/davidpdrsn) (axum creator) as a standalone alternative to adding more extractors directly into axum-extra.

## License

MIT