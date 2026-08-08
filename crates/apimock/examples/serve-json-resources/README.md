# Serve JSON resources from a folder

The headline feature: **drop JSON files into a folder and your API
immediately exists.** No rules, no rule set - `service.fallback_respond_dir`
is the only configuration this example needs. Every file under `data/`
becomes reachable at the URL path matching its location; the
extension is optional, and JSON, JSON5, and CSV are all recognised
automatically (CSV is converted to a JSON array of objects, one per
row, keyed by column header).

## Run it

```sh
cd crates/apimock/examples/serve-json-resources
apimock
```

## Try it

A JSON file at the root of `data/` becomes a collection endpoint:

```sh
$ curl http://127.0.0.1:3001/users
[{"email":"ada@example.com","id":1,"name":"Ada Lovelace"},{"email":"grace@example.com","id":2,"name":"Grace Hopper"}]
```

A JSON file in a subdirectory becomes a member endpoint - `data/users/1.json`
answers `/users/1`:

```sh
$ curl http://127.0.0.1:3001/users/1
{"email":"ada@example.com","id":1,"name":"Ada Lovelace"}
```

Another collection, `data/orders.json`:

```sh
$ curl http://127.0.0.1:3001/orders
[{"id":101,"total":42.5,"userId":1},{"id":102,"total":17.0,"userId":2}]
```

A CSV file converts to JSON automatically - `data/products.csv` answers
`/products` (extension optional) with each row keyed by its column
header, wrapped in `{"records": [...]}` (the default wrapper key; a
rule-based `respond.csv_records_key` can nest it elsewhere - see
`../match-headers-and-body/` for rule-based responses):

```sh
$ curl http://127.0.0.1:3001/products
{"records":[{"id":"1","name":"Widget","price":"9.99"},{"id":"2","name":"Gadget","price":"19.99"}]}
```

Note CSV values are always strings (`"9.99"`, not `9.99`) - the
conversion doesn't infer types.

## Why no rule set

This is the zero-configuration path: any request that doesn't match a
rule (or, as here, when there are no rules at all) falls back to
serving `service.fallback_respond_dir` directly by URL path. It's the
right starting point for "I just want to mock some JSON endpoints."
Once you need conditional matching - different responses per header,
body content, or HTTP method - see `../match-headers-and-body/`.
