# FAQ (Frequently Asked Questions)

## File-based routing

**Q: Can I start using the app without extensive initial setup?**    
A: Yes, absolutely ! You can get the mock server running quickly. Just run `npm install -D apimock-rs` followed by `npx apimock` (or execute the binary file after downloading it).

The directory where you run the command will be treated as the root directory for the server, and it will respond to HTTP requests by mapping the request paths to the relative paths of files (like `.json` files) within the root. The detail about File-based routing is [here](./getting-started/file-based-routing.md).

**Q: Can the server return responses for HTTP request paths that are directories, not specific files?**    
A: Yes. For requests like `/api/v1/myfunction` (without a file extension), rather than `/api/v1/my.json`, you can use an **"index" file** to provide a response.

If an `index.json`, `.json5`, `.csv`, or `.html` file exists within that directory, it will be served as the response for directory access. If none of the index files are present, the server will return `404 NOT FOUND`.

## Rule-based routing

**Q: I want to map specific HTTP requests to individual responses. Is this possible?**    
A: While not achievable with file-based routing alone, you can define custom rules in a configuration file to achieve it. How to create it is written below and the detail about Rule-based routing is [here](./getting-started/rule-based-routing.md).

**Q: How do I create a configuration file?**    
A: Simply run `npx apimock --init`. This command will generate a configuration file set in the current directory. You can then edit `apimock-rule-set.toml` to customize your routing rules.

**Q: Which option is supported as matching condition ?**    
A: You can use `url_path`, `method`, `headers` and `body.json` as conditions, which can be used both alone and combined with each other.

## Response via script

**Q: Can I dynamically generate responses ?**    
A: Yes, supported with rhai script to determine response file due to request condition. Moreover, custom JSON or text response body string is directly specified in script. Besides, static, file-based or rule-based responses are expected to fulfill most cases.

## Configuration

### Listener

**Q: Can I switch server port from the default ?**    
A: Yes. Two ways: run with `-p` | `--port` argument followed by specific port number. Alternatively, define it in `[listener]` section in `apimock.toml`, the root configuration.  (See [Configuration overview](../advanced-topics/configuration-overview.md).)

**Q: Can I let server listen to the external instead of localhost ?**    
A: Yes. See [listener](../advanced-topics/listener/README.md) in Advanced Topics.

**Q: Is IPv6 supported ?**    
A: Yes. See [listener](../advanced-topics/listener/README.md) in Advanced Topics.

### HTTPS support

**Q: Can I use https protocol ?**    
A: Yes.

**Q: Can I build servers to accept both of http and https ?**    
A: Yes.

The detail about HTTPS support is [here](./advanced-topics/listener/https-support.md).

## Architecture

**Q: How are rules loaded ?**    
A: At server startup.

**Q: How are response files loaded ?**    
A: At each response (via non-blocking file I/O).
