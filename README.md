# Java Method Finder (JMF)

A Rust command-line tool to find method invocations in Java bytecode. JMF helps you locate where specific methods are being called across your Java project.

## Features

- Find all invocations of a specific method in a given class
- Support scanning compiled Java class files
- Parallel processing for better performance
- Provide detailed output with line numbers
- Multiple output formats (TXT, JSON)
- Optional verbose mode for debugging
- Easy-to-use command-line interface

## Installation

1. Clone the repository:

```bash
git clone https://github.com/ywjno/java-method-finder-rs.git
```

2. Build the project:

```bash
cargo build --release
```

The executable will be created in `target/release/jmf`

## Usage

Basic usage:

```bash
jmf -c com.example.TargetClass -m targetMethod
```

### Command-line Options

| Option          | Description                                                                  |
| --------------- | ---------------------------------------------------------------------------- |
| `-c, --class`   | The fully qualified name of the target class to find method calls (required) |
| `-m, --method`  | The name of the target method to find its invocations (required)             |
| `-s, --scan`    | The root directory to scan for class files (default: ./target/classes)       |
| `-f, --format`  | Output format: txt or json (default: txt)                                    |
| `-v, --verbose` | Enable verbose output for debugging                                          |
| `-h, --help`    | Show this help message and exit                                              |

### Examples

Find all calls to `targetMethod()` in `com.example.TargetClass`:

```bash
jmf -c com.example.TargetClass -m targetMethod
```

Scan a specific directory with JSON output:

```bash
jmf -c com.example.TargetClass -m targetMethod -s ./build/classes -f json
```

Enable verbose logging:

```bash
jmf -c com.example.TargetClass -m targetMethod -v
```

### Output Formats

#### Text Output (Default)

```
com.example.TargetClass#targetMethod
 - com.example.CallerClass#callerMethod (L123)
 - com.example.AnotherClass#someMethod (L45)
```

#### JSON Output

```json
{
  "target": "com.example.TargetClass#targetMethod",
  "calls": [
    {
      "class_name": "com.example.CallerClass",
      "method_name": "callerMethod",
      "line_number": 123
    },
    {
      "class_name": "com.example.AnotherClass",
      "method_name": "someMethod",
      "line_number": 45
    }
  ]
}
```

## License

This project is dual-licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
