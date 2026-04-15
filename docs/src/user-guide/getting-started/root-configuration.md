# Root configuration

## Your first configuration files

Let's create your configuration files. You don't need to open a text editor just yet ! Instead, use the `--init` argument with the `npx apimock` command:

```sh
npx apimock --init
```

After running this, you'll see "created" printed in your terminal, and you'll find two new configuration files in your project:

- `apimock.toml`
- `apimock-rule-set.toml`

## Running the Server with Configuration

Now that you have your configuration files, let's try running the server !

```sh
npx apimock
```

You'll notice a line in the terminal like: "`@ rule_set #1 (./apimock-rule-set.toml)`". This indicates that the **example rule set** is now active and routing requests.

### Moving Configuration Files

You have the flexibility to move these configuration files. For example, if you place them in a `tests/` directory:

```
your_project/
├── tests/
│   ├── apimock.toml
│   └── apimock-rule-set.toml
└── ...
```

You can tell the server where to find your `apimock.toml` file using either the `-c` or `--config` argument:

```sh
npx apimock -c tests/apimock.toml
```

**Important:** If `apimock.toml` references `apimock-rule-set.toml` (perhaps by default), the rule set path is interpreted **relative to `apimock.toml` itself**, not from your current working directory. Keep this in mind when organizing your files.

You're now ready to define powerful, rule-based dynamic routing !
