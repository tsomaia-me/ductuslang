# semicolon

A semicolon outside strings and comments is an error. (SPEC §2.2)

A line break ends a line, so a semicolon would only repeat it.

```sym-error
let a = 1;
```

The line ends with `;`.

```sym
let a = 1
```
