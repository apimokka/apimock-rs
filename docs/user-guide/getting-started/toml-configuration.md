# Understanding TOML Configuration

Your `apimock.toml` and `apimock-rule-set.toml` files use **TOML (Tom's Obvious, Minimal Language)**. If you're new to TOML, don't worry ! It's designed to be human-readable and easy to learn. Here are the essentials you'll need to know:

## Basic Syntax

### Key-Value Pairs

The most fundamental part.

```toml
key = "value"
number_key = 123
boolean_key = true
```

### Comments

Use the hash symbol (`#`) to add comments. Anything from `#` to the end of the line is ignored by the parser.

```toml
# This is a full-line comment
key = "value" # This is an end-of-line comment
```

## Tables (Dictionaries)

TOML uses tables (similar to dictionaries or objects in other languages) to group related key-value pairs.

### Inline Tables

For compact, small tables.

```toml
user = { name = "Alice", age = 30 }
```

### Standard Tables

Defined with `[table-name]`.

```toml
[listener]
ip_address = "127.0.0.1"
port = 8080
```

### Nested Tables

You can define tables within tables using dot notation.

```toml
[rules.when.request.headers]
user = { value = "user1" }
```

### Key Naming Flexibility

You can use hyphens (`-`) in key names, not just underscores (`_`). While you can enclose key names in quotes, it's often not necessary unless the key contains special characters or needs to start with a number.

```toml
api-key = "my-secret-token"
# Same as above, quotes are optional here
"api-key" = "my-secret-token"

# quotes are required here because the key contains the special character `.`
"a.b.c" = { value = "d" }
```

## Array of Tables (Lists of Dictionaries)

This is crucial for defining your rules ! Arrays of tables are used to create a list of similar objects. Each `[[table-name]]` defines a new item in the list.

```toml
[[rules]] # First rule in the list
when.request.url_path = "/home"
respond.text = "Hello, world"

[[rules]] # Second rule in the list
when.request.url_path = "/"
respond.text = "I'm at root"
```

## For more in-depth learning

You can refer to the [official TOML specification](https://toml.io/) or other popular guides.
