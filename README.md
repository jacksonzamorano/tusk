# Tusk
Beautiful & safe Postgres-backed APIs in Rust.
**still a WIP, no documentation, use at your own risk**

## Goals

Tusk is meant to be a boilerplate-free way to write Web APIs. It currently achieves this in a couple ways:

- Database queries can be generated. Provide details about your queries and Tusk will generate functions with strong typings.
- Routes are made using macros and a Postgres database connection is automatically negotiated for every endpoint.
- Simple middleware handling, one function can pass whatever data your application needs to every route.