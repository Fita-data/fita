# 0003. Airbyte protocol for connectors

Status: accepted

## Context
Writing connectors for every application is not sustainable. A large open catalog already exists.

## Decision
Fita implements the runner side of the Airbyte protocol (spec, check, discover, read, RECORD and STATE messages). Database and file connectors are native Rust inside the binary but follow the same internal interface.

## Consequences
Community connectors work unchanged. Native connectors avoid containers for the most common sources. We depend on the protocol staying stable.
