# Response decision flow

A diagram view of [Matching order and precedence](./matching-order-and-precedence.md) — start there for the prose explanation and the code citations behind it.

```mermaid
flowchart TD
    A[Request received] --> B{Method is OPTIONS?}
    B -- yes --> B1[204 No Content<br/>CORS preflight headers]
    B -- no --> C{Any middleware<br/>answers it?}
    C -- yes --> C1[Middleware's response]
    C -- no --> D{Any rule set has<br/>a matching rule?}
    D -- yes --> D1[That rule's response,<br/>chosen by its strategy]
    D -- no --> E{File exists under<br/>fallback_respond_dir?}
    E -- yes --> E1[File content]
    E -- no --> F[404 Not Found]
```

Each middleware script is tried in the order it's listed; the first
one that returns a value wins. Each rule set is tried in the order
it's listed; the first one with a matching rule wins, and that rule
set's `strategy` decides which of its own matching rules answers. See
[Vary the response for one path](../guides/vary-the-response-for-one-path.md)
for the five strategies.
