# Swec - Suckless Web Endpoint Checker

(Or "Services Workin', Eventually Checkin")

This is a *very* minimal tool that just checks statuses of web services and outputs them in a machine-readable format.

It can check HTTP(S) endpoints in two ways:
- by checking the HTTP status code
- by checking the response body for a string

## Usage

Swec takes a list of services as STDIN, and outputs the results as STDOUT. You can also specify a global timeout for all checks as an argument.

### Input
The input format consists of as many lines as there are services to check, with each line containing four space-separated values:
```
<name> <url> <check type> <check value>
```

The ckeck type can be:
- `code` - checks the HTTP status code
- `body` - checks the response body for a string

The check value is the value to check for, depending on the check type:
- for `code`, this is the expected HTTP status code (e.g. `200`)
- for `body`, this is the string to look for in the response body.

#### Example

```
google https://google.com code 200
github https://github.com body GitHub
```

### Output

The output format resembles the input format.
```
<name> <status> <response time>
```

The status can be:
- `ok` - the check was successful
- `fail` - the check failed
- `timeout` - the check timed out

The response time is the time it took to get a response from the service, in milliseconds.

#### Example

```
google ok 218
github ok 434
```

### Arguments

Timeout: `-t <timeout in milliseconds>`

The timeout is also the maximum total time per check (checks are done asynchronously). If the timeout is reached, the check is aborted and the status is set to `timeout`.

Repeat: `-r <repeat interval in milliseconds>`

This is the interval between checks.
- If not specified, all checks are done only once.
- If specified, the program will count the checking time into the interval. If the timeout is shorter than the interval, the program will run anyway (you have to worry about providing a short enough timeout).


## Upcoming features

- regex checking for response body
- Count number of occurences of regex
- Check when a signal is received (e.g. SIGUSR1)

## License

GPLv2
