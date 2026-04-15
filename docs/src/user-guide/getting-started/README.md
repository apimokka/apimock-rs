# Getting started

It only takes a few steps to get your mock server up and running !

## System Requirements

* Node.js and npm (stable) only
    * (Optional) Without them, natively built executable is also [available](https://github.com/apimokka/apimock-rs/releases/latest)

## Installation

If not installed yet:

```sh
npm install -D apimock-rs
```

Note that the package name is `apimock-rs` (**ends with `-rs`**) and its command name below is `apimock`.

## Minimal Configuration

Nothing. Just JSON and go !

## Running the Server

```sh
npx apimock
```

Besides, if you use natively built executable, run `./apimock` instead.

## Test it

From another terminal (as web client), access the server:

```sh
curl -i http://localhost:3001/
```

Expected response: HTTP Status Code 404 (NOT FOUND)

This is correct, as no `.json` file exists on the server yet. Now, let's prepare for the next test. In the server terminal, run:

```sh
echo '{"hello": "world"}' > greetings.json

npx apimock
```

Then, access it again with:

```sh
curl http://localhost:3000/greetings
```

Expected response:

```json
{
  "hello": "world"
}
```

All set 😺 ?
