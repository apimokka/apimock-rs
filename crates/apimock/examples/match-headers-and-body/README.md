# Match on headers and body content

A `POST /orders` endpoint whose response depends on a header's presence
and on fields inside the JSON request body - the pattern behind most
conditional mocking. Rules are listed most-specific-first, since the
default `first_match` strategy returns the first rule that matches.

This set shows a representative slice of the operator surface, not
every operator - `HeaderOperator::{exists,absent}` and
`BodyOperator::{equal,contains,greater_than}`, plus the dotted-path
mini-syntax for nested and array fields. It is not a full operator
reference.

## Run it

```sh
cd crates/apimock/examples/match-headers-and-body
apimock
```

## Try it

**No API key** - rejected before the body is even inspected
(`HeaderOperator::absent`):

```sh
$ curl -i -X POST http://127.0.0.1:3001/orders \
    -H 'Content-Type: application/json' -d '{"total": 10}'
HTTP/1.1 401 Unauthorized
missing x-api-key header
```

**VIP customer** - a nested body field, `customer.tier`
(`BodyOperator::equal`):

```sh
$ curl -X POST http://127.0.0.1:3001/orders \
    -H 'x-api-key: k1' -H 'Content-Type: application/json' \
    -d '{"customer":{"tier":"gold"},"total":10}'
VIP customer order
```

**A specific product** - `items.0.sku` indexes the first element of
the `items` array (`BodyOperator::contains`):

```sh
$ curl -X POST http://127.0.0.1:3001/orders \
    -H 'x-api-key: k1' -H 'Content-Type: application/json' \
    -d '{"items":[{"sku":"WIDGET-42"}],"total":10}'
widget order
```

**High-value order** - a numeric comparison on `total`
(`BodyOperator::greater_than`; body values are compared as strings in
the TOML but parsed numerically for this operator):

```sh
$ curl -X POST http://127.0.0.1:3001/orders \
    -H 'x-api-key: k1' -H 'Content-Type: application/json' \
    -d '{"total":150}'
high-value order, manual review required
```

**Everything else, authenticated** - the fallback rule:

```sh
$ curl -i -X POST http://127.0.0.1:3001/orders \
    -H 'x-api-key: k1' -H 'Content-Type: application/json' \
    -d '{"total":10}'
HTTP/1.1 201 Created
order created
```

## Body paths are not JSONPath

`"items.0.sku"` and `"customer.tier"` use the routing crate's own
dotted-path mini-syntax: object keys joined by `.`, numeric segments
index arrays. This is deliberately **not** canonical JSONPath - don't
write `"$.items[0].sku"` or `"$.customer.tier"`; the leading `$.` has
no special meaning and `[0]` bracket syntax isn't recognised.
